//! AudioHandle: main-thread interface to the audio engine.
//!
//! Owns the command/feedback channels and shared monitor state. The
//! AudioEngine and playback ticking live on the audio thread.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel::Sender as CrossbeamSender;

use super::commands::{AudioCmd, AudioFeedback};
use super::osc_client::AudioMonitor;
use super::snapshot::{AutomationSnapshot, InstrumentSnapshot, PianoRollSnapshot, SessionSnapshot};
use super::ServerStatus;
use crate::action::AudioDirty;
use crate::state::arrangement::PlayMode;
use crate::state::automation::AutomationTarget;
use crate::state::{AppState, BufferId, EffectId, InstrumentId};

/// Audio-owned read state: values that the audio thread is the authority on.
/// UI reads these for display; audio feedback updates them.
#[derive(Debug, Clone)]
pub struct AudioReadState {
    pub playhead: u32,
    pub bpm: f32,
    pub is_recording: bool,
    pub recording_elapsed: Option<Duration>,
    pub server_status: ServerStatus,
    pub server_running: bool,
}

impl Default for AudioReadState {
    fn default() -> Self {
        Self {
            playhead: 0,
            bpm: 120.0,
            is_recording: false,
            recording_elapsed: None,
            server_status: ServerStatus::Stopped,
            server_running: false,
        }
    }
}

/// Main-thread handle to the audio subsystem.
///
/// Phase 3: communicates with a dedicated audio thread via MPSC channels.
/// Uses separate priority and normal channels for reduced latency on time-critical commands.
pub struct AudioHandle {
    /// Priority commands: voice spawn/release, param changes (time-critical)
    priority_tx: CrossbeamSender<AudioCmd>,
    /// Normal commands: state sync, routing rebuilds, recording control
    normal_tx: CrossbeamSender<AudioCmd>,
    feedback_rx: Receiver<AudioFeedback>,
    monitor: AudioMonitor,
    audio_state: AudioReadState,
    is_running: bool,
    join_handle: Option<JoinHandle<()>>,
}

impl AudioHandle {
    pub fn new() -> Self {
        // Create priority channel for time-critical commands (voice spawn, param changes)
        let (priority_tx, priority_rx) = crossbeam_channel::unbounded();
        // Create normal channel for less urgent commands (state sync, routing)
        let (normal_tx, normal_rx) = crossbeam_channel::unbounded();
        let (feedback_tx, feedback_rx) = mpsc::channel();
        let monitor = AudioMonitor::new();
        let thread_monitor = monitor.clone();

        let join_handle = thread::spawn(move || {
            let thread = super::audio_thread::AudioThread::new(
                priority_rx,
                normal_rx,
                feedback_tx,
                thread_monitor,
            );
            thread.run();
        });

        Self {
            priority_tx,
            normal_tx,
            feedback_rx,
            monitor,
            audio_state: AudioReadState::default(),
            is_running: false,
            join_handle: Some(join_handle),
        }
    }

    /// Send a command to the audio thread, routing to priority or normal channel.
    pub fn send_cmd(&self, cmd: AudioCmd) -> Result<(), String> {
        if cmd.is_priority() {
            self.priority_tx
                .send(cmd)
                .map_err(|_| "Audio thread disconnected".to_string())
        } else {
            self.normal_tx
                .send(cmd)
                .map_err(|_| "Audio thread disconnected".to_string())
        }
    }

    /// Fire-and-forget: send a command and log if the audio thread is disconnected.
    fn send(&self, cmd: AudioCmd) {
        if let Err(e) = self.send_cmd(cmd) {
            log::warn!(target: "audio", "command dropped: {}", e);
        }
    }

    pub fn drain_feedback(&mut self) -> Vec<AudioFeedback> {
        let mut out = Vec::new();
        while let Ok(msg) = self.feedback_rx.try_recv() {
            self.apply_feedback(&msg);
            out.push(msg);
        }
        out
    }

