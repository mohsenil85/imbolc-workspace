use crate::{
    BusId, EqParamKind, FilterType, InstrumentAction, InstrumentId, InstrumentState, Param,
    ParamValue, SessionState, SourceType,
};

pub(super) fn reduce(
    action: &InstrumentAction,
    instruments: &mut InstrumentState,
    session: &mut SessionState,
) -> bool {
    match action {
        InstrumentAction::Add(source_type) => {
            let id = instruments.add_instrument(*source_type);
            initialize_instrument_from_registries(id, *source_type, instruments, session);
            session.piano_roll.add_track(id);
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
                instrument.source = update.source;
                instrument.source_params = update.source_params.clone();
                instrument.processing_chain = update.processing_chain.clone();
                instrument.modulation.lfo = update.lfo.clone();
                instrument.modulation.amp_envelope = update.amp_envelope.clone();
                instrument.polyphonic = update.polyphonic;
                instrument.mixer.active = update.active;
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
        InstrumentAction::MoveStage(id, chain_idx, direction) => {
            if let Some(instrument) = instruments.instrument_mut(*id) {
                instrument.move_stage(*chain_idx, *direction);
            }
            true
        }
        InstrumentAction::SetFilter(id, filter_type) => {
            if let Some(instrument) = instruments.instrument_mut(*id) {
                instrument.set_filter(*filter_type);
            }
            true
        }
        InstrumentAction::ToggleEffectBypass(id, effect_id) => {
            if let Some(instrument) = instruments.instrument_mut(*id) {
                if let Some(effect) = instrument.effects_mut().find(|e| e.id == *effect_id) {
                    effect.enabled = !effect.enabled;
                }
            }
            true
        }
        InstrumentAction::ToggleFilter(id) => {
            if let Some(instrument) = instruments.instrument_mut(*id) {
                instrument.toggle_filter();
            }
            true
        }
        InstrumentAction::CycleFilterType(id) => {
            if let Some(instrument) = instruments.instrument_mut(*id) {
                if let Some(filter) = instrument.filter_mut() {
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
                if let Some(filter) = instrument.filter_mut() {
                    filter.cutoff.value = (filter.cutoff.value + delta * filter.cutoff.max * 0.02)
                        .clamp(filter.cutoff.min, filter.cutoff.max);
                }
            }
            true
        }
        InstrumentAction::AdjustFilterResonance(id, delta) => {
            if let Some(instrument) = instruments.instrument_mut(*id) {
                if let Some(filter) = instrument.filter_mut() {
                    filter.resonance.value = (filter.resonance.value + delta * 0.05)
                        .clamp(filter.resonance.min, filter.resonance.max);
                }
            }
            true
        }
        InstrumentAction::AdjustEffectParam(id, effect_id, param_idx, delta) => {
            if let Some(instrument) = instruments.instrument_mut(*id) {
                if let Some(effect) = instrument.effects_mut().find(|e| e.id == *effect_id) {
                    if let Some(param) = effect.params.get_mut(param_idx.get()) {
                        param.adjust_delta(*delta);
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
            let sample_name = path.file_stem().map(|s| s.to_string_lossy().to_string());
            if let Some(instrument) = instruments.instrument_mut(*instrument_id) {
                if let Some(config) = instrument.sampler_config_mut() {
                    config.buffer_id = Some(buffer_id);
                    config.sample_name = sample_name;
                }
            }
            true
        }
        InstrumentAction::ToggleArp(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.note_input.arpeggiator.enabled = !inst.note_input.arpeggiator.enabled;
            }
            true
        }
        InstrumentAction::CycleArpDirection(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.note_input.arpeggiator.direction =
                    inst.note_input.arpeggiator.direction.next();
            }
            true
        }
        InstrumentAction::CycleArpDirectionReverse(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.note_input.arpeggiator.direction =
                    inst.note_input.arpeggiator.direction.prev();
            }
            true
        }
        InstrumentAction::CycleArpRate(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.note_input.arpeggiator.rate = inst.note_input.arpeggiator.rate.next();
            }
            true
        }
        InstrumentAction::CycleArpRateReverse(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.note_input.arpeggiator.rate = inst.note_input.arpeggiator.rate.prev();
            }
            true
        }
        InstrumentAction::AdjustArpOctaves(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.note_input.arpeggiator.octaves =
                    (inst.note_input.arpeggiator.octaves as i8 + delta).clamp(1, 4) as u8;
            }
            true
        }
        InstrumentAction::AdjustArpGate(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.note_input.arpeggiator.gate =
                    (inst.note_input.arpeggiator.gate + delta).clamp(0.1, 1.0);
            }
            true
        }
        InstrumentAction::CycleChordShape(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.note_input.chord_shape = match inst.note_input.chord_shape {
                    None => Some(crate::ChordShape::Major),
                    Some(shape) => Some(shape.next()),
                };
            }
            true
        }
        InstrumentAction::CycleChordShapeReverse(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.note_input.chord_shape = match inst.note_input.chord_shape {
                    None => Some(crate::ChordShape::Octave),
                    Some(crate::ChordShape::Major) => None,
                    Some(shape) => Some(shape.prev()),
                };
            }
            true
        }
        InstrumentAction::ClearChordShape(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.note_input.chord_shape = None;
            }
            true
        }
        InstrumentAction::LoadIRResult(instrument_id, effect_id, path) => {
            let buffer_id = instruments.next_sampler_buffer_id;
            instruments.next_sampler_buffer_id += 1;
            if let Some(instrument) = instruments.instrument_mut(*instrument_id) {
                if let Some(effect) = instrument.effects_mut().find(|e| e.id == *effect_id) {
                    if let Some(param) = effect.params.iter_mut().find(|p| p.name == "ir_buffer") {
                        param.value = crate::ParamValue::Int(buffer_id as i32);
                    }
                }
                instrument.convolution_ir_path = Some(path.to_string_lossy().to_string());
            }
            true
        }
        // OpenVstEffectParams: navigation only, no state mutation
        InstrumentAction::OpenVstEffectParams(_, _) => true,

        InstrumentAction::SetEqParam(instrument_id, band_idx, param, value) => {
            if let Some(instrument) = instruments.instrument_mut(*instrument_id) {
                if let Some(eq) = instrument.eq_mut() {
                    if let Some(band) = eq.bands.get_mut(*band_idx) {
                        match param {
                            EqParamKind::Freq => band.freq = value.clamp(20.0, 20000.0),
                            EqParamKind::Gain => band.gain = value.clamp(-24.0, 24.0),
                            EqParamKind::Q => band.q = value.clamp(0.1, 10.0),
                            EqParamKind::Enabled => band.enabled = *value > 0.5,
                        }
                    }
                }
            }
            true
        }
        InstrumentAction::ToggleEq(instrument_id) => {
            if let Some(instrument) = instruments.instrument_mut(*instrument_id) {
                instrument.toggle_eq();
            }
            true
        }
        InstrumentAction::LinkLayer(a, b) => {
            reduce_link_layer(instruments, session, *a, *b);
            true
        }
        InstrumentAction::UnlinkLayer(id) => {
            reduce_unlink_layer(instruments, session, *id);
            true
        }
        InstrumentAction::AdjustLayerOctaveOffset(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.layer.octave_offset = (inst.layer.octave_offset + delta).clamp(-4, 4);
            }
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
                let current = inst
                    .groove
                    .swing_amount
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
                let current = inst
                    .groove
                    .humanize_velocity
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
                let current = inst
                    .groove
                    .humanize_timing
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
                inst.groove.timing_offset_ms =
                    (inst.groove.timing_offset_ms + delta).clamp(-50.0, 50.0);
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
                inst.modulation.lfo.enabled = !inst.modulation.lfo.enabled;
            }
            true
        }
        InstrumentAction::AdjustLfoRate(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.modulation.lfo.rate =
                    (inst.modulation.lfo.rate + delta * 0.5).clamp(0.1, 20.0);
            }
            true
        }
        InstrumentAction::AdjustLfoDepth(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.modulation.lfo.depth =
                    (inst.modulation.lfo.depth + delta * 0.05).clamp(0.0, 1.0);
            }
            true
        }
        InstrumentAction::SetLfoShape(id, shape) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.modulation.lfo.shape = *shape;
            }
            true
        }
        InstrumentAction::SetLfoTarget(id, target) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.modulation.lfo.target = *target;
            }
            true
        }
        // Envelope actions
        InstrumentAction::AdjustEnvelopeAttack(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.modulation.amp_envelope.attack =
                    (inst.modulation.amp_envelope.attack + delta * 0.1).clamp(0.001, 2.0);
            }
            true
        }
        InstrumentAction::AdjustEnvelopeDecay(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.modulation.amp_envelope.decay =
                    (inst.modulation.amp_envelope.decay + delta * 0.1).clamp(0.001, 2.0);
            }
            true
        }
        InstrumentAction::AdjustEnvelopeSustain(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.modulation.amp_envelope.sustain =
                    (inst.modulation.amp_envelope.sustain + delta * 0.05).clamp(0.0, 1.0);
            }
            true
        }
        InstrumentAction::AdjustEnvelopeRelease(id, delta) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.modulation.amp_envelope.release =
                    (inst.modulation.amp_envelope.release + delta * 0.2).clamp(0.001, 5.0);
            }
            true
        }
        // Channel config
        InstrumentAction::ToggleChannelConfig(id) => {
            if let Some(inst) = instruments.instrument_mut(*id) {
                inst.mixer.channel_config = inst.mixer.channel_config.toggle();
            }
            true
        }
    }
}

