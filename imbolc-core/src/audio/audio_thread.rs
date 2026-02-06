use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, TryRecvError};

use super::commands::{AudioCmd, AudioFeedback, ExportKind};
use super::engine::AudioEngine;
use super::osc_client::AudioMonitor;
use super::ServerStatus;
use crate::action::VstTarget;
use crate::state::arpeggiator::ArpPlayState;
use super::snapshot::{AutomationSnapshot, InstrumentSnapshot, PianoRollSnapshot, SessionSnapshot};
use crate::state::{InstrumentId, InstrumentState, SessionState};

/// Deferred server connection: after spawning scsynth, wait before connecting
/// so the server has time to initialize. Avoids blocking the audio thread.
struct PendingServerConnect {
    started_at: Instant,
    server_addr: String,
}

struct RenderState {
    instrument_id: InstrumentId,
    loop_end: u32,
    tail_ticks: u32,
}

struct ExportState {
    kind: ExportKind,
    loop_end: u32,
    tail_ticks: u32,
    #[allow(dead_code)]
    paths: Vec<PathBuf>,
}

/// Tracks an in-flight VST parameter query via OSC /param_query
struct PendingVstQuery {
    instrument_id: InstrumentId,
    target: VstTarget,
    vst_plugin_id: crate::state::vst_plugin::VstPluginId,
    node_id: i32,
    started_at: Instant,
    last_count: usize,
    last_change_at: Instant,
}

pub(crate) struct AudioThread {
    engine: AudioEngine,
    /// Priority commands: voice spawn/release, param changes (time-critical)
    priority_rx: Receiver<AudioCmd>,
    /// Normal commands: state sync, routing rebuilds, recording control
    normal_rx: Receiver<AudioCmd>,
    feedback_tx: Sender<AudioFeedback>,
    monitor: AudioMonitor,
    instruments: InstrumentSnapshot,
    session: SessionSnapshot,
    piano_roll: PianoRollSnapshot,
    automation_lanes: AutomationSnapshot,
    active_notes: Vec<(u32, u8, u32)>, // (instrument_id, pitch, duration_ticks)
    last_tick: Instant,
    last_recording_secs: u64,
    last_recording_state: bool,
    /// Simple LCG random seed for probability/humanization
    rng_state: u64,
    /// Per-instrument arpeggiator runtime state
    arp_states: HashMap<u32, ArpPlayState>,
    /// Active render-to-WAV state
    render_state: Option<RenderState>,
    /// Active export state (master bounce or stem export)
    export_state: Option<ExportState>,
    /// Last export progress sent (for throttling)
    last_export_progress: f32,
    /// Fractional tick accumulator for sub-tick precision (avoids truncation drift)
    tick_accumulator: f64,
    /// Last time /status was polled from SuperCollider
    last_status_poll: Instant,
    /// Deferred connection after server start (non-blocking restart)
    pending_server_connect: Option<PendingServerConnect>,
    /// In-flight VST parameter queries awaiting OSC replies
    pending_vst_queries: Vec<PendingVstQuery>,
}

impl AudioThread {
    pub(crate) fn new(
        priority_rx: Receiver<AudioCmd>,
        normal_rx: Receiver<AudioCmd>,
        feedback_tx: Sender<AudioFeedback>,
        monitor: AudioMonitor,
    ) -> Self {
        Self {
            engine: AudioEngine::new(),
            priority_rx,
            normal_rx,
            feedback_tx,
            monitor,
            instruments: InstrumentState::new(),
            session: SessionState::new(),
            piano_roll: PianoRollSnapshot::new(),
            automation_lanes: Vec::new(),
            active_notes: Vec::new(),
            last_tick: Instant::now(),
            last_recording_secs: 0,
            last_recording_state: false,
            rng_state: 12345,
            arp_states: HashMap::new(),
            render_state: None,
            export_state: None,
            last_export_progress: 0.0,
            tick_accumulator: 0.0,
            last_status_poll: Instant::now(),
            pending_server_connect: None,
            pending_vst_queries: Vec::new(),
        }
    }

