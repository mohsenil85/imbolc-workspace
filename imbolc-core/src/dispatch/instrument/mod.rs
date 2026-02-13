mod arpeggiator;
mod crud;
mod effects;
mod envelope;
mod eq;
mod filter;
mod groove;
mod layer;
mod lfo;
mod playback;
mod sample;
mod selection;

use crate::action::{AudioEffect, DispatchResult, InstrumentAction};
use crate::audio::AudioHandle;
use crate::state::AppState;

pub(super) fn dispatch_instrument(
    action: &InstrumentAction,
    state: &mut AppState,
    audio: &mut AudioHandle,
) -> DispatchResult {
    match action {
        InstrumentAction::Add(source_type) => crud::handle_add(state, *source_type),
        InstrumentAction::Delete(inst_id) => crud::handle_delete(state, audio, *inst_id),
        InstrumentAction::Edit(id) => crud::handle_edit(state, *id),
        InstrumentAction::Update(update) => crud::handle_update(state, update),
        InstrumentAction::PlayNote(pitch, velocity) => {
            playback::handle_play_note(state, audio, *pitch, *velocity)
        }
        InstrumentAction::PlayNotes(ref pitches, velocity) => {
            playback::handle_play_notes(state, audio, pitches, *velocity)
        }
        InstrumentAction::Select(_)
        | InstrumentAction::SelectNext
        | InstrumentAction::SelectPrev
        | InstrumentAction::SelectFirst
        | InstrumentAction::SelectLast => selection::handle_select(state, action),
        InstrumentAction::PlayDrumPad(pad_idx) => {
            playback::handle_play_drum_pad(state, audio, *pad_idx)
        }
        InstrumentAction::LoadSampleResult(instrument_id, ref path) => {
            sample::handle_load_sample_result(state, audio, *instrument_id, path)
        }
        InstrumentAction::AddEffect(id, ref effect_type) => {
            effects::handle_add_effect(state, *id, *effect_type)
        }
        InstrumentAction::RemoveEffect(id, effect_id) => {
            effects::handle_remove_effect(state, *id, *effect_id)
        }
        InstrumentAction::SetFilter(id, filter_type) => {
            filter::handle_set_filter(state, *id, *filter_type)
        }
        InstrumentAction::ToggleEffectBypass(id, effect_id) => {
            effects::handle_toggle_effect_bypass(state, *id, *effect_id)
        }
        InstrumentAction::ToggleFilter(id) => filter::handle_toggle_filter(state, *id),
        InstrumentAction::CycleFilterType(id) => filter::handle_cycle_filter_type(state, *id),
        InstrumentAction::AdjustFilterCutoff(id, delta) => {
            filter::handle_adjust_filter_cutoff(state, *id, *delta)
        }
        InstrumentAction::AdjustFilterResonance(id, delta) => {
            filter::handle_adjust_filter_resonance(state, *id, *delta)
        }
        InstrumentAction::AdjustEffectParam(id, effect_id, param_idx, delta) => {
            effects::handle_adjust_effect_param(state, *id, *effect_id, *param_idx, *delta)
        }
        InstrumentAction::ToggleArp(_)
        | InstrumentAction::CycleArpDirection(_)
        | InstrumentAction::CycleArpDirectionReverse(_)
        | InstrumentAction::CycleArpRate(_)
        | InstrumentAction::CycleArpRateReverse(_)
        | InstrumentAction::AdjustArpOctaves(_, _)
        | InstrumentAction::AdjustArpGate(_, _)
        | InstrumentAction::CycleChordShape(_)
        | InstrumentAction::CycleChordShapeReverse(_)
        | InstrumentAction::ClearChordShape(_) => arpeggiator::dispatch(state, action),
        InstrumentAction::LoadIRResult(instrument_id, effect_id, ref path) => {
            effects::handle_load_ir_result(state, audio, *instrument_id, *effect_id, path)
        }
        InstrumentAction::OpenVstEffectParams(instrument_id, effect_id) => {
            effects::handle_open_vst_effect_params(*instrument_id, *effect_id)
        }
        InstrumentAction::SetEqParam(instrument_id, band_idx, param, value) => {
            eq::handle_set_eq_param(state, audio, *instrument_id, *band_idx, *param, *value)
        }
        InstrumentAction::ToggleEq(instrument_id) => {
            eq::handle_toggle_eq(state, *instrument_id)
        }
        InstrumentAction::LinkLayer(a, b) => layer::handle_link_layer(state, *a, *b),
        InstrumentAction::UnlinkLayer(id) => layer::handle_unlink_layer(state, *id),
        InstrumentAction::AdjustLayerOctaveOffset(id, delta) => {
            layer::handle_adjust_layer_octave_offset(state, *id, *delta)
        }
        // Per-track groove settings
        InstrumentAction::SetTrackSwing(_, _)
        | InstrumentAction::SetTrackSwingGrid(_, _)
        | InstrumentAction::AdjustTrackSwing(_, _)
        | InstrumentAction::SetTrackHumanizeVelocity(_, _)
        | InstrumentAction::AdjustTrackHumanizeVelocity(_, _)
        | InstrumentAction::SetTrackHumanizeTiming(_, _)
        | InstrumentAction::AdjustTrackHumanizeTiming(_, _)
        | InstrumentAction::SetTrackTimingOffset(_, _)
        | InstrumentAction::AdjustTrackTimingOffset(_, _)
        | InstrumentAction::ResetTrackGroove(_)
        // Per-track time signature
        | InstrumentAction::SetTrackTimeSignature(_, _)
        | InstrumentAction::CycleTrackTimeSignature(_) => groove::dispatch(state, action),
        // LFO actions
        InstrumentAction::ToggleLfo(id) => lfo::handle_toggle_lfo(state, *id),
        InstrumentAction::AdjustLfoRate(id, delta) => {
            lfo::handle_adjust_lfo_rate(state, *id, *delta)
        }
        InstrumentAction::AdjustLfoDepth(id, delta) => {
            lfo::handle_adjust_lfo_depth(state, *id, *delta)
        }
        InstrumentAction::SetLfoShape(id, shape) => {
            lfo::handle_set_lfo_shape(state, *id, *shape)
        }
        InstrumentAction::SetLfoTarget(id, target) => {
            lfo::handle_set_lfo_target(state, *id, *target)
        }
        // Envelope actions
        InstrumentAction::AdjustEnvelopeAttack(id, delta) => {
            envelope::handle_adjust_envelope_attack(state, *id, *delta)
        }
        InstrumentAction::AdjustEnvelopeDecay(id, delta) => {
            envelope::handle_adjust_envelope_decay(state, *id, *delta)
        }
        InstrumentAction::AdjustEnvelopeSustain(id, delta) => {
            envelope::handle_adjust_envelope_sustain(state, *id, *delta)
        }
        InstrumentAction::AdjustEnvelopeRelease(id, delta) => {
            envelope::handle_adjust_envelope_release(state, *id, *delta)
        }
        // Channel config
        InstrumentAction::ToggleChannelConfig(id) => {
            handle_toggle_channel_config(state, *id)
        }
        // Processing chain reordering
        InstrumentAction::MoveStage(id, stage_idx, direction) => {
            handle_move_stage(state, *id, *stage_idx, *direction)
        }
    }
}

fn handle_move_stage(
    state: &mut AppState,
    id: crate::state::InstrumentId,
    stage_idx: usize,
    direction: i8,
) -> DispatchResult {
    imbolc_types::reduce::reduce_action(
        &imbolc_types::DomainAction::Instrument(InstrumentAction::MoveStage(
            id, stage_idx, direction,
        )),
        &mut state.instruments,
        &mut state.session,
    );
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
        .audio_effects
        .push(AudioEffect::RebuildRoutingForInstrument(id));
    result
}

fn handle_toggle_channel_config(
    state: &mut AppState,
    id: crate::state::InstrumentId,
) -> DispatchResult {
    imbolc_types::reduce::reduce_action(
        &imbolc_types::DomainAction::Instrument(InstrumentAction::ToggleChannelConfig(id)),
        &mut state.instruments,
        &mut state.session,
    );
    let mut result = DispatchResult::default();
    result
        .audio_effects
        .push(AudioEffect::RebuildRoutingForInstrument(id));
    result
}