/// Initialize instrument name and source_params from Custom/VST registries.
/// Called by the reducer and by `AppState::add_instrument()` to keep them aligned.
pub fn initialize_instrument_from_registries(
    id: InstrumentId,
    source_type: SourceType,
    instruments: &mut InstrumentState,
    session: &SessionState,
) {
    if let SourceType::Custom(custom_id) = source_type {
        if let Some(synthdef) = session.custom_synthdefs.get(custom_id) {
            if let Some(inst) = instruments.instrument_mut(id) {
                inst.name = format!("{}-{}", synthdef.synthdef_name, id);
                inst.source_params = synthdef
                    .params
                    .iter()
                    .map(|p| Param {
                        name: p.name.clone(),
                        value: ParamValue::Float(p.default),
                        min: p.min,
                        max: p.max,
                    })
                    .collect();
            }
        }
    }
    if let SourceType::Vst(vst_id) = source_type {
        if let Some(plugin) = session.vst_plugins.get(vst_id) {
            if let Some(inst) = instruments.instrument_mut(id) {
                inst.name = format!("{}-{}", plugin.name.to_lowercase(), id);
                inst.source_params = plugin
                    .params
                    .iter()
                    .map(|p| Param {
                        name: p.name.clone(),
                        value: ParamValue::Float(p.default),
                        min: 0.0,
                        max: 1.0,
                    })
                    .collect();
            }
        }
    }
}