    pub(crate) fn run(mut self) {
        // 0.5ms tick interval for reduced jitter (was 1ms)
        const TICK_INTERVAL: Duration = Duration::from_micros(500);

        loop {
            // Use crossbeam select to prioritize time-critical commands.
            // Priority channel is always checked first before normal channel.
            let remaining = TICK_INTERVAL.saturating_sub(self.last_tick.elapsed());

            crossbeam_channel::select! {
                // Priority commands (voice spawn, param changes) - always handled first
                recv(self.priority_rx) -> result => {
                    match result {
                        Ok(cmd) => {
                            if self.handle_cmd(cmd) {
                                break;
                            }
                        }
                        Err(_) => break, // Disconnected
                    }
                }
                // Normal commands - handled when no priority commands pending
                recv(self.normal_rx) -> result => {
                    match result {
                        Ok(cmd) => {
                            if self.handle_cmd(cmd) {
                                break;
                            }
                        }
                        Err(_) => break, // Disconnected
                    }
                }
                // Timeout - proceed with tick
                default(remaining) => {}
            }

            // Drain any additional priority commands first (critical path)
            if self.drain_priority_commands() {
                break;
            }
            // Then drain normal commands
            if self.drain_normal_commands() {
                break;
            }

            let now = Instant::now();
            let elapsed = now.duration_since(self.last_tick);
            if elapsed >= TICK_INTERVAL {
                self.last_tick = now;
                self.tick(elapsed);
            }

            self.poll_engine();
        }
    }

    /// Drain priority commands first (voice spawn, param changes)
    fn drain_priority_commands(&mut self) -> bool {
        const MAX_DRAIN_PER_TICK: usize = 64;
        for _ in 0..MAX_DRAIN_PER_TICK {
            match self.priority_rx.try_recv() {
                Ok(cmd) => {
                    if self.handle_cmd(cmd) {
                        return true;
                    }
                }
                Err(TryRecvError::Empty) => return false,
                Err(TryRecvError::Disconnected) => return true,
            }
        }
        false
    }

    /// Drain normal commands (state sync, routing)
    fn drain_normal_commands(&mut self) -> bool {
        const MAX_DRAIN_PER_TICK: usize = 32;
        for _ in 0..MAX_DRAIN_PER_TICK {
            match self.normal_rx.try_recv() {
                Ok(cmd) => {
                    if self.handle_cmd(cmd) {
                        return true;
                    }
                }
                Err(TryRecvError::Empty) => return false,
                Err(TryRecvError::Disconnected) => return true,
            }
        }
        false
    }