    fn apply_feedback(&mut self, feedback: &AudioFeedback) {
        match feedback {
            AudioFeedback::PlayheadPosition(pos) => {
                self.audio_state.playhead = *pos;
            }
            AudioFeedback::BpmUpdate(bpm) => {
                self.audio_state.bpm = *bpm;
            }
            AudioFeedback::DrumSequencerStep { .. } => {}
            AudioFeedback::ServerStatus { status, server_running, .. } => {
                self.audio_state.server_status = *status;
                self.audio_state.server_running = *server_running;
                self.is_running = matches!(status, ServerStatus::Connected);
            }
            AudioFeedback::RecordingState { is_recording, elapsed_secs } => {
                self.audio_state.is_recording = *is_recording;
                self.audio_state.recording_elapsed = if *is_recording {
                    Some(Duration::from_secs(*elapsed_secs))
                } else {
                    None
                };
            }
            AudioFeedback::RecordingStopped(_) => {}
            AudioFeedback::RenderComplete { .. } => {}
            AudioFeedback::CompileResult(_) => {}
            AudioFeedback::PendingBufferFreed => {}
            AudioFeedback::VstParamsDiscovered { .. } => {}
            AudioFeedback::VstStateSaved { .. } => {}
            AudioFeedback::ExportComplete { .. } => {}
            AudioFeedback::ExportProgress { .. } => {}
            AudioFeedback::ServerCrashed { .. } => {
                self.is_running = false;
            }
        }
    }

    pub fn sync_state(&mut self, state: &AppState) {
        self.flush_dirty(state, AudioDirty::all());
    }

    pub fn flush_dirty(&mut self, state: &AppState, dirty: AudioDirty) {
        if !dirty.any() {
            return;
        }

        let needs_full_state = dirty.instruments || dirty.session || dirty.routing;
        if needs_full_state {
            self.update_state(&state.instruments, &state.session);
        }
        if dirty.piano_roll {
            if state.session.arrangement.play_mode == PlayMode::Song
                && state.session.arrangement.editing_clip.is_none()
            {
                let mut flat_pr = state.session.piano_roll.clone();
                let flattened = state.session.arrangement.flatten_to_notes();
                for (&instrument_id, track) in &mut flat_pr.tracks {
                    track.notes = flattened.get(&instrument_id).cloned().unwrap_or_default();
                }
                let arr_len = state.session.arrangement.arrangement_length();
                if arr_len > 0 {
                    flat_pr.loop_end = arr_len;
                    flat_pr.looping = false;
                }
                self.update_piano_roll_data(&flat_pr);
            } else {
                self.update_piano_roll_data(&state.session.piano_roll);
            }
        }
        if dirty.automation {
            if state.session.arrangement.play_mode == PlayMode::Song
                && state.session.arrangement.editing_clip.is_none()
            {
                let mut merged = state.session.automation.lanes.clone();
                merged.extend(state.session.arrangement.flatten_automation());
                self.update_automation_lanes(&merged);
            } else {
                self.update_automation_lanes(&state.session.automation.lanes);
            }
        }
        if dirty.routing {
            self.send(AudioCmd::RebuildRouting);
        } else if let Some(instrument_id) = dirty.routing_instrument {
            // Targeted single-instrument rebuild (no full teardown)
            if needs_full_state {
                // State already sent above
            } else {
                self.update_state(&state.instruments, &state.session);
            }
            self.send(AudioCmd::RebuildInstrumentRouting { instrument_id });
        }
        if dirty.mixer_params {
            if needs_full_state {
                // Full state already sent — just trigger the engine update
                self.send(AudioCmd::UpdateMixerParams);
            } else {
                // Mixer-only change: send targeted updates (no full clone)
                self.send_mixer_params_incremental(state);
            }
        }

        // ── Targeted param updates (bypass full state clone + rebuild) ──
        if let Some((instrument_id, param_kind, value)) = dirty.filter_param {
            if let Err(e) = self.set_filter_param(instrument_id, param_kind.as_str(), value) {
                log::warn!(target: "audio", "set_filter_param dropped: {}", e);
            }
        }
        if let Some((instrument_id, effect_id, param_idx, value)) = dirty.effect_param {
            // Resolve param name from instrument state
            if let Some(inst) = state.instruments.instrument(instrument_id) {
                if let Some(effect) = inst.effect_by_id(effect_id) {
                    if let Some(param) = effect.params.get(param_idx) {
                        if let Err(e) = self.set_effect_param(instrument_id, effect_id, &param.name, value) {
                            log::warn!(target: "audio", "set_effect_param dropped: {}", e);
                        }
                    }
                }
            }
        }
        if let Some((instrument_id, param_kind, value)) = dirty.lfo_param {
            if let Err(e) = self.set_lfo_param(instrument_id, param_kind.as_str(), value) {
                log::warn!(target: "audio", "set_lfo_param dropped: {}", e);
            }
        }
    }

