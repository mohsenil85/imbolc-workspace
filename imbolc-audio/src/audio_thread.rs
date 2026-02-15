use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, TryRecvError};

use super::commands::{AudioCmd, AudioFeedback, ExportKind};
use super::engine::server::ServerSpawnResult;
use super::engine::AudioEngine;
use super::event_log::{EventLogReader, LogEntry, LogEntryKind};
use super::osc_client::AudioMonitor;
use super::snapshot::{AutomationSnapshot, InstrumentSnapshot, PianoRollSnapshot, SessionSnapshot};
use super::telemetry::AudioTelemetry;
use super::ServerStatus;
use crate::arp_state::ArpPlayState;
use imbolc_types::VstTarget;
use imbolc_types::{InstrumentId, InstrumentState, SessionState};

/// Deferred server connection: after spawning scsynth, wait before connecting
/// so the server has time to initialize. Avoids blocking the audio thread.
struct PendingServerConnect {
    started_at: Instant,
    server_addr: String,
}

/// Pending async scsynth process spawn (Phase 2: control-plane separation).
struct PendingServerStart {
    rx: std::sync::mpsc::Receiver<Result<ServerSpawnResult, String>>,
    server_addr: String,
    buffer_size: u32,
    sample_rate: u32,
}

/// Pending async OSC connect (Phase 3: control-plane separation).
struct PendingConnect {
    rx: std::sync::mpsc::Receiver<
        Result<Box<dyn super::engine::backend::AudioBackend + Send>, String>,
    >,
    /// Whether this is an initial connect (true) or a restart-connect (false).
    /// Restart-connects show "Server restarted" instead of "Connected".
    is_restart: bool,
    /// Reply channel for direct Connect commands (None for restart-connects).
    reply: Option<std::sync::mpsc::Sender<std::io::Result<()>>>,
}

/// Retry phased routing rebuilds after connect, giving async /d_loadDir time
/// to finish on slower systems before final graph creation.
struct PendingPostConnectRebuild {
    next_try_at: Instant,
    attempts_left: u8,
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
    vst_plugin_id: imbolc_types::VstPluginId,
    node_id: i32,
    started_at: Instant,
    last_count: usize,
    last_change_at: Instant,
}

pub(crate) struct AudioThread {
    engine: AudioEngine,
    /// Priority commands: voice spawn/release, param changes (time-critical)
    priority_rx: Receiver<AudioCmd>,
    /// Normal commands: routing rebuilds, recording control
    normal_rx: Receiver<AudioCmd>,
    /// Event log reader: state synchronization entries
    event_log: EventLogReader,
    feedback_tx: Sender<AudioFeedback>,
    monitor: AudioMonitor,
    instruments: InstrumentSnapshot,
    session: SessionSnapshot,
    piano_roll: PianoRollSnapshot,
    automation_lanes: AutomationSnapshot,
    active_notes: Vec<(InstrumentId, u8, u32)>, // (instrument_id, pitch, duration_ticks)
    last_tick: Instant,
    last_recording_secs: u64,
    last_recording_state: bool,
    /// Simple LCG random seed for probability/humanization
    rng_state: u64,
    /// Per-instrument arpeggiator runtime state
    arp_states: HashMap<InstrumentId, ArpPlayState>,
    /// Per-voice generative engine runtime state
    generative_states: super::generative_state::GenerativePlayState,
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
    /// Pending async scsynth spawn (background thread)
    pending_server_start: Option<PendingServerStart>,
    /// Pending async OSC connect (background thread)
    pending_connect: Option<PendingConnect>,
    /// In-progress phased routing rebuild (driven one step per poll_engine())
    routing_rebuild: Option<super::engine::routing::RoutingRebuildPhase>,
    /// Retry routing rebuild after connect so synthdefs loaded via /d_loadDir
    /// are available before instrument and meter nodes are recreated.
    pending_post_connect_rebuild: Option<PendingPostConnectRebuild>,
    /// In-flight VST parameter queries awaiting OSC replies
    pending_vst_queries: Vec<PendingVstQuery>,
    /// Tuner tone node ID (if currently playing)
    tuner_node_id: Option<i32>,
    /// Click track state (enabled, volume, muted)
    click_state: imbolc_types::ClickTrackState,
    /// Click track beat accumulator (fractional beats since last click)
    click_accumulator: f64,
    /// High-water mark for piano roll pre-scheduling.
    /// Tracks the furthest tick already scheduled, so the next tick only
    /// schedules notes beyond this point. Reset to None on playhead changes
    /// or piano roll edits.
    last_scheduled_tick: Option<u32>,
    /// Telemetry collector for tick duration metrics
    telemetry: AudioTelemetry,
    /// Last time telemetry was emitted
    last_telemetry_emit: Instant,
    /// Last time voice cleanup was performed (rate-limited to reduce overhead)
    last_voice_cleanup: Instant,
    /// Last time server health was checked (rate-limited to reduce overhead)
    last_health_check: Instant,
}