    fn handle_cmd(&mut self, cmd: AudioCmd) -> bool {
        match cmd {
            AudioCmd::Connect { server_addr, reply } => {
                let result = self.engine.connect_with_monitor(&server_addr, self.monitor.clone());
                match &result {
                    Ok(()) => {
                        let message = match self.load_synthdefs_and_samples() {
                            Ok(()) => "Connected".to_string(),
                            Err(e) => format!("Connected (synthdef warning: {})", e),
                        };
                        self.send_server_status(ServerStatus::Connected, message);
                    }
                    Err(err) => {
                        self.send_server_status(ServerStatus::Error, err.to_string());
                    }
                }
                let _ = reply.send(result);
            }
            AudioCmd::Disconnect => {
                self.engine.disconnect();
                self.send_server_status(self.engine.status(), "Disconnected");
            }
            AudioCmd::StartServer { input_device, output_device, reply } => {
                let result = self.engine.start_server_with_devices(
                    input_device.as_deref(),
                    output_device.as_deref(),
                );
                match &result {
                    Ok(()) => self.send_server_status(ServerStatus::Running, "Server started"),
                    Err(err) => self.send_server_status(ServerStatus::Error, err),
                }
                let _ = reply.send(result);
            }
            AudioCmd::StopServer => {
                self.engine.stop_server();
                self.send_server_status(ServerStatus::Stopped, "Server stopped");
            }
            AudioCmd::RestartServer { input_device, output_device, server_addr } => {
                self.engine.stop_server();
                self.pending_server_connect = None;
                self.send_server_status(ServerStatus::Stopped, "Restarting server...");

                let start_result = self.engine.start_server_with_devices(
                    input_device.as_deref(),
                    output_device.as_deref(),
                );
                match start_result {
                    Ok(()) => {
                        // Defer connection: let scsynth initialize before connecting
                        self.pending_server_connect = Some(PendingServerConnect {
                            started_at: Instant::now(),
                            server_addr,
                        });
                        self.send_server_status(ServerStatus::Starting, "Server starting...");
                    }
                    Err(err) => {
                        self.send_server_status(ServerStatus::Error, err);
                    }
                }
            }
            AudioCmd::CompileSynthDefs { scd_path, reply } => {
                let result = self.engine.compile_synthdefs_async(&scd_path);
                let _ = reply.send(result);
            }
            AudioCmd::LoadSynthDefs { dir, reply } => {
                let result = self.engine.load_synthdefs(&dir);
                let _ = reply.send(result);
            }
            AudioCmd::LoadSynthDefFile { path, reply } => {
                let result = self.engine.load_synthdef_file(&path);
                let _ = reply.send(result);
            }
            AudioCmd::UpdateState { instruments, session } => {
                self.apply_state_update(instruments, session);
            }
            AudioCmd::UpdatePianoRollData { piano_roll } => {
                self.apply_piano_roll_update(piano_roll);
            }
            AudioCmd::UpdateAutomationLanes { lanes } => {
                self.automation_lanes = lanes;
            }
            AudioCmd::SetPlaying { playing } => {
                self.piano_roll.playing = playing;
                if playing {
                    self.tick_accumulator = 0.0;
                }
            }
            AudioCmd::ResetPlayhead => {
                self.piano_roll.playhead = 0;
                self.tick_accumulator = 0.0;
                let _ = self.feedback_tx.send(AudioFeedback::PlayheadPosition(0));
            }
            AudioCmd::SetBpm { bpm } => {
                self.piano_roll.bpm = bpm;
                let _ = self.feedback_tx.send(AudioFeedback::BpmUpdate(bpm));
            }
            AudioCmd::RebuildRouting => {
                let _ = self.engine.rebuild_instrument_routing(&self.instruments, &self.session);
            }
            AudioCmd::RebuildInstrumentRouting { instrument_id } => {
                let _ = self.engine.rebuild_single_instrument_routing(instrument_id, &self.instruments, &self.session);
            }
            AudioCmd::UpdateMixerParams => {
                let _ = self.engine.update_all_instrument_mixer_params(&self.instruments, &self.session);
            }
            AudioCmd::SetMasterParams { level, mute } => {
                self.session.mixer.master_level = level;
                self.session.mixer.master_mute = mute;
            }
            AudioCmd::SetInstrumentMixerParams { instrument_id, level, pan, mute, solo } => {
                if let Some(inst) = self.instruments.instruments.iter_mut().find(|i| i.id == instrument_id) {
                    inst.level = level;
                    inst.pan = pan;
                    inst.mute = mute;
                    inst.solo = solo;
                }
            }
            AudioCmd::SetBusMixerParams { bus_id, level, mute, pan } => {
                let _ = self.engine.set_bus_mixer_params(bus_id, level, mute, pan);
            }
            AudioCmd::SetSourceParam { instrument_id, param, value } => {
                let _ = self.engine.set_source_param(instrument_id, &param, value);
            }
            AudioCmd::SetEqParam { instrument_id, param, value } => {
                let _ = self.engine.set_eq_param(instrument_id, &param, value);
            }
            AudioCmd::SetFilterParam { instrument_id, param, value } => {
                let _ = self.engine.set_filter_param(instrument_id, &param, value);
            }
            AudioCmd::SetEffectParam { instrument_id, effect_id, param, value } => {
                let _ = self.engine.set_effect_param(instrument_id, effect_id, &param, value);
            }
            AudioCmd::SetLfoParam { instrument_id, param, value } => {
                let _ = self.engine.set_lfo_param(instrument_id, &param, value);
            }
            AudioCmd::SpawnVoice { instrument_id, pitch, velocity, offset_secs } => {
                let _ = self.engine.spawn_voice(instrument_id, pitch, velocity, offset_secs, &self.instruments, &self.session);
            }
            AudioCmd::ReleaseVoice { instrument_id, pitch, offset_secs } => {
                let _ = self.engine.release_voice(instrument_id, pitch, offset_secs, &self.instruments);
            }
            AudioCmd::RegisterActiveNote { instrument_id, pitch, duration_ticks } => {
                self.active_notes.push((instrument_id, pitch, duration_ticks));
            }
            AudioCmd::ClearActiveNotes => {
                self.active_notes.clear();
            }
            AudioCmd::ReleaseAllVoices => {
                self.engine.release_all_voices();
            }
            AudioCmd::PlayDrumHit { buffer_id, amp, instrument_id, slice_start, slice_end, rate, offset_secs } => {
                let _ = self.engine.play_drum_hit_to_instrument(
                    buffer_id, amp, instrument_id, slice_start, slice_end, rate, offset_secs,
                );
            }
            AudioCmd::LoadSample { buffer_id, path, reply } => {
                let result = self.engine.load_sample(buffer_id, &path);
                let _ = reply.send(result);
            }
            AudioCmd::FreeSamples { buffer_ids } => {
                for id in buffer_ids {
                    let _ = self.engine.free_sample(id);
                }
            }
            AudioCmd::StartInstrumentRender { instrument_id, path, reply } => {
                let result = if let Some(&bus) = self.engine.instrument_final_buses.get(&instrument_id) {
                    self.engine.start_recording(bus, &path).map(|_| {
                        let ticks_per_second = (self.piano_roll.bpm / 60.0) * self.piano_roll.ticks_per_beat as f32;
                        self.render_state = Some(RenderState {
                            instrument_id,
                            loop_end: self.piano_roll.loop_end,
                            tail_ticks: ticks_per_second as u32,
                        });
                    })
                } else {
                    Err(format!("No audio bus for instrument {}", instrument_id))
                };
                let _ = reply.send(result);
            }
            AudioCmd::StartRecording { bus, path, reply } => {
                let result = self.engine.start_recording(bus, &path);
                let _ = reply.send(result);
            }
            AudioCmd::StopRecording { reply } => {
                let path = self.engine.stop_recording();
                let _ = reply.send(path);
            }
            AudioCmd::StartMasterBounce { path, reply } => {
                let result = self.engine.start_export_master(&path).map(|_| {
                    let ticks_per_second = (self.piano_roll.bpm / 60.0)
                        * self.piano_roll.ticks_per_beat as f32;
                    self.export_state = Some(ExportState {
                        kind: ExportKind::MasterBounce,
                        loop_end: self.piano_roll.loop_end,
                        tail_ticks: ticks_per_second as u32,
                        paths: vec![path],
                    });
                    self.last_export_progress = 0.0;
                });
                let _ = reply.send(result);
            }
            AudioCmd::StartStemExport { stems, reply } => {
                let instrument_buses: Vec<(u32, i32, PathBuf)> = stems
                    .iter()
                    .filter_map(|(inst_id, path)| {
                        self.engine
                            .instrument_final_buses
                            .get(inst_id)
                            .map(|&bus| (*inst_id, bus, path.clone()))
                    })
                    .collect();

                if instrument_buses.is_empty() {
                    let _ = reply.send(Err("No instrument buses available".to_string()));
                } else {
                    let paths: Vec<PathBuf> =
                        stems.iter().map(|(_, p)| p.clone()).collect();
                    let result = self.engine.start_export_stems(&instrument_buses).map(|_| {
                        let ticks_per_second = (self.piano_roll.bpm / 60.0)
                            * self.piano_roll.ticks_per_beat as f32;
                        self.export_state = Some(ExportState {
                            kind: ExportKind::StemExport,
                            loop_end: self.piano_roll.loop_end,
                            tail_ticks: ticks_per_second as u32,
                            paths,
                        });
                        self.last_export_progress = 0.0;
                    });
                    let _ = reply.send(result);
                }
            }
            AudioCmd::CancelExport => {
                if self.export_state.is_some() {
                    let _ = self.engine.stop_export();
                    self.export_state = None;
                    self.piano_roll.playing = false;
                    self.engine.release_all_voices();
                }
            }
            AudioCmd::ApplyAutomation { target, value } => {
                let _ = self.engine.apply_automation(&target, value, &mut self.instruments, &self.session);
            }
            AudioCmd::QueryVstParams { instrument_id, target } => {
                let node_id = self.resolve_vst_node_id(instrument_id, target);
                let vst_plugin_id = self.resolve_vst_plugin_id(instrument_id, target);
                if let (Some(node_id), Some(vst_plugin_id)) = (node_id, vst_plugin_id) {
                    // Clear any previous replies for this node
                    self.monitor.clear_vst_params(node_id);
                    // Send /param_query 0 256 — VSTPlugin silently ignores out-of-range indices
                    let _ = self.engine.query_vst_params_range(node_id, 0, 256);
                    // Remove any existing query for the same node
                    self.pending_vst_queries.retain(|q| q.node_id != node_id);
                    let now = Instant::now();
                    self.pending_vst_queries.push(PendingVstQuery {
                        instrument_id,
                        target,
                        vst_plugin_id,
                        node_id,
                        started_at: now,
                        last_count: 0,
                        last_change_at: now,
                    });
                }
            }
            AudioCmd::SetVstParam { instrument_id, target, param_index, value } => {
                if let Some(node_id) = self.resolve_vst_node_id(instrument_id, target) {
                    let _ = self.engine.set_vst_param_node(node_id, param_index, value);
                }
            }
            AudioCmd::SaveVstState { instrument_id, target, path } => {
                if let Some(node_id) = self.resolve_vst_node_id(instrument_id, target) {
                    let _ = self.engine.save_vst_state_node(node_id, &path);
                }
                let _ = self.feedback_tx.send(AudioFeedback::VstStateSaved {
                    instrument_id,
                    target,
                    path,
                });
            }
            AudioCmd::LoadVstState { instrument_id, target, path } => {
                if let Some(node_id) = self.resolve_vst_node_id(instrument_id, target) {
                    let _ = self.engine.load_vst_state_node(node_id, &path);
                }
            }
            AudioCmd::Shutdown => return true,
        }
        false
    }