    fn send_mixer_params_incremental(&self, state: &AppState) {
        self.send(AudioCmd::SetMasterParams {
            level: state.session.mixer.master_level,
            mute: state.session.mixer.master_mute,
        });
        for inst in &state.instruments.instruments {
            self.send(AudioCmd::SetInstrumentMixerParams {
                instrument_id: inst.id,
                level: inst.level,
                pan: inst.pan,
                mute: inst.mute,
                solo: inst.solo,
            });
        }
        // After all fields are updated on the audio thread, trigger engine apply
        self.send(AudioCmd::UpdateMixerParams);
    }

    pub fn update_state(&mut self, instruments: &InstrumentSnapshot, session: &SessionSnapshot) {
        self.send(AudioCmd::UpdateState {
            instruments: instruments.clone(),
            session: session.clone(),
        });
    }

    pub fn update_piano_roll_data(&mut self, piano_roll: &PianoRollSnapshot) {
        self.send(AudioCmd::UpdatePianoRollData {
            piano_roll: piano_roll.clone(),
        });
    }

    pub fn update_automation_lanes(&mut self, lanes: &AutomationSnapshot) {
        self.send(AudioCmd::UpdateAutomationLanes {
            lanes: lanes.clone(),
        });
    }

    pub fn set_playing(&mut self, playing: bool) {
        self.send(AudioCmd::SetPlaying { playing });
    }

    pub fn reset_playhead(&mut self) {
        self.send(AudioCmd::ResetPlayhead);
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.send(AudioCmd::SetBpm { bpm });
    }

    // ── State accessors ───────────────────────────────────────────

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn read_state(&self) -> &AudioReadState {
        &self.audio_state
    }

    pub fn status(&self) -> ServerStatus {
        self.audio_state.server_status
    }

    pub fn server_running(&self) -> bool {
        self.audio_state.server_running
    }

    pub fn master_peak(&self) -> f32 {
        let (l, r) = self.monitor.meter_peak();
        l.max(r)
    }

    pub fn audio_in_waveform(&self, instrument_id: u32) -> Vec<f32> {
        self.monitor.audio_in_waveform(instrument_id)
    }

    pub fn spectrum_bands(&self) -> [f32; 7] {
        self.monitor.spectrum_bands()
    }

    pub fn lufs_data(&self) -> (f32, f32, f32, f32) {
        self.monitor.lufs_data()
    }

    pub fn scope_buffer(&self) -> Vec<f32> {
        self.monitor.scope_buffer()
    }

    pub fn sc_cpu(&self) -> f32 {
        self.monitor.sc_cpu()
    }

    pub fn osc_latency_ms(&self) -> f32 {
        self.monitor.osc_latency_ms()
    }

    pub fn audio_latency_ms(&self) -> f32 {
        self.monitor.audio_latency_ms()
    }

    pub fn is_recording(&self) -> bool {
        self.audio_state.is_recording
    }

    pub fn recording_elapsed(&self) -> Option<Duration> {
        self.audio_state.recording_elapsed
    }

    // ── Server lifecycle ──────────────────────────────────────────

    pub fn connect_async(&mut self, server_addr: &str) -> Result<(), String> {
        let (reply_tx, _reply_rx) = mpsc::channel();
        self.send_cmd(AudioCmd::Connect {
            server_addr: server_addr.to_string(),
            reply: reply_tx,
        })
    }

    pub fn disconnect_async(&mut self) -> Result<(), String> {
        self.send_cmd(AudioCmd::Disconnect)
    }