impl AudioThread {
    pub(crate) fn new(
        priority_rx: Receiver<AudioCmd>,
        normal_rx: Receiver<AudioCmd>,
        event_log: EventLogReader,
        feedback_tx: Sender<AudioFeedback>,
        monitor: AudioMonitor,
    ) -> Self {
        Self {
            engine: AudioEngine::new(),
            priority_rx,
            normal_rx,
            event_log,
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
            generative_states: Default::default(),
            render_state: None,
            export_state: None,
            last_export_progress: 0.0,
            tick_accumulator: 0.0,
            last_status_poll: Instant::now(),
            pending_server_connect: None,
            pending_server_start: None,
            pending_connect: None,
            routing_rebuild: None,
            pending_post_connect_rebuild: None,
            pending_vst_queries: Vec::new(),
            tuner_node_id: None,
            click_state: imbolc_types::ClickTrackState::default(),
            click_accumulator: 0.0,
            last_scheduled_tick: None,
            telemetry: AudioTelemetry::new(),
            last_telemetry_emit: Instant::now(),
            last_voice_cleanup: Instant::now(),
            last_health_check: Instant::now(),
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
            // Then drain event log (state synchronization)
            self.drain_event_log();
            // Then drain normal commands
            if self.drain_normal_commands() {
                break;
            }

            let now = Instant::now();
            let elapsed = now.duration_since(self.last_tick);
            if elapsed >= TICK_INTERVAL {
                self.last_tick = now;

                // Record tick timing for telemetry
                let tick_start = Instant::now();
                self.tick(elapsed);
                self.telemetry
                    .record(tick_start.elapsed(), TICK_INTERVAL.as_micros() as u32);
            }

            self.poll_engine();
        }
    }