    fn apply_state_update(&mut self, mut instruments: InstrumentSnapshot, session: SessionSnapshot) {
        for new_inst in instruments.instruments.iter_mut() {
            if let Some(old_inst) = self.instruments.instruments.iter().find(|i| i.id == new_inst.id) {
                if let (Some(old_seq), Some(new_seq)) = (&old_inst.drum_sequencer, &mut new_inst.drum_sequencer) {
                    if new_seq.playing {
                        new_seq.current_step = old_seq.current_step;
                        new_seq.step_accumulator = old_seq.step_accumulator;
                        new_seq.last_played_step = old_seq.last_played_step;
                    }
                }
            }
        }
        self.instruments = instruments;
        self.session = session;
    }

    fn apply_piano_roll_update(&mut self, updated: PianoRollSnapshot) {
        let playhead = self.piano_roll.playhead;
        let playing = self.piano_roll.playing;
        self.piano_roll = updated;
        self.piano_roll.playhead = playhead;
        self.piano_roll.playing = playing;
    }

    /// Resolve a VstTarget to a SuperCollider node ID using the instrument snapshot and engine node map
    /// Check pending VST param queries — complete when replies stop arriving or timeout
    fn poll_vst_param_queries(&mut self) {
        let now = Instant::now();
        let mut completed = Vec::new();

        for query in &mut self.pending_vst_queries {
            let current_count = self.monitor.vst_param_count(query.node_id);
            if current_count != query.last_count {
                query.last_count = current_count;
                query.last_change_at = now;
            }
            // Complete if: no new params for 150ms, or total timeout of 2s
            let idle = now.duration_since(query.last_change_at) >= Duration::from_millis(150);
            let timeout = now.duration_since(query.started_at) >= Duration::from_secs(2);
            if (idle && current_count > 0) || timeout {
                completed.push((
                    query.instrument_id,
                    query.target,
                    query.vst_plugin_id,
                    query.node_id,
                ));
            }
        }

        for (instrument_id, target, vst_plugin_id, node_id) in completed {
            self.pending_vst_queries.retain(|q| q.node_id != node_id);
            let replies = self.monitor.take_vst_params(node_id).unwrap_or_default();
            if replies.is_empty() {
                // No replies received — send synthetic placeholders as fallback
                let params: Vec<(u32, String, Option<String>, f32)> = (0..128)
                    .map(|i| (i, format!("Param {}", i), None, 0.5))
                    .collect();
                let _ = self.feedback_tx.send(AudioFeedback::VstParamsDiscovered {
                    instrument_id,
                    target,
                    vst_plugin_id,
                    params,
                });
            } else {
                let mut params: Vec<(u32, String, Option<String>, f32)> = replies.iter()
                    .map(|r| {
                        // Use display string as name for now (Phase 1);
                        // Phase 2 VST3 probing will provide real names
                        let name = if r.display.is_empty() {
                            format!("Param {}", r.index)
                        } else {
                            r.display.clone()
                        };
                        (r.index, name, None, r.value)
                    })
                    .collect();
                params.sort_by_key(|(idx, _, _, _)| *idx);
                let _ = self.feedback_tx.send(AudioFeedback::VstParamsDiscovered {
                    instrument_id,
                    target,
                    vst_plugin_id,
                    params,
                });
            }
        }
    }