    pub fn start_server_async(
        &mut self,
        input_device: Option<&str>,
        output_device: Option<&str>,
        buffer_size: u32,
        sample_rate: u32,
    ) -> Result<(), String> {
        let (reply_tx, _reply_rx) = mpsc::channel();
        self.send_cmd(AudioCmd::StartServer {
            input_device: input_device.map(|s| s.to_string()),
            output_device: output_device.map(|s| s.to_string()),
            buffer_size,
            sample_rate,
            reply: reply_tx,
        })
    }

    pub fn stop_server_async(&mut self) -> Result<(), String> {
        self.send_cmd(AudioCmd::StopServer)
    }

    pub fn restart_server_async(
        &mut self,
        input_device: Option<&str>,
        output_device: Option<&str>,
        server_addr: &str,
        buffer_size: u32,
        sample_rate: u32,
    ) -> Result<(), String> {
        self.send_cmd(AudioCmd::RestartServer {
            input_device: input_device.map(|s| s.to_string()),
            output_device: output_device.map(|s| s.to_string()),
            server_addr: server_addr.to_string(),
            buffer_size,
            sample_rate,
        })
    }

    pub fn connect(&mut self, server_addr: &str) -> std::io::Result<()> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.normal_tx
            .send(AudioCmd::Connect {
                server_addr: server_addr.to_string(),
                reply: reply_tx,
            })
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Audio thread disconnected"))?;
        match reply_rx.recv() {
            Ok(result) => {
                if result.is_ok() {
                    self.audio_state.server_status = ServerStatus::Connected;
                    self.is_running = true;
                } else {
                    self.audio_state.server_status = ServerStatus::Error;
                    self.is_running = false;
                }
                result
            }
            Err(_) => Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Audio thread disconnected")),
        }
    }

    pub fn disconnect(&mut self) {
        self.send(AudioCmd::Disconnect);
        self.is_running = false;
        self.audio_state.server_status = if self.audio_state.server_running {
            ServerStatus::Running
        } else {
            ServerStatus::Stopped
        };
    }

    pub fn start_server_with_devices(
        &mut self,
        input_device: Option<&str>,
        output_device: Option<&str>,
        buffer_size: u32,
        sample_rate: u32,
    ) -> Result<(), String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send_cmd(AudioCmd::StartServer {
            input_device: input_device.map(|s| s.to_string()),
            output_device: output_device.map(|s| s.to_string()),
            buffer_size,
            sample_rate,
            reply: reply_tx,
        })?;
        match reply_rx.recv() {
            Ok(result) => {
                if result.is_ok() {
                    self.audio_state.server_status = ServerStatus::Running;
                    self.audio_state.server_running = true;
                } else {
                    self.audio_state.server_status = ServerStatus::Error;
                }
                result
            }
            Err(_) => Err("Audio thread disconnected".to_string()),
        }
    }

    pub fn stop_server(&mut self) {
        self.send(AudioCmd::StopServer);
        self.audio_state.server_status = ServerStatus::Stopped;
        self.audio_state.server_running = false;
        self.is_running = false;
    }

    pub fn compile_synthdefs_async(&mut self, scd_path: &Path) -> Result<(), String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send_cmd(AudioCmd::CompileSynthDefs {
            scd_path: scd_path.to_path_buf(),
            reply: reply_tx,
        })?;
        match reply_rx.recv() {
            Ok(result) => result,
            Err(_) => Err("Audio thread disconnected".to_string()),
        }
    }

    pub fn load_synthdefs(&self, dir: &Path) -> Result<(), String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send_cmd(AudioCmd::LoadSynthDefs {
            dir: dir.to_path_buf(),
            reply: reply_tx,
        })?;
        match reply_rx.recv() {
            Ok(result) => result,
            Err(_) => Err("Audio thread disconnected".to_string()),
        }
    }

    pub fn load_synthdef_file(&self, path: &Path) -> Result<(), String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send_cmd(AudioCmd::LoadSynthDefFile {
            path: path.to_path_buf(),
            reply: reply_tx,
        })?;
        match reply_rx.recv() {
            Ok(result) => result,
            Err(_) => Err("Audio thread disconnected".to_string()),
        }
    }

    // ── SynthDefs & samples ───────────────────────────────────────

    pub fn load_sample(&mut self, buffer_id: BufferId, path: &str) -> Result<i32, String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send_cmd(AudioCmd::LoadSample {
            buffer_id,
            path: path.to_string(),
            reply: reply_tx,
        })?;
        match reply_rx.recv() {
            Ok(result) => result,
            Err(_) => Err("Audio thread disconnected".to_string()),
        }
    }

    pub fn free_samples(&self, buffer_ids: Vec<BufferId>) {
        if !buffer_ids.is_empty() {
            self.send(AudioCmd::FreeSamples { buffer_ids });
        }
    }

    // ── Routing & mixing ──────────────────────────────────────────

    pub fn rebuild_instrument_routing(&mut self) -> Result<(), String> {
        self.send_cmd(AudioCmd::RebuildRouting)
    }

    pub fn set_bus_mixer_params(
        &self,
        bus_id: u8,
        level: f32,
        mute: bool,
        pan: f32,
    ) -> Result<(), String> {
        self.send_cmd(AudioCmd::SetBusMixerParams {
            bus_id,
            level,
            mute,
            pan,
        })
    }

    pub fn update_all_instrument_mixer_params(&self) -> Result<(), String> {
        self.send_cmd(AudioCmd::UpdateMixerParams)
    }

    pub fn set_source_param(
        &self,
        instrument_id: InstrumentId,
        param: &str,
        value: f32,
    ) -> Result<(), String> {
        self.send_cmd(AudioCmd::SetSourceParam {
            instrument_id,
            param: param.to_string(),
            value,
        })
    }

    pub fn set_eq_param(
        &self,
        instrument_id: InstrumentId,
        param: &str,
        value: f32,
    ) -> Result<(), String> {
        self.send_cmd(AudioCmd::SetEqParam {
            instrument_id,
            param: param.to_string(),
            value,
        })
    }

    pub fn set_filter_param(
        &self,
        instrument_id: InstrumentId,
        param: &str,
        value: f32,
    ) -> Result<(), String> {
        self.send_cmd(AudioCmd::SetFilterParam {
            instrument_id,
            param: param.to_string(),
            value,
        })
    }

    pub fn set_effect_param(
        &self,
        instrument_id: InstrumentId,
        effect_id: EffectId,
        param: &str,
        value: f32,
    ) -> Result<(), String> {
        self.send_cmd(AudioCmd::SetEffectParam {
            instrument_id,
            effect_id,
            param: param.to_string(),
            value,
        })
    }

    pub fn set_lfo_param(
        &self,
        instrument_id: InstrumentId,
        param: &str,
        value: f32,
    ) -> Result<(), String> {
        self.send_cmd(AudioCmd::SetLfoParam {
            instrument_id,
            param: param.to_string(),
            value,
        })
    }

    // ── Voice management ──────────────────────────────────────────

    pub fn spawn_voice(
        &mut self,
        instrument_id: InstrumentId,
        pitch: u8,
        velocity: f32,
        offset_secs: f64,
    ) -> Result<(), String> {
        self.send_cmd(AudioCmd::SpawnVoice {
            instrument_id,
            pitch,
            velocity,
            offset_secs,
        })
    }

    pub fn release_voice(
        &mut self,
        instrument_id: InstrumentId,
        pitch: u8,
        offset_secs: f64,
    ) -> Result<(), String> {
        self.send_cmd(AudioCmd::ReleaseVoice {
            instrument_id,
            pitch,
            offset_secs,
        })
    }

    pub fn push_active_note(&mut self, instrument_id: u32, pitch: u8, duration_ticks: u32) {
        self.send(AudioCmd::RegisterActiveNote {
            instrument_id,
            pitch,
            duration_ticks,
        });
    }

    pub fn clear_active_notes(&mut self) {
        self.send(AudioCmd::ClearActiveNotes);
    }

    pub fn release_all_voices(&mut self) {
        self.send(AudioCmd::ReleaseAllVoices);
    }

    pub fn play_drum_hit_to_instrument(
        &mut self,
        buffer_id: BufferId,
        amp: f32,
        instrument_id: InstrumentId,
        slice_start: f32,
        slice_end: f32,
        rate: f32,
        offset_secs: f64,
    ) -> Result<(), String> {
        self.send_cmd(AudioCmd::PlayDrumHit {
            buffer_id,
            amp,
            instrument_id,
            slice_start,
            slice_end,
            rate,
            offset_secs,
        })
    }

    // ── Recording ─────────────────────────────────────────────────

    pub fn start_instrument_render(&mut self, instrument_id: InstrumentId, path: &Path) -> Result<(), String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send_cmd(AudioCmd::StartInstrumentRender {
            instrument_id,
            path: path.to_path_buf(),
            reply: reply_tx,
        })?;
        match reply_rx.recv() {
            Ok(result) => {
                if result.is_ok() {
                    self.audio_state.is_recording = true;
                    self.audio_state.recording_elapsed = Some(Duration::from_secs(0));
                }
                result
            }
            Err(_) => Err("Audio thread disconnected".to_string()),
        }
    }

    pub fn start_recording(&mut self, bus: i32, path: &Path) -> Result<(), String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send_cmd(AudioCmd::StartRecording {
            bus,
            path: path.to_path_buf(),
            reply: reply_tx,
        })?;
        match reply_rx.recv() {
            Ok(result) => {
                if result.is_ok() {
                    self.audio_state.is_recording = true;
                    self.audio_state.recording_elapsed = Some(Duration::from_secs(0));
                }
                result
            }
            Err(_) => Err("Audio thread disconnected".to_string()),
        }
    }

    pub fn stop_recording(&mut self) -> Option<PathBuf> {
        let (reply_tx, reply_rx) = mpsc::channel();
        if self
            .send_cmd(AudioCmd::StopRecording { reply: reply_tx })
            .is_err()
        {
            return None;
        }
        match reply_rx.recv() {
            Ok(result) => {
                self.audio_state.is_recording = false;
                self.audio_state.recording_elapsed = None;
                result
            }
            Err(_) => None,
        }
    }

    // ── Export (bounce / stems) ──────────────────────────────────

    pub fn start_master_bounce(&mut self, path: &Path) -> Result<(), String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send_cmd(AudioCmd::StartMasterBounce {
            path: path.to_path_buf(),
            reply: reply_tx,
        })?;
        match reply_rx.recv() {
            Ok(result) => result,
            Err(_) => Err("Audio thread disconnected".to_string()),
        }
    }

    pub fn start_stem_export(
        &mut self,
        stems: &[(InstrumentId, PathBuf)],
    ) -> Result<(), String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send_cmd(AudioCmd::StartStemExport {
            stems: stems.to_vec(),
            reply: reply_tx,
        })?;
        match reply_rx.recv() {
            Ok(result) => result,
            Err(_) => Err("Audio thread disconnected".to_string()),
        }
    }

    pub fn cancel_export(&mut self) -> Result<(), String> {
        self.send_cmd(AudioCmd::CancelExport)
    }

    // ── Automation ────────────────────────────────────────────────

    pub fn apply_automation(
        &self,
        target: &AutomationTarget,
        value: f32,
    ) -> Result<(), String> {
        self.send_cmd(AudioCmd::ApplyAutomation {
            target: target.clone(),
            value,
        })
    }

    // ── Click Track ──────────────────────────────────────────────

    pub fn set_click_enabled(&self, enabled: bool) -> Result<(), String> {
        self.send_cmd(AudioCmd::SetClickEnabled { enabled })
    }

    pub fn set_click_volume(&self, volume: f32) -> Result<(), String> {
        self.send_cmd(AudioCmd::SetClickVolume { volume })
    }

    pub fn set_click_muted(&self, muted: bool) -> Result<(), String> {
        self.send_cmd(AudioCmd::SetClickMuted { muted })
    }
}

impl Drop for AudioHandle {
    fn drop(&mut self) {
        let _ = self.send_cmd(AudioCmd::Shutdown);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Default for AudioHandle {
    fn default() -> Self {
        Self::new()
    }
}
