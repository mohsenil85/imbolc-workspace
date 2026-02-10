use std::collections::VecDeque;
use std::time::Instant;

use super::{InstrumentState, SessionState};
use crate::action::{
    Action, BusAction, InstrumentAction, MixerAction, SequencerAction, SessionAction,
    VstParamAction,
};
use imbolc_types::InstrumentId;
use super::instrument::Instrument;

/// What scope of state an undo entry covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoScope {
    /// Only one instrument changed (most common — parameter tweaks).
    SingleInstrument(InstrumentId),
    /// The instrument collection changed (add/remove would use Full instead,
    /// but this is available as a defensive fallback).
    Instruments,
    /// Only session state changed (piano roll, automation, mixer buses, etc.).
    Session,
    /// Both session and instruments changed (add/remove instrument, bus add/remove, etc.).
    Full,
}

/// Identifies a gesture for undo coalescing. Sequential actions with the same
/// key within `COALESCE_WINDOW` share a single undo snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoalesceKey {
    /// Parameter tweaks on the same instrument (filter, LFO, envelope, effects, etc.)
    InstrumentParam(InstrumentId),
    /// Session-level parameter tweaks (BPM, master level, humanize, etc.)
    SessionParam,
    /// No coalescing — structural changes always get their own snapshot.
    None,
}

/// Maximum time between coalesced actions (500ms).
const COALESCE_WINDOW: std::time::Duration = std::time::Duration::from_millis(500);

/// A single undo/redo entry storing only the state that was affected.
enum UndoEntry {
    SingleInstrument {
        id: InstrumentId,
        instrument: Box<Instrument>,
    },
    Instruments(Box<InstrumentState>),
    Session(Box<SessionState>),
    Full {
        session: Box<SessionState>,
        instruments: Box<InstrumentState>,
    },
}

pub struct UndoHistory {
    undo_stack: VecDeque<UndoEntry>,
    redo_stack: VecDeque<UndoEntry>,
    max_depth: usize,
    last_coalesce_key: CoalesceKey,
    last_push_time: Instant,
}

