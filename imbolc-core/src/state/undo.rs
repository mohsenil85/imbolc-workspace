use super::{InstrumentState, SessionState};
use crate::action::Action;

#[derive(Clone)]
pub struct UndoSnapshot {
    pub session: SessionState,
    pub instruments: InstrumentState,
}

pub struct UndoHistory {
    undo_stack: Vec<UndoSnapshot>,
    redo_stack: Vec<UndoSnapshot>,
    max_depth: usize,
}

impl UndoHistory {
    pub fn new(max_depth: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth,
        }
    }

    pub fn push(&mut self, session: &SessionState, instruments: &InstrumentState) {
        if self.undo_stack.len() >= self.max_depth {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(UndoSnapshot {
            session: session.clone(),
            instruments: instruments.clone(),
        });
        self.redo_stack.clear();
    }

    /// Push a snapshot from owned values (avoids borrow conflicts when called from dispatch)
    pub fn push_from(&mut self, session: SessionState, instruments: InstrumentState) {
        if self.undo_stack.len() >= self.max_depth {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(UndoSnapshot {
            session,
            instruments,
        });
        self.redo_stack.clear();
    }

    pub fn undo(
        &mut self,
        current_session: &SessionState,
        current_instruments: &InstrumentState,
    ) -> Option<UndoSnapshot> {
        if let Some(snapshot) = self.undo_stack.pop() {
            self.redo_stack.push(UndoSnapshot {
                session: current_session.clone(),
                instruments: current_instruments.clone(),
            });
            Some(snapshot)
        } else {
            None
        }
    }

    pub fn redo(
        &mut self,
        current_session: &SessionState,
        current_instruments: &InstrumentState,
    ) -> Option<UndoSnapshot> {
        if let Some(snapshot) = self.redo_stack.pop() {
            self.undo_stack.push(UndoSnapshot {
                session: current_session.clone(),
                instruments: current_instruments.clone(),
            });
            Some(snapshot)
        } else {
            None
        }
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

    #[test]
    fn test_undo_push_pop() {
        let mut history = UndoHistory::new(5);
        let session = SessionState::new();
        let instruments = InstrumentState::new();

        assert!(!history.can_undo());

        history.push(&session, &instruments);
        assert!(history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(history.undo_stack.len(), 1);

        let snapshot = history.undo(&session, &instruments);
        assert!(snapshot.is_some());
        assert!(!history.can_undo());
        assert!(history.can_redo());
    }

    #[test]
    fn test_redo() {
        let mut history = UndoHistory::new(5);
        let mut session = SessionState::new();
        let instruments = InstrumentState::new();

        // Initial state
        session.mixer.master_level = 1.0;
        history.push(&session, &instruments);

        // Modified state
        let mut session2 = session.clone();
        session2.mixer.master_level = 0.5;
        
        // Undo to initial
        let snapshot = history.undo(&session2, &instruments).unwrap();
        assert_eq!(snapshot.session.mixer.master_level, 1.0);
        
        // Redo to modified
        let snapshot_redo = history.redo(&snapshot.session, &snapshot.instruments).unwrap();
        assert_eq!(snapshot_redo.session.mixer.master_level, 0.5);
    }
    
    #[test]
    fn test_max_depth() {
        let mut history = UndoHistory::new(2);
        let session = SessionState::new();
        let instruments = InstrumentState::new();

        history.push(&session, &instruments);
        history.push(&session, &instruments);
        history.push(&session, &instruments);

        assert_eq!(history.undo_stack.len(), 2);
    }
    
    #[test]
    fn test_push_clears_redo() {
        let mut history = UndoHistory::new(5);
        let session = SessionState::new();
        let instruments = InstrumentState::new();

        history.push(&session, &instruments);
        history.undo(&session, &instruments);
        assert!(history.can_redo());
        
        history.push(&session, &instruments);
        assert!(!history.can_redo());
    }
}
