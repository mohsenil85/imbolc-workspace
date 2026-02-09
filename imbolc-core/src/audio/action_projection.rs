//! Action projection: applies Action state mutations to the audio thread's local copies.
//!
//! Phase 2 of action-based audio sync. The audio thread receives forwarded Actions
//! and incrementally updates its local InstrumentState and SessionState copies,
//! avoiding full-state clones for steady-state operations.
//!
//! Each `project_*` function replicates the state mutation from the corresponding
//! dispatcher, but:
//! - Does NOT construct DispatchResult (no nav intents, no status events)
//! - Does NOT record automation
//! - Does NOT push undo snapshots
//! - Does NOT produce AudioSideEffect enums (those are handled by Phase 1)

use imbolc_types::{
    Action, InstrumentAction, MixerAction,
    InstrumentId, MixerSelection, FilterConfig, FilterType, OutputTarget, EqConfig,
};
use crate::state::{InstrumentState, SessionState};

/// Apply an action's state mutations to the audio thread's local copies.
/// Returns true if the action was handled (state was mutated or no-op).
/// Returns false if the action is not projectable (caller should use full sync).
pub fn project_action(
    action: &Action,
    instruments: &mut InstrumentState,
    session: &mut SessionState,
) -> bool {
    match action {
        // Actions that don't affect audio-thread state (no-op, handled)
        Action::None | Action::Quit | Action::SaveAndQuit
        | Action::Nav(_) | Action::Midi(_)
        | Action::ExitPerformanceMode | Action::PushLayer(_) | Action::PopLayer(_)
        | Action::Tuner(_) | Action::AudioFeedback(_) => true,

        // Undo/Redo: not projectable (wholesale state replacement)
        Action::Undo | Action::Redo => false,

        // Phase 2a: InstrumentAction and MixerAction projections
        Action::Instrument(a) => project_instrument(a, instruments, session),
        Action::Mixer(a) => project_mixer(a, instruments, session),

        // Phase 2b: remaining action types
        Action::PianoRoll(_) => false,
        Action::Automation(_) => false,
        Action::Bus(_) => false,
        Action::VstParam(_) => false,
        Action::Session(_) => false,
        Action::Click(_) => false,

        // Phase 2c: remaining action types
        Action::Arrangement(_) => false,
        Action::Sequencer(_) => false,
        Action::Chopper(_) => false,
        Action::Server(_) => false,
    }
}

// ============================================================================
// InstrumentAction projection
// ============================================================================

