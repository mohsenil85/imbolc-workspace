use std::time::Instant;

use super::backend::{AudioBackend, BackendMessage, RawArg};
use super::{AudioEngine, VoiceChain, GROUP_SOURCES};
use imbolc_types::{
    BufferId, InstrumentId, InstrumentState, ParamValue, ParameterTarget, SessionState,
};

/// Anti-click fade time for voice stealing/freeing.
/// Must exceed the midi control node's gate release (10ms) plus margin
/// for the source ADSR to begin releasing.
const ANTI_CLICK_FADE_SECS: f64 = 0.030;

/// Minimum ADSR attack/release time (seconds) enforced on all spawned voices.
/// Prevents clicks from sub-control-block ADSR ramps. 5ms is the
/// industry standard used by Ableton Live, Logic Pro, etc.
const MIN_ONSET_SECS: f32 = 0.005;

impl AudioEngine {
    /// Spawn a voice for an instrument
    pub fn spawn_voice(
        &mut self,
        instrument_id: InstrumentId,
        pitch: u8,
        velocity: f32,
        offset_secs: f64,
        state: &InstrumentState,
        session: &SessionState,
    ) -> Result<(), String> {
        let instrument = state
            .instrument(instrument_id)
            .ok_or_else(|| format!("No instrument with id {}", instrument_id))?;

        // AudioIn, BusIn, and VSTi instruments don't use voice spawning - they have persistent synths
        if instrument.source.is_audio_input() || instrument.source.is_bus_in() {
            return Ok(());
        }

        // VSTi instruments: send MIDI note-on via /u_cmd
        if instrument.source.is_vst() {
            return self.send_vsti_note_on(instrument_id, pitch, velocity);
        }

        // Sampler and TimeStretch instruments need special handling
        if instrument.source.is_sample() || instrument.source.is_time_stretch() {
            return self.spawn_sampler_voice(
                instrument_id,
                pitch,
                velocity,
                offset_secs,
                state,
                session,
            );
        }

        // Smart voice stealing — timed to align with new voice onset
        self.steal_voice_if_needed(instrument_id, pitch, velocity, offset_secs)?;

        if self.backend.is_none() {
            return Err("Not connected".to_string());
        }

        // Get the audio bus where voices should write their output
        let source_out_bus = self
            .bus_allocator
            .get_audio_bus(instrument_id, "source_out")
            .unwrap_or(16);

        // Create a group for this voice chain
        let group_id = self.next_node_id;
        self.next_node_id += 1;

        // Allocate per-voice control buses (with pooling)
        let (voice_freq_bus, voice_gate_bus, voice_vel_bus) =
            self.voice_allocator.alloc_control_buses();

        let tuning = session.tuning_a4 as f64;
        let freq = tuning * (2.0_f64).powf((pitch as f64 - 69.0) / 12.0);

        let mut messages: Vec<BackendMessage> = Vec::new();

        // 1. Create group
        messages.push(BackendMessage {
            addr: "/g_new".to_string(),
            args: vec![
                RawArg::Int(group_id),
                RawArg::Int(1), // addToTail
                RawArg::Int(GROUP_SOURCES),
            ],
        });

        // 2. MIDI control node
        let midi_node_id = self.next_node_id;
        self.next_node_id += 1;
        {
            let mut args: Vec<RawArg> = vec![
                RawArg::Str("imbolc_midi".to_string()),
                RawArg::Int(midi_node_id),
                RawArg::Int(1), // addToTail
                RawArg::Int(group_id),
            ];
            let params: Vec<(String, f32)> = vec![
                ("note".to_string(), pitch as f32),
                ("freq".to_string(), freq as f32),
                ("vel".to_string(), velocity),
                ("gate".to_string(), 1.0),
                ("freq_out".to_string(), voice_freq_bus as f32),
                ("gate_out".to_string(), voice_gate_bus as f32),
                ("vel_out".to_string(), voice_vel_bus as f32),
            ];
            for (name, value) in &params {
                args.push(RawArg::Str(name.clone()));
                args.push(RawArg::Float(*value));
            }
            messages.push(BackendMessage {
                addr: "/s_new".to_string(),
                args,
            });
        }

        // 3. Source synth
        let source_node_id = self.next_node_id;
        self.next_node_id += 1;
        let is_mono = instrument.mixer.channel_config.is_mono();
        {
            let mut args: Vec<RawArg> = vec![
                RawArg::Str(Self::source_synth_def(
                    instrument.source,
                    &session.custom_synthdefs,
                    is_mono,
                )),
                RawArg::Int(source_node_id),
                RawArg::Int(1),
                RawArg::Int(group_id),
            ];
            // Source params
            for p in &instrument.source_params {
                args.push(RawArg::Str(p.name.clone()));
                args.push(RawArg::Float(p.value.to_f32()));
            }
            // Wire control inputs
            args.push(RawArg::Str("freq_in".to_string()));
            args.push(RawArg::Float(voice_freq_bus as f32));
            args.push(RawArg::Str("gate_in".to_string()));
            args.push(RawArg::Float(voice_gate_bus as f32));
            // Amp envelope (ADSR) — enforce minimum onset/offset time
            args.push(RawArg::Str("attack".to_string()));
            args.push(RawArg::Float(
                instrument
                    .modulation
                    .amp_envelope
                    .attack
                    .max(MIN_ONSET_SECS),
            ));
            args.push(RawArg::Str("decay".to_string()));
            args.push(RawArg::Float(instrument.modulation.amp_envelope.decay));
            args.push(RawArg::Str("sustain".to_string()));
            args.push(RawArg::Float(instrument.modulation.amp_envelope.sustain));
            args.push(RawArg::Str("release".to_string()));
            args.push(RawArg::Float(
                instrument
                    .modulation
                    .amp_envelope
                    .release
                    .max(MIN_ONSET_SECS),
            ));
            // Output to source_out_bus
            args.push(RawArg::Str("out".to_string()));
            args.push(RawArg::Float(source_out_bus as f32));

            // Wire LFO mod inputs based on target
            if instrument.modulation.lfo.enabled {
                if let Some(lfo_bus) = self.bus_allocator.get_control_bus(instrument_id, "lfo_out")
                {
                    match instrument.modulation.lfo.target {
                        ParameterTarget::Level => {
                            args.push(RawArg::Str("amp_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::Pitch => {
                            args.push(RawArg::Str("pitch_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::Detune => {
                            args.push(RawArg::Str("detune_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::PulseWidth => {
                            args.push(RawArg::Str("width_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::Attack => {
                            args.push(RawArg::Str("attack_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::Release => {
                            args.push(RawArg::Str("release_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::FmIndex => {
                            args.push(RawArg::Str("index_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::WavetablePosition => {
                            args.push(RawArg::Str("position_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::FormantFreq => {
                            args.push(RawArg::Str("formant_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::SyncRatio => {
                            args.push(RawArg::Str("sync_ratio_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::Pressure => {
                            args.push(RawArg::Str("pressure_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::Embouchure => {
                            args.push(RawArg::Str("embouchure_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::GrainSize => {
                            args.push(RawArg::Str("grain_size_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::GrainDensity => {
                            args.push(RawArg::Str("density_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::FbFeedback => {
                            args.push(RawArg::Str("feedback_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::RingModDepth => {
                            args.push(RawArg::Str("mod_depth_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::ChaosParam => {
                            args.push(RawArg::Str("chaos_param_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::AdditiveRolloff => {
                            args.push(RawArg::Str("rolloff_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::MembraneTension => {
                            args.push(RawArg::Str("tension_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::Decay => {
                            args.push(RawArg::Str("decay_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::Sustain => {
                            args.push(RawArg::Str("sustain_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        _ => {} // Routing-level targets handled in routing.rs
                    }
                }
            }

            messages.push(BackendMessage {
                addr: "/s_new".to_string(),
                args,
            });
        }

        // Send all as one timed bundle via the OSC sender thread
        self.queue_timed_bundle(messages, offset_secs)?;

        // Register voice nodes in the node registry
        self.node_registry.register(group_id);
        self.node_registry.register(midi_node_id);
        self.node_registry.register(source_node_id);

        self.voice_allocator.add(VoiceChain {
            instrument_id,
            pitch,
            velocity,
            group_id,
            midi_node_id,
            source_node: source_node_id,
            spawn_time: Instant::now(),
            release_secs: instrument
                .modulation
                .amp_envelope
                .release
                .max(MIN_ONSET_SECS),
            release_state: None,
            control_buses: (voice_freq_bus, voice_gate_bus, voice_vel_bus),
        });

        Ok(())
    }

    /// Spawn a sampler voice (separate method for sampler-specific handling)
    fn spawn_sampler_voice(
        &mut self,
        instrument_id: InstrumentId,
        pitch: u8,
        velocity: f32,
        offset_secs: f64,
        state: &InstrumentState,
        session: &SessionState,
    ) -> Result<(), String> {
        let instrument = state
            .instrument(instrument_id)
            .ok_or_else(|| format!("No instrument with id {}", instrument_id))?;

        let sampler_config = instrument
            .sampler_config()
            .ok_or("Sampler instrument has no sampler config")?;

        let buffer_id = sampler_config
            .buffer_id
            .ok_or("Sampler has no buffer loaded")?;

        let bufnum = self
            .buffer_map
            .get(&buffer_id)
            .copied()
            .ok_or("Buffer not loaded in audio engine")?;

        // Get slice for this note (or current selected slice)
        let (slice_start, slice_end) = sampler_config
            .slice_for_note(pitch)
            .map(|s| (s.start, s.end))
            .unwrap_or((0.0, 1.0));

        // Smart voice stealing — timed to align with new voice onset
        self.steal_voice_if_needed(instrument_id, pitch, velocity, offset_secs)?;

        if self.backend.is_none() {
            return Err("Not connected".to_string());
        }

        // Get the audio bus where voices should write their output
        let source_out_bus = self
            .bus_allocator
            .get_audio_bus(instrument_id, "source_out")
            .unwrap_or(16);

        // Create a group for this voice chain
        let group_id = self.next_node_id;
        self.next_node_id += 1;

        // Allocate per-voice control buses (with pooling)
        let (voice_freq_bus, voice_gate_bus, voice_vel_bus) =
            self.voice_allocator.alloc_control_buses();

        let tuning = session.tuning_a4 as f64;
        let freq = tuning * (2.0_f64).powf((pitch as f64 - 69.0) / 12.0);

        let mut messages: Vec<BackendMessage> = Vec::new();

        // 1. Create group
        messages.push(BackendMessage {
            addr: "/g_new".to_string(),
            args: vec![
                RawArg::Int(group_id),
                RawArg::Int(1), // addToTail
                RawArg::Int(GROUP_SOURCES),
            ],
        });

        // 2. MIDI control node
        let midi_node_id = self.next_node_id;
        self.next_node_id += 1;
        {
            let mut args: Vec<RawArg> = vec![
                RawArg::Str("imbolc_midi".to_string()),
                RawArg::Int(midi_node_id),
                RawArg::Int(1), // addToTail
                RawArg::Int(group_id),
            ];
            let params: Vec<(String, f32)> = vec![
                ("note".to_string(), pitch as f32),
                ("freq".to_string(), freq as f32),
                ("vel".to_string(), velocity),
                ("gate".to_string(), 1.0),
                ("freq_out".to_string(), voice_freq_bus as f32),
                ("gate_out".to_string(), voice_gate_bus as f32),
                ("vel_out".to_string(), voice_vel_bus as f32),
            ];
            for (name, value) in &params {
                args.push(RawArg::Str(name.clone()));
                args.push(RawArg::Float(*value));
            }
            messages.push(BackendMessage {
                addr: "/s_new".to_string(),
                args,
            });
        }

        // 3. Sampler/TimeStretch synth
        let sampler_node_id = self.next_node_id;
        self.next_node_id += 1;
        let is_time_stretch = instrument.source.is_time_stretch();
        {
            let synthdef_name = if is_time_stretch {
                "imbolc_timestretch"
            } else {
                "imbolc_sampler"
            };

            let mut args: Vec<RawArg> = vec![
                RawArg::Str(synthdef_name.to_string()),
                RawArg::Int(sampler_node_id),
                RawArg::Int(1),
                RawArg::Int(group_id),
            ];

            // Helper to get float param
            let get_param = |name: &str, default: f32| -> f32 {
                instrument
                    .source_params
                    .iter()
                    .find(|p| p.name == name)
                    .map(|p| match p.value {
                        ParamValue::Float(v) => v,
                        ParamValue::Int(v) => v as f32,
                        _ => default,
                    })
                    .unwrap_or(default)
            };

            let amp = get_param("amp", 0.8);
            let loop_mode = sampler_config.loop_mode;

            // Common params for both
            args.push(RawArg::Str("bufnum".to_string()));
            args.push(RawArg::Float(bufnum as f32));
            args.push(RawArg::Str("sliceStart".to_string()));
            args.push(RawArg::Float(slice_start));
            args.push(RawArg::Str("sliceEnd".to_string()));
            args.push(RawArg::Float(slice_end));
            args.push(RawArg::Str("amp".to_string()));
            args.push(RawArg::Float(amp));

            if is_time_stretch {
                // TimeStretch-specific params
                let stretch = get_param("stretch", 1.0);
                let pitch = get_param("pitch", 0.0);
                let grain_size = get_param("grain_size", 0.1);
                let overlap = get_param("overlap", 4.0);

                args.push(RawArg::Str("stretch".to_string()));
                args.push(RawArg::Float(stretch));
                args.push(RawArg::Str("pitch".to_string()));
                args.push(RawArg::Float(pitch));
                args.push(RawArg::Str("grain_size".to_string()));
                args.push(RawArg::Float(grain_size));
                args.push(RawArg::Str("overlap".to_string()));
                args.push(RawArg::Float(overlap));
            } else {
                // PitchedSampler-specific params
                let rate = get_param("rate", 1.0);
                args.push(RawArg::Str("rate".to_string()));
                args.push(RawArg::Float(rate));
                args.push(RawArg::Str("loop".to_string()));
                args.push(RawArg::Float(if loop_mode { 1.0 } else { 0.0 }));
            }

            // Wire control inputs (for pitch tracking if enabled)
            if sampler_config.pitch_tracking {
                args.push(RawArg::Str("freq_in".to_string()));
                args.push(RawArg::Float(voice_freq_bus as f32));
            }
            args.push(RawArg::Str("gate_in".to_string()));
            args.push(RawArg::Float(voice_gate_bus as f32));
            args.push(RawArg::Str("vel_in".to_string()));
            args.push(RawArg::Float(voice_vel_bus as f32));

            // Amp envelope (ADSR) — enforce minimum onset/offset time
            args.push(RawArg::Str("attack".to_string()));
            args.push(RawArg::Float(
                instrument
                    .modulation
                    .amp_envelope
                    .attack
                    .max(MIN_ONSET_SECS),
            ));
            args.push(RawArg::Str("decay".to_string()));
            args.push(RawArg::Float(instrument.modulation.amp_envelope.decay));
            args.push(RawArg::Str("sustain".to_string()));
            args.push(RawArg::Float(instrument.modulation.amp_envelope.sustain));
            args.push(RawArg::Str("release".to_string()));
            args.push(RawArg::Float(
                instrument
                    .modulation
                    .amp_envelope
                    .release
                    .max(MIN_ONSET_SECS),
            ));

            // Output to source_out_bus
            args.push(RawArg::Str("out".to_string()));
            args.push(RawArg::Float(source_out_bus as f32));

            // Wire LFO mod inputs for sampler/timestretch voice
            if instrument.modulation.lfo.enabled {
                if let Some(lfo_bus) = self.bus_allocator.get_control_bus(instrument_id, "lfo_out")
                {
                    match instrument.modulation.lfo.target {
                        ParameterTarget::Level => {
                            args.push(RawArg::Str("amp_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::SampleRate if !is_time_stretch => {
                            args.push(RawArg::Str("srate_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::Attack => {
                            args.push(RawArg::Str("attack_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::Release => {
                            args.push(RawArg::Str("release_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::Decay => {
                            args.push(RawArg::Str("decay_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::Sustain => {
                            args.push(RawArg::Str("sustain_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::StretchRatio if is_time_stretch => {
                            args.push(RawArg::Str("stretch_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::PitchShift if is_time_stretch => {
                            args.push(RawArg::Str("pitch_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::GrainSize if is_time_stretch => {
                            args.push(RawArg::Str("grain_size_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        _ => {} // Routing-level targets handled in routing.rs
                    }
                }
            }

            messages.push(BackendMessage {
                addr: "/s_new".to_string(),
                args,
            });
        }

        // Send all as one timed bundle via the OSC sender thread
        self.queue_timed_bundle(messages, offset_secs)?;

        // Register voice nodes in the node registry
        self.node_registry.register(group_id);
        self.node_registry.register(midi_node_id);
        self.node_registry.register(sampler_node_id);

        self.voice_allocator.add(VoiceChain {
            instrument_id,
            pitch,
            velocity,
            group_id,
            midi_node_id,
            source_node: sampler_node_id,
            spawn_time: Instant::now(),
            release_secs: instrument
                .modulation
                .amp_envelope
                .release
                .max(MIN_ONSET_SECS),
            release_state: None,
            control_buses: (voice_freq_bus, voice_gate_bus, voice_vel_bus),
        });

        Ok(())
    }

    /// Release a specific voice by instrument and pitch (note-off).
    /// Marks the voice as released instead of removing it, so it remains
    /// available as a steal candidate while its envelope fades out.
    pub fn release_voice(
        &mut self,
        instrument_id: InstrumentId,
        pitch: u8,
        offset_secs: f64,
        state: &InstrumentState,
    ) -> Result<(), String> {
        // VSTi instruments: send MIDI note-off via /u_cmd
        if let Some(instrument) = state.instrument(instrument_id) {
            if instrument.source.is_vst() {
                return self.send_vsti_note_off(instrument_id, pitch);
            }
        }

        if self.backend.is_none() {
            return Err("Not connected".to_string());
        }

        // Find and mark an active voice as released via the allocator
        let release_time = state
            .instrument(instrument_id)
            .map(|s| s.modulation.amp_envelope.release)
            .unwrap_or(1.0);

        if let Some(pos) = self
            .voice_allocator
            .mark_released(instrument_id, pitch, release_time)
        {
            let voice = &self.voice_allocator.chains()[pos];
            let gate_bus = voice.control_buses.1;

            // Force deterministic release:
            // 1) free the MIDI helper node so it stops writing gate values
            // 2) hard-set gate control bus to 0 so the source ADSR enters release now
            self.queue_timed_bundle(
                vec![
                    BackendMessage {
                        addr: "/n_free".to_string(),
                        args: vec![RawArg::Int(voice.midi_node_id)],
                    },
                    BackendMessage {
                        addr: "/c_set".to_string(),
                        args: vec![RawArg::Int(gate_bus), RawArg::Float(0.0)],
                    },
                ],
                offset_secs,
            )?;

            // Schedule deferred /n_free after envelope completes (+1s margin)
            let cleanup_offset = offset_secs + release_time as f64 + 1.0;
            let group_id = voice.group_id;
            self.queue_timed_bundle(
                vec![BackendMessage {
                    addr: "/n_free".to_string(),
                    args: vec![RawArg::Int(group_id)],
                }],
                cleanup_offset,
            )?;
        }
        Ok(())
    }

    /// Release all active voices with anti-click fade (force gate bus to 0, then delayed free)
    pub fn release_all_voices(&mut self) {
        if let Some(ref backend) = self.backend {
            for chain in self.voice_allocator.drain_all() {
                self.node_registry.unregister(chain.group_id);
                self.node_registry.unregister(chain.midi_node_id);
                self.node_registry.unregister(chain.source_node);
                let _ = Self::anti_click_free(backend.as_ref(), &chain);
            }
        }
    }

    /// Remove voices whose release envelope has fully expired.
    /// Called periodically from the audio thread to prevent unbounded growth.
    pub fn cleanup_expired_voices(&mut self) {
        let expired = self.voice_allocator.cleanup_expired();
        for voice in &expired {
            self.node_registry.unregister(voice.group_id);
            self.node_registry.unregister(voice.midi_node_id);
            self.node_registry.unregister(voice.source_node);
        }
    }

    /// Steal a voice if needed before spawning a new one.
    /// Delegates to the voice allocator for candidate selection,
    /// then handles anti-click freeing via the backend.
    /// `offset_secs` aligns the steal timing with the new voice onset
    /// so the crossfade is seamless (no gap between old fade-out and new attack).
    pub(crate) fn steal_voice_if_needed(
        &mut self,
        instrument_id: InstrumentId,
        pitch: u8,
        _velocity: f32,
        offset_secs: f64,
    ) -> Result<(), String> {
        let backend = self.backend.as_ref().ok_or("Not connected")?;

        let stolen = self.voice_allocator.steal_voices(instrument_id, pitch);
        for voice in &stolen {
            self.node_registry.unregister(voice.group_id);
            self.node_registry.unregister(voice.midi_node_id);
            self.node_registry.unregister(voice.source_node);
            // Buses for stolen voices must be returned only after /n_end confirms free.
            self.oneshot_buses
                .insert(voice.group_id, voice.control_buses);
            Self::anti_click_free_at(backend.as_ref(), voice, offset_secs)?;
        }

        Ok(())
    }

    /// Free a voice with a brief anti-click fade by forcing gate bus to 0,
    /// then /n_free after fade. For already-released voices, skip gate forcing
    /// (already fading) but still delay the free.
    fn anti_click_free(backend: &dyn AudioBackend, voice: &VoiceChain) -> Result<(), String> {
        Self::anti_click_free_at(backend, voice, 0.0)
    }

    /// Free a voice with anti-click fade starting at `offset_secs` from now.
    /// Used by voice stealing to align the fade-out with the replacement voice's onset,
    /// producing a seamless crossfade instead of a gap.
    fn anti_click_free_at(
        backend: &dyn AudioBackend,
        voice: &VoiceChain,
        offset_secs: f64,
    ) -> Result<(), String> {
        if let Some((released_at, release_dur)) = voice.release_state {
            // Already releasing — compute remaining release time so we don't cut the tail
            let elapsed = Instant::now().duration_since(released_at).as_secs_f64();
            let remaining = (release_dur as f64 - elapsed).max(ANTI_CLICK_FADE_SECS);
            backend
                .send_bundle(
                    vec![BackendMessage {
                        addr: "/n_free".to_string(),
                        args: vec![RawArg::Int(voice.group_id)],
                    }],
                    offset_secs + remaining,
                )
                .map_err(|e| e.to_string())?;
        } else {
            // Active voice: hard-zero gate bus at the offset, then free after envelope release.
            // This avoids release timing ambiguity from the helper MIDI envelope.
            let fade = (voice.release_secs as f64).max(ANTI_CLICK_FADE_SECS);
            backend
                .send_bundle(
                    vec![
                        BackendMessage {
                            addr: "/n_free".to_string(),
                            args: vec![RawArg::Int(voice.midi_node_id)],
                        },
                        BackendMessage {
                            addr: "/c_set".to_string(),
                            args: vec![RawArg::Int(voice.control_buses.1), RawArg::Float(0.0)],
                        },
                    ],
                    offset_secs,
                )
                .map_err(|e| e.to_string())?;
            backend
                .send_bundle(
                    vec![BackendMessage {
                        addr: "/n_free".to_string(),
                        args: vec![RawArg::Int(voice.group_id)],
                    }],
                    offset_secs + fade,
                )
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Spawn a click track sound (metronome tick).
    /// Downbeats use a higher frequency (1500 Hz) than other beats (1000 Hz).
    pub fn spawn_click(
        &mut self,
        is_downbeat: bool,
        volume: f32,
        offset_secs: f64,
    ) -> Result<(), String> {
        if self.backend.is_none() {
            return Err("Not connected".to_string());
        }

        let node_id = self.next_node_id;
        self.next_node_id += 1;

        // Downbeats get higher pitch
        let freq = if is_downbeat { 1500.0 } else { 1000.0 };

        let msg = BackendMessage {
            addr: "/s_new".to_string(),
            args: vec![
                RawArg::Str("imbolc_click".to_string()),
                RawArg::Int(node_id),
                RawArg::Int(0), // addToHead
                RawArg::Int(GROUP_SOURCES),
                RawArg::Str("freq".to_string()),
                RawArg::Float(freq),
                RawArg::Str("amp".to_string()),
                RawArg::Float(volume),
            ],
        };
        self.queue_timed_bundle(vec![msg], offset_secs)?;

        Ok(())
    }

    /// Play a one-shot drum sample routed through an instrument's signal chain
    #[allow(clippy::too_many_arguments)]
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
        if self.backend.is_none() {
            return Err("Not connected".to_string());
        }
        let bufnum = *self.buffer_map.get(&buffer_id).ok_or("Buffer not loaded")?;
        let out_bus = self
            .bus_allocator
            .get_audio_bus(instrument_id, "source_out")
            .unwrap_or(0);

        let node_id = self.next_node_id;
        self.next_node_id += 1;

        let msg = BackendMessage {
            addr: "/s_new".to_string(),
            args: vec![
                RawArg::Str("imbolc_sampler_oneshot".to_string()),
                RawArg::Int(node_id),
                RawArg::Int(0), // addToHead
                RawArg::Int(GROUP_SOURCES),
                RawArg::Str("bufnum".to_string()),
                RawArg::Int(bufnum),
                RawArg::Str("amp".to_string()),
                RawArg::Float(amp),
                RawArg::Str("sliceStart".to_string()),
                RawArg::Float(slice_start),
                RawArg::Str("sliceEnd".to_string()),
                RawArg::Float(slice_end),
                RawArg::Str("rate".to_string()),
                RawArg::Float(rate),
                RawArg::Str("out".to_string()),
                RawArg::Int(out_bus), // Route to instrument's source bus
            ],
        };
        self.queue_timed_bundle(vec![msg], offset_secs)?;

        Ok(())
    }

    /// Trigger an instrument as a one-shot (spawn voice + immediate release).
    /// The voice goes through Attack → Release, skipping sustained hold.
    /// Used by drum sequencer pads that trigger synth instruments.
    pub fn trigger_instrument_oneshot(
        &mut self,
        target_instrument_id: InstrumentId,
        freq: f32,
        velocity: f32,
        offset_secs: f64,
        state: &InstrumentState,
        session: &SessionState,
    ) -> Result<(), String> {
        let instrument = state
            .instrument(target_instrument_id)
            .ok_or_else(|| format!("No instrument with id {}", target_instrument_id))?;

        // Skip unsupported instrument types
        if instrument.source.is_audio_input()
            || instrument.source.is_bus_in()
            || instrument.source.is_vst()
        {
            return Ok(());
        }

        // Sampler instruments need buffer - skip if none
        if (instrument.source.is_sample() || instrument.source.is_time_stretch())
            && instrument
                .sampler_config()
                .and_then(|c| c.buffer_id)
                .is_none()
        {
            return Ok(());
        }

        if self.backend.is_none() {
            return Err("Not connected".to_string());
        }

        // Get the audio bus where voices should write their output
        let source_out_bus = self
            .bus_allocator
            .get_audio_bus(target_instrument_id, "source_out")
            .unwrap_or(16);

        // Create a group for this one-shot voice chain
        let group_id = self.next_node_id;
        self.next_node_id += 1;

        // Allocate per-voice control buses
        let (voice_freq_bus, voice_gate_bus, voice_vel_bus) =
            self.voice_allocator.alloc_control_buses();

        let mut messages: Vec<BackendMessage> = Vec::new();

        // 1. Create group
        messages.push(BackendMessage {
            addr: "/g_new".to_string(),
            args: vec![
                RawArg::Int(group_id),
                RawArg::Int(1), // addToTail
                RawArg::Int(GROUP_SOURCES),
            ],
        });

        // 2. MIDI control node (starts with gate=1)
        let midi_node_id = self.next_node_id;
        self.next_node_id += 1;
        {
            // Convert freq to approximate MIDI note for the note parameter
            // (used for display, the actual freq comes from freq parameter)
            let note = 69.0 + 12.0 * (freq / 440.0).log2();
            let note_clamped = note.clamp(0.0, 127.0);

            let mut args: Vec<RawArg> = vec![
                RawArg::Str("imbolc_midi".to_string()),
                RawArg::Int(midi_node_id),
                RawArg::Int(1), // addToTail
                RawArg::Int(group_id),
            ];
            let params: Vec<(String, f32)> = vec![
                ("note".to_string(), note_clamped),
                ("freq".to_string(), freq),
                ("vel".to_string(), velocity),
                ("gate".to_string(), 1.0),
                ("freq_out".to_string(), voice_freq_bus as f32),
                ("gate_out".to_string(), voice_gate_bus as f32),
                ("vel_out".to_string(), voice_vel_bus as f32),
            ];
            for (name, value) in &params {
                args.push(RawArg::Str(name.clone()));
                args.push(RawArg::Float(*value));
            }
            messages.push(BackendMessage {
                addr: "/s_new".to_string(),
                args,
            });
        }

        // 3. Source synth
        let source_node_id = self.next_node_id;
        self.next_node_id += 1;
        let is_mono = instrument.mixer.channel_config.is_mono();
        {
            let mut args: Vec<RawArg> = vec![
                RawArg::Str(Self::source_synth_def(
                    instrument.source,
                    &session.custom_synthdefs,
                    is_mono,
                )),
                RawArg::Int(source_node_id),
                RawArg::Int(1),
                RawArg::Int(group_id),
            ];
            // Source params
            for p in &instrument.source_params {
                args.push(RawArg::Str(p.name.clone()));
                args.push(RawArg::Float(p.value.to_f32()));
            }
            // Wire control inputs
            args.push(RawArg::Str("freq_in".to_string()));
            args.push(RawArg::Float(voice_freq_bus as f32));
            args.push(RawArg::Str("gate_in".to_string()));
            args.push(RawArg::Float(voice_gate_bus as f32));
            // Amp envelope (ADSR) — enforce minimum onset/offset time
            args.push(RawArg::Str("attack".to_string()));
            args.push(RawArg::Float(
                instrument
                    .modulation
                    .amp_envelope
                    .attack
                    .max(MIN_ONSET_SECS),
            ));
            args.push(RawArg::Str("decay".to_string()));
            args.push(RawArg::Float(instrument.modulation.amp_envelope.decay));
            args.push(RawArg::Str("sustain".to_string()));
            args.push(RawArg::Float(instrument.modulation.amp_envelope.sustain));
            args.push(RawArg::Str("release".to_string()));
            args.push(RawArg::Float(
                instrument
                    .modulation
                    .amp_envelope
                    .release
                    .max(MIN_ONSET_SECS),
            ));
            // Output to source_out_bus
            args.push(RawArg::Str("out".to_string()));
            args.push(RawArg::Float(source_out_bus as f32));

            // Wire LFO mod inputs if enabled
            if instrument.modulation.lfo.enabled {
                if let Some(lfo_bus) = self
                    .bus_allocator
                    .get_control_bus(target_instrument_id, "lfo_out")
                {
                    match instrument.modulation.lfo.target {
                        ParameterTarget::Level => {
                            args.push(RawArg::Str("amp_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        ParameterTarget::Pitch => {
                            args.push(RawArg::Str("pitch_mod_in".to_string()));
                            args.push(RawArg::Float(lfo_bus as f32));
                        }
                        _ => {} // Other targets handled in routing
                    }
                }
            }

            messages.push(BackendMessage {
                addr: "/s_new".to_string(),
                args,
            });
        }

        // Send the spawn bundle via the OSC sender thread
        self.queue_timed_bundle(messages, offset_secs)?;

        // 4. Schedule gate=0 shortly after spawn to trigger release phase
        //    Small delay (5ms) ensures attack transient is heard
        let release_offset = offset_secs + 0.005;
        // gate=0 as a timed bundle through the queue
        self.queue_timed_bundle(
            vec![BackendMessage {
                addr: "/n_set".to_string(),
                args: vec![
                    RawArg::Int(midi_node_id),
                    RawArg::Str("gate".to_string()),
                    RawArg::Float(0.0),
                ],
            }],
            release_offset,
        )?;

        // 5. Schedule cleanup after envelope completes
        let release_time = instrument.modulation.amp_envelope.release;
        let cleanup_offset = release_offset + release_time as f64 + 0.5;
        self.queue_timed_bundle(
            vec![BackendMessage {
                addr: "/n_free".to_string(),
                args: vec![RawArg::Int(group_id)],
            }],
            cleanup_offset,
        )?;

        // Track control buses for return when /n_end arrives
        self.oneshot_buses
            .insert(group_id, (voice_freq_bus, voice_gate_bus, voice_vel_bus));

        // Register nodes for the node registry
        self.node_registry.register(group_id);
        self.node_registry.register(midi_node_id);
        self.node_registry.register(source_node_id);

        Ok(())
    }

    /// Process /n_end notifications from SuperCollider.
    /// For each ended node, remove it from voice tracking, return control buses,
    /// and unregister from the node registry.
    pub fn process_node_ends(&mut self, ended_node_ids: &[i32]) {
        for &node_id in ended_node_ids {
            if let Some(voice) = self.voice_allocator.remove_by_group_id(node_id) {
                // Polyphonic voice freed — buses already returned by remove_by_group_id
                self.node_registry.unregister(voice.group_id);
                self.node_registry.unregister(voice.midi_node_id);
                self.node_registry.unregister(voice.source_node);
            } else if let Some(buses) = self.oneshot_buses.remove(&node_id) {
                // One-shot voice freed — return buses manually
                self.voice_allocator
                    .return_control_buses(buses.0, buses.1, buses.2);
                self.node_registry.unregister(node_id);
            } else {
                // Unknown node (routing synth, meter, etc.) — just unregister
                self.node_registry.unregister(node_id);
            }
        }
    }
}
