mod automation;
pub mod backend;
pub(crate) mod node_registry;
mod recording;
pub(crate) mod routing;
mod samples;
pub(crate) mod server;
pub(crate) mod voice_allocator;
mod voices;
mod vst;

use std::collections::HashMap;
use std::process::Child;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use backend::AudioBackend;
use super::bus_allocator::BusAllocator;
use crate::state::{BufferId, EffectId, InstrumentId};
use node_registry::NodeRegistry;
use voice_allocator::VoiceAllocator;

use recording::{ExportRecordingState, RecordingState};

#[allow(dead_code)]
pub type ModuleId = u32;

// SuperCollider group IDs for execution ordering
pub const GROUP_SOURCES: i32 = 100;
pub const GROUP_PROCESSING: i32 = 200;
pub const GROUP_OUTPUT: i32 = 300;
pub const GROUP_RECORD: i32 = 400;
pub const GROUP_SAFETY: i32 = 999;

// Wavetable buffer range for VOsc (imbolc_wavetable SynthDef)
pub const WAVETABLE_BUFNUM_START: i32 = 100;
pub const WAVETABLE_NUM_TABLES: i32 = 8;

/// Minimum scheduling lookahead (10ms floor).
pub const MIN_LOOKAHEAD_SECS: f64 = 0.010;

/// Jitter margin added on top of buffer-derived latency (5ms).
pub const JITTER_MARGIN_SECS: f64 = 0.005;

/// Default scheduling lookahead (15ms) — used when buffer_size/sample_rate unknown.
pub const DEFAULT_LOOKAHEAD_SECS: f64 = 0.015;

/// Compute the scheduling lookahead from audio buffer parameters.
///
/// Formula: `max(buffer_size / sample_rate + JITTER_MARGIN, MIN_LOOKAHEAD)`
///
/// This gives enough headroom for the audio driver to consume the current buffer
/// before the scheduled event's timetag arrives, plus a jitter margin for tick
/// scheduling variance.
pub fn compute_lookahead(buffer_size: u32, sample_rate: u32) -> f64 {
    if sample_rate == 0 {
        return DEFAULT_LOOKAHEAD_SECS;
    }
    let buffer_latency = buffer_size as f64 / sample_rate as f64;
    (buffer_latency + JITTER_MARGIN_SECS).max(MIN_LOOKAHEAD_SECS)
}

pub use imbolc_types::ServerStatus;

/// VSTPlugin UGen index within wrapper SynthDefs (imbolc_vst_instrument, imbolc_vst_effect).
/// This is 0 because VSTPlugin is the first (and only) UGen in our wrappers.
const VST_UGEN_INDEX: i32 = 0;

/// A polyphonic voice chain: entire signal chain spawned per note
#[derive(Debug, Clone)]
pub struct VoiceChain {
    pub instrument_id: InstrumentId,
    pub pitch: u8,
    pub velocity: f32,
    pub group_id: i32,
    pub midi_node_id: i32,
    pub source_node: i32,
    pub spawn_time: Instant,
    /// If set, voice has been released: (released_at, release_duration_secs)
    pub release_state: Option<(Instant, f32)>,
    /// Per-voice control bus triple (freq, gate, vel) for pool return
    pub control_buses: (i32, i32, i32),
}

#[derive(Debug, Clone)]
pub struct InstrumentNodes {
    pub source: Option<i32>,
    pub lfo: Option<i32>,
    pub filter: Option<i32>,
    pub eq: Option<i32>,
    pub effects: HashMap<EffectId, i32>,
    /// Ordered list of effect IDs matching the signal chain order (only enabled effects)
    pub effect_order: Vec<EffectId>,
    pub output: i32,
}

impl InstrumentNodes {
    pub fn all_node_ids(&self) -> Vec<i32> {
        let mut ids = Vec::new();
        if let Some(id) = self.source { ids.push(id); }
        if let Some(id) = self.lfo { ids.push(id); }
        if let Some(id) = self.filter { ids.push(id); }
        if let Some(id) = self.eq { ids.push(id); }
        for eid in &self.effect_order {
            if let Some(&nid) = self.effects.get(eid) {
                ids.push(nid);
            }
        }
        ids.push(self.output);
        ids
    }
}