    fn resolve_vst_node_id(&self, instrument_id: u32, target: VstTarget) -> Option<i32> {
        let nodes = self.engine.node_map.get(&instrument_id)?;
        match target {
            VstTarget::Source => nodes.source,
            VstTarget::Effect(effect_id) => {
                nodes.effects.get(&effect_id).copied()
            }
        }
    }

    /// Resolve the VstPluginId for a given instrument and target
    fn resolve_vst_plugin_id(&self, instrument_id: u32, target: VstTarget) -> Option<crate::state::vst_plugin::VstPluginId> {
        let inst = self.instruments.instruments.iter()
            .find(|i| i.id == instrument_id)?;
        match target {
            VstTarget::Source => {
                if let crate::state::SourceType::Vst(id) = inst.source {
                    Some(id)
                } else {
                    None
                }
            }
            VstTarget::Effect(effect_id) => {
                inst.effect_by_id(effect_id).and_then(|effect| {
                    if let crate::state::EffectType::Vst(id) = effect.effect_type {
                        Some(id)
                    } else {
                        None
                    }
                })
            }
        }
    }

    fn send_server_status(&self, status: ServerStatus, message: impl Into<String>) {
        let _ = self.feedback_tx.send(AudioFeedback::ServerStatus {
            status,
            message: message.into(),
            server_running: self.engine.server_running(),
        });
    }

