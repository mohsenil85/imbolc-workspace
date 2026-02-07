use crate::action::{DispatchResult, NavIntent, VstTarget};
use crate::audio::commands::AudioFeedback;
use crate::audio::AudioHandle;
use crate::state::AppState;

pub fn dispatch_audio_feedback(
    feedback: &AudioFeedback,
    state: &mut AppState,
    audio: &mut AudioHandle,
) -> DispatchResult {
    let mut result = DispatchResult::default();

    match feedback {
        AudioFeedback::PlayheadPosition(playhead) => {
            state.audio.playhead = *playhead;
        }
        AudioFeedback::BpmUpdate(bpm) => {
            state.audio.bpm = *bpm;
        }
        AudioFeedback::DrumSequencerStep { instrument_id, step } => {
            if let Some(inst) = state.instruments.instrument_mut(*instrument_id) {
                if let Some(seq) = inst.drum_sequencer.as_mut() {
                    seq.current_step = *step;
                    seq.last_played_step = Some(*step);
                }
            }
        }
        AudioFeedback::ServerStatus { status, message, server_running } => {
            result.push_status_with_running(*status, message.clone(), *server_running);
        }
        AudioFeedback::RecordingState { is_recording, elapsed_secs } => {
            state.recording.recording = *is_recording;
            state.recording.recording_secs = *elapsed_secs;
        }
        AudioFeedback::RecordingStopped(path) => {
            state.recording.pending_recording_path = Some(path.clone());
        }
        AudioFeedback::RenderComplete { instrument_id, path } => {
            // Stop playback and restore looping
            state.session.piano_roll.playing = false;
            state.audio.playhead = 0;
            if let Some(render) = state.io.pending_render.take() {
                state.session.piano_roll.looping = render.was_looping;
            }
            result.stop_playback = true;
            result.reset_playhead = true;

            // Convert instrument to PitchedSampler
            let buffer_id = state.instruments.next_sampler_buffer_id;
            state.instruments.next_sampler_buffer_id += 1;
            let _ = audio.load_sample(buffer_id, &path.to_string_lossy());

            if let Some(inst) = state.instruments.instrument_mut(*instrument_id) {
                use crate::state::{SourceType, ParamValue};
                inst.source = SourceType::PitchedSampler;
                inst.source_params = SourceType::PitchedSampler.default_params();
                // Override buffer param with our rendered WAV
                if let Some(p) = inst.source_params.iter_mut().find(|p| p.name == "buffer") {
                    p.value = ParamValue::Int(buffer_id as i32);
                }
            }

            result.audio_dirty.instruments = true;
            result.audio_dirty.routing_instrument = Some(*instrument_id);
            result.push_status(audio.status(), "Render complete");
        }
        AudioFeedback::CompileResult(res) => {
            match res {
                Ok(msg) => result.push_status(audio.status(), msg.clone()),
                Err(e) => result.push_status(audio.status(), e.clone()),
            }
        }
        AudioFeedback::PendingBufferFreed => {
            if let Some(path) = state.recording.pending_recording_path.take() {
                let (peaks, _) = super::helpers::compute_waveform_peaks(&path.to_string_lossy());
                if !peaks.is_empty() {
                    state.recorded_waveform_peaks = Some(peaks);
                    result.push_nav(NavIntent::SwitchTo("waveform"));
                }
            }
        }
        AudioFeedback::VstParamsDiscovered { instrument_id, target, vst_plugin_id, params } => {
            // Update plugin registry with discovered param specs
            if let Some(plugin) = state.session.vst_plugins.get_mut(*vst_plugin_id) {
                plugin.params.clear();
                for (index, name, label, default) in params {
                    plugin.params.push(crate::state::VstParamSpec {
                        index: *index,
                        name: name.clone(),
                        default: *default,
                        label: label.clone(),
                    });
                }
            }
            // Initialize per-instance param values from defaults
            if let Some(instrument) = state.instruments.instrument_mut(*instrument_id) {
                match target {
                    VstTarget::Source => {
                        instrument.vst_param_values.clear();
                        for (index, _, _, default) in params {
                            instrument.vst_param_values.push((*index, *default));
                        }
                    }
                    VstTarget::Effect(effect_id) => {
                        if let Some(effect) = instrument.effect_by_id_mut(*effect_id) {
                            effect.vst_param_values.clear();
                            for (index, _, _, default) in params {
                                effect.vst_param_values.push((*index, *default));
                            }
                        }
                    }
                }
            }
        }
        AudioFeedback::ExportComplete { kind, paths } => {
            state.session.piano_roll.playing = false;
            state.audio.playhead = 0;
            if let Some(export) = state.io.pending_export.take() {
                state.session.piano_roll.looping = export.was_looping;
            }
            state.io.export_progress = 0.0;
            result.stop_playback = true;
            result.reset_playhead = true;

            let message = match kind {
                crate::audio::commands::ExportKind::MasterBounce => {
                    format!("Bounce complete: {}", paths.first().map(|p| p.display().to_string()).unwrap_or_default())
                }
                crate::audio::commands::ExportKind::StemExport => {
                    format!("Stem export complete: {} files", paths.len())
                }
            };
            result.push_status(audio.status(), message);
        }
        AudioFeedback::ExportProgress { progress } => {
            state.io.export_progress = *progress;
        }
        AudioFeedback::VstStateSaved { instrument_id, target, path } => {
            if let Some(instrument) = state.instruments.instrument_mut(*instrument_id) {
                match target {
                    VstTarget::Source => {
                        instrument.vst_state_path = Some(path.clone());
                    }
                    VstTarget::Effect(effect_id) => {
                        if let Some(effect) = instrument.effect_by_id_mut(*effect_id) {
                            effect.vst_state_path = Some(path.clone());
                        }
                    }
                }
            }
        }
        AudioFeedback::ServerCrashed { message } => {
            result.push_status(crate::audio::ServerStatus::Error, format!("SERVER CRASHED: {}", message));
            state.session.piano_roll.playing = false;
            result.stop_playback = true;
        }
        AudioFeedback::TelemetrySummary { avg_tick_us, max_tick_us, p95_tick_us, overruns } => {
            // Log telemetry for monitoring; could be exposed to UI in future
            log::debug!(target: "audio",
                "Telemetry: avg={}us max={}us p95={}us overruns={}",
                avg_tick_us, max_tick_us, p95_tick_us, overruns
            );
        }
    }

    result
}
