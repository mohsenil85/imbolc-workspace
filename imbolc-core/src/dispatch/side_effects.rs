//! AudioSideEffect: describes audio operations produced by dispatchers.
//!
//! Dispatch functions push side effects into a `Vec<AudioSideEffect>` instead of
//! calling `AudioHandle` methods directly. This decouples state mutation from
//! audio I/O, enabling future event-log architecture.

use std::path::PathBuf;

use imbolc_audio::AudioHandle;
use imbolc_types::BusId;
use crate::state::automation::AutomationTarget;
use crate::state::{BufferId, InstrumentId};
use crate::action::VstTarget;

/// A deferred audio operation produced during action dispatch.
///
/// Variants map 1:1 to `AudioHandle` methods called from dispatchers.
/// The top-level `dispatch_with_audio` collects these and applies them
/// after dispatch returns.
#[derive(Debug, Clone)]
pub enum AudioSideEffect {
    // ── Voice management ──
    SpawnVoice {
        instrument_id: InstrumentId,
        pitch: u8,
        velocity: f32,
        offset_secs: f64,
    },
    ReleaseVoice {
        instrument_id: InstrumentId,
        pitch: u8,
        offset_secs: f64,
    },
    ReleaseAllVoices,
    PushActiveNote {
        instrument_id: InstrumentId,
        pitch: u8,
        duration_ticks: u32,
    },
    ClearActiveNotes,

    // ── Transport ──
    SetPlaying {
        playing: bool,
    },
    ResetPlayhead,

    // ── Sample management ──
    LoadSample {
        buffer_id: BufferId,
        path: String,
    },
    FreeSamples {
        buffer_ids: Vec<BufferId>,
    },

    // ── Mixer ──
    SetBusMixerParams {
        bus_id: BusId,
        level: f32,
        mute: bool,
        pan: f32,
    },
    SetLayerGroupMixerParams {
        group_id: u32,
        level: f32,
        mute: bool,
        pan: f32,
    },

    // ── Click track ──
    SetClickEnabled {
        enabled: bool,
    },
    SetClickMuted {
        muted: bool,
    },
    SetClickVolume {
        volume: f32,
    },

    // ── Tuner ──
    StartTunerTone {
        freq: f32,
    },
    StopTunerTone,

    // ── Drum / sampler ──
    PlayDrumHit {
        buffer_id: BufferId,
        amp: f32,
        instrument_id: InstrumentId,
        slice_start: f32,
        slice_end: f32,
        rate: f32,
        offset_secs: f64,
    },

    // ── Automation ──
    ApplyAutomation {
        target: AutomationTarget,
        value: f32,
    },

    // ── EQ param ──
    SetEqParam {
        instrument_id: InstrumentId,
        param: String,
        value: f32,
    },
    SetLayerGroupEqParam {
        group_id: u32,
        param: String,
        value: f32,
    },

    // ── Server lifecycle ──
    UpdateState,
    Connect {
        server_addr: String,
    },
    Disconnect,
    StartServer {
        input_device: Option<String>,
        output_device: Option<String>,
        buffer_size: u32,
        sample_rate: u32,
    },
    StopServer,
    RestartServer {
        input_device: Option<String>,
        output_device: Option<String>,
        server_addr: String,
        buffer_size: u32,
        sample_rate: u32,
    },
    CompileSynthDefs {
        scd_path: PathBuf,
    },
    LoadSynthDefs {
        dir: PathBuf,
    },

    // ── Recording ──
    StartRecording {
        bus: i32,
        path: PathBuf,
    },
    StopRecording,
    StartInstrumentRender {
        instrument_id: InstrumentId,
        path: PathBuf,
    },
    StartMasterBounce {
        path: PathBuf,
    },
    StartStemExport {
        stems: Vec<(InstrumentId, PathBuf)>,
    },
    CancelExport,

    // ── VST ──
    SetVstParam {
        instrument_id: InstrumentId,
        target: VstTarget,
        param_index: u32,
        value: f32,
    },
    QueryVstParams {
        instrument_id: InstrumentId,
        target: VstTarget,
    },
    SaveVstState {
        instrument_id: InstrumentId,
        target: VstTarget,
        path: std::path::PathBuf,
    },
}

/// Apply collected side effects to the audio handle.
///
/// Called by `LocalDispatcher::dispatch_with_audio` after `dispatch_action` returns.
/// Effects are applied in order.
pub fn apply_side_effects(effects: &[AudioSideEffect], audio: &mut AudioHandle) {
    for effect in effects {
        apply_one(effect, audio);
    }
}

