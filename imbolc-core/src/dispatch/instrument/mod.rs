mod arpeggiator;
mod crud;
mod effects;
mod eq;
mod filter;
mod groove;
mod layer;
mod playback;
mod sample;
mod selection;

use crate::audio::AudioHandle;
use crate::state::AppState;
use crate::action::{DispatchResult, InstrumentAction};

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
        InstrumentAction::Select(idx) => selection::handle_select(state, *idx),
        InstrumentAction::SelectNext => selection::handle_select_next(state),
        InstrumentAction::SelectPrev => selection::handle_select_prev(state),
        InstrumentAction::SelectFirst => selection::handle_select_first(state),
        InstrumentAction::SelectLast => selection::handle_select_last(state),
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
        InstrumentAction::MoveEffect(id, effect_id, direction) => {
            effects::handle_move_effect(state, *id, *effect_id, *direction)
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
        InstrumentAction::ToggleArp(id) => arpeggiator::handle_toggle_arp(state, *id),
        InstrumentAction::CycleArpDirection(id) => arpeggiator::handle_cycle_arp_direction(state, *id),
        InstrumentAction::CycleArpRate(id) => arpeggiator::handle_cycle_arp_rate(state, *id),
        InstrumentAction::AdjustArpOctaves(id, delta) => {
            arpeggiator::handle_adjust_arp_octaves(state, *id, *delta)
        }
        InstrumentAction::AdjustArpGate(id, delta) => {
            arpeggiator::handle_adjust_arp_gate(state, *id, *delta)
        }
        InstrumentAction::CycleChordShape(id) => arpeggiator::handle_cycle_chord_shape(state, *id),
        InstrumentAction::ClearChordShape(id) => arpeggiator::handle_clear_chord_shape(state, *id),
        InstrumentAction::LoadIRResult(instrument_id, effect_id, ref path) => {
            effects::handle_load_ir_result(state, audio, *instrument_id, *effect_id, path)
        }
        InstrumentAction::OpenVstEffectParams(instrument_id, effect_id) => {
            effects::handle_open_vst_effect_params(*instrument_id, *effect_id)
        }
        InstrumentAction::SetEqParam(instrument_id, band_idx, ref param_name, value) => {
            eq::handle_set_eq_param(state, audio, *instrument_id, *band_idx, param_name, *value)
        }
        InstrumentAction::ToggleEq(instrument_id) => {
            eq::handle_toggle_eq(state, *instrument_id)
        }
        InstrumentAction::LinkLayer(a, b) => layer::handle_link_layer(state, *a, *b),
        InstrumentAction::UnlinkLayer(id) => layer::handle_unlink_layer(state, *id),
        // Per-track groove settings
        InstrumentAction::SetTrackSwing(id, value) => {
            groove::handle_set_track_swing(state, *id, *value)
        }
        InstrumentAction::SetTrackSwingGrid(id, grid) => {
            groove::handle_set_track_swing_grid(state, *id, *grid)
        }
        InstrumentAction::AdjustTrackSwing(id, delta) => {
            groove::handle_adjust_track_swing(state, *id, *delta)
        }
        InstrumentAction::SetTrackHumanizeVelocity(id, value) => {
            groove::handle_set_track_humanize_velocity(state, *id, *value)
        }
        InstrumentAction::AdjustTrackHumanizeVelocity(id, delta) => {
            groove::handle_adjust_track_humanize_velocity(state, *id, *delta)
        }
        InstrumentAction::SetTrackHumanizeTiming(id, value) => {
            groove::handle_set_track_humanize_timing(state, *id, *value)
        }
        InstrumentAction::AdjustTrackHumanizeTiming(id, delta) => {
            groove::handle_adjust_track_humanize_timing(state, *id, *delta)
        }
        InstrumentAction::SetTrackTimingOffset(id, value) => {
            groove::handle_set_track_timing_offset(state, *id, *value)
        }
        InstrumentAction::AdjustTrackTimingOffset(id, delta) => {
            groove::handle_adjust_track_timing_offset(state, *id, *delta)
        }
        InstrumentAction::ResetTrackGroove(id) => {
            groove::handle_reset_track_groove(state, *id)
        }
    }
}
