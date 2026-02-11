//! Rendering: FPS throttle, meter/visualization updates, frame rendering.

use std::time::Instant;

use super::AppRuntime;
use crate::panes::WaveformPane;

impl AppRuntime {
    /// Render at ~60fps if needed.
    pub(crate) fn maybe_render(
        &mut self,
        backend: &mut crate::ui::RatatuiBackend,
    ) -> std::io::Result<()> {
        let now = Instant::now();
        if now.duration_since(self.last_render_time).as_millis() < 16 {
            return Ok(());
        }

        // Always render when audio is running (meters, playhead, recording timer)
        if self.audio.is_running() {
            self.render_needed = true;
        }

        // Re-render while status bar has a visible message (for auto-dismiss)
        if self.app_frame.status_bar.current().is_some() {
            self.render_needed = true;
        }

        if !self.render_needed {
            return Ok(());
        }

        self.last_render_time = now;

        // Update master meter from real audio peak
        {
            let peak = if self.audio.is_running() {
                self.audio.master_peak()
            } else {
                0.0
            };
            let mute = self.dispatcher.state().session.mixer.master_mute;
            self.app_frame.set_master_peak(peak, mute);
        }

        // Update SC CPU and latency indicators
        {
            let cpu = if self.audio.is_running() {
                self.audio.sc_cpu()
            } else {
                0.0
            };
            let osc_latency = if self.audio.is_running() {
                self.audio.osc_latency_ms()
            } else {
                0.0
            };
            let audio_latency = self.audio.audio_latency_ms();
            self.app_frame
                .set_sc_metrics(cpu, osc_latency, audio_latency);
        }

        // Update recording state
        {
            let state = self.dispatcher.state_mut();
            state.recording.recording = self.audio.is_recording();
            state.recording.recording_secs = self
                .audio
                .recording_elapsed()
                .map(|d| d.as_secs())
                .unwrap_or(0);
            self.app_frame.recording = state.recording.recording;
            self.app_frame.recording_secs = state.recording.recording_secs;
        }

        // Update visualization data only when waveform pane is active
        if self.panes.active().id() == "waveform" {
            let state = self.dispatcher.state_mut();
            state.audio.visualization.spectrum_bands = self.audio.spectrum_bands();
            let (peak_l, peak_r, rms_l, rms_r) = self.audio.lufs_data();
            state.audio.visualization.peak_l = peak_l;
            state.audio.visualization.peak_r = peak_r;
            state.audio.visualization.rms_l = rms_l;
            state.audio.visualization.rms_r = rms_r;
            let scope = self.audio.scope_buffer();
            state.audio.visualization.scope_buffer.clear();
            state.audio.visualization.scope_buffer.extend(scope);
        }

        // Update waveform cache for waveform pane
        if self.panes.active().id() == "waveform" {
            if let Some(wf) = self.panes.get_pane_mut::<WaveformPane>("waveform") {
                if self.dispatcher.state().recorded_waveform_peaks.is_none() {
                    let inst_data = self
                        .dispatcher
                        .state()
                        .instruments
                        .selected_instrument()
                        .filter(|s| s.source.is_audio_input() || s.source.is_bus_in())
                        .map(|s| s.id);
                    wf.audio_in_waveform = inst_data.map(|id| self.audio.audio_in_waveform(id));
                }
            }
        } else {
            if let Some(wf) = self.panes.get_pane_mut::<WaveformPane>("waveform") {
                wf.audio_in_waveform = None;
            }
            self.dispatcher.state_mut().recorded_waveform_peaks = None;
        }

        // Copy audio-owned state into AppState for pane rendering
        {
            let ars = self.audio.read_state();
            let state = self.dispatcher.state_mut();
            state.audio.playhead = ars.playhead;
            state.audio.bpm = ars.bpm;
            state.audio.playing = ars.playing;
            state.audio.server_status = ars.server_status;
        }

        // Render
        crate::global_actions::render_frame(
            backend,
            &self.app_frame,
            &mut self.panes,
            self.dispatcher.state(),
            &mut self.last_area,
        )?;

        self.render_needed = false;

        Ok(())
    }
}