fn reduce_link_layer(
    instruments: &mut InstrumentState,
    session: &mut SessionState,
    a: InstrumentId,
    b: InstrumentId,
) {
    if a == b {
        return;
    }
    let group_b = instruments.instrument(b).and_then(|i| i.layer.group);
    let group_a = instruments.instrument(a).and_then(|i| i.layer.group);
    let group_id = match (group_a, group_b) {
        (_, Some(g)) => g,
        (Some(g), None) => g,
        (None, None) => instruments.next_layer_group(),
    };
    if let Some(inst) = instruments.instrument_mut(a) {
        inst.layer.group = Some(group_id);
    }
    if let Some(inst) = instruments.instrument_mut(b) {
        inst.layer.group = Some(group_id);
    }
    // Auto-create LayerGroupMixer if new group
    let bus_ids: Vec<BusId> = session.mixer.bus_ids().collect();
    if session.mixer.layer_group_mixer(group_id).is_none() {
        session.mixer.add_layer_group_mixer(group_id, &bus_ids);
    }
}

fn reduce_unlink_layer(
    instruments: &mut InstrumentState,
    session: &mut SessionState,
    id: InstrumentId,
) {
    let old_group = instruments.instrument(id).and_then(|i| i.layer.group);
    if let Some(inst) = instruments.instrument_mut(id) {
        inst.layer.group = None;
    }
    if let Some(g) = old_group {
        let remaining: Vec<InstrumentId> = instruments
            .instruments
            .iter()
            .filter(|i| i.layer.group == Some(g))
            .map(|i| i.id)
            .collect();
        if remaining.len() <= 1 {
            if remaining.len() == 1 {
                if let Some(inst) = instruments.instrument_mut(remaining[0]) {
                    inst.layer.group = None;
                }
            }
            session.mixer.remove_layer_group_mixer(g);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::custom_synthdef::{CustomSynthDef, ParamSpec};
    use crate::state::vst::{VstParamSpec, VstPlugin, VstPluginKind};
    use crate::{CustomSynthDefId, VstPluginId};
    use std::path::PathBuf;

    fn make_session_with_custom() -> SessionState {
        let mut session = SessionState::new();
        let synthdef = CustomSynthDef {
            id: CustomSynthDefId::new(0),
            name: "test".to_string(),
            synthdef_name: "imbolc_test".to_string(),
            source_path: PathBuf::from("/tmp/test.scd"),
            params: vec![
                ParamSpec {
                    name: "freq".to_string(),
                    default: 440.0,
                    min: 20.0,
                    max: 20000.0,
                },
                ParamSpec {
                    name: "amp".to_string(),
                    default: 0.5,
                    min: 0.0,
                    max: 1.0,
                },
            ],
        };
        session.custom_synthdefs.add(synthdef);
        session
    }

    fn make_session_with_vst() -> SessionState {
        let mut session = SessionState::new();
        let plugin = VstPlugin {
            id: VstPluginId::new(0),
            name: "TestSynth".to_string(),
            plugin_path: PathBuf::from("/tmp/test.vst3"),
            kind: VstPluginKind::Instrument,
            params: vec![
                VstParamSpec {
                    index: 0,
                    name: "cutoff".to_string(),
                    default: 0.7,
                    label: None,
                },
                VstParamSpec {
                    index: 1,
                    name: "resonance".to_string(),
                    default: 0.3,
                    label: None,
                },
            ],
        };
        session.vst_plugins.add(plugin);
        session
    }

    #[test]
    fn add_custom_initializes_params_from_registry() {
        let mut session = make_session_with_custom();
        let mut instruments = InstrumentState::new();
        let custom_id = CustomSynthDefId::new(0);

        reduce(
            &InstrumentAction::Add(SourceType::Custom(custom_id)),
            &mut instruments,
            &mut session,
        );

        let inst = &instruments.instruments[0];
        assert!(inst.name.starts_with("imbolc_test-"));
        assert_eq!(inst.source_params.len(), 2);
        assert_eq!(inst.source_params[0].name, "freq");
        assert_eq!(inst.source_params[0].min, 20.0);
        assert_eq!(inst.source_params[0].max, 20000.0);
        assert_eq!(inst.source_params[1].name, "amp");
    }

    #[test]
    fn add_vst_initializes_params_from_registry() {
        let mut session = make_session_with_vst();
        let mut instruments = InstrumentState::new();
        let vst_id = VstPluginId::new(0);

        reduce(
            &InstrumentAction::Add(SourceType::Vst(vst_id)),
            &mut instruments,
            &mut session,
        );

        let inst = &instruments.instruments[0];
        assert!(inst.name.starts_with("testsynth-"));
        assert_eq!(inst.source_params.len(), 2);
        assert_eq!(inst.source_params[0].name, "cutoff");
        assert_eq!(inst.source_params[0].min, 0.0);
        assert_eq!(inst.source_params[0].max, 1.0);
        assert_eq!(inst.source_params[1].name, "resonance");
    }

    #[test]
    fn add_basic_source_uses_default_params() {
        let mut session = SessionState::new();
        let mut instruments = InstrumentState::new();

        reduce(
            &InstrumentAction::Add(SourceType::Saw),
            &mut instruments,
            &mut session,
        );

        let inst = &instruments.instruments[0];
        // Saw gets default_params() from SourceType, not from registries
        assert_eq!(inst.source_params, SourceType::Saw.default_params());
    }

    #[test]
    fn initialize_helper_matches_reducer_for_custom() {
        let mut session = make_session_with_custom();
        let custom_id = CustomSynthDefId::new(0);

        // Path A: via reducer
        let mut instruments_a = InstrumentState::new();
        reduce(
            &InstrumentAction::Add(SourceType::Custom(custom_id)),
            &mut instruments_a,
            &mut session,
        );

        // Path B: via helper directly
        let mut instruments_b = InstrumentState::new();
        let id = instruments_b.add_instrument(SourceType::Custom(custom_id));
        initialize_instrument_from_registries(
            id,
            SourceType::Custom(custom_id),
            &mut instruments_b,
            &session,
        );

        let a = &instruments_a.instruments[0];
        let b = &instruments_b.instruments[0];
        assert_eq!(a.name, b.name);
        assert_eq!(a.source_params.len(), b.source_params.len());
        for (pa, pb) in a.source_params.iter().zip(b.source_params.iter()) {
            assert_eq!(pa.name, pb.name);
            assert_eq!(pa.min, pb.min);
            assert_eq!(pa.max, pb.max);
        }
    }

    #[test]
    fn initialize_helper_matches_reducer_for_vst() {
        let mut session = make_session_with_vst();
        let vst_id = VstPluginId::new(0);

        // Path A: via reducer
        let mut instruments_a = InstrumentState::new();
        reduce(
            &InstrumentAction::Add(SourceType::Vst(vst_id)),
            &mut instruments_a,
            &mut session,
        );

        // Path B: via helper directly
        let mut instruments_b = InstrumentState::new();
        let id = instruments_b.add_instrument(SourceType::Vst(vst_id));
        initialize_instrument_from_registries(
            id,
            SourceType::Vst(vst_id),
            &mut instruments_b,
            &session,
        );

        let a = &instruments_a.instruments[0];
        let b = &instruments_b.instruments[0];
        assert_eq!(a.name, b.name);
        assert_eq!(a.source_params.len(), b.source_params.len());
        for (pa, pb) in a.source_params.iter().zip(b.source_params.iter()) {
            assert_eq!(pa.name, pb.name);
            assert_eq!(pa.min, pb.min);
            assert_eq!(pa.max, pb.max);
        }
    }
}