fn project_instrument(
    action: &InstrumentAction,
    instruments: &mut InstrumentState,
    session: &mut SessionState,
) -> bool {
    match action {
        InstrumentAction::Add(source_type) => {
            instruments.add_instrument(*source_type);
            true
        }
        InstrumentAction::Delete(id) => {
            instruments.remove_instrument(*id);
            session.piano_roll.remove_track(*id);
            session.automation.remove_lanes_for_instrument(*id);
            session.arrangement.remove_instrument_data(*id);
            true
        }
        InstrumentAction::Edit(id) => {
            instruments.editing_instrument_id = Some(*id);
            true
        }
        InstrumentAction::Update(update) => {
            if let Some(instrument) = instruments.instrument_mut(update.id) {
                instrument.source = update.source.clone();
                instrument.source_params = update.source_params.clone();
                instrument.filter = update.filter.clone();
                instrument.eq = update.eq.clone();
                instrument.effects = update.effects.clone();
                instrument.lfo = update.lfo.clone();
                instrument.amp_envelope = update.amp_envelope.clone();
                instrument.polyphonic = update.polyphonic;
                instrument.active = update.active;
            }
            true
        }
        InstrumentAction::AddEffect(id, effect_type) => {
            if let Some(instrument) = instruments.instrument_mut(*id) {
                instrument.add_effect(*effect_type);
            }
            true
        }
        InstrumentAction::RemoveEffect(id, effect_id) => {
            if let Some(instrument) = instruments.instrument_mut(*id) {
                instrument.remove_effect(*effect_id);
            }
            true
        }
        InstrumentAction::MoveEffect(id, effect_id, direction) => {
            if let Some(instrument) = instruments.instrument_mut(*id) {
                instrument.move_effect(*effect_id, *direction);
            }
            true
        }
        InstrumentAction::SetFilter(id, filter_type) => {
            if let Some(instrument) = instruments.instrument_mut(*id) {
                instrument.filter = filter_type.map(FilterConfig::new);
            }
            true
        }
        InstrumentAction::ToggleEffectBypass(id, effect_id) => {
            if let Some(instrument) = instruments.instrument_mut(*id) {
                if let Some(effect) = instrument.effects.iter_mut().find(|e| e.id == *effect_id) {
                    effect.enabled = !effect.enabled;
                }
            }
            true
        }
        InstrumentAction::ToggleFilter(id) => {
            if let Some(instrument) = instruments.instrument_mut(*id) {
                if instrument.filter.is_some() {
                    instrument.filter = None;
                } else {
                    instrument.filter = Some(FilterConfig::new(FilterType::Lpf));
                }
            }
            true
        }
        InstrumentAction::CycleFilterType(id) => {
            if let Some(instrument) = instruments.instrument_mut(*id) {
                if let Some(ref mut filter) = instrument.filter {
                    filter.filter_type = match filter.filter_type {
                        FilterType::Lpf => FilterType::Hpf,
                        FilterType::Hpf => FilterType::Bpf,
                        FilterType::Bpf => FilterType::Notch,
                        FilterType::Notch => FilterType::Comb,
                        FilterType::Comb => FilterType::Allpass,
                        FilterType::Allpass => FilterType::Vowel,
                        FilterType::Vowel => FilterType::ResDrive,
                        FilterType::ResDrive => FilterType::Lpf,
                    };
                    filter.extra_params = filter.filter_type.default_extra_params();
                }
            }
            true
        }
        InstrumentAction::AdjustFilterCutoff(id, delta) => {
            if let Some(instrument) = instruments.instrument_mut(*id) {
                if let Some(ref mut filter) = instrument.filter {
                    filter.cutoff.value = (filter.cutoff.value + delta * filter.cutoff.max * 0.02)
                        .clamp(filter.cutoff.min, filter.cutoff.max);
                }
            }
            true
        }
        InstrumentAction::AdjustFilterResonance(id, delta) => {
            if let Some(instrument) = instruments.instrument_mut(*id) {
                if let Some(ref mut filter) = instrument.filter {
                    filter.resonance.value = (filter.resonance.value + delta * 0.05)
                        .clamp(filter.resonance.min, filter.resonance.max);
                }
            }
            true
        }
        InstrumentAction::AdjustEffectParam(id, effect_id, param_idx, delta) => {
            if let Some(instrument) = instruments.instrument_mut(*id) {
                if let Some(effect) = instrument.effects.iter_mut().find(|e| e.id == *effect_id) {
                    if let Some(param) = effect.params.get_mut(*param_idx) {
                        use imbolc_types::ParamValue;
                        match &mut param.value {
                            ParamValue::Float(v) => {
                                let range = param.max - param.min;
                                *v = (*v + delta * range * 0.02).clamp(param.min, param.max);
                            }
                            ParamValue::Int(v) => {
                                let range = param.max - param.min;
                                *v = (*v + (delta * range * 0.02) as i32)
                                    .clamp(param.min as i32, param.max as i32);
                            }
                            ParamValue::Bool(v) => {
                                *v = !*v;
                            }
                        }
                    }
                }
            }
            true
        }
        // PlayNote/PlayNotes/PlayDrumPad: audio side effects only, no state mutation
        InstrumentAction::PlayNote(_, _)
        | InstrumentAction::PlayNotes(_, _)
        | InstrumentAction::PlayDrumPad(_) => true,

        InstrumentAction::Select(idx) => {
            if *idx < instruments.instruments.len() {
                instruments.selected = Some(*idx);
            }
            true
        }
        InstrumentAction::SelectNext => {
            instruments.select_next();
            true
        }
        InstrumentAction::SelectPrev => {
            instruments.select_prev();
            true
        }
        InstrumentAction::SelectFirst => {
            if !instruments.instruments.is_empty() {
                instruments.selected = Some(0);
            }
            true
        }
        InstrumentAction::SelectLast => {
            if !instruments.instruments.is_empty() {
                instruments.selected = Some(instruments.instruments.len() - 1);
            }
            true
        }
        InstrumentAction::LoadSampleResult(instrument_id, path) => {
            let buffer_id = instruments.next_sampler_buffer_id;
            instruments.next_sampler_buffer_id += 1;
            let sample_name = path.file_stem()
                .map(|s| s.to_string_lossy().to_string());
            if let Some(instrument) = instruments.instrument_mut(*instrument_id) {
                if let Some(ref mut config) = instrument.sampler_config {
                    config.buffer_id = Some(buffer_id);
                    config.sample_name = sample_name;
                }
            }
            true
        }
        InstrumentAction::ToggleArp(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.arpeggiator.enabled = !inst.arpeggiator.enabled;
            }
            true
        }
        InstrumentAction::CycleArpDirection(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.arpeggiator.direction = inst.arpeggiator.direction.next();
            }
            true
        }
        InstrumentAction::CycleArpRate(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.arpeggiator.rate = inst.arpeggiator.rate.next();
            }
            true
        }
        InstrumentAction::AdjustArpOctaves(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.arpeggiator.octaves = (inst.arpeggiator.octaves as i8 + delta)
                    .clamp(1, 4) as u8;
            }
            true
        }
        InstrumentAction::AdjustArpGate(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.arpeggiator.gate = (inst.arpeggiator.gate + delta).clamp(0.1, 1.0);
            }
            true
        }
        InstrumentAction::CycleChordShape(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.chord_shape = match inst.chord_shape {
                    None => Some(imbolc_types::ChordShape::Major),
                    Some(shape) => Some(shape.next()),
                };
            }
            true
        }
        InstrumentAction::ClearChordShape(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.chord_shape = None;
            }
            true
        }
        InstrumentAction::LoadIRResult(instrument_id, effect_id, path) => {
            let buffer_id = instruments.next_sampler_buffer_id;
            instruments.next_sampler_buffer_id += 1;
            if let Some(instrument) = instruments.instrument_mut(*instrument_id) {
                if let Some(effect) = instrument.effects.iter_mut().find(|e| e.id == *effect_id) {
                    if let Some(param) = effect.params.iter_mut().find(|p| p.name == "ir_buffer") {
                        param.value = imbolc_types::ParamValue::Int(buffer_id as i32);
                    }
                }
                instrument.convolution_ir_path = Some(path.to_string_lossy().to_string());
            }
            true
        }
        // OpenVstEffectParams: navigation only, no state mutation
        InstrumentAction::OpenVstEffectParams(_, _) => true,

        InstrumentAction::SetEqParam(instrument_id, band_idx, param_name, value) => {
            if let Some(instrument) = instruments.instrument_mut(*instrument_id) {
                if let Some(ref mut eq) = instrument.eq {
                    if let Some(band) = eq.bands.get_mut(*band_idx) {
                        match param_name.as_str() {
                            "freq" => band.freq = value.clamp(20.0, 20000.0),
                            "gain" => band.gain = value.clamp(-24.0, 24.0),
                            "q" => band.q = value.clamp(0.1, 10.0),
                            "on" => band.enabled = *value > 0.5,
                            _ => {}
                        }
                    }
                }
            }
            true
        }
        InstrumentAction::ToggleEq(instrument_id) => {
            if let Some(instrument) = instruments.instrument_mut(*instrument_id) {
                if instrument.eq.is_some() {
                    instrument.eq = None;
                } else {
                    instrument.eq = Some(EqConfig::default());
                }
            }
            true
        }
        InstrumentAction::LinkLayer(a, b) => {
            project_link_layer(instruments, session, *a, *b);
            true
        }
        InstrumentAction::UnlinkLayer(id) => {
            project_unlink_layer(instruments, session, *id);
            true
        }
        // Groove settings
        InstrumentAction::SetTrackSwing(id, value) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.groove.swing_amount = value.map(|v| v.clamp(0.0, 1.0));
            }
            true
        }
        InstrumentAction::SetTrackSwingGrid(id, grid) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.groove.swing_grid = *grid;
            }
            true
        }
        InstrumentAction::AdjustTrackSwing(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                let current = inst.groove.swing_amount
                    .unwrap_or(session.piano_roll.swing_amount);
                inst.groove.swing_amount = Some((current + delta).clamp(0.0, 1.0));
            }
            true
        }
        InstrumentAction::SetTrackHumanizeVelocity(id, value) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.groove.humanize_velocity = value.map(|v| v.clamp(0.0, 1.0));
            }
            true
        }
        InstrumentAction::AdjustTrackHumanizeVelocity(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                let current = inst.groove.humanize_velocity
                    .unwrap_or(session.humanize.velocity);
                inst.groove.humanize_velocity = Some((current + delta).clamp(0.0, 1.0));
            }
            true
        }
        InstrumentAction::SetTrackHumanizeTiming(id, value) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.groove.humanize_timing = value.map(|v| v.clamp(0.0, 1.0));
            }
            true
        }
        InstrumentAction::AdjustTrackHumanizeTiming(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                let current = inst.groove.humanize_timing
                    .unwrap_or(session.humanize.timing);
                inst.groove.humanize_timing = Some((current + delta).clamp(0.0, 1.0));
            }
            true
        }
        InstrumentAction::SetTrackTimingOffset(id, value) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.groove.timing_offset_ms = value.clamp(-50.0, 50.0);
            }
            true
        }
        InstrumentAction::AdjustTrackTimingOffset(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.groove.timing_offset_ms = (inst.groove.timing_offset_ms + delta)
                    .clamp(-50.0, 50.0);
            }
            true
        }
        InstrumentAction::ResetTrackGroove(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.groove.reset();
            }
            true
        }
        InstrumentAction::SetTrackTimeSignature(id, ts) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.groove.time_signature = *ts;
            }
            true
        }
        InstrumentAction::CycleTrackTimeSignature(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.groove.time_signature = match inst.groove.time_signature {
                    None => Some((4, 4)),
                    Some((4, 4)) => Some((3, 4)),
                    Some((3, 4)) => Some((5, 4)),
                    Some((5, 4)) => Some((6, 8)),
                    Some((6, 8)) => Some((7, 8)),
                    Some((7, 8)) => Some((12, 8)),
                    Some((12, 8)) => None,
                    Some(_) => Some((4, 4)),
                };
            }
            true
        }
        // LFO actions
        InstrumentAction::ToggleLfo(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.lfo.enabled = !inst.lfo.enabled;
            }
            true
        }
        InstrumentAction::AdjustLfoRate(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.lfo.rate = (inst.lfo.rate + delta * 0.5).clamp(0.1, 20.0);
            }
            true
        }
        InstrumentAction::AdjustLfoDepth(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.lfo.depth = (inst.lfo.depth + delta * 0.05).clamp(0.0, 1.0);
            }
            true
        }
        InstrumentAction::SetLfoShape(id, shape) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.lfo.shape = *shape;
            }
            true
        }
        InstrumentAction::SetLfoTarget(id, target) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.lfo.target = target.clone();
            }
            true
        }
        // Envelope actions
        InstrumentAction::AdjustEnvelopeAttack(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.amp_envelope.attack = (inst.amp_envelope.attack + delta * 0.1)
                    .clamp(0.001, 2.0);
            }
            true
        }
        InstrumentAction::AdjustEnvelopeDecay(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.amp_envelope.decay = (inst.amp_envelope.decay + delta * 0.1)
                    .clamp(0.001, 2.0);
            }
            true
        }
        InstrumentAction::AdjustEnvelopeSustain(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.amp_envelope.sustain = (inst.amp_envelope.sustain + delta * 0.05)
                    .clamp(0.0, 1.0);
            }
            true
        }
        InstrumentAction::AdjustEnvelopeRelease(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.amp_envelope.release = (inst.amp_envelope.release + delta * 0.2)
                    .clamp(0.001, 5.0);
            }
            true
        }
        // Channel config
        InstrumentAction::ToggleChannelConfig(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.channel_config = inst.channel_config.toggle();
            }
            true
        }
    }
}