impl UndoHistory {
    pub fn new(max_depth: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_depth,
            last_coalesce_key: CoalesceKey::None,
            last_push_time: Instant::now(),
        }
    }

    /// Push a scoped snapshot before mutating state.
    pub fn push_scoped(
        &mut self,
        scope: UndoScope,
        session: &SessionState,
        instruments: &InstrumentState,
    ) {
        let entry = match scope {
            UndoScope::SingleInstrument(id) => {
                match instruments.instrument(id) {
                    Some(inst) => UndoEntry::SingleInstrument {
                        id,
                        instrument: Box::new(inst.clone()),
                    },
                    // Instrument not found — fall back to full instruments snapshot
                    None => UndoEntry::Instruments(Box::new(instruments.clone())),
                }
            }
            UndoScope::Instruments => {
                UndoEntry::Instruments(Box::new(instruments.clone()))
            }
            UndoScope::Session => {
                UndoEntry::Session(Box::new(session.clone()))
            }
            UndoScope::Full => UndoEntry::Full {
                session: Box::new(session.clone()),
                instruments: Box::new(instruments.clone()),
            },
        };

        if self.undo_stack.len() >= self.max_depth {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(entry);
        self.redo_stack.clear();
    }

    /// Push a snapshot from owned values (used by automation.rs when starting recording).
    pub fn push_from(&mut self, session: SessionState, instruments: InstrumentState) {
        if self.undo_stack.len() >= self.max_depth {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(UndoEntry::Full {
            session: Box::new(session),
            instruments: Box::new(instruments),
        });
        self.redo_stack.clear();
    }

    /// Push a scoped snapshot with coalescing support. If `key` matches the
    /// previous push's key and less than `COALESCE_WINDOW` has elapsed, the
    /// push is skipped — keeping the pre-gesture snapshot already on the stack.
    pub fn push_coalesced(
        &mut self,
        scope: UndoScope,
        session: &SessionState,
        instruments: &InstrumentState,
        key: CoalesceKey,
    ) {
        let now = Instant::now();
        if key != CoalesceKey::None
            && key == self.last_coalesce_key
            && now.duration_since(self.last_push_time) < COALESCE_WINDOW
        {
            // Same gesture, within window — skip the push to keep the
            // original pre-gesture snapshot on the stack.
            self.last_push_time = now;
            return;
        }
        self.push_scoped(scope, session, instruments);
        self.last_coalesce_key = key;
        self.last_push_time = now;
    }

    fn clear_coalesce(&mut self) {
        self.last_coalesce_key = CoalesceKey::None;
    }

    /// Undo: pop from undo stack, create inverse from current state, apply stored entry.
    /// Returns the scope of the undone entry, or `None` if nothing to undo.
    pub fn undo(
        &mut self,
        session: &mut SessionState,
        instruments: &mut InstrumentState,
    ) -> Option<UndoScope> {
        self.clear_coalesce();
        let entry = self.undo_stack.pop_back()?;
        let scope = entry_scope(&entry);
        let inverse = create_inverse(&entry, session, instruments);
        apply_entry(entry, session, instruments);
        self.redo_stack.push_back(inverse);
        Some(scope)
    }

    /// Redo: pop from redo stack, create inverse from current state, apply stored entry.
    /// Returns the scope of the redone entry, or `None` if nothing to redo.
    pub fn redo(
        &mut self,
        session: &mut SessionState,
        instruments: &mut InstrumentState,
    ) -> Option<UndoScope> {
        self.clear_coalesce();
        let entry = self.redo_stack.pop_back()?;
        let scope = entry_scope(&entry);
        let inverse = create_inverse(&entry, session, instruments);
        apply_entry(entry, session, instruments);
        self.undo_stack.push_back(inverse);
        Some(scope)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.clear_coalesce();
    }
}

/// Map an undo entry back to its scope.
fn entry_scope(entry: &UndoEntry) -> UndoScope {
    match entry {
        UndoEntry::SingleInstrument { id, .. } => UndoScope::SingleInstrument(*id),
        UndoEntry::Instruments(_) => UndoScope::Instruments,
        UndoEntry::Session(_) => UndoScope::Session,
        UndoEntry::Full { .. } => UndoScope::Full,
    }
}

/// Create an inverse entry by snapshotting the *current* state at the same scope.
/// If the entry is SingleInstrument but the instrument no longer exists,
/// defensively escalate to Instruments scope.
fn create_inverse(
    entry: &UndoEntry,
    session: &SessionState,
    instruments: &InstrumentState,
) -> UndoEntry {
    match entry {
        UndoEntry::SingleInstrument { id, .. } => {
            match instruments.instrument(*id) {
                Some(inst) => UndoEntry::SingleInstrument {
                    id: *id,
                    instrument: Box::new(inst.clone()),
                },
                // Instrument was deleted between push and undo — snapshot everything
                None => UndoEntry::Instruments(Box::new(instruments.clone())),
            }
        }
        UndoEntry::Instruments(_) => {
            UndoEntry::Instruments(Box::new(instruments.clone()))
        }
        UndoEntry::Session(_) => {
            UndoEntry::Session(Box::new(session.clone()))
        }
        UndoEntry::Full { .. } => UndoEntry::Full {
            session: Box::new(session.clone()),
            instruments: Box::new(instruments.clone()),
        },
    }
}

/// Apply a stored entry onto the live state.
fn apply_entry(
    entry: UndoEntry,
    session: &mut SessionState,
    instruments: &mut InstrumentState,
) {
    match entry {
        UndoEntry::SingleInstrument { id, instrument } => {
            if let Some(live) = instruments.instrument_mut(id) {
                *live = *instrument;
            }
            // If instrument not found (shouldn't happen due to Full escalation),
            // silently skip — the state is already consistent.
        }
        UndoEntry::Instruments(stored) => {
            *instruments = *stored;
            instruments.rebuild_index();
        }
        UndoEntry::Session(stored) => {
            *session = *stored;
        }
        UndoEntry::Full {
            session: s,
            instruments: i,
        } => {
            *session = *s;
            *instruments = *i;
            instruments.rebuild_index();
        }
    }
}

/// Determine the undo scope for an action based on what state it will touch.
/// `automation_recording` should be true only when automation is actively being
/// recorded during playback — this escalates param tweaks to Full scope so the
/// automation lane changes are also captured.
pub fn undo_scope(action: &Action, session: &SessionState, instruments: &InstrumentState, automation_recording: bool) -> UndoScope {
    let recording = automation_recording;

    match action {
        // Instrument add/delete always touch both state trees
        Action::Instrument(InstrumentAction::Add(_))
        | Action::Instrument(InstrumentAction::Delete(_)) => UndoScope::Full,

        // Layer link/unlink modifies instruments + session.mixer.layer_group_mixers
        Action::Instrument(InstrumentAction::LinkLayer(_, _))
        | Action::Instrument(InstrumentAction::UnlinkLayer(_)) => UndoScope::Full,

        // Instrument Update carries an explicit id
        Action::Instrument(InstrumentAction::Update(update)) => {
            if recording {
                UndoScope::Full
            } else {
                UndoScope::SingleInstrument(update.id)
            }
        }

        // Other instrument actions — use target_instrument_id()
        Action::Instrument(a) => {
            match a.target_instrument_id() {
                Some(id) => {
                    if recording {
                        UndoScope::Full
                    } else {
                        UndoScope::SingleInstrument(id)
                    }
                }
                // No target ID (shouldn't happen for undoable actions, but be safe)
                None => UndoScope::Full,
            }
        }

        // Mixer actions: depends on what's selected
        Action::Mixer(a) => {
            mixer_scope(a, session, instruments, recording)
        }

        // Session-only domains
        Action::PianoRoll(_)
        | Action::Automation(_)
        | Action::Arrangement(_)
        | Action::Session(_)
        | Action::Midi(_) => UndoScope::Session,

        // Bus add/remove syncs instrument sends — touches both trees
        Action::Bus(BusAction::Add | BusAction::Remove(_)) => UndoScope::Full,
        Action::Bus(BusAction::Rename(_, _)) => UndoScope::Session,

        // Sequencer/Chopper operate on the selected instrument's drum sequencer
        Action::Sequencer(_) | Action::Chopper(_) => {
            match instruments.selected_instrument() {
                Some(inst) => UndoScope::SingleInstrument(inst.id),
                None => UndoScope::Full,
            }
        }

        // VstParam carries InstrumentId as first field
        Action::VstParam(a) => {
            let id = match a {
                VstParamAction::SetParam(id, _, _, _)
                | VstParamAction::AdjustParam(id, _, _, _)
                | VstParamAction::ResetParam(id, _, _)
                | VstParamAction::DiscoverParams(id, _)
                | VstParamAction::SaveState(id, _) => *id,
            };
            if recording {
                UndoScope::Full
            } else {
                UndoScope::SingleInstrument(id)
            }
        }

        // Everything else (non-undoable actions shouldn't reach here, but be safe)
        _ => UndoScope::Full,
    }
}

/// Determine mixer action scope based on selection target.
fn mixer_scope(
    _action: &MixerAction,
    session: &SessionState,
    instruments: &InstrumentState,
    recording: bool,
) -> UndoScope {
    match session.mixer.selection {
        super::session::MixerSelection::Instrument(idx) => {
            match instruments.instruments.get(idx) {
                Some(inst) => {
                    if recording {
                        UndoScope::Full
                    } else {
                        UndoScope::SingleInstrument(inst.id)
                    }
                }
                None => UndoScope::Full,
            }
        }
        // Bus, LayerGroup, Master selections all live in SessionState
        super::session::MixerSelection::Bus(_)
        | super::session::MixerSelection::LayerGroup(_)
        | super::session::MixerSelection::Master => {
            if recording {
                UndoScope::Full
            } else {
                UndoScope::Session
            }
        }
    }
}

/// Map an action to a coalesce key. Actions with the same key that arrive
/// within `COALESCE_WINDOW` share a single undo snapshot.
pub fn coalesce_key(action: &Action, session: &SessionState, instruments: &InstrumentState) -> CoalesceKey {
    match action {
        // Instrument parameter tweaks — coalesce by instrument ID
        Action::Instrument(a) => match a {
            InstrumentAction::AdjustFilterCutoff(id, _)
            | InstrumentAction::AdjustFilterResonance(id, _)
            | InstrumentAction::AdjustEffectParam(id, _, _, _)
            | InstrumentAction::AdjustLfoRate(id, _)
            | InstrumentAction::AdjustLfoDepth(id, _)
            | InstrumentAction::AdjustEnvelopeAttack(id, _)
            | InstrumentAction::AdjustEnvelopeDecay(id, _)
            | InstrumentAction::AdjustEnvelopeSustain(id, _)
            | InstrumentAction::AdjustEnvelopeRelease(id, _)
            | InstrumentAction::AdjustArpOctaves(id, _)
            | InstrumentAction::AdjustArpGate(id, _)
            | InstrumentAction::AdjustLayerOctaveOffset(id, _)
            | InstrumentAction::AdjustTrackSwing(id, _)
            | InstrumentAction::AdjustTrackHumanizeVelocity(id, _)
            | InstrumentAction::AdjustTrackHumanizeTiming(id, _)
            | InstrumentAction::AdjustTrackTimingOffset(id, _) => {
                CoalesceKey::InstrumentParam(*id)
            }
            _ => CoalesceKey::None,
        },

        // Mixer level/pan/send — coalesce by mixer selection target
        Action::Mixer(a) => match a {
            MixerAction::AdjustLevel(_) | MixerAction::AdjustPan(_) | MixerAction::AdjustSend(_, _) => {
                match session.mixer.selection {
                    super::session::MixerSelection::Instrument(idx) => {
                        match instruments.instruments.get(idx) {
                            Some(inst) => CoalesceKey::InstrumentParam(inst.id),
                            None => CoalesceKey::None,
                        }
                    }
                    _ => CoalesceKey::SessionParam,
                }
            }
            _ => CoalesceKey::None,
        },

        // VST param tweaks
        Action::VstParam(a) => match a {
            VstParamAction::SetParam(id, _, _, _)
            | VstParamAction::AdjustParam(id, _, _, _) => CoalesceKey::InstrumentParam(*id),
            _ => CoalesceKey::None,
        },

        // Sequencer continuous adjustments — operate on selected instrument
        Action::Sequencer(a) => match a {
            SequencerAction::AdjustVelocity(_, _, _)
            | SequencerAction::AdjustPadLevel(_, _)
            | SequencerAction::AdjustSwing(_)
            | SequencerAction::AdjustProbability(_, _, _)
            | SequencerAction::AdjustPadPitch(_, _)
            | SequencerAction::AdjustStepPitch(_, _, _) => {
                match instruments.selected_instrument() {
                    Some(inst) => CoalesceKey::InstrumentParam(inst.id),
                    None => CoalesceKey::None,
                }
            }
            _ => CoalesceKey::None,
        },

        // Session-level adjustments
        Action::Session(a) => match a {
            SessionAction::AdjustHumanizeVelocity(_)
            | SessionAction::AdjustHumanizeTiming(_) => CoalesceKey::SessionParam,
            _ => CoalesceKey::None,
        },

        // Everything else — no coalescing
        _ => CoalesceKey::None,
    }
}

pub fn is_undoable(action: &Action) -> bool {
    match action {
        Action::Instrument(a) => match a {
            crate::action::InstrumentAction::PlayNote(_, _)
            | crate::action::InstrumentAction::PlayNotes(_, _)
            | crate::action::InstrumentAction::PlayDrumPad(_)
            | crate::action::InstrumentAction::Select(_)
            | crate::action::InstrumentAction::SelectNext
            | crate::action::InstrumentAction::SelectPrev
            | crate::action::InstrumentAction::SelectFirst
            | crate::action::InstrumentAction::SelectLast
            | crate::action::InstrumentAction::Edit(_)
            | crate::action::InstrumentAction::OpenVstEffectParams(_, _) => false,
            _ => true,
        },
        Action::Mixer(a) => match a {
            crate::action::MixerAction::Move(_)
            | crate::action::MixerAction::Jump(_)
            | crate::action::MixerAction::SelectAt(_)
            | crate::action::MixerAction::CycleSection => false,
            _ => true,
        },
        Action::PianoRoll(a) => match a {
            crate::action::PianoRollAction::ToggleNote { .. }
            | crate::action::PianoRollAction::ToggleLoop
            | crate::action::PianoRollAction::SetLoopStart(_)
            | crate::action::PianoRollAction::SetLoopEnd(_)
            | crate::action::PianoRollAction::CycleTimeSig
            | crate::action::PianoRollAction::TogglePolyMode(_)
            | crate::action::PianoRollAction::AdjustSwing(_)
            | crate::action::PianoRollAction::DeleteNotesInRegion { .. }
            | crate::action::PianoRollAction::PasteNotes { .. } => true,
            crate::action::PianoRollAction::CopyNotes { .. } => false,
            _ => false,
        },
        Action::Session(a) => match a {
            crate::action::SessionAction::Save
            | crate::action::SessionAction::SaveAs(_)
            | crate::action::SessionAction::Load
            | crate::action::SessionAction::LoadFrom(_)
            | crate::action::SessionAction::NewProject
            | crate::action::SessionAction::OpenFileBrowser(_) => false,
            _ => true,
        },
        Action::Sequencer(a) => match a {
            crate::action::SequencerAction::PlayStop
            | crate::action::SequencerAction::LoadSample(_)
            | crate::action::SequencerAction::LoadSampleResult(_, _)
            | crate::action::SequencerAction::CopySteps { .. } => false,
            _ => true,
        },
        Action::Chopper(a) => match a {
            crate::action::ChopperAction::LoadSample
            | crate::action::ChopperAction::LoadSampleResult(_)
            | crate::action::ChopperAction::PreviewSlice
            | crate::action::ChopperAction::SelectSlice(_)
            | crate::action::ChopperAction::MoveCursor(_) => false,
            _ => true,
        },
        Action::Automation(a) => match a {
            crate::action::AutomationAction::SelectLane(_)
            | crate::action::AutomationAction::ToggleRecording
            | crate::action::AutomationAction::ToggleLaneArm(_)
            | crate::action::AutomationAction::ArmAllLanes
            | crate::action::AutomationAction::DisarmAllLanes
            | crate::action::AutomationAction::RecordValue(_, _)
            | crate::action::AutomationAction::CopyPoints(_, _, _) => false,
            _ => true,
        },
        Action::Midi(a) => match a {
            crate::action::MidiAction::ConnectPort(_)
            | crate::action::MidiAction::DisconnectPort => false,
            _ => true,
        },
        Action::Arrangement(a) => match a {
            crate::action::ArrangementAction::TogglePlayMode
            | crate::action::ArrangementAction::SelectPlacement(_)
            | crate::action::ArrangementAction::SelectLane(_)
            | crate::action::ArrangementAction::MoveCursor(_)
            | crate::action::ArrangementAction::ScrollView(_)
            | crate::action::ArrangementAction::PlayStop => false,
            _ => true,
        },
        Action::VstParam(a) => match a {
            crate::action::VstParamAction::SetParam(_, _, _, _)
            | crate::action::VstParamAction::AdjustParam(_, _, _, _)
            | crate::action::VstParamAction::ResetParam(_, _, _) => true,
            _ => false,
        },
        Action::Undo | Action::Redo => false,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::SessionState;
    use crate::state::InstrumentState;
    use imbolc_types::{BusId, SourceType};

    #[test]
    fn test_undo_push_pop() {
        let mut history = UndoHistory::new(5);
        let mut session = SessionState::new();
        let mut instruments = InstrumentState::new();

        assert!(!history.can_undo());

        history.push_scoped(UndoScope::Full, &session, &instruments);
        assert!(history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(history.undo_stack.len(), 1);

        let undone = history.undo(&mut session, &mut instruments);
        assert!(undone.is_some());
        assert!(!history.can_undo());
        assert!(history.can_redo());
    }

    #[test]
    fn test_redo() {
        let mut history = UndoHistory::new(5);
        let mut session = SessionState::new();
        let mut instruments = InstrumentState::new();

        // Initial state
        session.mixer.master_level = 1.0;
        history.push_scoped(UndoScope::Full, &session, &instruments);

        // Modify state
        session.mixer.master_level = 0.5;

        // Undo — should restore master_level to 1.0
        assert!(history.undo(&mut session, &mut instruments).is_some());
        assert_eq!(session.mixer.master_level, 1.0);

        // Redo — should restore master_level to 0.5
        assert!(history.redo(&mut session, &mut instruments).is_some());
        assert_eq!(session.mixer.master_level, 0.5);
    }

    #[test]
    fn test_max_depth() {
        let mut history = UndoHistory::new(2);
        let session = SessionState::new();
        let instruments = InstrumentState::new();

        history.push_scoped(UndoScope::Full, &session, &instruments);
        history.push_scoped(UndoScope::Full, &session, &instruments);
        history.push_scoped(UndoScope::Full, &session, &instruments);

        assert_eq!(history.undo_stack.len(), 2);
    }

    #[test]
    fn test_push_clears_redo() {
        let mut history = UndoHistory::new(5);
        let mut session = SessionState::new();
        let mut instruments = InstrumentState::new();

        history.push_scoped(UndoScope::Full, &session, &instruments);
        history.undo(&mut session, &mut instruments);
        assert!(history.can_redo());

        history.push_scoped(UndoScope::Full, &session, &instruments);
        assert!(!history.can_redo());
    }

    #[test]
    fn clear_empties_both_stacks() {
        let mut history = UndoHistory::new(5);
        let mut session = SessionState::new();
        let mut instruments = InstrumentState::new();

        history.push_scoped(UndoScope::Full, &session, &instruments);
        history.push_scoped(UndoScope::Full, &session, &instruments);
        history.undo(&mut session, &mut instruments);
        assert!(history.can_undo());
        assert!(history.can_redo());

        history.clear();
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn push_from_owned_works() {
        let mut history = UndoHistory::new(5);
        let session = SessionState::new();
        let instruments = InstrumentState::new();

        history.push_from(session.clone(), instruments.clone());
        assert!(history.can_undo());
        assert_eq!(history.undo_stack.len(), 1);
    }

    #[test]
    fn undo_empty_returns_none() {
        let mut history = UndoHistory::new(5);
        let mut session = SessionState::new();
        let mut instruments = InstrumentState::new();
        assert!(history.undo(&mut session, &mut instruments).is_none());
    }

    #[test]
    fn redo_empty_returns_none() {
        let mut history = UndoHistory::new(5);
        let mut session = SessionState::new();
        let mut instruments = InstrumentState::new();
        assert!(history.redo(&mut session, &mut instruments).is_none());
    }

    #[test]
    fn is_undoable_instrument_add() {
        let action = Action::Instrument(crate::action::InstrumentAction::Add(SourceType::Saw));
        assert!(is_undoable(&action));
    }

    #[test]
    fn is_undoable_select_is_not() {
        let action = Action::Instrument(crate::action::InstrumentAction::Select(0));
        assert!(!is_undoable(&action));
    }

    // --- New scope-aware tests ---

    #[test]
    fn test_single_instrument_scope() {
        let mut history = UndoHistory::new(10);
        let mut session = SessionState::new();
        let mut instruments = InstrumentState::new();

        let id1 = instruments.add_instrument(SourceType::Saw);
        let id2 = instruments.add_instrument(SourceType::Sin);

        // Snapshot instrument 1 before modifying
        history.push_scoped(
            UndoScope::SingleInstrument(id1),
            &session,
            &instruments,
        );

        // Modify instrument 1's level
        instruments.instrument_mut(id1).unwrap().mixer.level = 0.3;
        // Also modify instrument 2 (should NOT be reverted)
        instruments.instrument_mut(id2).unwrap().mixer.level = 0.7;

        // Undo should only revert instrument 1
        assert!(history.undo(&mut session, &mut instruments).is_some());
        // Instrument 1 reverted to default (0.8)
        assert!(
            (instruments.instrument(id1).unwrap().mixer.level - 0.8).abs() < f32::EPSILON,
            "instrument 1 level should be reverted to 0.8, got {}",
            instruments.instrument(id1).unwrap().mixer.level
        );
        // Instrument 2 unchanged
        assert!(
            (instruments.instrument(id2).unwrap().mixer.level - 0.7).abs() < f32::EPSILON,
            "instrument 2 level should remain 0.7, got {}",
            instruments.instrument(id2).unwrap().mixer.level
        );
    }

    #[test]
    fn test_session_scope() {
        let mut history = UndoHistory::new(10);
        let mut session = SessionState::new();
        let mut instruments = InstrumentState::new();

        let id1 = instruments.add_instrument(SourceType::Saw);

        // Snapshot session before modifying
        history.push_scoped(UndoScope::Session, &session, &instruments);

        // Modify session
        session.mixer.master_level = 0.3;
        // Also modify an instrument (should NOT be reverted)
        instruments.instrument_mut(id1).unwrap().mixer.level = 0.1;

        // Undo should only revert session
        assert!(history.undo(&mut session, &mut instruments).is_some());
        assert!(
            (session.mixer.master_level - 1.0).abs() < f32::EPSILON,
            "master_level should be reverted to 1.0, got {}",
            session.mixer.master_level
        );
        // Instrument unchanged
        assert!(
            (instruments.instrument(id1).unwrap().mixer.level - 0.1).abs() < f32::EPSILON,
            "instrument level should remain 0.1, got {}",
            instruments.instrument(id1).unwrap().mixer.level
        );
    }

    #[test]
    fn test_scope_classification() {
        let session = SessionState::new();
        let mut instruments = InstrumentState::new();
        let id1 = instruments.add_instrument(SourceType::Saw);

        // Instrument Add => Full
        let action = Action::Instrument(InstrumentAction::Add(SourceType::Saw));
        assert_eq!(undo_scope(&action, &session, &instruments, false), UndoScope::Full);

        // Instrument Delete => Full
        let action = Action::Instrument(InstrumentAction::Delete(id1));
        assert_eq!(undo_scope(&action, &session, &instruments, false), UndoScope::Full);

        // Instrument param tweak => SingleInstrument (no automation recording)
        let action = Action::Instrument(InstrumentAction::AdjustFilterCutoff(id1, 0.1));
        assert_eq!(
            undo_scope(&action, &session, &instruments, false),
            UndoScope::SingleInstrument(id1)
        );

        // Instrument param tweak => Full (automation recording active)
        assert_eq!(
            undo_scope(&action, &session, &instruments, true),
            UndoScope::Full
        );

        // PianoRoll => Session
        let action = Action::PianoRoll(crate::action::PianoRollAction::ToggleLoop);
        assert_eq!(undo_scope(&action, &session, &instruments, false), UndoScope::Session);

        // Bus Add => Full
        let action = Action::Bus(BusAction::Add);
        assert_eq!(undo_scope(&action, &session, &instruments, false), UndoScope::Full);

        // Bus Rename => Session
        let action = Action::Bus(BusAction::Rename(BusId::new(1), "Test".to_string()));
        assert_eq!(undo_scope(&action, &session, &instruments, false), UndoScope::Session);

        // Sequencer (with selected instrument) => SingleInstrument
        instruments.selected = Some(0);
        let action = Action::Sequencer(crate::action::SequencerAction::ToggleStep(0, 0));
        assert_eq!(
            undo_scope(&action, &session, &instruments, false),
            UndoScope::SingleInstrument(id1)
        );

        // VstParam => SingleInstrument
        let action = Action::VstParam(VstParamAction::SetParam(
            id1,
            crate::action::VstTarget::Source,
            0,
            0.5,
        ));
        assert_eq!(
            undo_scope(&action, &session, &instruments, false),
            UndoScope::SingleInstrument(id1)
        );
    }
}