    /// Drain priority commands first (voice spawn, param changes)
    /// Uses time-budgeted draining: stops when time budget OR count limit is reached
    fn drain_priority_commands(&mut self) -> bool {
        const MAX_DURATION: Duration = Duration::from_micros(200);
        const MAX_COUNT: usize = 128;

        let start = Instant::now();
        for _ in 0..MAX_COUNT {
            // Check time budget before processing each command
            if start.elapsed() >= MAX_DURATION {
                break;
            }
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

    /// Drain normal commands (state sync, routing, bulk mixer updates)
    /// Uses time-budgeted draining: stops when time budget OR count limit is reached
    fn drain_normal_commands(&mut self) -> bool {
        const MAX_DURATION: Duration = Duration::from_micros(100);
        const MAX_COUNT: usize = 64;

        let start = Instant::now();
        for _ in 0..MAX_COUNT {
            // Check time budget before processing each command
            if start.elapsed() >= MAX_DURATION {
                break;
            }
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
        use AudioCmd::*;
        match cmd {
            // Server lifecycle
            Connect { .. }
            | Disconnect
            | StartServer { .. }
            | StopServer
            | RestartServer { .. }
            | CompileSynthDefs { .. }
            | LoadSynthDefs { .. }
            | LoadSynthDefFile { .. } => self.handle_server_cmd(cmd),

            // Playback control
            SetPlaying { .. }
            | ResetPlayhead
            | SetBpm { .. }
            | SetClickEnabled { .. }
            | SetClickVolume { .. }
            | SetClickMuted { .. }
            | StartTunerTone { .. }
            | StopTunerTone => self.handle_playback_cmd(cmd),

            // Routing & mixing parameters
            RebuildRouting
            | RebuildInstrumentRouting { .. }
            | UpdateMixerParams
            | SetMasterParams { .. }
            | SetInstrumentMixerParams { .. }
            | SetBusMixerParams { .. }
            | SetLayerGroupMixerParams { .. }
            | SetSourceParam { .. }
            | SetEqParam { .. }
            | SetFilterParam { .. }
            | SetEffectParam { .. }
            | SetLfoParam { .. }
            | SetBusEffectParam { .. }
            | SetLayerGroupEffectParam { .. }
            | SetLayerGroupEqParam { .. }
            | ApplyAutomation { .. } => self.handle_mixer_cmd(cmd),

            // Voice management
            SpawnVoice { .. }
            | ReleaseVoice { .. }
            | RegisterActiveNote { .. }
            | ClearActiveNotes
            | ReleaseAllVoices
            | PlayDrumHit { .. } => self.handle_voice_cmd(cmd),

            // Sample & recording
            LoadSample { .. }
            | FreeSamples { .. }
            | StartInstrumentRender { .. }
            | StartRecording { .. }
            | StopRecording { .. }
            | StartMasterBounce { .. }
            | StartStemExport { .. }
            | CancelExport => self.handle_recording_cmd(cmd),

            // VST parameters
            QueryVstParams { .. }
            | SetVstParam { .. }
            | SaveVstState { .. }
            | LoadVstState { .. } => self.handle_vst_cmd(cmd),

            // Shutdown (special case - returns true)
            Shutdown => return true,
        }
        false
    }

    // =========================================================================
    // Server lifecycle commands
    // =========================================================================

    fn handle_server_cmd(&mut self, cmd: AudioCmd) {
        match cmd {
            AudioCmd::Connect { server_addr, reply } => {
                if self.pending_connect.is_some() {
                    let _ = reply.send(Err(std::io::Error::other("Connect already in progress")));
                } else {
                    self.pending_post_connect_rebuild = None;
                    let rx =
                        AudioEngine::connect_with_monitor_async(server_addr, self.monitor.clone());
                    self.pending_connect = Some(PendingConnect {
                        rx,
                        is_restart: false,
                        reply: Some(reply),
                    });
                    self.send_server_status(ServerStatus::Starting, "Connecting...");
                }
            }
            AudioCmd::Disconnect => {
                self.pending_connect = None;
                self.routing_rebuild = None;
                self.pending_post_connect_rebuild = None;
                self.engine.disconnect();
                self.send_server_status(self.engine.status(), "Disconnected");
            }
            AudioCmd::StartServer {
                input_device,
                output_device,
                buffer_size,
                sample_rate,
                scsynth_args,
                reply,
            } => {
                if self.pending_server_start.is_some() {
                    let _ = reply.send(Err("Server start already in progress".to_string()));
                } else {
                    let result = self.engine.start_server_with_devices(
                        input_device.as_deref(),
                        output_device.as_deref(),
                        buffer_size,
                        sample_rate,
                        &scsynth_args,
                    );
                    if result.is_ok() {
                        self.monitor.set_audio_latency(buffer_size, sample_rate);
                        self.engine.set_lookahead(buffer_size, sample_rate);
                    }
                    match &result {
                        Ok(()) => self.send_server_status(ServerStatus::Running, "Server started"),
                        Err(err) => self.send_server_status(ServerStatus::Error, err),
                    }
                    let _ = reply.send(result);
                }
            }
            AudioCmd::StopServer => {
                self.pending_server_start = None;
                self.pending_connect = None;
                self.routing_rebuild = None;
                self.pending_post_connect_rebuild = None;
                self.engine.stop_server();
                self.send_server_status(ServerStatus::Stopped, "Server stopped");
            }
            AudioCmd::RestartServer {
                input_device,
                output_device,
                server_addr,
                buffer_size,
                sample_rate,
                scsynth_args,
            } => {
                self.engine.stop_server();
                self.pending_server_connect = None;
                self.pending_server_start = None;
                self.pending_connect = None;
                self.routing_rebuild = None;
                self.pending_post_connect_rebuild = None;
                self.send_server_status(ServerStatus::Stopped, "Restarting server...");

                match self.engine.start_server_async(
                    input_device,
                    output_device,
                    buffer_size,
                    scsynth_args,
                ) {
                    Ok(rx) => {
                        self.pending_server_start = Some(PendingServerStart {
                            rx,
                            server_addr,
                            buffer_size,
                            sample_rate,
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
            AudioCmd::LoadSynthDefs { dir } => {
                let result = self
                    .engine
                    .load_synthdefs(&dir)
                    .map(|()| format!("Loaded synthdefs from {}", dir.display()));
                let _ = self.feedback_tx.send(AudioFeedback::LoadResult(result));
            }
            AudioCmd::LoadSynthDefFile { path } => {
                let result = self
                    .engine
                    .load_synthdef_file(&path)
                    .map(|()| format!("Loaded synthdef {}", path.display()));
                let _ = self.feedback_tx.send(AudioFeedback::LoadResult(result));
            }
            _ => {}
        }
    }

    // =========================================================================
    // Event log (state synchronization)
    // =========================================================================

    /// Drain event log entries within a 100µs budget.
    fn drain_event_log(&mut self) {
        const BUDGET: Duration = Duration::from_micros(100);
        let entries = self.event_log.drain(BUDGET);
        for entry in &entries {
            self.apply_log_entry(entry);
        }
    }

    /// Apply a single event log entry — same logic as the former handle_state_cmd().
    fn apply_log_entry(&mut self, entry: &LogEntry) {
        match &entry.kind {
            LogEntryKind::Action {
                action,
                rebuild_routing,
                rebuild_instrument_routing,
                add_instrument_routing,
                delete_instrument_routing,
                rebuild_bus_processing,
                mixer_dirty,
            } => {
                let reduced = imbolc_types::reduce::reduce_action(
                    action,
                    &mut self.instruments,
                    &mut self.session,
                );
                if !reduced {
                    log::debug!(target: "audio::reduce", "unreducible action: {:?}", std::mem::discriminant(&**action));
                }

                if *rebuild_routing {
                    self.routing_rebuild =
                        Some(super::engine::routing::RoutingRebuildPhase::TearDown);
                } else {
                    // Targeted routing operations (only if no full rebuild)
                    if let Some(id) = delete_instrument_routing {
                        let _ = self.engine.delete_instrument_routing(*id);
                    }
                    if let Some(id) = add_instrument_routing {
                        let _ = self.engine.add_instrument_routing(
                            *id,
                            &self.instruments,
                            &self.session,
                        );
                    }
                    for id in rebuild_instrument_routing.iter().flatten() {
                        let _ = self.engine.rebuild_single_instrument_routing(
                            *id,
                            &self.instruments,
                            &self.session,
                        );
                    }
                    if *rebuild_bus_processing {
                        let _ = self
                            .engine
                            .rebuild_bus_processing(&self.instruments, &self.session);
                    }
                }
                if *mixer_dirty {
                    let _ = self
                        .engine
                        .update_all_instrument_mixer_params(&self.instruments, &self.session);
                }
            }
            LogEntryKind::Checkpoint {
                instruments,
                session,
                piano_roll,
                automation_lanes,
                rebuild_routing,
            } => {
                // Preserve drum sequencer playback state from old instruments
                let mut instruments = instruments.clone();
                let old_instruments: HashMap<InstrumentId, &_> = self
                    .instruments
                    .instruments
                    .iter()
                    .map(|i| (i.id, i))
                    .collect();
                for new_inst in instruments.instruments.iter_mut() {
                    if let Some(old_inst) = old_instruments.get(&new_inst.id) {
                        if let (Some(old_seq), Some(new_seq)) =
                            (old_inst.drum_sequencer(), new_inst.drum_sequencer_mut())
                        {
                            if new_seq.playing {
                                new_seq.current_step = old_seq.current_step;
                                new_seq.step_accumulator = old_seq.step_accumulator;
                                new_seq.last_played_step = old_seq.last_played_step;
                            }
                        }
                    }
                }
                self.instruments = instruments;
                self.session = session.clone();
                // Preserve runtime state (playhead, playing)
                let playhead = self.piano_roll.playhead;
                let playing = self.piano_roll.playing;
                self.piano_roll = piano_roll.clone();
                self.piano_roll.playhead = playhead;
                self.piano_roll.playing = playing;
                self.automation_lanes = automation_lanes.clone();
                if *rebuild_routing {
                    self.routing_rebuild =
                        Some(super::engine::routing::RoutingRebuildPhase::TearDown);
                }
                self.last_scheduled_tick = None;
            }
            LogEntryKind::PianoRollUpdate(piano_roll) => {
                self.apply_piano_roll_update(piano_roll.clone());
                self.last_scheduled_tick = None;
            }
            LogEntryKind::AutomationUpdate(lanes) => {
                self.automation_lanes = lanes.clone();
            }
        }
    }

    // =========================================================================
    // Playback control commands
    // =========================================================================

    fn handle_playback_cmd(&mut self, cmd: AudioCmd) {
        match cmd {
            AudioCmd::SetPlaying { playing } => {
                self.piano_roll.playing = playing;
                let _ = self
                    .feedback_tx
                    .send(AudioFeedback::PlayingChanged(playing));
                self.last_scheduled_tick = None;
                if playing {
                    self.tick_accumulator = 0.0;
                    self.click_accumulator = 0.0;
                }
            }
            AudioCmd::ResetPlayhead => {
                self.piano_roll.playhead = 0;
                self.tick_accumulator = 0.0;
                self.click_accumulator = 0.0;
                self.last_scheduled_tick = None;
                let _ = self.feedback_tx.send(AudioFeedback::PlayheadPosition(0));
            }
            AudioCmd::SetBpm { bpm } => {
                self.piano_roll.bpm = bpm;
                let _ = self.feedback_tx.send(AudioFeedback::BpmUpdate(bpm));
            }
            AudioCmd::SetClickEnabled { enabled } => {
                self.click_state.enabled = enabled;
                if enabled {
                    // Reset accumulator when enabling to start fresh
                    self.click_accumulator = 0.0;
                }
            }
            AudioCmd::SetClickVolume { volume } => {
                self.click_state.volume = volume.clamp(0.0, 1.0);
            }
            AudioCmd::SetClickMuted { muted } => {
                self.click_state.muted = muted;
            }
            AudioCmd::StartTunerTone { freq } => {
                if let Some(node_id) = self.tuner_node_id {
                    // Tone already playing — just update frequency
                    self.engine.set_node_param(node_id, "freq", freq);
                } else if let Some(node_id) = self.engine.create_tuner_synth(freq) {
                    self.tuner_node_id = Some(node_id);
                }
            }
            AudioCmd::StopTunerTone => {
                if let Some(node_id) = self.tuner_node_id.take() {
                    // Set gate=0 for graceful release (doneAction:2 will self-free)
                    self.engine.set_node_param(node_id, "gate", 0.0);
                }
            }
            _ => {}
        }
    }

    // =========================================================================
    // Routing & mixing parameter commands
    // =========================================================================

    fn handle_mixer_cmd(&mut self, cmd: AudioCmd) {
        match cmd {
            AudioCmd::RebuildRouting => {
                // Start (or restart) the phased routing rebuild state machine.
                // Work is amortized across ticks in poll_engine().
                self.routing_rebuild = Some(super::engine::routing::RoutingRebuildPhase::TearDown);
            }
            AudioCmd::RebuildInstrumentRouting { instrument_id } => {
                let _ = self.engine.rebuild_single_instrument_routing(
                    instrument_id,
                    &self.instruments,
                    &self.session,
                );
            }
            AudioCmd::UpdateMixerParams => {
                let _ = self
                    .engine
                    .update_all_instrument_mixer_params(&self.instruments, &self.session);
            }
            AudioCmd::SetMasterParams { level, mute } => {
                self.session.mixer.master_level = level;
                self.session.mixer.master_mute = mute;
            }
            AudioCmd::SetInstrumentMixerParams {
                instrument_id,
                level,
                pan,
                mute,
                solo,
            } => {
                if let Some(inst) = self
                    .instruments
                    .instruments
                    .iter_mut()
                    .find(|i| i.id == instrument_id)
                {
                    inst.mixer.level = level;
                    inst.mixer.pan = pan;
                    inst.mixer.mute = mute;
                    inst.mixer.solo = solo;
                }
            }
            AudioCmd::SetBusMixerParams {
                bus_id,
                level,
                mute,
                pan,
            } => {
                let _ = self.engine.set_bus_mixer_params(bus_id, level, mute, pan);
            }
            AudioCmd::SetLayerGroupMixerParams {
                group_id,
                level,
                mute,
                pan,
            } => {
                let _ = self
                    .engine
                    .set_layer_group_mixer_params(group_id, level, mute, pan);
            }
            AudioCmd::SetSourceParam {
                instrument_id,
                param,
                value,
            } => {
                let _ = self.engine.set_source_param(instrument_id, &param, value);
            }
            AudioCmd::SetEqParam {
                instrument_id,
                param,
                value,
            } => {
                let _ = self.engine.set_eq_param(instrument_id, &param, value);
            }
            AudioCmd::SetFilterParam {
                instrument_id,
                param,
                value,
            } => {
                let _ = self.engine.set_filter_param(instrument_id, &param, value);
            }
            AudioCmd::SetEffectParam {
                instrument_id,
                effect_id,
                param,
                value,
            } => {
                let _ = self
                    .engine
                    .set_effect_param(instrument_id, effect_id, &param, value);
            }
            AudioCmd::SetLfoParam {
                instrument_id,
                param,
                value,
            } => {
                let _ = self.engine.set_lfo_param(instrument_id, &param, value);
            }
            AudioCmd::SetBusEffectParam {
                bus_id,
                effect_id,
                param,
                value,
            } => {
                let _ = self
                    .engine
                    .set_bus_effect_param(bus_id, effect_id, &param, value);
            }
            AudioCmd::SetLayerGroupEffectParam {
                group_id,
                effect_id,
                param,
                value,
            } => {
                let _ = self
                    .engine
                    .set_layer_group_effect_param(group_id, effect_id, &param, value);
            }
            AudioCmd::SetLayerGroupEqParam {
                group_id,
                param,
                value,
            } => {
                let _ = self
                    .engine
                    .set_layer_group_eq_param(group_id, &param, value);
            }
            AudioCmd::ApplyAutomation { target, value } => {
                let _ = self.engine.apply_automation(
                    &target,
                    value,
                    &mut self.instruments,
                    &self.session,
                );
            }
            _ => {}
        }
    }

    // =========================================================================
    // Voice management commands
    // =========================================================================

    fn handle_voice_cmd(&mut self, cmd: AudioCmd) {
        match cmd {
            AudioCmd::SpawnVoice {
                instrument_id,
                pitch,
                velocity,
                offset_secs,
            } => {
                let _ = self.engine.spawn_voice(
                    instrument_id,
                    pitch,
                    velocity,
                    offset_secs,
                    &self.instruments,
                    &self.session,
                );
            }
            AudioCmd::ReleaseVoice {
                instrument_id,
                pitch,
                offset_secs,
            } => {
                let _ =
                    self.engine
                        .release_voice(instrument_id, pitch, offset_secs, &self.instruments);
            }
            AudioCmd::RegisterActiveNote {
                instrument_id,
                pitch,
                duration_ticks,
            } => {
                self.active_notes
                    .push((instrument_id, pitch, duration_ticks));
            }
            AudioCmd::ClearActiveNotes => {
                self.active_notes.clear();
            }
            AudioCmd::ReleaseAllVoices => {
                self.engine.release_all_voices();
            }
            AudioCmd::PlayDrumHit {
                buffer_id,
                amp,
                instrument_id,
                slice_start,
                slice_end,
                rate,
                offset_secs,
            } => {
                let _ = self.engine.play_drum_hit_to_instrument(
                    buffer_id,
                    amp,
                    instrument_id,
                    slice_start,
                    slice_end,
                    rate,
                    offset_secs,
                );
            }
            _ => {}
        }
    }

    // =========================================================================
    // Sample & recording commands
    // =========================================================================

    fn handle_recording_cmd(&mut self, cmd: AudioCmd) {
        match cmd {
            AudioCmd::LoadSample {
                buffer_id,
                path,
                reply,
            } => {
                let result = self.engine.load_sample(buffer_id, &path);
                let _ = reply.send(result);
            }
            AudioCmd::FreeSamples { buffer_ids } => {
                for id in buffer_ids {
                    let _ = self.engine.free_sample(id);
                }
            }
            AudioCmd::StartInstrumentRender {
                instrument_id,
                path,
                reply,
            } => {
                let result =
                    if let Some(&bus) = self.engine.instrument_final_buses.get(&instrument_id) {
                        self.engine.start_recording(bus, &path).map(|_| {
                            self.render_state = Some(RenderState {
                                instrument_id,
                                loop_end: self.piano_roll.loop_end,
                                tail_ticks: self.calculate_tail_ticks(),
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
                    self.export_state = Some(ExportState {
                        kind: ExportKind::MasterBounce,
                        loop_end: self.piano_roll.loop_end,
                        tail_ticks: self.calculate_tail_ticks(),
                        paths: vec![path],
                    });
                    self.last_export_progress = 0.0;
                });
                let _ = reply.send(result);
            }
            AudioCmd::StartStemExport { stems, reply } => {
                let instrument_buses: Vec<(InstrumentId, i32, PathBuf)> = stems
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
                    let paths: Vec<PathBuf> = stems.iter().map(|(_, p)| p.clone()).collect();
                    let result = self.engine.start_export_stems(&instrument_buses).map(|_| {
                        self.export_state = Some(ExportState {
                            kind: ExportKind::StemExport,
                            loop_end: self.piano_roll.loop_end,
                            tail_ticks: self.calculate_tail_ticks(),
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
                    let _ = self.feedback_tx.send(AudioFeedback::PlayingChanged(false));
                    self.engine.release_all_voices();
                }
            }
            _ => {}
        }
    }

    // =========================================================================
    // VST parameter commands
    // =========================================================================

    fn handle_vst_cmd(&mut self, cmd: AudioCmd) {
        match cmd {
            AudioCmd::QueryVstParams {
                instrument_id,
                target,
            } => {
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
            AudioCmd::SetVstParam {
                instrument_id,
                target,
                param_index,
                value,
            } => {
                if let Some(node_id) = self.resolve_vst_node_id(instrument_id, target) {
                    let _ = self.engine.set_vst_param_node(node_id, param_index, value);
                }
            }
            AudioCmd::SaveVstState {
                instrument_id,
                target,
                path,
            } => {
                if let Some(node_id) = self.resolve_vst_node_id(instrument_id, target) {
                    let _ = self.engine.save_vst_state_node(node_id, &path);
                }
                let _ = self.feedback_tx.send(AudioFeedback::VstStateSaved {
                    instrument_id,
                    target,
                    path,
                });
            }
            AudioCmd::LoadVstState {
                instrument_id,
                target,
                path,
            } => {
                if let Some(node_id) = self.resolve_vst_node_id(instrument_id, target) {
                    let _ = self.engine.load_vst_state_node(node_id, &path);
                }
            }
            _ => {}
        }
    }

    /// Calculate tail duration in ticks (1 second of tail time)
    fn calculate_tail_ticks(&self) -> u32 {
        let ticks_per_second = (self.piano_roll.bpm / 60.0) * self.piano_roll.ticks_per_beat as f32;
        ticks_per_second as u32
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
        // Drain any new VST param replies from the channel into accumulated storage
        self.monitor.drain_vst_param_channel();

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
                let mut params: Vec<(u32, String, Option<String>, f32)> = replies
                    .iter()
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

    fn resolve_vst_node_id(&self, instrument_id: InstrumentId, target: VstTarget) -> Option<i32> {
        let nodes = self.engine.node_map.get(&instrument_id)?;
        match target {
            VstTarget::Source => nodes.source,
            VstTarget::Effect(effect_id) => nodes.effects.get(&effect_id).copied(),
        }
    }

    /// Resolve the VstPluginId for a given instrument and target
    fn resolve_vst_plugin_id(
        &self,
        instrument_id: InstrumentId,
        target: VstTarget,
    ) -> Option<imbolc_types::VstPluginId> {
        let inst = self
            .instruments
            .instruments
            .iter()
            .find(|i| i.id == instrument_id)?;
        match target {
            VstTarget::Source => {
                if let imbolc_types::SourceType::Vst(id) = inst.source {
                    Some(id)
                } else {
                    None
                }
            }
            VstTarget::Effect(effect_id) => inst.effect_by_id(effect_id).and_then(|effect| {
                if let imbolc_types::EffectType::Vst(id) = effect.effect_type {
                    Some(id)
                } else {
                    None
                }
            }),
        }
    }

    fn send_server_status(&self, status: ServerStatus, message: impl Into<String>) {
        let _ = self.feedback_tx.send(AudioFeedback::ServerStatus {
            status,
            message: message.into(),
            server_running: self.engine.server_running(),
        });
    }

    fn schedule_post_connect_rebuilds(&mut self) {
        // Linux/PipeWire can take noticeably longer to finish async /d_loadDir.
        // A few delayed retries avoids a startup race with missing synthdefs.
        let attempts = if cfg!(target_os = "linux") { 8 } else { 3 };
        self.pending_post_connect_rebuild = Some(PendingPostConnectRebuild {
            next_try_at: Instant::now() + Duration::from_millis(350),
            attempts_left: attempts,
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
            if let Some(seq) = instrument.drum_sequencer() {
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
            &mut self.session,
            &self.automation_lanes,
            &mut self.engine,
            &mut self.active_notes,
            &mut self.arp_states,
            &mut self.rng_state,
            &self.feedback_tx,
            elapsed,
            &mut self.tick_accumulator,
            &mut self.last_scheduled_tick,
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
            let _ = self.feedback_tx.send(AudioFeedback::PlayingChanged(false));
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
            let _ = self.feedback_tx.send(AudioFeedback::PlayingChanged(false));
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
        super::generative_tick::tick_generative(
            &self.instruments,
            &self.session,
            self.piano_roll.bpm,
            &mut self.generative_states,
            &mut self.engine,
            &mut self.rng_state,
            elapsed,
            &self.feedback_tx,
        );
        super::click_tick::tick_click(
            &mut self.engine,
            &self.click_state,
            &self.session,
            &self.piano_roll,
            elapsed,
            &mut self.click_accumulator,
        );
    }

    fn poll_engine(&mut self) {
        // Process /n_end notifications for authoritative voice cleanup
        let ended_nodes = self.monitor.drain_node_ends();
        if !ended_nodes.is_empty() {
            self.engine.process_node_ends(&ended_nodes);
        }

        // Rate-limit timer-based voice cleanup to every 100ms (fallback safety net)
        if self.last_voice_cleanup.elapsed() >= Duration::from_millis(100) {
            self.last_voice_cleanup = Instant::now();
            self.engine.cleanup_expired_voices();
        }

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

        // Poll async server spawn (Phase 2: off-thread start_server)
        if let Some(ref pending) = self.pending_server_start {
            match pending.rx.try_recv() {
                Ok(Ok(result)) => {
                    let buffer_size = pending.buffer_size;
                    let sample_rate = pending.sample_rate;
                    let server_addr = pending.server_addr.clone();
                    self.pending_server_start = None;

                    self.engine.install_server_child(result);
                    self.monitor.set_audio_latency(buffer_size, sample_rate);
                    self.engine.set_lookahead(buffer_size, sample_rate);
                    self.send_server_status(ServerStatus::Running, "Server started, connecting...");

                    // Chain into deferred connect (let scsynth initialize before OSC connect)
                    self.pending_server_connect = Some(PendingServerConnect {
                        started_at: Instant::now(),
                        server_addr,
                    });
                }
                Ok(Err(msg)) => {
                    self.pending_server_start = None;
                    self.engine.set_status(ServerStatus::Error);
                    self.send_server_status(ServerStatus::Error, msg);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.pending_server_start = None;
                    self.engine.set_status(ServerStatus::Error);
                    self.send_server_status(
                        ServerStatus::Error,
                        "Server spawn thread terminated unexpectedly",
                    );
                }
            }
        }

        // Deferred server connection: wait for scsynth to initialize before connecting
        if let Some(ref pending) = self.pending_server_connect {
            if pending.started_at.elapsed() >= Duration::from_millis(500) {
                let server_addr = pending.server_addr.clone();
                self.pending_server_connect = None;

                // Check if scsynth is still alive after startup
                if let Some(msg) = self.engine.check_server_health() {
                    self.send_server_status(ServerStatus::Error, msg);
                } else if self.pending_connect.is_none() {
                    // Spawn async connect (Phase 3: off-thread connect)
                    self.send_server_status(ServerStatus::Running, "Server started, connecting...");
                    let rx =
                        AudioEngine::connect_with_monitor_async(server_addr, self.monitor.clone());
                    self.pending_connect = Some(PendingConnect {
                        rx,
                        is_restart: true,
                        reply: None,
                    });
                }
            }
        }

        // Poll async connect (Phase 3: off-thread connect_with_monitor)
        if let Some(ref pending) = self.pending_connect {
            match pending.rx.try_recv() {
                Ok(Ok(backend)) => {
                    let is_restart = pending.is_restart;
                    let reply = self.pending_connect.take().unwrap().reply;
                    self.engine.install_backend(backend);
                    let message = match self.load_synthdefs_and_samples() {
                        Ok(()) => {
                            if is_restart {
                                "Server restarted".to_string()
                            } else {
                                "Connected".to_string()
                            }
                        }
                        Err(e) => {
                            if is_restart {
                                format!("Restarted (synthdef warning: {})", e)
                            } else {
                                format!("Connected (synthdef warning: {})", e)
                            }
                        }
                    };
                    self.send_server_status(ServerStatus::Connected, message);
                    self.schedule_post_connect_rebuilds();
                    if let Some(reply) = reply {
                        let _ = reply.send(Ok(()));
                    }
                }
                Ok(Err(msg)) => {
                    let reply = self.pending_connect.take().unwrap().reply;
                    self.pending_post_connect_rebuild = None;
                    self.engine.set_status(ServerStatus::Error);
                    self.send_server_status(ServerStatus::Error, &msg);
                    if let Some(reply) = reply {
                        let _ = reply.send(Err(std::io::Error::other(msg)));
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    let reply = self.pending_connect.take().unwrap().reply;
                    self.pending_post_connect_rebuild = None;
                    self.engine.set_status(ServerStatus::Error);
                    self.send_server_status(
                        ServerStatus::Error,
                        "Connect thread terminated unexpectedly",
                    );
                    if let Some(reply) = reply {
                        let _ = reply.send(Err(std::io::Error::other(
                            "Connect thread terminated unexpectedly",
                        )));
                    }
                }
            }
        }

        // /d_loadDir is asynchronous; retry rebuild a few times after connect so
        // slow machines still converge on a valid graph + meter.
        if let Some(mut pending) = self.pending_post_connect_rebuild.take() {
            let now = Instant::now();
            if now < pending.next_try_at {
                self.pending_post_connect_rebuild = Some(pending);
            } else if !self.engine.is_running() {
                // Connection dropped before retries completed.
            } else if self.routing_rebuild.is_some() {
                // Let the in-progress rebuild finish before scheduling another.
                pending.next_try_at = now + Duration::from_millis(100);
                self.pending_post_connect_rebuild = Some(pending);
            } else {
                self.routing_rebuild = Some(super::engine::routing::RoutingRebuildPhase::TearDown);
                if pending.attempts_left > 1 {
                    pending.attempts_left -= 1;
                    pending.next_try_at = now + Duration::from_millis(500);
                    self.pending_post_connect_rebuild = Some(pending);
                }
            }
        }

        // Drive phased routing rebuild (Phase 4: amortized across ticks)
        if let Some(phase) = self.routing_rebuild.take() {
            use super::engine::routing::RebuildStepResult;
            match self
                .engine
                .routing_rebuild_step(phase, &self.instruments, &self.session)
            {
                Ok(RebuildStepResult::Continue(next)) => {
                    self.routing_rebuild = Some(next);
                }
                Ok(RebuildStepResult::Done) => {
                    // Rebuild complete
                }
                Err(e) => {
                    log::warn!(target: "audio", "Routing rebuild phase failed: {}", e);
                    // Abandon rebuild on error
                }
            }
        }

        // Rate-limit server health checks to every 5s (reduces syscall overhead)
        if self.last_health_check.elapsed() >= Duration::from_secs(5) {
            self.last_health_check = Instant::now();
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
        }

        // Poll SuperCollider /status for CPU load and latency
        if self.engine.is_running() && self.last_status_poll.elapsed() >= Duration::from_secs(1) {
            self.last_status_poll = Instant::now();
            self.monitor.mark_status_sent();
            self.engine.send_status_query();
        }

        // Emit telemetry summary every 1s
        if self.last_telemetry_emit.elapsed() >= Duration::from_secs(1) {
            self.last_telemetry_emit = Instant::now();
            let (avg_tick_us, max_tick_us, p95_tick_us, overruns) = self.telemetry.take_summary();
            let _ = self.feedback_tx.send(AudioFeedback::TelemetrySummary {
                avg_tick_us,
                max_tick_us,
                p95_tick_us,
                overruns,
                schedule_lookahead_ms: (self.engine.schedule_lookahead_secs * 1000.0) as f32,
                osc_send_queue_depth: self.engine.osc_send_queue_depth(),
            });
            let _ = self
                .feedback_tx
                .send(AudioFeedback::TuningDrift(self.engine.last_drift_cents));
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

        if is_recording != self.last_recording_state
            || (is_recording && elapsed_secs != self.last_recording_secs)
        {
            self.last_recording_state = is_recording;
            self.last_recording_secs = elapsed_secs;
            let _ = self.feedback_tx.send(AudioFeedback::RecordingState {
                is_recording,
                elapsed_secs,
            });
        }
    }
}