// ============================================================================
// Instrument helpers
// ============================================================================

fn project_link_layer(
    instruments: &mut InstrumentState,
    session: &mut SessionState,
    a: InstrumentId,
    b: InstrumentId,
) {
    if a == b {
        return;
    }
    let group_b = instruments.instrument(b).and_then(|i| i.layer_group);
    let group_a = instruments.instrument(a).and_then(|i| i.layer_group);
    let group_id = match (group_a, group_b) {
        (_, Some(g)) => g,
        (Some(g), None) => g,
        (None, None) => instruments.next_layer_group(),
    };
    if let Some(inst) = instruments.instrument_mut(a) {
        inst.layer_group = Some(group_id);
    }
    if let Some(inst) = instruments.instrument_mut(b) {
        inst.layer_group = Some(group_id);
    }
    // Auto-create LayerGroupMixer if new group
    let bus_ids: Vec<u8> = session.mixer.bus_ids().collect();
    if session.mixer.layer_group_mixer(group_id).is_none() {
        session.mixer.add_layer_group_mixer(group_id, &bus_ids);
    }
}

fn project_unlink_layer(
    instruments: &mut InstrumentState,
    session: &mut SessionState,
    id: InstrumentId,
) {
    let old_group = instruments.instrument(id).and_then(|i| i.layer_group);
    if let Some(inst) = instruments.instrument_mut(id) {
        inst.layer_group = None;
    }
    if let Some(g) = old_group {
        let remaining: Vec<InstrumentId> = instruments.instruments.iter()
            .filter(|i| i.layer_group == Some(g))
            .map(|i| i.id)
            .collect();
        if remaining.len() <= 1 {
            if remaining.len() == 1 {
                if let Some(inst) = instruments.instrument_mut(remaining[0]) {
                    inst.layer_group = None;
                }
            }
            session.mixer.remove_layer_group_mixer(g);
        }
    }
}

