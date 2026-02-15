use crate::action::{AudioEffect, DispatchResult, NavIntent, PaneId, VstTarget};
use crate::state::AppState;
use imbolc_audio::commands::AudioFeedback;
use imbolc_audio::AudioHandle;
use imbolc_types::SourceExtra;

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
        AudioFeedback::PlayingChanged(playing) => {
            state.audio.playing = *playing;
        }
        AudioFeedback::DrumSequencerStep {
            instrument_id,
            step,
        } => {
            if let Some(inst) = state.instruments.instrument_mut(*instrument_id) {
                if let Some(seq) = inst.drum_sequencer_mut() {
                    seq.current_step = *step;
                    seq.last_played_step = Some(*step);
                }
            }
        }
        AudioFeedback::ServerStatus {
            status,
            message,
            server_running,
        } => {
            result.push_status_with_running(*status, message.clone(), *server_running);
        }
        AudioFeedback::RecordingState {
            is_recording,
            elapsed_secs,
        } => {
            state.recording.recording = *is_recording;
            state.recording.recording_secs = *elapsed_secs;
        }
        AudioFeedback::RecordingStopped(path) => {
            state.recording.pending_recording_path = Some(path.clone());
        }
        AudioFeedback::RenderComplete {
            instrument_id,
            path,
        } => {
            // Stop playback and restore looping
            state.session.piano_roll.playing = false;
            state.audio.playing = false;
            state.audio.playhead = 0;
            if let Some(render) = state.io.pending_render.take() {
                state.session.piano_roll.looping = render.was_looping;
            }
            result.stop_playback = true;
            result.reset_playhead = true;

            // Convert instrument to PitchedSampler
            let buffer_id = state.instruments.next_sampler_buffer_id;
            state.instruments.next_sampler_buffer_id += 1;
            let path_str = path.to_string_lossy().to_string();
            let _ = audio.load_sample(buffer_id, &path_str);

            if let Some(inst) = state.instruments.instrument_mut(*instrument_id) {
                use crate::state::{ParamValue, SourceType};
                inst.source = SourceType::PitchedSampler;
                inst.source_params = SourceType::PitchedSampler.default_params();
                // Override buffer param with our rendered WAV
                if let Some(p) = inst.source_params.iter_mut().find(|p| p.name == "buffer") {
                    p.value = ParamValue::Int(buffer_id as i32);
                }
            }

            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
                .audio_effects
                .push(AudioEffect::RebuildRoutingForInstrument(*instrument_id));
            result.push_status(audio.status(), "Render complete".to_string());
        }
        AudioFeedback::CompileResult(res) | AudioFeedback::LoadResult(res) => match res {
            Ok(msg) => result.push_status(audio.status(), msg.clone().to_string()),
            Err(e) => result.push_status(audio.status(), e.clone().to_string()),
        },
        AudioFeedback::PendingBufferFreed => {
            if let Some(path) = state.recording.pending_recording_path.take() {
                let (peaks, _) = super::helpers::compute_waveform_peaks(&path.to_string_lossy());
                if !peaks.is_empty() {
                    state.recorded_waveform_peaks = Some(peaks);
                    result.push_nav(NavIntent::SwitchTo(PaneId::Waveform));
                }
            }
        }
        AudioFeedback::VstParamsDiscovered {
            instrument_id,
            target,
            vst_plugin_id,
            params,
        } => {
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
                        if let SourceExtra::Vst {
                            ref mut param_values,
                            ..
                        } = instrument.source_extra
                        {
                            param_values.clear();
                            for (index, _, _, default) in params {
                                param_values.push((*index, *default));
                            }
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
            state.audio.playing = false;
            state.audio.playhead = 0;
            if let Some(export) = state.io.pending_export.take() {
                state.session.piano_roll.looping = export.was_looping;
            }
            state.io.export_progress = 0.0;
            result.stop_playback = true;
            result.reset_playhead = true;

            let message = match kind {
                imbolc_audio::commands::ExportKind::MasterBounce => {
                    format!(
                        "Bounce complete: {}",
                        paths
                            .first()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default()
                    )
                }
                imbolc_audio::commands::ExportKind::StemExport => {
                    format!("Stem export complete: {} files", paths.len())
                }
            };
            result.push_status(audio.status(), message.to_string());
        }
        AudioFeedback::ExportProgress { progress } => {
            state.io.export_progress = *progress;
        }
        AudioFeedback::VstStateSaved {
            instrument_id,
            target,
            path,
        } => {
            if let Some(instrument) = state.instruments.instrument_mut(*instrument_id) {
                match target {
                    VstTarget::Source => {
                        if let SourceExtra::Vst {
                            ref mut state_path, ..
                        } = instrument.source_extra
                        {
                            *state_path = Some(path.clone());
                        }
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
            result.push_status(
                imbolc_audio::ServerStatus::Error,
                format!("SERVER CRASHED: {}", message),
            );
            state.session.piano_roll.playing = false;
            state.audio.playing = false;
            result.stop_playback = true;
        }
        AudioFeedback::GenerativeEvent {
            instrument_id,
            pitch,
            velocity,
            duration_ticks,
            tick,
        } => {
            if state.session.generative.capture_enabled {
                state.session.generative.captured_events.push(
                    imbolc_types::CapturedGenEvent {
                        instrument_id: *instrument_id,
                        pitch: *pitch,
                        velocity: *velocity,
                        duration_ticks: *duration_ticks,
                        tick: *tick,
                    },
                );
            }
        }
        AudioFeedback::TelemetrySummary {
            avg_tick_us,
            max_tick_us,
            p95_tick_us,
            overruns,
            schedule_lookahead_ms,
            osc_send_queue_depth,
        } => {
            state.audio.telemetry_avg_tick_us = *avg_tick_us;
            state.audio.telemetry_max_tick_us = *max_tick_us;
            state.audio.telemetry_p95_tick_us = *p95_tick_us;
            state.audio.telemetry_overruns = *overruns;
            state.audio.telemetry_lookahead_ms = *schedule_lookahead_ms;
            state.audio.telemetry_osc_queue_depth = *osc_send_queue_depth;
            log::debug!(target: "audio",
                "Telemetry: avg={}us max={}us p95={}us overruns={} lookahead={:.1}ms osc_q={}",
                avg_tick_us, max_tick_us, p95_tick_us, overruns, schedule_lookahead_ms, osc_send_queue_depth
            );
        }
    }

    result
}