    fn load_synthdefs_and_samples(&mut self) -> Result<(), String> {
        let synthdef_dir = crate::paths::synthdefs_dir();
        log::debug!(target: "audio", "Loading synthdefs from: {:?}", synthdef_dir);
        log::debug!(target: "audio", "Path exists: {}", synthdef_dir.exists());

        let builtin_result = self.engine.load_synthdefs(&synthdef_dir);
        log::debug!(target: "audio", "Builtin load result: {:?}", builtin_result);

        let config_dir = crate::paths::custom_synthdefs_dir();
        let custom_result = if config_dir.exists() {
            self.engine.load_synthdefs(&config_dir)
        } else {
            Ok(())
        };

        // Initialize wavetable buffers for VOsc before any voices can play
        let _ = self.engine.initialize_wavetables();

        self.load_drum_samples();

        match (builtin_result, custom_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(e), _) | (_, Err(e)) => Err(e),
        }
    }

    fn load_drum_samples(&mut self) {
        for instrument in &self.instruments.instruments {
            if let Some(seq) = &instrument.drum_sequencer {
                for pad in &seq.pads {
                    if let (Some(buffer_id), Some(path)) = (pad.buffer_id, pad.path.as_ref()) {
                        let _ = self.engine.load_sample(buffer_id, path);
                    }
                }
            }
        }
    }

    fn tick(&mut self, elapsed: Duration) {
        super::playback::tick_playback(
            &mut self.piano_roll,
            &mut self.instruments,
            &self.session,
            &self.automation_lanes,
            &mut self.engine,
            &mut self.active_notes,
            &mut self.arp_states,
            &mut self.rng_state,
            &self.feedback_tx,
            elapsed,
            &mut self.tick_accumulator,
        );

        // Check if render-to-WAV should stop
        let mut render_finished = false;
        if let Some(render) = &self.render_state {
            if self.piano_roll.playhead >= render.loop_end + render.tail_ticks {
                render_finished = true;
            }
        }
        if render_finished {
            let path = self.engine.stop_recording();
            self.piano_roll.playing = false;
            self.engine.release_all_voices();
            if let Some(render) = self.render_state.take() {
                if let Some(wav_path) = path {
                    let _ = self.feedback_tx.send(AudioFeedback::RenderComplete {
                        instrument_id: render.instrument_id,
                        path: wav_path,
                    });
                }
            }
        }

        // Check if export should stop
        let mut export_finished = false;
        if let Some(export) = &self.export_state {
            if self.piano_roll.playhead >= export.loop_end + export.tail_ticks {
                export_finished = true;
            } else {
                // Send progress feedback (throttled to ~2% increments)
                let total = export.loop_end + export.tail_ticks;
                if total > 0 {
                    let progress = self.piano_roll.playhead as f32 / total as f32;
                    if (progress - self.last_export_progress).abs() > 0.02 {
                        self.last_export_progress = progress;
                        let _ = self
                            .feedback_tx
                            .send(AudioFeedback::ExportProgress { progress });
                    }
                }
            }
        }
        if export_finished {
            let paths = self.engine.stop_export();
            self.piano_roll.playing = false;
            self.engine.release_all_voices();
            if let Some(export) = self.export_state.take() {
                let _ = self.feedback_tx.send(AudioFeedback::ExportComplete {
                    kind: export.kind,
                    paths,
                });
            }
        }

        super::drum_tick::tick_drum_sequencer(
            &mut self.instruments,
            &self.session,
            self.piano_roll.bpm,
            &mut self.engine,
            &mut self.rng_state,
            &self.feedback_tx,
            elapsed,
        );
        super::arpeggiator_tick::tick_arpeggiator(
            &self.instruments,
            &self.session,
            self.piano_roll.bpm,
            &mut self.arp_states,
            &mut self.engine,
            &mut self.rng_state,
            elapsed,
        );
    }

    fn poll_engine(&mut self) {
        self.engine.cleanup_expired_voices();

        if let Some(result) = self.engine.poll_compile_result() {
            let result = match result {
                Ok(msg) => {
                    // Auto-reload synthdefs after successful compile
                    let mut reload_msg = msg;
                    let builtin_dir = crate::paths::synthdefs_dir();
                    if builtin_dir.exists() {
                        match self.engine.load_synthdefs(&builtin_dir) {
                            Ok(()) => reload_msg += " — reloaded",
                            Err(e) => reload_msg += &format!(" — reload failed: {e}"),
                        }
                    }
                    // Also reload custom synthdefs from config dir
                    let config_dir = crate::paths::custom_synthdefs_dir();
                    if config_dir.exists() {
                        let _ = self.engine.load_synthdefs(&config_dir);
                    }
                    Ok(reload_msg)
                }
                Err(e) => Err(e),
            };
            let _ = self.feedback_tx.send(AudioFeedback::CompileResult(result));
        }

        // Deferred server connection: wait for scsynth to initialize before connecting
        if let Some(ref pending) = self.pending_server_connect {
            if pending.started_at.elapsed() >= Duration::from_millis(500) {
                let server_addr = pending.server_addr.clone();
                self.pending_server_connect = None;

                // Check if scsynth is still alive after startup
                if let Some(msg) = self.engine.check_server_health() {
                    self.send_server_status(ServerStatus::Error, msg);
                } else {
                    self.send_server_status(ServerStatus::Running, "Server started, connecting...");
                    let connect_result = self.engine.connect_with_monitor(&server_addr, self.monitor.clone());
                    match connect_result {
                        Ok(()) => {
                            let message = match self.load_synthdefs_and_samples() {
                                Ok(()) => "Server restarted".to_string(),
                                Err(e) => format!("Restarted (synthdef warning: {})", e),
                            };
                            self.send_server_status(ServerStatus::Connected, message);
                        }
                        Err(err) => {
                            self.send_server_status(ServerStatus::Error, err.to_string());
                        }
                    }
                }
            }
        }

        if let Some(msg) = self.engine.check_server_health() {
            if self.engine.status() == ServerStatus::Error {
                let _ = self.feedback_tx.send(AudioFeedback::ServerCrashed {
                    message: msg.clone(),
                });
            }
            let _ = self.feedback_tx.send(AudioFeedback::ServerStatus {
                status: self.engine.status(),
                message: msg,
                server_running: self.engine.server_running(),
            });
        }

        // Poll SuperCollider /status for CPU load and latency
        if self.engine.is_running() && self.last_status_poll.elapsed() >= Duration::from_secs(1) {
            self.last_status_poll = Instant::now();
            self.monitor.mark_status_sent();
            self.engine.send_status_query();
        }

        // Poll pending VST param queries for completed OSC replies
        self.poll_vst_param_queries();

        if self.engine.poll_pending_buffer_free() {
            let _ = self.feedback_tx.send(AudioFeedback::PendingBufferFreed);
        }
        self.engine.poll_pending_export_buffer_frees();

        let is_recording = self.engine.is_recording();
        let elapsed_secs = self
            .engine
            .recording_elapsed()
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if is_recording != self.last_recording_state || (is_recording && elapsed_secs != self.last_recording_secs) {
            self.last_recording_state = is_recording;
            self.last_recording_secs = elapsed_secs;
            let _ = self.feedback_tx.send(AudioFeedback::RecordingState {
                is_recording,
                elapsed_secs,
            });
        }
    }
}