// ============================================================================
// MixerAction projection
// ============================================================================

fn project_mixer(
    action: &MixerAction,
    instruments: &mut InstrumentState,
    session: &mut SessionState,
) -> bool {
    match action {
        MixerAction::Move(delta) => {
            // Replicate AppState::mixer_move logic
            session.mixer.selection = match session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    let new_idx = (idx as i32 + *delta as i32)
                        .clamp(0, instruments.instruments.len().saturating_sub(1) as i32)
                        as usize;
                    MixerSelection::Instrument(new_idx)
                }
                MixerSelection::LayerGroup(current_id) => {
                    let group_ids: Vec<u32> = session.mixer.layer_group_mixers.iter()
                        .map(|g| g.group_id).collect();
                    if group_ids.is_empty() {
                        return true;
                    }
                    let current_pos = group_ids.iter().position(|&id| id == current_id).unwrap_or(0);
                    let new_pos = (current_pos as i32 + *delta as i32)
                        .clamp(0, group_ids.len().saturating_sub(1) as i32) as usize;
                    MixerSelection::LayerGroup(group_ids[new_pos])
                }
                MixerSelection::Bus(current_id) => {
                    let bus_ids: Vec<u8> = session.bus_ids().collect();
                    if bus_ids.is_empty() {
                        return true;
                    }
                    let current_pos = bus_ids.iter().position(|&id| id == current_id).unwrap_or(0);
                    let new_pos = (current_pos as i32 + *delta as i32)
                        .clamp(0, bus_ids.len().saturating_sub(1) as i32) as usize;
                    MixerSelection::Bus(bus_ids[new_pos])
                }
                MixerSelection::Master => MixerSelection::Master,
            };
            if let MixerSelection::Instrument(idx) = session.mixer.selection {
                instruments.selected = Some(idx);
            }
            true
        }
        MixerAction::Jump(direction) => {
            // Replicate AppState::mixer_jump logic
            session.mixer.selection = match session.mixer.selection {
                MixerSelection::Instrument(_) => {
                    if *direction > 0 {
                        MixerSelection::Instrument(0)
                    } else {
                        MixerSelection::Instrument(instruments.instruments.len().saturating_sub(1))
                    }
                }
                MixerSelection::LayerGroup(_) => {
                    let group_ids: Vec<u32> = session.mixer.layer_group_mixers.iter()
                        .map(|g| g.group_id).collect();
                    if group_ids.is_empty() {
                        return true;
                    }
                    if *direction > 0 {
                        MixerSelection::LayerGroup(group_ids[0])
                    } else {
                        MixerSelection::LayerGroup(*group_ids.last().unwrap())
                    }
                }
                MixerSelection::Bus(_) => {
                    let bus_ids: Vec<u8> = session.bus_ids().collect();
                    if bus_ids.is_empty() {
                        return true;
                    }
                    if *direction > 0 {
                        MixerSelection::Bus(bus_ids[0])
                    } else {
                        MixerSelection::Bus(*bus_ids.last().unwrap())
                    }
                }
                MixerSelection::Master => MixerSelection::Master,
            };
            if let MixerSelection::Instrument(idx) = session.mixer.selection {
                instruments.selected = Some(idx);
            }
            true
        }
        MixerAction::SelectAt(selection) => {
            session.mixer.selection = *selection;
            if let MixerSelection::Instrument(idx) = *selection {
                instruments.selected = Some(idx);
            }
            true
        }
        MixerAction::AdjustLevel(delta) => {
            match session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(instrument) = instruments.instruments.get_mut(idx) {
                        instrument.level = (instrument.level + delta).clamp(0.0, 1.0);
                    }
                }
                MixerSelection::LayerGroup(group_id) => {
                    if let Some(gm) = session.mixer.layer_group_mixer_mut(group_id) {
                        gm.level = (gm.level + delta).clamp(0.0, 1.0);
                    }
                }
                MixerSelection::Bus(id) => {
                    if let Some(bus) = session.bus_mut(id) {
                        bus.level = (bus.level + delta).clamp(0.0, 1.0);
                    }
                }
                MixerSelection::Master => {
                    session.mixer.master_level = (session.mixer.master_level + delta).clamp(0.0, 1.0);
                }
            }
            true
        }
        MixerAction::ToggleMute => {
            match session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(instrument) = instruments.instruments.get_mut(idx) {
                        instrument.mute = !instrument.mute;
                    }
                }
                MixerSelection::LayerGroup(group_id) => {
                    if let Some(gm) = session.mixer.layer_group_mixer_mut(group_id) {
                        gm.mute = !gm.mute;
                    }
                }
                MixerSelection::Bus(id) => {
                    if let Some(bus) = session.bus_mut(id) {
                        bus.mute = !bus.mute;
                    }
                }
                MixerSelection::Master => {
                    session.mixer.master_mute = !session.mixer.master_mute;
                }
            }
            true
        }
        MixerAction::ToggleSolo => {
            match session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(instrument) = instruments.instruments.get_mut(idx) {
                        instrument.solo = !instrument.solo;
                    }
                }
                MixerSelection::LayerGroup(group_id) => {
                    if let Some(gm) = session.mixer.layer_group_mixer_mut(group_id) {
                        gm.solo = !gm.solo;
                    }
                }
                MixerSelection::Bus(id) => {
                    if let Some(bus) = session.bus_mut(id) {
                        bus.solo = !bus.solo;
                    }
                }
                MixerSelection::Master => {}
            }
            true
        }
        MixerAction::CycleSection => {
            session.mixer.cycle_section();
            // When cycling back to Instrument section, sync to global selection
            if let MixerSelection::Instrument(_) = session.mixer.selection {
                if let Some(idx) = instruments.selected {
                    session.mixer.selection = MixerSelection::Instrument(idx);
                }
            }
            true
        }
        MixerAction::CycleOutput => {
            // Replicate AppState::mixer_cycle_output logic
            let bus_ids: Vec<u8> = session.bus_ids().collect();
            match session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(inst) = instruments.instruments.get_mut(idx) {
                        inst.output_target = match inst.output_target {
                            OutputTarget::Master => {
                                bus_ids.first().map(|&id| OutputTarget::Bus(id))
                                    .unwrap_or(OutputTarget::Master)
                            }
                            OutputTarget::Bus(current_id) => {
                                let pos = bus_ids.iter().position(|&id| id == current_id);
                                match pos {
                                    Some(p) if p + 1 < bus_ids.len() => OutputTarget::Bus(bus_ids[p + 1]),
                                    _ => OutputTarget::Master,
                                }
                            }
                        };
                    }
                }
                MixerSelection::LayerGroup(group_id) => {
                    if let Some(gm) = session.mixer.layer_group_mixer_mut(group_id) {
                        gm.output_target = match gm.output_target {
                            OutputTarget::Master => {
                                bus_ids.first().map(|&id| OutputTarget::Bus(id))
                                    .unwrap_or(OutputTarget::Master)
                            }
                            OutputTarget::Bus(current_id) => {
                                let pos = bus_ids.iter().position(|&id| id == current_id);
                                match pos {
                                    Some(p) if p + 1 < bus_ids.len() => OutputTarget::Bus(bus_ids[p + 1]),
                                    _ => OutputTarget::Master,
                                }
                            }
                        };
                    }
                }
                _ => {}
            }
            true
        }
        MixerAction::CycleOutputReverse => {
            // Replicate AppState::mixer_cycle_output_reverse logic
            let bus_ids: Vec<u8> = session.bus_ids().collect();
            match session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(inst) = instruments.instruments.get_mut(idx) {
                        inst.output_target = match inst.output_target {
                            OutputTarget::Master => {
                                bus_ids.last().map(|&id| OutputTarget::Bus(id))
                                    .unwrap_or(OutputTarget::Master)
                            }
                            OutputTarget::Bus(current_id) => {
                                let pos = bus_ids.iter().position(|&id| id == current_id);
                                match pos {
                                    Some(0) | None => OutputTarget::Master,
                                    Some(p) => OutputTarget::Bus(bus_ids[p - 1]),
                                }
                            }
                        };
                    }
                }
                MixerSelection::LayerGroup(group_id) => {
                    if let Some(gm) = session.mixer.layer_group_mixer_mut(group_id) {
                        gm.output_target = match gm.output_target {
                            OutputTarget::Master => {
                                bus_ids.last().map(|&id| OutputTarget::Bus(id))
                                    .unwrap_or(OutputTarget::Master)
                            }
                            OutputTarget::Bus(current_id) => {
                                let pos = bus_ids.iter().position(|&id| id == current_id);
                                match pos {
                                    Some(0) | None => OutputTarget::Master,
                                    Some(p) => OutputTarget::Bus(bus_ids[p - 1]),
                                }
                            }
                        };
                    }
                }
                _ => {}
            }
            true
        }
        MixerAction::AdjustSend(bus_id, delta) => {
            match session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(instrument) = instruments.instruments.get_mut(idx) {
                        if let Some(send) = instrument.sends.iter_mut().find(|s| s.bus_id == *bus_id) {
                            send.level = (send.level + delta).clamp(0.0, 1.0);
                        }
                    }
                }
                MixerSelection::LayerGroup(group_id) => {
                    if let Some(gm) = session.mixer.layer_group_mixer_mut(group_id) {
                        if let Some(send) = gm.sends.iter_mut().find(|s| s.bus_id == *bus_id) {
                            send.level = (send.level + delta).clamp(0.0, 1.0);
                        }
                    }
                }
                _ => {}
            }
            true
        }
        MixerAction::ToggleSend(bus_id) => {
            match session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(instrument) = instruments.instruments.get_mut(idx) {
                        if let Some(send) = instrument.sends.iter_mut().find(|s| s.bus_id == *bus_id) {
                            send.enabled = !send.enabled;
                            if send.enabled && send.level <= 0.0 {
                                send.level = 0.5;
                            }
                        }
                    }
                }
                MixerSelection::LayerGroup(group_id) => {
                    if let Some(gm) = session.mixer.layer_group_mixer_mut(group_id) {
                        if let Some(send) = gm.sends.iter_mut().find(|s| s.bus_id == *bus_id) {
                            send.enabled = !send.enabled;
                            if send.enabled && send.level <= 0.0 {
                                send.level = 0.5;
                            }
                        }
                    }
                }
                _ => {}
            }
            true
        }
        MixerAction::CycleSendTapPoint(bus_id) => {
            match session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(instrument) = instruments.instruments.get_mut(idx) {
                        if let Some(send) = instrument.sends.iter_mut().find(|s| s.bus_id == *bus_id) {
                            send.tap_point = send.tap_point.cycle();
                        }
                    }
                }
                MixerSelection::LayerGroup(group_id) => {
                    if let Some(gm) = session.mixer.layer_group_mixer_mut(group_id) {
                        if let Some(send) = gm.sends.iter_mut().find(|s| s.bus_id == *bus_id) {
                            send.tap_point = send.tap_point.cycle();
                        }
                    }
                }
                _ => {}
            }
            true
        }
        MixerAction::AdjustPan(delta) => {
            match session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(instrument) = instruments.instruments.get_mut(idx) {
                        instrument.pan = (instrument.pan + delta).clamp(-1.0, 1.0);
                    }
                }
                MixerSelection::LayerGroup(group_id) => {
                    if let Some(gm) = session.mixer.layer_group_mixer_mut(group_id) {
                        gm.pan = (gm.pan + delta).clamp(-1.0, 1.0);
                    }
                }
                MixerSelection::Bus(id) => {
                    if let Some(bus) = session.bus_mut(id) {
                        bus.pan = (bus.pan + delta).clamp(-1.0, 1.0);
                    }
                }
                MixerSelection::Master => {}
            }
            true
        }
    }
}