fn apply_one(effect: &AudioSideEffect, audio: &mut AudioHandle) {
    match effect {
        // Voice management
        AudioSideEffect::SpawnVoice { instrument_id, pitch, velocity, offset_secs } => {
            let _ = audio.spawn_voice(*instrument_id, *pitch, *velocity, *offset_secs);
        }
        AudioSideEffect::ReleaseVoice { instrument_id, pitch, offset_secs } => {
            let _ = audio.release_voice(*instrument_id, *pitch, *offset_secs);
        }
        AudioSideEffect::ReleaseAllVoices => {
            audio.release_all_voices();
        }
        AudioSideEffect::PushActiveNote { instrument_id, pitch, duration_ticks } => {
            audio.push_active_note(*instrument_id, *pitch, *duration_ticks);
        }
        AudioSideEffect::ClearActiveNotes => {
            audio.clear_active_notes();
        }

        // Transport
        AudioSideEffect::SetPlaying { playing } => {
            audio.set_playing(*playing);
        }
        AudioSideEffect::ResetPlayhead => {
            audio.reset_playhead();
        }

        // Sample management
        AudioSideEffect::LoadSample { buffer_id, path } => {
            let _ = audio.load_sample(*buffer_id, path);
        }
        AudioSideEffect::FreeSamples { buffer_ids } => {
            audio.free_samples(buffer_ids.clone());
        }

        // Mixer
        AudioSideEffect::SetBusMixerParams { bus_id, level, mute, pan } => {
            let _ = audio.set_bus_mixer_params(*bus_id, *level, *mute, *pan);
        }
        AudioSideEffect::SetLayerGroupMixerParams { group_id, level, mute, pan } => {
            let _ = audio.set_layer_group_mixer_params(*group_id, *level, *mute, *pan);
        }

        // Click track
        AudioSideEffect::SetClickEnabled { enabled } => {
            let _ = audio.set_click_enabled(*enabled);
        }
        AudioSideEffect::SetClickMuted { muted } => {
            let _ = audio.set_click_muted(*muted);
        }
        AudioSideEffect::SetClickVolume { volume } => {
            let _ = audio.set_click_volume(*volume);
        }

        // Tuner
        AudioSideEffect::StartTunerTone { freq } => {
            audio.start_tuner_tone(*freq);
        }
        AudioSideEffect::StopTunerTone => {
            audio.stop_tuner_tone();
        }

        // Drum
        AudioSideEffect::PlayDrumHit { buffer_id, amp, instrument_id, slice_start, slice_end, rate, offset_secs } => {
            let _ = audio.play_drum_hit_to_instrument(*buffer_id, *amp, *instrument_id, *slice_start, *slice_end, *rate, *offset_secs);
        }

        // Automation
        AudioSideEffect::ApplyAutomation { target, value } => {
            let _ = audio.apply_automation(target, *value);
        }

        // EQ
        AudioSideEffect::SetEqParam { instrument_id, param, value } => {
            let _ = audio.set_eq_param(*instrument_id, param, *value);
        }
        AudioSideEffect::SetLayerGroupEqParam { group_id, param, value } => {
            let _ = audio.set_layer_group_eq_param(*group_id, param, *value);
        }

        // Server lifecycle
        AudioSideEffect::UpdateState => {
            // No-op — state sync is handled by the event log + apply_dirty.
            // Variant kept for server.rs side effect enumeration.
        }
        AudioSideEffect::Connect { server_addr } => {
            let _ = audio.connect_async(server_addr);
        }
        AudioSideEffect::Disconnect => {
            let _ = audio.disconnect_async();
        }
        AudioSideEffect::StartServer { input_device, output_device, buffer_size, sample_rate } => {
            let _ = audio.start_server_async(
                input_device.as_deref(),
                output_device.as_deref(),
                *buffer_size,
                *sample_rate,
            );
        }
        AudioSideEffect::StopServer => {
            let _ = audio.stop_server_async();
        }
        AudioSideEffect::RestartServer { input_device, output_device, server_addr, buffer_size, sample_rate } => {
            let _ = audio.restart_server_async(
                input_device.as_deref(),
                output_device.as_deref(),
                server_addr,
                *buffer_size,
                *sample_rate,
            );
        }
        AudioSideEffect::CompileSynthDefs { scd_path } => {
            let _ = audio.compile_synthdefs_async(scd_path);
        }
        AudioSideEffect::LoadSynthDefs { dir } => {
            let _ = audio.load_synthdefs(dir);
        }

        // Recording
        AudioSideEffect::StartRecording { bus, path } => {
            let _ = audio.start_recording(*bus, path);
        }
        AudioSideEffect::StopRecording => {
            let _ = audio.stop_recording();
        }
        AudioSideEffect::StartInstrumentRender { instrument_id, path } => {
            let _ = audio.start_instrument_render(*instrument_id, path);
        }
        AudioSideEffect::StartMasterBounce { path } => {
            let _ = audio.start_master_bounce(path);
        }
        AudioSideEffect::StartStemExport { stems } => {
            let _ = audio.start_stem_export(stems);
        }
        AudioSideEffect::CancelExport => {
            let _ = audio.cancel_export();
        }

        // VST
        AudioSideEffect::SetVstParam { instrument_id, target, param_index, value } => {
            use imbolc_audio::commands::AudioCmd;
            if let Err(e) = audio.send_cmd(AudioCmd::SetVstParam {
                instrument_id: *instrument_id,
                target: *target,
                param_index: *param_index,
                value: *value,
            }) {
                log::warn!(target: "dispatch::vst", "SetVstParam dropped: {}", e);
            }
        }
        AudioSideEffect::QueryVstParams { instrument_id, target } => {
            use imbolc_audio::commands::AudioCmd;
            if let Err(e) = audio.send_cmd(AudioCmd::QueryVstParams {
                instrument_id: *instrument_id,
                target: *target,
            }) {
                log::warn!(target: "dispatch::vst", "QueryVstParams dropped: {}", e);
            }
        }
        AudioSideEffect::SaveVstState { instrument_id, target, path } => {
            use imbolc_audio::commands::AudioCmd;
            if let Err(e) = audio.send_cmd(AudioCmd::SaveVstState {
                instrument_id: *instrument_id,
                target: *target,
                path: path.clone(),
            }) {
                log::warn!(target: "dispatch::vst", "SaveVstState dropped: {}", e);
            }
        }
    }
}