pub struct AudioEngine {
    backend: Option<Box<dyn AudioBackend>>,
    pub(crate) node_map: HashMap<InstrumentId, InstrumentNodes>,
    next_node_id: i32,
    is_running: bool,
    scsynth_process: Option<Child>,
    server_status: ServerStatus,
    compile_receiver: Option<Receiver<Result<String, String>>>,
    is_compiling: bool,
    bus_allocator: BusAllocator,
    groups_created: bool,
    /// Dedicated audio bus per mixer bus (bus_id -> SC audio bus index)
    bus_audio_buses: HashMap<u8, i32>,
    /// Send synth nodes: (instrument_id, bus_id) -> node_id
    send_node_map: HashMap<(InstrumentId, u8), i32>,
    /// Bus output synth nodes: bus_id -> node_id
    bus_node_map: HashMap<u8, i32>,
    /// Layer group audio buses: group_id -> SC audio bus index
    layer_group_audio_buses: HashMap<u32, i32>,
    /// Layer group output synth nodes: group_id -> node_id
    layer_group_node_map: HashMap<u32, i32>,
    /// Layer group send synth nodes: (group_id, bus_id) -> node_id
    layer_group_send_node_map: HashMap<(u32, u8), i32>,
    /// Instrument final buses: instrument_id -> SC audio bus index (post-effects, pre-mixer)
    pub(crate) instrument_final_buses: HashMap<InstrumentId, i32>,
    /// Voice allocation, tracking, stealing, and control bus pooling
    pub(crate) voice_allocator: VoiceAllocator,
    /// Safety limiter synth node ID (persistent, never freed during routing rebuilds)
    safety_node_id: Option<i32>,
    /// Meter synth node ID
    meter_node_id: Option<i32>,
    /// Analysis synth node IDs (spectrum, LUFS, scope)
    analysis_node_ids: Vec<i32>,
    /// Sample buffer mapping: BufferId -> SuperCollider buffer number
    buffer_map: HashMap<BufferId, i32>,
    /// Next available buffer number for SuperCollider
    #[allow(dead_code)]
    next_bufnum: i32,
    /// Whether wavetable buffers (100–107) have been initialized
    wavetables_initialized: bool,
    /// Active disk recording session
    recording: Option<RecordingState>,
    /// Buffer pending free after recording stop (bufnum, when to free)
    pending_buffer_free: Option<(i32, Instant)>,
    /// Active export session (master bounce or stem export)
    export_state: Option<ExportRecordingState>,
    /// Buffers pending free after export stop
    pending_export_buffer_frees: Vec<(i32, Instant)>,
    /// Best-effort registry of which SC nodes are believed to be alive
    pub(crate) node_registry: NodeRegistry,
    /// One-shot voice group_id -> control bus triple (for bus return on /n_end)
    pub(crate) oneshot_buses: HashMap<i32, (i32, i32, i32)>,
    /// Dynamic scheduling lookahead for sequenced playback.
    /// Derived from buffer_size/sample_rate via `compute_lookahead()`.
    pub schedule_lookahead_secs: f64,
    /// OSC sender thread channel (None when no backend or test backend).
    osc_send_tx: Option<crossbeam_channel::Sender<super::osc_sender::OscSendEntry>>,
    /// Atomic queue depth counter for telemetry.
    osc_queue_depth: Option<std::sync::Arc<std::sync::atomic::AtomicUsize>>,
    /// Join handle for the OSC sender thread.
    osc_sender_handle: Option<std::thread::JoinHandle<()>>,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            backend: None,
            node_map: HashMap::new(),
            next_node_id: 1000,
            is_running: false,
            scsynth_process: None,
            server_status: ServerStatus::Stopped,
            compile_receiver: None,
            is_compiling: false,
            bus_allocator: BusAllocator::new(),
            groups_created: false,
            bus_audio_buses: HashMap::new(),
            send_node_map: HashMap::new(),
            bus_node_map: HashMap::new(),
            layer_group_audio_buses: HashMap::new(),
            layer_group_node_map: HashMap::new(),
            layer_group_send_node_map: HashMap::new(),
            instrument_final_buses: HashMap::new(),
            voice_allocator: VoiceAllocator::new(),
            safety_node_id: None,
            meter_node_id: None,
            analysis_node_ids: Vec::new(),
            buffer_map: HashMap::new(),
            next_bufnum: WAVETABLE_BUFNUM_START + WAVETABLE_NUM_TABLES, // Start after wavetable range
            wavetables_initialized: false,
            recording: None,
            pending_buffer_free: None,
            export_state: None,
            pending_export_buffer_frees: Vec::new(),
            node_registry: NodeRegistry::new(),
            oneshot_buses: HashMap::new(),
            schedule_lookahead_secs: DEFAULT_LOOKAHEAD_SECS,
            osc_send_tx: None,
            osc_queue_depth: None,
            osc_sender_handle: None,
        }
    }

    /// Update the scheduling lookahead based on audio device parameters.
    pub fn set_lookahead(&mut self, buffer_size: u32, sample_rate: u32) {
        self.schedule_lookahead_secs = compute_lookahead(buffer_size, sample_rate);
    }

    /// Start the OSC sender thread using a cloned socket from the backend.
    pub fn start_osc_sender(&mut self) {
        if self.osc_send_tx.is_some() {
            return; // Already running
        }
        if let Some(ref backend) = self.backend {
            if let (Some(socket), Some(addr)) = (backend.try_clone_socket(), backend.server_socket_addr()) {
                let (tx, depth, handle) = super::osc_sender::spawn_osc_sender(socket, addr);
                self.osc_send_tx = Some(tx);
                self.osc_queue_depth = Some(depth);
                self.osc_sender_handle = Some(handle);
            }
        }
    }

    /// Stop the OSC sender thread (called on disconnect/shutdown).
    pub fn stop_osc_sender(&mut self) {
        // Drop the sender to signal the thread to exit
        self.osc_send_tx = None;
        self.osc_queue_depth = None;
        if let Some(handle) = self.osc_sender_handle.take() {
            let _ = handle.join();
        }
    }

    /// Queue a pre-built timed bundle for the OSC sender thread.
    ///
    /// Encodes BackendMessages into an OSC bundle with the given offset, then
    /// pushes to the sender thread. Falls back to direct backend send if no
    /// sender thread is running or the queue is full.
    pub fn queue_timed_bundle(&self, messages: Vec<backend::BackendMessage>, offset_secs: f64) -> Result<(), String> {
        if messages.is_empty() {
            return Ok(());
        }

        // Try the sender thread path first
        if let (Some(tx), Some(depth)) = (&self.osc_send_tx, &self.osc_queue_depth) {
            // Encode the bundle
            let time = super::osc_client::osc_time_from_now(offset_secs);
            let osc_messages: Vec<rosc::OscMessage> = messages
                .iter()
                .map(|m| rosc::OscMessage {
                    addr: m.addr.clone(),
                    args: m.args.iter().cloned().map(backend::raw_to_osc_pub).collect(),
                })
                .collect();
            let content = osc_messages.into_iter().map(rosc::OscPacket::Message).collect();
            let bundle = rosc::OscPacket::Bundle(rosc::OscBundle {
                timetag: time,
                content,
            });
            match rosc::encoder::encode(&bundle) {
                Ok(encoded) => {
                    if super::osc_sender::try_queue_bundle(tx, depth, encoded) {
                        return Ok(());
                    }
                    // Fall through to direct send
                }
                Err(e) => {
                    return Err(format!("OSC encode error: {e}"));
                }
            }
        }

        // Fallback: direct send via backend (tests, NullBackend, or queue full)
        let backend = self.backend.as_ref().ok_or("Not connected")?;
        backend.send_bundle(messages, offset_secs).map_err(|e| e.to_string())
    }

    /// Get current OSC send queue depth for telemetry.
    pub fn osc_send_queue_depth(&self) -> u16 {
        self.osc_queue_depth
            .as_ref()
            .map(|d| d.load(std::sync::atomic::Ordering::Relaxed) as u16)
            .unwrap_or(0)
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn status(&self) -> ServerStatus {
        self.server_status
    }

    pub(crate) fn set_status(&mut self, status: ServerStatus) {
        self.server_status = status;
    }

    pub fn server_running(&self) -> bool {
        self.scsynth_process.is_some()
    }

    #[allow(dead_code)]
    pub fn is_compiling(&self) -> bool {
        self.is_compiling
    }

    /// Create a simple synth on bus 0 (hardware out) for the tuner reference tone.
    /// Returns the allocated node ID, or None if not connected.
    pub fn create_tuner_synth(&mut self, freq: f32) -> Option<i32> {
        let backend = self.backend.as_ref()?;
        let node_id = self.next_node_id;
        self.next_node_id += 1;
        let params = vec![
            ("freq".to_string(), freq),
            ("amp".to_string(), 0.3),
            ("gate".to_string(), 1.0),
        ];
        let _ = backend.create_synth("imbolc_tuner_tone", node_id, 0, &params);
        Some(node_id)
    }

    /// Update a parameter on a node (e.g. tuner freq or gate).
    pub fn set_node_param(&self, node_id: i32, param: &str, value: f32) {
        if let Some(ref backend) = self.backend {
            let _ = backend.set_param(node_id, param, value);
        }
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.stop_osc_sender();
        self.stop_server();
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::voice_allocator::MAX_VOICES_PER_INSTRUMENT;
    use crate::audio::engine::backend::NullBackend;
    use crate::state::{AppState, AutomationTarget, FilterConfig, ParamValue};
    use crate::state::instrument::{EffectType, FilterType, SourceType};

    fn connect_engine() -> AudioEngine {
        let mut engine = AudioEngine::new();
        engine.backend = Some(Box::new(NullBackend));
        engine.is_running = true;
        engine.server_status = ServerStatus::Connected;
        engine
    }

    #[test]
    fn rebuild_routing_creates_nodes_for_audio_in_with_effects_and_sends() {
        let mut engine = connect_engine();
        let mut state = AppState::new();

        let inst_id = state.add_instrument(SourceType::AudioIn);
        if let Some(inst) = state.instruments.instrument_mut(inst_id) {
            inst.filter = Some(FilterConfig::new(FilterType::Lpf));
            inst.lfo.enabled = true;
            inst.add_effect(EffectType::Delay);
            inst.sends[0].enabled = true;
            inst.sends[0].level = 0.5;
        }

        engine
            .rebuild_instrument_routing(&state.instruments, &state.session)
            .expect("rebuild routing");

        let nodes = engine.node_map.get(&inst_id).expect("nodes");
        assert!(nodes.source.is_some());
        assert!(nodes.filter.is_some());
        assert!(nodes.lfo.is_some());
        assert_eq!(nodes.effects.len(), 1);
        assert!(engine.send_node_map.contains_key(&(inst_id, 1)));
        assert_eq!(engine.bus_node_map.len(), state.session.mixer.buses.len());
    }

    #[test]
    fn rebuild_routing_handles_bus_in_with_sidechain_effect() {
        let mut engine = connect_engine();
        let mut state = AppState::new();

        let inst_id = state.add_instrument(SourceType::BusIn);
        if let Some(inst) = state.instruments.instrument_mut(inst_id) {
            let effect_id = inst.add_effect(EffectType::SidechainComp);
            if let Some(effect) = inst.effect_by_id_mut(effect_id) {
                if let Some(param) = effect.params.iter_mut().find(|p| p.name == "sc_bus") {
                    param.value = ParamValue::Int(1);
                }
            }
        }

        engine
            .rebuild_instrument_routing(&state.instruments, &state.session)
            .expect("rebuild routing");

        let nodes = engine.node_map.get(&inst_id).expect("nodes");
        assert!(nodes.source.is_some());
        assert_eq!(nodes.effects.len(), 1);
    }

    #[test]
    fn apply_automation_covers_all_targets() {
        let mut engine = connect_engine();
        let mut state = AppState::new();

        let inst_id = state.add_instrument(SourceType::Saw);
        if let Some(inst) = state.instruments.instrument_mut(inst_id) {
            inst.filter = Some(FilterConfig::new(FilterType::Hpf));
            let disabled_id = inst.add_effect(EffectType::Delay);
            if let Some(disabled) = inst.effect_by_id_mut(disabled_id) {
                disabled.enabled = false;
            }
            inst.add_effect(EffectType::Reverb);
        }

        engine
            .rebuild_instrument_routing(&state.instruments, &state.session)
            .expect("rebuild routing");

        engine.voice_allocator.add(VoiceChain {
            instrument_id: inst_id,
            pitch: 60,
            velocity: 0.8,
            group_id: 0,
            midi_node_id: 0,
            source_node: 1234,
            spawn_time: Instant::now(),
            release_state: None,
            control_buses: (0, 0, 0),
        });

        engine
            .apply_automation(
                &AutomationTarget::level(inst_id),
                0.5,
                &mut state.instruments,
                &state.session,
            )
            .unwrap();
        engine
            .apply_automation(
                &AutomationTarget::pan(inst_id),
                -0.25,
                &mut state.instruments,
                &state.session,
            )
            .unwrap();
        engine
            .apply_automation(
                &AutomationTarget::filter_cutoff(inst_id),
                800.0,
                &mut state.instruments,
                &state.session,
            )
            .unwrap();
        engine
            .apply_automation(
                &AutomationTarget::filter_resonance(inst_id),
                0.5,
                &mut state.instruments,
                &state.session,
            )
            .unwrap();
        engine
            .apply_automation(
                &AutomationTarget::effect_param(inst_id, 1, 0),
                0.7,
                &mut state.instruments,
                &state.session,
            )
            .unwrap();
        engine
            .apply_automation(
                &AutomationTarget::sample_rate(inst_id),
                1.2,
                &mut state.instruments,
                &state.session,
            )
            .unwrap();
        engine
            .apply_automation(
                &AutomationTarget::sample_amp(inst_id),
                0.8,
                &mut state.instruments,
                &state.session,
            )
            .unwrap();
        // Envelope targets — update state and active voices
        engine
            .apply_automation(
                &AutomationTarget::attack(inst_id),
                0.05,
                &mut state.instruments,
                &state.session,
            )
            .unwrap();
        engine
            .apply_automation(
                &AutomationTarget::decay(inst_id),
                0.2,
                &mut state.instruments,
                &state.session,
            )
            .unwrap();
        engine
            .apply_automation(
                &AutomationTarget::sustain(inst_id),
                0.7,
                &mut state.instruments,
                &state.session,
            )
            .unwrap();
        engine
            .apply_automation(
                &AutomationTarget::release(inst_id),
                1.5,
                &mut state.instruments,
                &state.session,
            )
            .unwrap();

        // Verify envelope state was mutated
        let env = &state.instruments.instrument(inst_id).unwrap().amp_envelope;
        assert!((env.attack - 0.05).abs() < f32::EPSILON);
        assert!((env.decay - 0.2).abs() < f32::EPSILON);
        assert!((env.sustain - 0.7).abs() < f32::EPSILON);
        assert!((env.release - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn set_source_param_bus_translates_bus_id() {
        let mut engine = connect_engine();
        let mut state = AppState::new();

        let inst_id = state.add_instrument(SourceType::BusIn);
        engine
            .rebuild_instrument_routing(&state.instruments, &state.session)
            .expect("rebuild routing");

        engine
            .set_source_param(inst_id, "bus", 1.0)
            .expect("set_source_param");
    }

    #[test]
    fn set_bus_mixer_params_uses_bus_nodes() {
        let mut engine = connect_engine();
        let mut state = AppState::new();
        state.add_instrument(SourceType::Saw);

        engine
            .rebuild_instrument_routing(&state.instruments, &state.session)
            .expect("rebuild routing");

        engine
            .set_bus_mixer_params(1, 0.5, false, 0.0)
            .expect("set_bus_mixer_params");
    }

    fn make_voice(inst_id: InstrumentId, pitch: u8, velocity: f32, age_ms: u64) -> VoiceChain {
        VoiceChain {
            instrument_id: inst_id,
            pitch,
            velocity,
            group_id: 0,
            midi_node_id: 0,
            source_node: 0,
            spawn_time: Instant::now() - std::time::Duration::from_millis(age_ms),
            release_state: None,
            control_buses: (0, 0, 0),
        }
    }

    fn make_released_voice(
        inst_id: InstrumentId,
        pitch: u8,
        velocity: f32,
        released_ago_ms: u64,
        release_dur: f32,
    ) -> VoiceChain {
        VoiceChain {
            instrument_id: inst_id,
            pitch,
            velocity,
            group_id: 0,
            midi_node_id: 0,
            source_node: 0,
            spawn_time: Instant::now() - std::time::Duration::from_millis(released_ago_ms + 100),
            release_state: Some((
                Instant::now() - std::time::Duration::from_millis(released_ago_ms),
                release_dur,
            )),
            control_buses: (0, 0, 0),
        }
    }

    #[test]
    fn test_same_pitch_retrigger() {
        let mut engine = connect_engine();
        let inst_id = 1;

        // Add a voice at pitch 60
        engine.voice_allocator.add(make_voice(inst_id, 60, 0.8, 100));

        // Steal for a new note at the same pitch
        engine
            .steal_voice_if_needed(inst_id, 60, 0.9)
            .expect("steal");

        // The old voice should be removed
        assert!(
            engine.voice_allocator.chains().iter().all(|v| !(v.instrument_id == inst_id && v.pitch == 60)),
            "same-pitch voice should have been stolen"
        );
    }

    #[test]
    fn test_released_voices_stolen_first() {
        let mut engine = connect_engine();
        let inst_id = 1;

        // Fill to limit with active voices
        for i in 0..MAX_VOICES_PER_INSTRUMENT {
            engine.voice_allocator.add(make_voice(inst_id, 40 + i as u8, 0.8, 100));
        }
        // Add an extra released voice (active count is already at limit)
        engine.voice_allocator.add(make_released_voice(inst_id, 80, 0.8, 500, 1.0));

        // Trigger steal
        engine
            .steal_voice_if_needed(inst_id, 90, 0.8)
            .expect("steal");

        // The released voice should be gone, not any active voice
        assert!(
            !engine.voice_allocator.chains().iter().any(|v| v.pitch == 80 && v.instrument_id == inst_id),
            "released voice should be stolen before active voices"
        );
        // All original active voices should still be present
        assert_eq!(
            engine.voice_allocator.chains().iter().filter(|v| v.instrument_id == inst_id).count(),
            MAX_VOICES_PER_INSTRUMENT,
        );
    }

    #[test]
    fn test_lowest_velocity_stolen() {
        let mut engine = connect_engine();
        let inst_id = 1;

        // Fill to limit — all same age, varying velocity
        for i in 0..MAX_VOICES_PER_INSTRUMENT {
            let vel = 0.2 + (i as f32 * 0.05); // 0.2, 0.25, 0.30, ...
            engine.voice_allocator.add(make_voice(inst_id, 40 + i as u8, vel, 100));
        }
        let quietest_pitch = engine.voice_allocator.chains()[0].pitch; // velocity 0.2

        engine
            .steal_voice_if_needed(inst_id, 90, 0.8)
            .expect("steal");

        assert!(
            !engine.voice_allocator.chains().iter().any(|v| v.pitch == quietest_pitch && v.instrument_id == inst_id),
            "lowest velocity voice should be stolen"
        );
    }

    #[test]
    fn test_age_tiebreaker() {
        let mut engine = connect_engine();
        let inst_id = 1;

        // Fill to limit — all same velocity, varying age
        for i in 0..MAX_VOICES_PER_INSTRUMENT {
            let age = 1000 - (i as u64 * 50); // oldest first: 1000, 950, 900, ...
            engine.voice_allocator.add(make_voice(inst_id, 40 + i as u8, 0.5, age));
        }
        let oldest_pitch = engine.voice_allocator.chains()[0].pitch; // age 1000ms

        engine
            .steal_voice_if_needed(inst_id, 90, 0.5)
            .expect("steal");

        assert!(
            !engine.voice_allocator.chains().iter().any(|v| v.pitch == oldest_pitch && v.instrument_id == inst_id),
            "oldest voice should be stolen as tiebreaker"
        );
    }

    #[test]
    fn test_cleanup_expired_voices() {
        let mut engine = connect_engine();
        let inst_id = 1;

        // Add a voice released long ago (should be cleaned up)
        engine.voice_allocator.add(make_released_voice(inst_id, 60, 0.5, 5000, 0.5));
        // Add a voice released recently (should be kept)
        engine.voice_allocator.add(make_released_voice(inst_id, 72, 0.5, 100, 1.0));
        // Add an active voice (should be kept)
        engine.voice_allocator.add(make_voice(inst_id, 48, 0.8, 200));

        engine.cleanup_expired_voices();

        assert_eq!(engine.voice_allocator.chains().len(), 2);
        assert!(engine.voice_allocator.chains().iter().any(|v| v.pitch == 72));
        assert!(engine.voice_allocator.chains().iter().any(|v| v.pitch == 48));
        assert!(!engine.voice_allocator.chains().iter().any(|v| v.pitch == 60));
    }

    #[test]
    fn test_process_node_ends_removes_voice() {
        let mut engine = connect_engine();
        let inst_id = 1;
        let group_id = 5000;
        let midi_node_id = 5001;
        let source_node = 5002;
        let buses = engine.voice_allocator.alloc_control_buses();

        engine.node_registry.register(group_id);
        engine.node_registry.register(midi_node_id);
        engine.node_registry.register(source_node);

        engine.voice_allocator.add(VoiceChain {
            instrument_id: inst_id,
            pitch: 60,
            velocity: 0.8,
            group_id,
            midi_node_id,
            source_node,
            spawn_time: Instant::now(),
            release_state: None,
            control_buses: buses,
        });

        assert_eq!(engine.voice_allocator.chains().len(), 1);
        assert_eq!(engine.voice_allocator.control_bus_pool_size(), 0);

        // Simulate /n_end for the group
        engine.process_node_ends(&[group_id]);

        assert_eq!(engine.voice_allocator.chains().len(), 0);
        assert_eq!(engine.voice_allocator.control_bus_pool_size(), 1);
        assert!(!engine.node_registry.is_live(group_id));
        assert!(!engine.node_registry.is_live(midi_node_id));
        assert!(!engine.node_registry.is_live(source_node));
    }

    #[test]
    fn test_process_node_ends_returns_oneshot_buses() {
        let mut engine = connect_engine();
        let group_id = 6000;
        let buses = engine.voice_allocator.alloc_control_buses();

        engine.node_registry.register(group_id);
        engine.oneshot_buses.insert(group_id, buses);

        assert_eq!(engine.voice_allocator.control_bus_pool_size(), 0);

        engine.process_node_ends(&[group_id]);

        assert_eq!(engine.voice_allocator.control_bus_pool_size(), 1);
        assert!(engine.oneshot_buses.is_empty());
        assert!(!engine.node_registry.is_live(group_id));
    }

    #[test]
    fn test_process_node_ends_unknown_node() {
        let mut engine = connect_engine();
        let node_id = 7000;
        engine.node_registry.register(node_id);

        // Unknown node — just unregister
        engine.process_node_ends(&[node_id]);
        assert!(!engine.node_registry.is_live(node_id));
    }

    mod backend_routing_tests {
        use super::*;
        use crate::audio::engine::backend::{TestBackend, TestOp, SharedTestBackend};
        use crate::state::instrument::OutputTarget;
        use crate::state::SendTapPoint;
        use std::sync::Arc;

        fn engine_with_test_backend() -> (AudioEngine, Arc<TestBackend>) {
            let backend = Arc::new(TestBackend::new());
            let mut engine = AudioEngine::new();
            engine.backend = Some(Box::new(SharedTestBackend(Arc::clone(&backend))));
            engine.is_running = true;
            engine.server_status = ServerStatus::Connected;
            (engine, backend)
        }

        #[test]
        fn routing_creates_correct_synth_chain_for_saw() {
            let (mut engine, backend) = engine_with_test_backend();
            let mut state = AppState::new();
            let inst_id = state.add_instrument(SourceType::Saw);
            engine.rebuild_instrument_routing(&state.instruments, &state.session).expect("rebuild routing");
            let nodes = engine.node_map.get(&inst_id).expect("nodes must exist");
            assert!(nodes.source.is_none(), "oscillator instruments have no persistent source");
            assert!(nodes.lfo.is_none(), "LFO disabled by default");
            assert!(nodes.filter.is_none(), "no filter by default");
            let synths = backend.synths_created();
            let output_synth = synths.iter().find(|op| matches!(op, TestOp::CreateSynth { def_name, group_id, .. } if def_name == "imbolc_output" && *group_id == GROUP_OUTPUT));
            assert!(output_synth.is_some(), "output synth must be created in GROUP_OUTPUT");
            let bus_out_count = backend.count(|op| matches!(op, TestOp::CreateSynth { def_name, .. } if def_name == "imbolc_bus_out"));
            assert_eq!(bus_out_count, state.session.mixer.buses.len(), "one bus output synth per mixer bus");
        }

        #[test]
        fn routing_creates_filter_and_effects_for_audio_in() {
            let (mut engine, backend) = engine_with_test_backend();
            let mut state = AppState::new();
            let inst_id = state.add_instrument(SourceType::AudioIn);
            if let Some(inst) = state.instruments.instrument_mut(inst_id) {
                inst.filter = Some(FilterConfig::new(FilterType::Lpf));
                inst.add_effect(EffectType::Delay);
                inst.add_effect(EffectType::Reverb);
            }
            engine.rebuild_instrument_routing(&state.instruments, &state.session).expect("rebuild routing");
            let nodes = engine.node_map.get(&inst_id).expect("nodes");
            assert!(nodes.source.is_some(), "AudioIn has a persistent source");
            assert!(nodes.filter.is_some(), "filter was added");
            assert_eq!(nodes.effects.len(), 2, "two effects were added");
            let synths = backend.synths_created();
            assert!(synths.iter().any(|op| matches!(op, TestOp::CreateSynth { def_name, .. } if def_name == "imbolc_audio_in")));
            assert!(synths.iter().any(|op| matches!(op, TestOp::CreateSynth { def_name, .. } if def_name == "imbolc_lpf")));
            assert!(synths.iter().any(|op| matches!(op, TestOp::CreateSynth { def_name, .. } if def_name == "imbolc_delay")));
            assert!(synths.iter().any(|op| matches!(op, TestOp::CreateSynth { def_name, .. } if def_name == "imbolc_reverb")));
            assert!(synths.iter().any(|op| matches!(op, TestOp::CreateSynth { def_name, .. } if def_name == "imbolc_output")));
        }

        #[test]
        fn routing_creates_send_synths() {
            let (mut engine, backend) = engine_with_test_backend();
            let mut state = AppState::new();
            let inst_id = state.add_instrument(SourceType::AudioIn);
            if let Some(inst) = state.instruments.instrument_mut(inst_id) {
                inst.sends[0].enabled = true;
                inst.sends[0].level = 0.5;
            }
            engine.rebuild_instrument_routing(&state.instruments, &state.session).expect("rebuild routing");
            let send_count = backend.count(|op| matches!(op, TestOp::CreateSynth { def_name, .. } if def_name == "imbolc_send"));
            assert_eq!(send_count, 1, "one send synth for the enabled send");
            assert!(engine.send_node_map.contains_key(&(inst_id, 1)), "send node registered for bus 1");
        }

        #[test]
        fn routing_buses_are_chained_correctly() {
            let (mut engine, backend) = engine_with_test_backend();
            let mut state = AppState::new();
            let inst_id = state.add_instrument(SourceType::AudioIn);
            if let Some(inst) = state.instruments.instrument_mut(inst_id) {
                inst.filter = Some(FilterConfig::new(FilterType::Hpf));
                inst.add_effect(EffectType::Delay);
            }
            engine.rebuild_instrument_routing(&state.instruments, &state.session).expect("rebuild routing");
            let synths = backend.synths_created();
            let source_out = synths.iter().find_map(|op| if let TestOp::CreateSynth { def_name, params, .. } = op { if def_name == "imbolc_audio_in" { params.iter().find(|(k, _)| k == "out").map(|(_, v)| *v) } else { None } } else { None }).expect("source out bus");
            let filter_in = synths.iter().find_map(|op| if let TestOp::CreateSynth { def_name, params, .. } = op { if def_name == "imbolc_hpf" { params.iter().find(|(k, _)| k == "in").map(|(_, v)| *v) } else { None } } else { None }).expect("filter in bus");
            assert_eq!(source_out, filter_in, "filter input bus must match source output bus");
            let filter_out = synths.iter().find_map(|op| if let TestOp::CreateSynth { def_name, params, .. } = op { if def_name == "imbolc_hpf" { params.iter().find(|(k, _)| k == "out").map(|(_, v)| *v) } else { None } } else { None }).expect("filter out bus");
            let delay_in = synths.iter().find_map(|op| if let TestOp::CreateSynth { def_name, params, .. } = op { if def_name == "imbolc_delay" { params.iter().find(|(k, _)| k == "in").map(|(_, v)| *v) } else { None } } else { None }).expect("delay in bus");
            assert_eq!(filter_out, delay_in, "delay input bus must match filter output bus");
            let delay_out = synths.iter().find_map(|op| if let TestOp::CreateSynth { def_name, params, .. } = op { if def_name == "imbolc_delay" { params.iter().find(|(k, _)| k == "out").map(|(_, v)| *v) } else { None } } else { None }).expect("delay out bus");
            let output_in = synths.iter().find_map(|op| if let TestOp::CreateSynth { def_name, params, .. } = op { if def_name == "imbolc_output" { params.iter().find(|(k, _)| k == "in").map(|(_, v)| *v) } else { None } } else { None }).expect("output in bus");
            assert_eq!(delay_out, output_in, "output input bus must match delay output bus");
        }

        #[test]
        fn rebuild_frees_old_nodes() {
            let (mut engine, backend) = engine_with_test_backend();
            let mut state = AppState::new();
            let inst_id = state.add_instrument(SourceType::AudioIn);
            engine.rebuild_instrument_routing(&state.instruments, &state.session).expect("first build");
            let first_output_node = engine.node_map.get(&inst_id).unwrap().output;
            backend.clear();
            engine.rebuild_instrument_routing(&state.instruments, &state.session).expect("second build");
            let freed = backend.nodes_freed();
            assert!(freed.contains(&first_output_node), "old output node should be freed on rebuild");
            let new_output_node = engine.node_map.get(&inst_id).unwrap().output;
            assert_ne!(first_output_node, new_output_node, "new output node should be a different ID");
        }

        #[test]
        fn disabled_effects_are_not_created() {
            let (mut engine, backend) = engine_with_test_backend();
            let mut state = AppState::new();
            let inst_id = state.add_instrument(SourceType::Saw);
            if let Some(inst) = state.instruments.instrument_mut(inst_id) {
                let eid = inst.add_effect(EffectType::Delay);
                if let Some(effect) = inst.effect_by_id_mut(eid) { effect.enabled = false; }
                inst.add_effect(EffectType::Reverb);
            }
            engine.rebuild_instrument_routing(&state.instruments, &state.session).expect("rebuild routing");
            let nodes = engine.node_map.get(&inst_id).expect("nodes");
            assert_eq!(nodes.effects.len(), 1, "only enabled effects get nodes");
            assert_eq!(backend.count(|op| matches!(op, TestOp::CreateSynth { def_name, .. } if def_name == "imbolc_delay")), 0);
            assert_eq!(backend.count(|op| matches!(op, TestOp::CreateSynth { def_name, .. } if def_name == "imbolc_reverb")), 1);
        }

        #[test]
        fn set_param_records_operation() {
            let (mut engine, backend) = engine_with_test_backend();
            let mut state = AppState::new();
            let inst_id = state.add_instrument(SourceType::AudioIn);
            engine.rebuild_instrument_routing(&state.instruments, &state.session).expect("rebuild routing");
            engine.set_source_param(inst_id, "gain", 0.75).expect("set_source_param");
            let set_ops = backend.find(|op| matches!(op, TestOp::SetParam { param, value, .. } if param == "gain" && (*value - 0.75).abs() < 0.001));
            assert!(set_ops.is_some(), "set_param for gain=0.75 should be recorded");
        }

        #[test]
        fn groups_are_created_on_first_routing() {
            let (mut engine, backend) = engine_with_test_backend();
            let mut state = AppState::new();
            state.add_instrument(SourceType::Saw);
            engine.rebuild_instrument_routing(&state.instruments, &state.session).expect("rebuild routing");
            let group_count = backend.count(|op| matches!(op, TestOp::CreateGroup { .. }));
            assert_eq!(group_count, 5, "five execution groups");
            assert!(backend.find(|op| matches!(op, TestOp::CreateGroup { group_id, .. } if *group_id == GROUP_SOURCES)).is_some());
            assert!(backend.find(|op| matches!(op, TestOp::CreateGroup { group_id, .. } if *group_id == GROUP_PROCESSING)).is_some());
            assert!(backend.find(|op| matches!(op, TestOp::CreateGroup { group_id, .. } if *group_id == GROUP_OUTPUT)).is_some());
            assert!(backend.find(|op| matches!(op, TestOp::CreateGroup { group_id, .. } if *group_id == GROUP_RECORD)).is_some());
            assert!(backend.find(|op| matches!(op, TestOp::CreateGroup { group_id, .. } if *group_id == GROUP_SAFETY)).is_some());
        }

        #[test]
        fn muted_instrument_creates_output_with_mute_flag() {
            let (mut engine, backend) = engine_with_test_backend();
            let mut state = AppState::new();
            let inst_id = state.add_instrument(SourceType::Saw);
            if let Some(inst) = state.instruments.instrument_mut(inst_id) { inst.mute = true; }
            engine.rebuild_instrument_routing(&state.instruments, &state.session).expect("rebuild routing");
            let synths = backend.synths_created();
            let output = synths.iter().find(|op| matches!(op, TestOp::CreateSynth { def_name, .. } if def_name == "imbolc_output"));
            assert!(output.is_some(), "output synth created");
            if let Some(TestOp::CreateSynth { params, .. }) = output {
                assert_eq!(params.iter().find(|(k, _)| k == "mute").map(|(_, v)| *v), Some(1.0));
            }
        }

        #[test]
        fn lfo_creates_synth_with_correct_params() {
            let (mut engine, backend) = engine_with_test_backend();
            let mut state = AppState::new();
            let inst_id = state.add_instrument(SourceType::AudioIn);
            if let Some(inst) = state.instruments.instrument_mut(inst_id) {
                inst.lfo.enabled = true;
                inst.lfo.target = crate::state::ParameterTarget::Pan;
                inst.lfo.rate = 2.0;
                inst.lfo.depth = 0.5;
            }
            engine.rebuild_instrument_routing(&state.instruments, &state.session).expect("rebuild routing");
            let nodes = engine.node_map.get(&inst_id).expect("nodes");
            assert!(nodes.lfo.is_some(), "LFO node should exist");
            let synths = backend.synths_created();
            let lfo_synth = synths.iter().find(|op| matches!(op, TestOp::CreateSynth { def_name, .. } if def_name == "imbolc_lfo"));
            assert!(lfo_synth.is_some(), "LFO synth created");
            if let Some(TestOp::CreateSynth { params, .. }) = lfo_synth {
                assert_eq!(params.iter().find(|(k, _)| k == "rate").map(|(_, v)| *v), Some(2.0));
                assert_eq!(params.iter().find(|(k, _)| k == "depth").map(|(_, v)| *v), Some(0.5));
            }
        }

        #[test]
        fn output_target_bus_routes_to_bus_audio_bus() {
            let (mut engine, backend) = engine_with_test_backend();
            let mut state = AppState::new();
            let inst_id = state.add_instrument(SourceType::Saw);
            if let Some(inst) = state.instruments.instrument_mut(inst_id) {
                inst.output_target = OutputTarget::Bus(1);
            }
            engine.rebuild_instrument_routing(&state.instruments, &state.session).expect("rebuild routing");
            let synths = backend.synths_created();
            let output_out = synths.iter().find_map(|op| {
                if let TestOp::CreateSynth { def_name, params, .. } = op {
                    if def_name == "imbolc_output" {
                        return params.iter().find(|(k, _)| k == "out").map(|(_, v)| *v);
                    }
                }
                None
            }).expect("output synth out param");
            let expected_bus = *engine.bus_audio_buses.get(&1).expect("bus 1 audio bus must exist");
            assert_eq!(output_out, expected_bus as f32, "output should route to bus 1 audio bus");
            assert_ne!(output_out, 0.0, "output should NOT route to hardware bus 0");
        }

        #[test]
        fn output_target_master_routes_to_hardware() {
            let (mut engine, backend) = engine_with_test_backend();
            let mut state = AppState::new();
            let inst_id = state.add_instrument(SourceType::Saw);
            // output_target defaults to Master, no change needed
            assert_eq!(state.instruments.instrument(inst_id).unwrap().output_target, OutputTarget::Master);
            engine.rebuild_instrument_routing(&state.instruments, &state.session).expect("rebuild routing");
            let synths = backend.synths_created();
            let output_out = synths.iter().find_map(|op| {
                if let TestOp::CreateSynth { def_name, params, .. } = op {
                    if def_name == "imbolc_output" {
                        return params.iter().find(|(k, _)| k == "out").map(|(_, v)| *v);
                    }
                }
                None
            }).expect("output synth out param");
            assert_eq!(output_out, 0.0, "Master output target should route to hardware bus 0");
        }

        #[test]
        fn send_post_insert_taps_final_bus() {
            let (mut engine, backend) = engine_with_test_backend();
            let mut state = AppState::new();
            let inst_id = state.add_instrument(SourceType::AudioIn);
            if let Some(inst) = state.instruments.instrument_mut(inst_id) {
                inst.filter = Some(FilterConfig::new(FilterType::Lpf));
                inst.add_effect(EffectType::Delay);
                inst.sends[0].enabled = true;
                inst.sends[0].level = 0.5;
                inst.sends[0].tap_point = SendTapPoint::PostInsert;
            }
            engine.rebuild_instrument_routing(&state.instruments, &state.session).expect("rebuild routing");
            let synths = backend.synths_created();
            // Get the send synth's `in` param
            let send_in = synths.iter().find_map(|op| {
                if let TestOp::CreateSynth { def_name, params, .. } = op {
                    if def_name == "imbolc_send" {
                        return params.iter().find(|(k, _)| k == "in").map(|(_, v)| *v);
                    }
                }
                None
            }).expect("send synth in param");
            // Get the delay (last effect) synth's `out` param — this is the final bus
            let delay_out = synths.iter().find_map(|op| {
                if let TestOp::CreateSynth { def_name, params, .. } = op {
                    if def_name == "imbolc_delay" {
                        return params.iter().find(|(k, _)| k == "out").map(|(_, v)| *v);
                    }
                }
                None
            }).expect("delay synth out param");
            assert_eq!(send_in, delay_out, "PostInsert send should tap from instrument final bus (after effects)");
        }

        #[test]
        fn send_pre_insert_taps_source_out() {
            let (mut engine, backend) = engine_with_test_backend();
            let mut state = AppState::new();
            let inst_id = state.add_instrument(SourceType::AudioIn);
            if let Some(inst) = state.instruments.instrument_mut(inst_id) {
                inst.filter = Some(FilterConfig::new(FilterType::Lpf));
                inst.add_effect(EffectType::Delay);
                inst.sends[0].enabled = true;
                inst.sends[0].level = 0.5;
                inst.sends[0].tap_point = SendTapPoint::PreInsert;
            }
            engine.rebuild_instrument_routing(&state.instruments, &state.session).expect("rebuild routing");
            let synths = backend.synths_created();
            // Get the send synth's `in` param
            let send_in = synths.iter().find_map(|op| {
                if let TestOp::CreateSynth { def_name, params, .. } = op {
                    if def_name == "imbolc_send" {
                        return params.iter().find(|(k, _)| k == "in").map(|(_, v)| *v);
                    }
                }
                None
            }).expect("send synth in param");
            // Get the audio_in source synth's `out` param — this is the source_out bus
            let source_out = synths.iter().find_map(|op| {
                if let TestOp::CreateSynth { def_name, params, .. } = op {
                    if def_name == "imbolc_audio_in" {
                        return params.iter().find(|(k, _)| k == "out").map(|(_, v)| *v);
                    }
                }
                None
            }).expect("audio_in synth out param");
            assert_eq!(send_in, source_out, "PreInsert send should tap from source_out bus (before filter/effects)");
            // Also verify it differs from the final bus (which goes through filter + delay)
            let delay_out = synths.iter().find_map(|op| {
                if let TestOp::CreateSynth { def_name, params, .. } = op {
                    if def_name == "imbolc_delay" {
                        return params.iter().find(|(k, _)| k == "out").map(|(_, v)| *v);
                    }
                }
                None
            }).expect("delay synth out param");
            assert_ne!(send_in, delay_out, "PreInsert tap should differ from post-effects final bus");
        }
    }

    mod lookahead_tests {
        use super::super::{compute_lookahead, MIN_LOOKAHEAD_SECS, JITTER_MARGIN_SECS, DEFAULT_LOOKAHEAD_SECS};

        #[test]
        fn compute_lookahead_64_at_44100() {
            let result = compute_lookahead(64, 44100);
            // 64/44100 = 1.45ms + 5ms = 6.45ms → clamped to 10ms
            assert!((result - MIN_LOOKAHEAD_SECS).abs() < 1e-9);
        }

        #[test]
        fn compute_lookahead_128_at_44100() {
            let result = compute_lookahead(128, 44100);
            // 128/44100 = 2.9ms + 5ms = 7.9ms → clamped to 10ms
            assert!((result - MIN_LOOKAHEAD_SECS).abs() < 1e-9);
        }

        #[test]
        fn compute_lookahead_256_at_44100() {
            let result = compute_lookahead(256, 44100);
            // 256/44100 = 5.8ms + 5ms = 10.8ms → above minimum
            let expected = 256.0 / 44100.0 + JITTER_MARGIN_SECS;
            assert!((result - expected).abs() < 1e-9);
            assert!(result > MIN_LOOKAHEAD_SECS);
        }

        #[test]
        fn compute_lookahead_512_at_44100() {
            let result = compute_lookahead(512, 44100);
            let expected = 512.0 / 44100.0 + JITTER_MARGIN_SECS;
            assert!((result - expected).abs() < 1e-9);
        }

        #[test]
        fn compute_lookahead_1024_at_44100() {
            let result = compute_lookahead(1024, 44100);
            let expected = 1024.0 / 44100.0 + JITTER_MARGIN_SECS;
            assert!((result - expected).abs() < 1e-9);
        }

        #[test]
        fn compute_lookahead_2048_at_44100() {
            let result = compute_lookahead(2048, 44100);
            let expected = 2048.0 / 44100.0 + JITTER_MARGIN_SECS;
            assert!((result - expected).abs() < 1e-9);
        }

        #[test]
        fn compute_lookahead_64_at_48000() {
            let result = compute_lookahead(64, 48000);
            // 64/48000 = 1.3ms + 5ms = 6.3ms → clamped to 10ms
            assert!((result - MIN_LOOKAHEAD_SECS).abs() < 1e-9);
        }

        #[test]
        fn compute_lookahead_128_at_48000() {
            let result = compute_lookahead(128, 48000);
            assert!((result - MIN_LOOKAHEAD_SECS).abs() < 1e-9);
        }

        #[test]
        fn compute_lookahead_256_at_48000() {
            let result = compute_lookahead(256, 48000);
            let expected = 256.0 / 48000.0 + JITTER_MARGIN_SECS;
            assert!((result - expected).abs() < 1e-9);
        }

        #[test]
        fn compute_lookahead_512_at_48000() {
            let result = compute_lookahead(512, 48000);
            let expected = 512.0 / 48000.0 + JITTER_MARGIN_SECS;
            assert!((result - expected).abs() < 1e-9);
        }

        #[test]
        fn compute_lookahead_1024_at_48000() {
            let result = compute_lookahead(1024, 48000);
            let expected = 1024.0 / 48000.0 + JITTER_MARGIN_SECS;
            assert!((result - expected).abs() < 1e-9);
        }

        #[test]
        fn compute_lookahead_2048_at_48000() {
            let result = compute_lookahead(2048, 48000);
            let expected = 2048.0 / 48000.0 + JITTER_MARGIN_SECS;
            assert!((result - expected).abs() < 1e-9);
        }

        #[test]
        fn compute_lookahead_zero_sample_rate() {
            let result = compute_lookahead(512, 0);
            assert!((result - DEFAULT_LOOKAHEAD_SECS).abs() < 1e-9);
        }

        #[test]
        fn default_engine_uses_default_lookahead() {
            let engine = super::super::AudioEngine::new();
            assert!((engine.schedule_lookahead_secs - DEFAULT_LOOKAHEAD_SECS).abs() < 1e-9);
        }

        #[test]
        fn set_lookahead_updates_field() {
            let mut engine = super::super::AudioEngine::new();
            engine.set_lookahead(1024, 44100);
            let expected = 1024.0 / 44100.0 + JITTER_MARGIN_SECS;
            assert!((engine.schedule_lookahead_secs - expected).abs() < 1e-9);
        }
    }
}
