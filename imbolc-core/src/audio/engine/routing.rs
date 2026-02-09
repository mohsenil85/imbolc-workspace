use super::AudioEngine;
use super::{InstrumentNodes, GROUP_SOURCES, GROUP_PROCESSING, GROUP_OUTPUT, GROUP_BUS_PROCESSING, VST_UGEN_INDEX};
use super::backend::RawArg;
use std::collections::HashMap;
use crate::state::{CustomSynthDefRegistry, EffectId, EffectType, FilterType, Instrument, InstrumentId, InstrumentState, LayerGroupMixer, MixerBus, ParameterTarget, ParamValue, SendTapPoint, SessionState, SourceType, SourceTypeExt};

/// State machine for amortized routing rebuild across multiple ticks.
/// Each step performs a bounded amount of work so the audio thread is never
/// starved for more than ~0.5ms.
pub(crate) enum RoutingRebuildPhase {
    /// Tear down all existing nodes and clear maps.
    TearDown,
    /// Allocate buses for mixer buses and layer groups.
    AllocBuses,
    /// Build chain + sends for instrument at index `i` in the snapshot.
    BuildInstrument(usize),
    /// Build bus output synths, layer group outputs, restore VST params.
    BuildOutputs,
    /// Restart meter, sync bus watermarks.
    Finalize,
}

/// Return value from `routing_rebuild_step`: either continue or done.
pub(crate) enum RebuildStepResult {
    /// More work to do — call `routing_rebuild_step` again next tick.
    Continue(RoutingRebuildPhase),
    /// Rebuild is complete.
    Done,
}

impl AudioEngine {
    pub(super) fn source_synth_def(source: SourceType, registry: &CustomSynthDefRegistry, mono: bool) -> String {
        if mono && source.has_mono_variant() {
            source.synth_def_name_mono().to_string()
        } else {
            source.synth_def_name_with_registry(registry)
        }
    }

    pub(super) fn filter_synth_def(ft: FilterType, mono: bool) -> &'static str {
        if mono {
            ft.synth_def_name_mono()
        } else {
            ft.synth_def_name()
        }
    }

    pub(super) fn effect_synth_def(et: EffectType, mono: bool) -> &'static str {
        if mono && et.has_mono_variant() {
            et.synth_def_name_mono()
        } else {
            et.synth_def_name()
        }
    }

    // ── Shared helpers for per-instrument chain building ──────────

    /// Build the signal chain for a single instrument: source → LFO → filter → EQ → effects → output.
    /// Allocates buses, creates synth nodes, and registers everything in node_map/node_registry.
    fn build_instrument_chain(
        &mut self,
        instrument: &Instrument,
        any_solo: bool,
        session: &SessionState,
    ) -> Result<(), String> {
        let mut source_node: Option<i32> = None;
        let mut lfo_node: Option<i32> = None;
        let mut filter_node: Option<i32> = None;
        let mut effect_nodes: HashMap<EffectId, i32> = HashMap::new();
        let mut effect_order: Vec<EffectId> = Vec::new();

        // Determine channel count based on channel config
        let is_mono = instrument.channel_config.is_mono();
        let channels = instrument.channel_config.channels();

        let source_out_bus = self.bus_allocator.get_or_alloc_audio_bus_with_channels(
            instrument.id, "source_out", channels,
        );
        let mut current_bus = source_out_bus;

        // Source synth (AudioIn, BusIn, VST — oscillator voices are spawned dynamically)
        if instrument.source.is_audio_input() {
            let node_id = self.next_node_id;
            self.next_node_id += 1;

            let mut params: Vec<(String, f32)> = vec![
                ("out".to_string(), source_out_bus as f32),
                ("instrument_id".to_string(), instrument.id as f32),
            ];
            for p in &instrument.source_params {
                let val = p.value.to_f32();
                let val = if p.name == "gain" && !instrument.active { 0.0 } else { val };
                params.push((p.name.clone(), val));
            }

            let client = self.backend.as_ref().ok_or("Not connected")?;
            client.create_synth("imbolc_audio_in", node_id, GROUP_SOURCES, &params)
                .map_err(|e| e.to_string())?;
            source_node = Some(node_id);
        } else if instrument.source.is_bus_in() {
            let node_id = self.next_node_id;
            self.next_node_id += 1;

            let bus_id = instrument.source_params.iter()
                .find(|p| p.name == "bus")
                .map(|p| match &p.value {
                    crate::state::param::ParamValue::Int(v) => *v as u8,
                    _ => 1,
                })
                .unwrap_or(1);
            let bus_audio_bus = self.bus_audio_buses.get(&bus_id).copied().unwrap_or(16);
            let gain = instrument.source_params.iter()
                .find(|p| p.name == "gain")
                .map(|p| match &p.value {
                    crate::state::param::ParamValue::Float(v) => *v,
                    _ => 1.0,
                })
                .unwrap_or(1.0);

            let params: Vec<(String, f32)> = vec![
                ("out".to_string(), source_out_bus as f32),
                ("in".to_string(), bus_audio_bus as f32),
                ("gain".to_string(), gain),
                ("instrument_id".to_string(), instrument.id as f32),
            ];

            let client = self.backend.as_ref().ok_or("Not connected")?;
            client.create_synth("imbolc_bus_in", node_id, GROUP_SOURCES, &params)
                .map_err(|e| e.to_string())?;
            source_node = Some(node_id);
        } else if instrument.source.is_vst() {
            let node_id = self.next_node_id;
            self.next_node_id += 1;

            let params: Vec<(String, f32)> = vec![
                ("out".to_string(), source_out_bus as f32),
            ];

            let client = self.backend.as_ref().ok_or("Not connected")?;
            client.create_synth("imbolc_vst_instrument", node_id, GROUP_SOURCES, &params)
                .map_err(|e| e.to_string())?;

            if let SourceType::Vst(vst_id) = instrument.source {
                if let Some(plugin) = session.vst_plugins.get(vst_id) {
                    let _ = client.send_unit_cmd(
                        node_id, VST_UGEN_INDEX, "/open",
                        vec![RawArg::Str(plugin.plugin_path.to_string_lossy().to_string())],
                    );
                }
            }
            source_node = Some(node_id);
        }

        // LFO (if enabled)
        let lfo_control_bus: Option<i32> = if instrument.lfo.enabled {
            let lfo_node_id = self.next_node_id;
            self.next_node_id += 1;
            let lfo_out_bus = self.bus_allocator.get_or_alloc_control_bus(instrument.id, "lfo_out");

            let params = vec![
                ("out".to_string(), lfo_out_bus as f32),
                ("rate".to_string(), instrument.lfo.rate),
                ("depth".to_string(), instrument.lfo.depth),
                ("shape".to_string(), instrument.lfo.shape.index() as f32),
            ];

            let client = self.backend.as_ref().ok_or("Not connected")?;
            client.create_synth("imbolc_lfo", lfo_node_id, GROUP_SOURCES, &params)
                .map_err(|e| e.to_string())?;

            lfo_node = Some(lfo_node_id);
            Some(lfo_out_bus)
        } else {
            None
        };

        // Filter (if present)
        if let Some(ref filter) = instrument.filter {
            let node_id = self.next_node_id;
            self.next_node_id += 1;
            let filter_out_bus = self.bus_allocator.get_or_alloc_audio_bus_with_channels(
                instrument.id, "filter_out", channels,
            );

            let cutoff_mod_bus = if instrument.lfo.enabled && instrument.lfo.target == ParameterTarget::FilterCutoff {
                lfo_control_bus.map(|b| b as f32).unwrap_or(-1.0)
            } else {
                -1.0
            };
            let res_mod_bus = if instrument.lfo.enabled && instrument.lfo.target == ParameterTarget::FilterResonance {
                lfo_control_bus.map(|b| b as f32).unwrap_or(-1.0)
            } else {
                -1.0
            };

            let mut params = vec![
                ("in".to_string(), current_bus as f32),
                ("out".to_string(), filter_out_bus as f32),
                ("cutoff".to_string(), filter.cutoff.value),
                ("resonance".to_string(), filter.resonance.value),
                ("cutoff_mod_in".to_string(), cutoff_mod_bus),
                ("res_mod_in".to_string(), res_mod_bus),
            ];
            for p in &filter.extra_params {
                params.push((p.name.clone(), p.value.to_f32()));
            }

            let client = self.backend.as_ref().ok_or("Not connected")?;
            client.create_synth(
                Self::filter_synth_def(filter.filter_type, is_mono), node_id, GROUP_PROCESSING, &params,
            ).map_err(|e| e.to_string())?;

            filter_node = Some(node_id);
            current_bus = filter_out_bus;
        }

        // EQ (12-band parametric, if present)
        // Note: EQ doesn't have mono variants yet, stays stereo
        let mut eq_node: Option<i32> = None;
        if let Some(ref eq) = instrument.eq {
            let node_id = self.next_node_id;
            self.next_node_id += 1;
            let eq_out_bus = self.bus_allocator.get_or_alloc_audio_bus(instrument.id, "eq_out");

            let mut params: Vec<(String, f32)> = vec![
                ("in".to_string(), current_bus as f32),
                ("out".to_string(), eq_out_bus as f32),
            ];
            for (i, band) in eq.bands.iter().enumerate() {
                params.push((format!("b{}_freq", i), band.freq));
                params.push((format!("b{}_gain", i), band.gain));
                params.push((format!("b{}_q", i), 1.0 / band.q)); // SC expects reciprocal Q
                params.push((format!("b{}_on", i), if band.enabled { 1.0 } else { 0.0 }));
            }

            let client = self.backend.as_ref().ok_or("Not connected")?;
            client.create_synth("imbolc_eq12", node_id, GROUP_PROCESSING, &params)
                .map_err(|e| e.to_string())?;

            eq_node = Some(node_id);
            current_bus = eq_out_bus;
        }

        // Effects
        for effect in instrument.effects.iter() {
            if !effect.enabled {
                continue;
            }
            let node_id = self.next_node_id;
            self.next_node_id += 1;
            // Use mono bus unless the effect doesn't have a mono variant (inherently stereo)
            let effect_channels = if is_mono && effect.effect_type.has_mono_variant() {
                1
            } else {
                2
            };
            let effect_out_bus = self.bus_allocator.get_or_alloc_audio_bus_with_channels(
                instrument.id,
                &format!("fx_{}_out", effect.id),
                effect_channels,
            );

            let mut params: Vec<(String, f32)> = vec![
                ("in".to_string(), current_bus as f32),
                ("out".to_string(), effect_out_bus as f32),
            ];
            for p in &effect.params {
                if effect.effect_type == EffectType::SidechainComp && p.name == "sc_bus" {
                    let bus_id = match &p.value {
                        ParamValue::Int(v) => *v as u8,
                        _ => 0,
                    };
                    let sidechain_in = if bus_id == 0 {
                        0.0
                    } else {
                        self.bus_audio_buses.get(&bus_id).copied().unwrap_or(0) as f32
                    };
                    params.push(("sidechain_in".to_string(), sidechain_in));
                    continue;
                }
                if effect.effect_type == EffectType::ConvolutionReverb && p.name == "ir_buffer" {
                    let buffer_id = match &p.value {
                        ParamValue::Int(v) => *v,
                        _ => -1,
                    };
                    let sc_bufnum = if buffer_id >= 0 {
                        self.buffer_map.get(&(buffer_id as u32)).copied().unwrap_or(-1) as f32
                    } else {
                        -1.0
                    };
                    params.push(("ir_buffer".to_string(), sc_bufnum));
                    continue;
                }
                params.push((p.name.clone(), p.value.to_f32()));
            }

            // Inject LFO mod bus if targeting this effect type
            if instrument.lfo.enabled {
                if let Some(lfo_bus) = lfo_control_bus {
                    match (instrument.lfo.target, effect.effect_type) {
                        (ParameterTarget::DelayTime, EffectType::Delay) => {
                            params.push(("time_mod_in".to_string(), lfo_bus as f32));
                        }
                        (ParameterTarget::DelayFeedback, EffectType::Delay) => {
                            params.push(("feedback_mod_in".to_string(), lfo_bus as f32));
                        }
                        (ParameterTarget::ReverbMix, EffectType::Reverb) => {
                            params.push(("mix_mod_in".to_string(), lfo_bus as f32));
                        }
                        (ParameterTarget::GateRate, EffectType::Gate) => {
                            params.push(("rate_mod_in".to_string(), lfo_bus as f32));
                        }
                        _ => {}
                    }
                }
            }

            let client = self.backend.as_ref().ok_or("Not connected")?;
            let use_mono_effect = is_mono && effect.effect_type.has_mono_variant();
            client.create_synth(
                Self::effect_synth_def(effect.effect_type, use_mono_effect), node_id, GROUP_PROCESSING, &params,
            ).map_err(|e| e.to_string())?;

            // For VST effects, open the plugin after creating the node
            if let EffectType::Vst(vst_id) = effect.effect_type {
                if let Some(plugin) = session.vst_plugins.get(vst_id) {
                    let _ = client.send_unit_cmd(
                        node_id, VST_UGEN_INDEX, "/open",
                        vec![RawArg::Str(plugin.plugin_path.to_string_lossy().to_string())],
                    );
                }
            }

            effect_nodes.insert(effect.id, node_id);
            effect_order.push(effect.id);
            current_bus = effect_out_bus;
        }

        // Output synth
        let output_node_id = {
            let node_id = self.next_node_id;
            self.next_node_id += 1;
            let mute = if any_solo { !instrument.solo } else { instrument.mute || session.mixer.master_mute };

            let pan_mod_bus = if instrument.lfo.enabled && instrument.lfo.target == ParameterTarget::Pan {
                lfo_control_bus.map(|b| b as f32).unwrap_or(-1.0)
            } else {
                -1.0
            };

            // Determine output destination: layer group bus, mixer bus, or master (0)
            let output_bus = if let Some(group_id) = instrument.layer_group {
                self.layer_group_audio_buses.get(&group_id).copied().unwrap_or(0) as f32
            } else {
                match instrument.output_target {
                    crate::state::instrument::OutputTarget::Bus(id) => {
                        self.bus_audio_buses.get(&id).copied().unwrap_or(0) as f32
                    }
                    crate::state::instrument::OutputTarget::Master => 0.0,
                }
            };

            let params = vec![
                ("in".to_string(), current_bus as f32),
                ("out".to_string(), output_bus),
                ("level".to_string(), instrument.level * session.mixer.master_level),
                ("mute".to_string(), if mute { 1.0 } else { 0.0 }),
                ("pan".to_string(), instrument.pan),
                ("pan_mod_in".to_string(), pan_mod_bus),
            ];

            // Use mono output synth for mono instruments (uses Pan2 instead of Balance2)
            let output_synth_def = if is_mono { "imbolc_output_mono" } else { "imbolc_output" };
            let client = self.backend.as_ref().ok_or("Not connected")?;
            client.create_synth(output_synth_def, node_id, GROUP_OUTPUT, &params)
                .map_err(|e| e.to_string())?;
            node_id
        };

        self.instrument_final_buses.insert(instrument.id, current_bus);

        let inst_nodes = InstrumentNodes {
            source: source_node,
            lfo: lfo_node,
            filter: filter_node,
            eq: eq_node,
            effects: effect_nodes,
            effect_order,
            output: output_node_id,
        };
        for nid in inst_nodes.all_node_ids() {
            self.node_registry.register(nid);
        }
        self.node_map.insert(instrument.id, inst_nodes);

        Ok(())
    }

    /// Create send synths for a single instrument.
    fn build_instrument_sends(
        &mut self,
        instrument: &Instrument,
    ) -> Result<(), String> {
        let source_out_bus = self.bus_allocator.get_audio_bus(instrument.id, "source_out").unwrap_or(16);
        let is_mono = instrument.channel_config.is_mono();

        let send_lfo_bus = if instrument.lfo.enabled && matches!(instrument.lfo.target, ParameterTarget::SendLevel(_)) {
            self.bus_allocator.get_control_bus(instrument.id, "lfo_out")
                .map(|b| b as f32)
                .unwrap_or(-1.0)
        } else {
            -1.0
        };

        for send in &instrument.sends {
            if !send.enabled || send.level <= 0.0 {
                continue;
            }
            if let Some(&bus_audio) = self.bus_audio_buses.get(&send.bus_id) {
                let tap_bus = match send.tap_point {
                    SendTapPoint::PreInsert => source_out_bus,
                    SendTapPoint::PostInsert => {
                        self.instrument_final_buses.get(&instrument.id)
                            .copied()
                            .unwrap_or(source_out_bus)
                    }
                };
                let node_id = self.next_node_id;
                self.next_node_id += 1;
                let mut params = vec![
                    ("in".to_string(), tap_bus as f32),
                    ("out".to_string(), bus_audio as f32),
                    ("level".to_string(), send.level),
                ];
                if send_lfo_bus >= 0.0 {
                    params.push(("level_mod_in".to_string(), send_lfo_bus));
                }
                let send_synth_def = if is_mono { "imbolc_send_mono" } else { "imbolc_send" };
                let client = self.backend.as_ref().ok_or("Not connected")?;
                client.create_synth(send_synth_def, node_id, GROUP_OUTPUT, &params)
                    .map_err(|e| e.to_string())?;
                self.node_registry.register(node_id);
                self.send_node_map.insert((instrument.id, send.bus_id), node_id);
            }
        }

        Ok(())
    }

    /// Restore saved VST param values for a single instrument's source and effects.
    fn restore_instrument_vst_params(
        &self,
        instrument: &Instrument,
    ) {
        let client = match self.backend.as_ref() {
            Some(c) => c,
            None => return,
        };

        if matches!(instrument.source, SourceType::Vst(_)) {
            if let Some(source_node) = self.node_map.get(&instrument.id).and_then(|n| n.source) {
                for &(param_index, value) in &instrument.vst_param_values {
                    let _ = client.send_unit_cmd(
                        source_node, VST_UGEN_INDEX, "/set",
                        vec![RawArg::Int(param_index as i32), RawArg::Float(value)],
                    );
                }
            }
        }
        for effect in &instrument.effects {
            if !effect.enabled { continue; }
            if matches!(effect.effect_type, EffectType::Vst(_)) {
                if let Some(&node) = self.node_map.get(&instrument.id)
                    .and_then(|n| n.effects.get(&effect.id)) {
                    for &(param_index, value) in &effect.vst_param_values {
                        let _ = client.send_unit_cmd(
                            node, VST_UGEN_INDEX, "/set",
                            vec![RawArg::Int(param_index as i32), RawArg::Float(value)],
                        );
                    }
                }
            }
        }

    }

    // ── Bus / layer group effect chain builders ───────────────────

    /// Build an effect chain for a mixer bus. Returns the final audio bus after effects.
    /// If no enabled effects, returns `bus_audio` unchanged.
    fn build_bus_effect_chain(
        &mut self,
        bus: &MixerBus,
        bus_audio: i32,
        session: &SessionState,
    ) -> Result<i32, String> {
        let mut current_bus = bus_audio;

        for effect in &bus.effects {
            if !effect.enabled {
                continue;
            }
            let node_id = self.next_node_id;
            self.next_node_id += 1;
            let effect_out_bus = self.bus_allocator.get_or_alloc_audio_bus(
                u32::MAX - bus.id as u32,
                &format!("bus_fx_{}_out", effect.id),
            );

            let mut params: Vec<(String, f32)> = vec![
                ("in".to_string(), current_bus as f32),
                ("out".to_string(), effect_out_bus as f32),
            ];
            for p in &effect.params {
                if effect.effect_type == EffectType::SidechainComp && p.name == "sc_bus" {
                    let sc_bus_id = match &p.value {
                        ParamValue::Int(v) => *v as u8,
                        _ => 0,
                    };
                    let sidechain_in = if sc_bus_id == 0 {
                        0.0
                    } else {
                        self.bus_audio_buses.get(&sc_bus_id).copied().unwrap_or(0) as f32
                    };
                    params.push(("sidechain_in".to_string(), sidechain_in));
                    continue;
                }
                if effect.effect_type == EffectType::ConvolutionReverb && p.name == "ir_buffer" {
                    let buffer_id = match &p.value {
                        ParamValue::Int(v) => *v,
                        _ => -1,
                    };
                    let sc_bufnum = if buffer_id >= 0 {
                        self.buffer_map.get(&(buffer_id as u32)).copied().unwrap_or(-1) as f32
                    } else {
                        -1.0
                    };
                    params.push(("ir_buffer".to_string(), sc_bufnum));
                    continue;
                }
                params.push((p.name.clone(), p.value.to_f32()));
            }

            let client = self.backend.as_ref().ok_or("Not connected")?;
            client.create_synth(
                Self::effect_synth_def(effect.effect_type, false),
                node_id,
                GROUP_BUS_PROCESSING,
                &params,
            ).map_err(|e| e.to_string())?;

            // For VST effects, open the plugin after creating the node
            if let EffectType::Vst(vst_id) = effect.effect_type {
                if let Some(plugin) = session.vst_plugins.get(vst_id) {
                    let _ = client.send_unit_cmd(
                        node_id, VST_UGEN_INDEX, "/open",
                        vec![RawArg::Str(plugin.plugin_path.to_string_lossy().to_string())],
                    );
                }
            }

            self.node_registry.register(node_id);
            self.bus_effect_node_map.insert((bus.id, effect.id), node_id);
            current_bus = effect_out_bus;
        }

        Ok(current_bus)
    }

    /// Build an effect chain for a layer group mixer. Returns the final audio bus after effects.
    /// If no enabled effects, returns `group_bus` unchanged.
    fn build_layer_group_effect_chain(
        &mut self,
        gm: &LayerGroupMixer,
        group_bus: i32,
        session: &SessionState,
    ) -> Result<i32, String> {
        let mut current_bus = group_bus;

        for effect in &gm.effects {
            if !effect.enabled {
                continue;
            }
            let node_id = self.next_node_id;
            self.next_node_id += 1;
            let effect_out_bus = self.bus_allocator.get_or_alloc_audio_bus(
                u32::MAX - 256 - gm.group_id,
                &format!("group_fx_{}_out", effect.id),
            );

            let mut params: Vec<(String, f32)> = vec![
                ("in".to_string(), current_bus as f32),
                ("out".to_string(), effect_out_bus as f32),
            ];
            for p in &effect.params {
                if effect.effect_type == EffectType::SidechainComp && p.name == "sc_bus" {
                    let sc_bus_id = match &p.value {
                        ParamValue::Int(v) => *v as u8,
                        _ => 0,
                    };
                    let sidechain_in = if sc_bus_id == 0 {
                        0.0
                    } else {
                        self.bus_audio_buses.get(&sc_bus_id).copied().unwrap_or(0) as f32
                    };
                    params.push(("sidechain_in".to_string(), sidechain_in));
                    continue;
                }
                if effect.effect_type == EffectType::ConvolutionReverb && p.name == "ir_buffer" {
                    let buffer_id = match &p.value {
                        ParamValue::Int(v) => *v,
                        _ => -1,
                    };
                    let sc_bufnum = if buffer_id >= 0 {
                        self.buffer_map.get(&(buffer_id as u32)).copied().unwrap_or(-1) as f32
                    } else {
                        -1.0
                    };
                    params.push(("ir_buffer".to_string(), sc_bufnum));
                    continue;
                }
                params.push((p.name.clone(), p.value.to_f32()));
            }

            let client = self.backend.as_ref().ok_or("Not connected")?;
            client.create_synth(
                Self::effect_synth_def(effect.effect_type, false),
                node_id,
                GROUP_BUS_PROCESSING,
                &params,
            ).map_err(|e| e.to_string())?;

            // For VST effects, open the plugin after creating the node
            if let EffectType::Vst(vst_id) = effect.effect_type {
                if let Some(plugin) = session.vst_plugins.get(vst_id) {
                    let _ = client.send_unit_cmd(
                        node_id, VST_UGEN_INDEX, "/open",
                        vec![RawArg::Str(plugin.plugin_path.to_string_lossy().to_string())],
                    );
                }
            }

            self.node_registry.register(node_id);
            self.layer_group_effect_node_map.insert((gm.group_id, effect.id), node_id);
            current_bus = effect_out_bus;
        }

        Ok(current_bus)
    }

    // ── Public routing methods ────────────────────────────────────

    /// Rebuild all routing based on instrument state.
    /// Per instrument, create a deterministic synth chain:
    /// 1. Source synth
    /// 2. Optional filter synth
    /// 3. Effect synths in order
    /// 4. Output synth with level/pan/mute
    pub fn rebuild_instrument_routing(&mut self, state: &InstrumentState, session: &SessionState) -> Result<(), String> {
        if !self.is_running {
            return Ok(());
        }

        self.ensure_groups()?;
        self.ensure_safety_limiter()?;

        // Free all existing synths and voices
        if let Some(ref client) = self.backend {
            for nodes in self.node_map.values() {
                for node_id in nodes.all_node_ids() {
                    let _ = client.free_node(node_id);
                }
            }
            for &node_id in self.send_node_map.values() {
                let _ = client.free_node(node_id);
            }
            for &node_id in self.bus_node_map.values() {
                let _ = client.free_node(node_id);
            }
            for &node_id in self.layer_group_node_map.values() {
                let _ = client.free_node(node_id);
            }
            for &node_id in self.layer_group_send_node_map.values() {
                let _ = client.free_node(node_id);
            }
            for &node_id in self.bus_effect_node_map.values() {
                let _ = client.free_node(node_id);
            }
            for &node_id in self.layer_group_effect_node_map.values() {
                let _ = client.free_node(node_id);
            }
            for chain in self.voice_allocator.drain_all() {
                let _ = client.free_node(chain.group_id);
            }
        }
        self.node_map.clear();
        self.send_node_map.clear();
        self.bus_node_map.clear();
        self.bus_effect_node_map.clear();
        self.layer_group_effect_node_map.clear();
        self.layer_group_audio_buses.clear();
        self.layer_group_node_map.clear();
        self.layer_group_send_node_map.clear();
        self.bus_audio_buses.clear();
        self.instrument_final_buses.clear();
        self.bus_allocator.reset();
        self.node_registry.invalidate_all();

        // Allocate audio buses for each mixer bus first (needed by BusIn instruments)
        for bus in &session.mixer.buses {
            let bus_audio = self.bus_allocator.get_or_alloc_audio_bus(
                u32::MAX - bus.id as u32,
                "bus_out",
            );
            self.bus_audio_buses.insert(bus.id, bus_audio);
        }

        // Allocate audio buses for each active layer group
        for group_id in state.active_layer_groups() {
            let group_bus = self.bus_allocator.get_or_alloc_audio_bus(
                u32::MAX - 256 - group_id,
                "layer_group_out",
            );
            self.layer_group_audio_buses.insert(group_id, group_bus);
        }

        // Build signal chain for each instrument
        let any_solo = state.any_instrument_solo();
        for instrument in &state.instruments {
            self.build_instrument_chain(instrument, any_solo, session)?;
        }

        // Sync voice allocator bus watermarks from bus allocator
        self.voice_allocator.sync_bus_watermarks(self.bus_allocator.next_audio_bus, self.bus_allocator.next_control_bus);

        // Create send synths
        for instrument in &state.instruments {
            self.build_instrument_sends(instrument)?;
        }

        // Create layer group effects + outputs + sends (before buses so group outputs
        // mix into bus_audio before bus effects read it)
        for group_mixer in &session.mixer.layer_group_mixers {
            if let Some(&group_bus) = self.layer_group_audio_buses.get(&group_mixer.group_id) {
                // Build effect chain for this layer group
                let post_effect_bus = self.build_layer_group_effect_chain(group_mixer, group_bus, session)?;

                let node_id = self.next_node_id;
                self.next_node_id += 1;
                let mute = session.mixer.effective_layer_group_mute(group_mixer);

                // Group output destination
                let group_out = match group_mixer.output_target {
                    crate::state::instrument::OutputTarget::Bus(id) => {
                        self.bus_audio_buses.get(&id).copied().unwrap_or(0) as f32
                    }
                    crate::state::instrument::OutputTarget::Master => 0.0,
                };

                let params = vec![
                    ("in".to_string(), post_effect_bus as f32),
                    ("out".to_string(), group_out),
                    ("level".to_string(), group_mixer.level),
                    ("mute".to_string(), if mute { 1.0 } else { 0.0 }),
                    ("pan".to_string(), group_mixer.pan),
                ];
                if let Some(ref client) = self.backend {
                    client
                        .create_synth("imbolc_bus_out", node_id, GROUP_BUS_PROCESSING, &params)
                        .map_err(|e| e.to_string())?;
                }
                self.node_registry.register(node_id);
                self.layer_group_node_map.insert(group_mixer.group_id, node_id);

                // Create group-level sends
                for send in &group_mixer.sends {
                    if !send.enabled || send.level <= 0.0 {
                        continue;
                    }
                    if let Some(&bus_audio) = self.bus_audio_buses.get(&send.bus_id) {
                        let send_node_id = self.next_node_id;
                        self.next_node_id += 1;
                        let send_params = vec![
                            ("in".to_string(), group_bus as f32),
                            ("out".to_string(), bus_audio as f32),
                            ("level".to_string(), send.level),
                        ];
                        if let Some(ref client) = self.backend {
                            client
                                .create_synth("imbolc_send", send_node_id, GROUP_BUS_PROCESSING, &send_params)
                                .map_err(|e| e.to_string())?;
                        }
                        self.node_registry.register(send_node_id);
                        self.layer_group_send_node_map
                            .insert((group_mixer.group_id, send.bus_id), send_node_id);
                    }
                }
            }
        }

        // Create bus effects + output synths
        for bus in &session.mixer.buses {
            if let Some(&bus_audio) = self.bus_audio_buses.get(&bus.id) {
                // Build effect chain for this bus
                let post_effect_bus = self.build_bus_effect_chain(bus, bus_audio, session)?;

                let node_id = self.next_node_id;
                self.next_node_id += 1;
                let mute = session.effective_bus_mute(bus);
                let params = vec![
                    ("in".to_string(), post_effect_bus as f32),
                    ("level".to_string(), bus.level),
                    ("mute".to_string(), if mute { 1.0 } else { 0.0 }),
                    ("pan".to_string(), bus.pan),
                ];
                if let Some(ref client) = self.backend {
                    client
                        .create_synth("imbolc_bus_out", node_id, GROUP_BUS_PROCESSING, &params)
                        .map_err(|e| e.to_string())?;
                }
                self.node_registry.register(node_id);
                self.bus_node_map.insert(bus.id, node_id);
            }
        }

        // Restore saved VST param values
        for instrument in &state.instruments {
            self.restore_instrument_vst_params(instrument);
        }

        // (Re)create meter synth
        self.restart_meter();

        Ok(())
    }

    /// Rebuild routing for a single instrument without tearing down the entire graph.
    /// Frees only that instrument's nodes (source, filter, EQ, effects, output, sends)
    /// and recreates them. Other instruments remain untouched.
    pub fn rebuild_single_instrument_routing(
        &mut self,
        instrument_id: InstrumentId,
        state: &InstrumentState,
        session: &SessionState,
    ) -> Result<(), String> {
        if !self.is_running {
            return Ok(());
        }

        let instrument = match state.instruments.iter().find(|i| i.id == instrument_id) {
            Some(i) => i,
            None => return Err(format!("Instrument {} not found", instrument_id)),
        };

        // 1. Free existing nodes for this instrument
        {
            let client = self.backend.as_ref().ok_or("Not connected")?;

            if let Some(nodes) = self.node_map.remove(&instrument_id) {
                for node_id in nodes.all_node_ids() {
                    self.node_registry.unregister(node_id);
                    let _ = client.free_node(node_id);
                }
            }

            // Free this instrument's send nodes
            let send_keys: Vec<(InstrumentId, u8)> = self.send_node_map.keys()
                .filter(|(id, _)| *id == instrument_id)
                .copied()
                .collect();
            for key in send_keys {
                if let Some(node_id) = self.send_node_map.remove(&key) {
                    self.node_registry.unregister(node_id);
                    let _ = client.free_node(node_id);
                }
            }

            // Free active voices for this instrument
            let drained = self.voice_allocator.drain_instrument(instrument_id);
            for voice in &drained {
                self.node_registry.unregister(voice.group_id);
                self.node_registry.unregister(voice.midi_node_id);
                self.node_registry.unregister(voice.source_node);
                let _ = client.free_node(voice.group_id);
            }
        }

        // Remove old final bus entry
        self.instrument_final_buses.remove(&instrument_id);

        // 2. Recreate the signal chain
        let any_solo = state.any_instrument_solo();
        self.build_instrument_chain(instrument, any_solo, session)?;

        // Sync voice allocator bus watermarks
        self.voice_allocator.sync_bus_watermarks(self.bus_allocator.next_audio_bus, self.bus_allocator.next_control_bus);

        self.build_instrument_sends(instrument)?;
        self.restore_instrument_vst_params(instrument);

        Ok(())
    }

    // ── Phased routing rebuild (driven by AudioThread) ─────────

    /// Execute one phase of the amortized routing rebuild state machine.
    /// Returns `Continue(next_phase)` if more work remains, `Done` if complete.
    /// Errors are non-fatal: the rebuild is abandoned and engine state may be partial.
    pub(crate) fn routing_rebuild_step(
        &mut self,
        phase: RoutingRebuildPhase,
        state: &InstrumentState,
        session: &SessionState,
    ) -> Result<RebuildStepResult, String> {
        match phase {
            RoutingRebuildPhase::TearDown => {
                if !self.is_running {
                    return Ok(RebuildStepResult::Done);
                }
                self.ensure_groups()?;
                self.ensure_safety_limiter()?;

                // Free all existing synths and voices
                if let Some(ref client) = self.backend {
                    for nodes in self.node_map.values() {
                        for node_id in nodes.all_node_ids() {
                            let _ = client.free_node(node_id);
                        }
                    }
                    for &node_id in self.send_node_map.values() {
                        let _ = client.free_node(node_id);
                    }
                    for &node_id in self.bus_node_map.values() {
                        let _ = client.free_node(node_id);
                    }
                    for &node_id in self.layer_group_node_map.values() {
                        let _ = client.free_node(node_id);
                    }
                    for &node_id in self.layer_group_send_node_map.values() {
                        let _ = client.free_node(node_id);
                    }
                    for &node_id in self.bus_effect_node_map.values() {
                        let _ = client.free_node(node_id);
                    }
                    for &node_id in self.layer_group_effect_node_map.values() {
                        let _ = client.free_node(node_id);
                    }
                    for chain in self.voice_allocator.drain_all() {
                        let _ = client.free_node(chain.group_id);
                    }
                }
                self.node_map.clear();
                self.send_node_map.clear();
                self.bus_node_map.clear();
                self.bus_effect_node_map.clear();
                self.layer_group_effect_node_map.clear();
                self.layer_group_audio_buses.clear();
                self.layer_group_node_map.clear();
                self.layer_group_send_node_map.clear();
                self.bus_audio_buses.clear();
                self.instrument_final_buses.clear();
                self.bus_allocator.reset();
                self.node_registry.invalidate_all();

                Ok(RebuildStepResult::Continue(RoutingRebuildPhase::AllocBuses))
            }

            RoutingRebuildPhase::AllocBuses => {
                // Allocate audio buses for each mixer bus (needed by BusIn instruments)
                for bus in &session.mixer.buses {
                    let bus_audio = self.bus_allocator.get_or_alloc_audio_bus(
                        u32::MAX - bus.id as u32,
                        "bus_out",
                    );
                    self.bus_audio_buses.insert(bus.id, bus_audio);
                }

                // Allocate audio buses for each active layer group
                for group_id in state.active_layer_groups() {
                    let group_bus = self.bus_allocator.get_or_alloc_audio_bus(
                        u32::MAX - 256 - group_id,
                        "layer_group_out",
                    );
                    self.layer_group_audio_buses.insert(group_id, group_bus);
                }

                if state.instruments.is_empty() {
                    Ok(RebuildStepResult::Continue(RoutingRebuildPhase::BuildOutputs))
                } else {
                    Ok(RebuildStepResult::Continue(RoutingRebuildPhase::BuildInstrument(0)))
                }
            }

            RoutingRebuildPhase::BuildInstrument(i) => {
                let any_solo = state.any_instrument_solo();
                if let Some(instrument) = state.instruments.get(i) {
                    self.build_instrument_chain(instrument, any_solo, session)?;
                    self.build_instrument_sends(instrument)?;

                    let next = i + 1;
                    if next < state.instruments.len() {
                        Ok(RebuildStepResult::Continue(RoutingRebuildPhase::BuildInstrument(next)))
                    } else {
                        Ok(RebuildStepResult::Continue(RoutingRebuildPhase::BuildOutputs))
                    }
                } else {
                    Ok(RebuildStepResult::Continue(RoutingRebuildPhase::BuildOutputs))
                }
            }

            RoutingRebuildPhase::BuildOutputs => {
                // Create layer group effects + outputs + sends (before buses so group
                // outputs mix into bus_audio before bus effects read it)
                for group_mixer in &session.mixer.layer_group_mixers {
                    if let Some(&group_bus) = self.layer_group_audio_buses.get(&group_mixer.group_id) {
                        let post_effect_bus = self.build_layer_group_effect_chain(group_mixer, group_bus, session)?;

                        let node_id = self.next_node_id;
                        self.next_node_id += 1;
                        let mute = session.mixer.effective_layer_group_mute(group_mixer);

                        let group_out = match group_mixer.output_target {
                            crate::state::instrument::OutputTarget::Bus(id) => {
                                self.bus_audio_buses.get(&id).copied().unwrap_or(0) as f32
                            }
                            crate::state::instrument::OutputTarget::Master => 0.0,
                        };

                        let params = vec![
                            ("in".to_string(), post_effect_bus as f32),
                            ("out".to_string(), group_out),
                            ("level".to_string(), group_mixer.level),
                            ("mute".to_string(), if mute { 1.0 } else { 0.0 }),
                            ("pan".to_string(), group_mixer.pan),
                        ];
                        if let Some(ref client) = self.backend {
                            client
                                .create_synth("imbolc_bus_out", node_id, GROUP_BUS_PROCESSING, &params)
                                .map_err(|e| e.to_string())?;
                        }
                        self.node_registry.register(node_id);
                        self.layer_group_node_map.insert(group_mixer.group_id, node_id);

                        for send in &group_mixer.sends {
                            if !send.enabled || send.level <= 0.0 {
                                continue;
                            }
                            if let Some(&bus_audio) = self.bus_audio_buses.get(&send.bus_id) {
                                let send_node_id = self.next_node_id;
                                self.next_node_id += 1;
                                let send_params = vec![
                                    ("in".to_string(), group_bus as f32),
                                    ("out".to_string(), bus_audio as f32),
                                    ("level".to_string(), send.level),
                                ];
                                if let Some(ref client) = self.backend {
                                    client
                                        .create_synth("imbolc_send", send_node_id, GROUP_BUS_PROCESSING, &send_params)
                                        .map_err(|e| e.to_string())?;
                                }
                                self.node_registry.register(send_node_id);
                                self.layer_group_send_node_map
                                    .insert((group_mixer.group_id, send.bus_id), send_node_id);
                            }
                        }
                    }
                }

                // Create bus effects + output synths
                for bus in &session.mixer.buses {
                    if let Some(&bus_audio) = self.bus_audio_buses.get(&bus.id) {
                        let post_effect_bus = self.build_bus_effect_chain(bus, bus_audio, session)?;

                        let node_id = self.next_node_id;
                        self.next_node_id += 1;
                        let mute = session.effective_bus_mute(bus);
                        let params = vec![
                            ("in".to_string(), post_effect_bus as f32),
                            ("level".to_string(), bus.level),
                            ("mute".to_string(), if mute { 1.0 } else { 0.0 }),
                            ("pan".to_string(), bus.pan),
                        ];
                        if let Some(ref client) = self.backend {
                            client
                                .create_synth("imbolc_bus_out", node_id, GROUP_BUS_PROCESSING, &params)
                                .map_err(|e| e.to_string())?;
                        }
                        self.node_registry.register(node_id);
                        self.bus_node_map.insert(bus.id, node_id);
                    }
                }

                // Restore saved VST param values
                for instrument in &state.instruments {
                    self.restore_instrument_vst_params(instrument);
                }

                Ok(RebuildStepResult::Continue(RoutingRebuildPhase::Finalize))
            }

            RoutingRebuildPhase::Finalize => {
                // Sync voice allocator bus watermarks
                self.voice_allocator.sync_bus_watermarks(
                    self.bus_allocator.next_audio_bus,
                    self.bus_allocator.next_control_bus,
                );

                // (Re)create meter synth
                self.restart_meter();

                Ok(RebuildStepResult::Done)
            }
        }
    }

    /// Set bus output mixer params (level, mute, pan) in real-time
    pub fn set_bus_mixer_params(&self, bus_id: u8, level: f32, mute: bool, pan: f32) -> Result<(), String> {
        let client = self.backend.as_ref().ok_or("Not connected")?;
        let node_id = self.bus_node_map
            .get(&bus_id)
            .ok_or_else(|| format!("No bus output node for bus{}", bus_id))?;
        client.set_param(*node_id, "level", level).map_err(|e| e.to_string())?;
        client.set_param(*node_id, "mute", if mute { 1.0 } else { 0.0 }).map_err(|e| e.to_string())?;
        client.set_param(*node_id, "pan", pan).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Set layer group output mixer params (level, mute, pan) in real-time
    pub fn set_layer_group_mixer_params(&self, group_id: u32, level: f32, mute: bool, pan: f32) -> Result<(), String> {
        let client = self.backend.as_ref().ok_or("Not connected")?;
        let node_id = self.layer_group_node_map
            .get(&group_id)
            .ok_or_else(|| format!("No layer group output node for group {}", group_id))?;
        client.set_param(*node_id, "level", level).map_err(|e| e.to_string())?;
        client.set_param(*node_id, "mute", if mute { 1.0 } else { 0.0 }).map_err(|e| e.to_string())?;
        client.set_param(*node_id, "pan", pan).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Update all instrument output mixer params (level, mute, pan) in real-time without rebuilding the graph
    pub fn update_all_instrument_mixer_params(&self, state: &InstrumentState, session: &SessionState) -> Result<(), String> {
        if !self.is_running { return Ok(()); }
        let client = self.backend.as_ref().ok_or("Not connected")?;
        let any_solo = state.any_instrument_solo();
        for instrument in &state.instruments {
            if let Some(nodes) = self.node_map.get(&instrument.id) {
                let mute = instrument.mute || session.mixer.master_mute || (any_solo && !instrument.solo);
                client.set_param(nodes.output, "level", instrument.level * session.mixer.master_level)
                    .map_err(|e| e.to_string())?;
                client.set_param(nodes.output, "mute", if mute { 1.0 } else { 0.0 })
                    .map_err(|e| e.to_string())?;
                client.set_param(nodes.output, "pan", instrument.pan)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    /// Set a source parameter on an instrument in real-time.
    /// Updates the persistent source node (AudioIn) and all active voice source nodes.
    pub fn set_source_param(&self, instrument_id: InstrumentId, param: &str, value: f32) -> Result<(), String> {
        if !self.is_running { return Ok(()); }
        let client = self.backend.as_ref().ok_or("Not connected")?;

        // BusIn "bus" param: translate bus_id (1-8) to SC audio bus number
        if param == "bus" {
            let bus_id = value as u8;
            if let Some(&audio_bus) = self.bus_audio_buses.get(&bus_id) {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(source_node) = nodes.source {
                        let _ = client.set_param(source_node, "in", audio_bus as f32);
                    }
                }
            }
            return Ok(());
        }

        // Set on persistent source node (AudioIn/BusIn instruments)
        if let Some(nodes) = self.node_map.get(&instrument_id) {
            if let Some(source_node) = nodes.source {
                let _ = client.set_param(source_node, param, value);
            }
        }

        // Set on all active voice source nodes (oscillator/sampler instruments)
        for voice in self.voice_allocator.chains() {
            if voice.instrument_id == instrument_id {
                let _ = client.set_param(voice.source_node, param, value);
            }
        }

        Ok(())
    }

    /// Set an EQ parameter on an instrument in real-time.
    pub fn set_eq_param(&self, instrument_id: InstrumentId, param: &str, value: f32) -> Result<(), String> {
        if !self.is_running { return Ok(()); }
        let client = self.backend.as_ref().ok_or("Not connected")?;

        if let Some(nodes) = self.node_map.get(&instrument_id) {
            if let Some(eq_node) = nodes.eq {
                let _ = client.set_param(eq_node, param, value);
            }
        }

        Ok(())
    }

    /// Set a filter parameter on an instrument in real-time (targeted /n_set, no rebuild).
    pub fn set_filter_param(&self, instrument_id: InstrumentId, param: &str, value: f32) -> Result<(), String> {
        if !self.is_running { return Ok(()); }
        let client = self.backend.as_ref().ok_or("Not connected")?;

        if let Some(nodes) = self.node_map.get(&instrument_id) {
            if let Some(filter_node) = nodes.filter {
                let _ = client.set_param(filter_node, param, value);
            }
        }

        Ok(())
    }

    /// Set an effect parameter on an instrument in real-time (targeted /n_set, no rebuild).
    pub fn set_effect_param(&self, instrument_id: InstrumentId, effect_id: EffectId, param: &str, value: f32) -> Result<(), String> {
        if !self.is_running { return Ok(()); }
        let client = self.backend.as_ref().ok_or("Not connected")?;

        if let Some(nodes) = self.node_map.get(&instrument_id) {
            if let Some(&effect_node) = nodes.effects.get(&effect_id) {
                let _ = client.set_param(effect_node, param, value);
            }
        }

        Ok(())
    }

    /// Set an LFO parameter on an instrument in real-time (targeted /n_set, no rebuild).
    pub fn set_lfo_param(&self, instrument_id: InstrumentId, param: &str, value: f32) -> Result<(), String> {
        if !self.is_running { return Ok(()); }
        let client = self.backend.as_ref().ok_or("Not connected")?;

        if let Some(nodes) = self.node_map.get(&instrument_id) {
            if let Some(lfo_node) = nodes.lfo {
                let _ = client.set_param(lfo_node, param, value);
            }
        }

        Ok(())
    }

    /// Set a bus effect parameter in real-time (targeted /n_set, no rebuild).
    pub fn set_bus_effect_param(&self, bus_id: u8, effect_id: EffectId, param: &str, value: f32) -> Result<(), String> {
        if !self.is_running { return Ok(()); }
        let client = self.backend.as_ref().ok_or("Not connected")?;

        if let Some(&node_id) = self.bus_effect_node_map.get(&(bus_id, effect_id)) {
            let _ = client.set_param(node_id, param, value);
        }

        Ok(())
    }

    /// Set a layer group effect parameter in real-time (targeted /n_set, no rebuild).
    pub fn set_layer_group_effect_param(&self, group_id: u32, effect_id: EffectId, param: &str, value: f32) -> Result<(), String> {
        if !self.is_running { return Ok(()); }
        let client = self.backend.as_ref().ok_or("Not connected")?;

        if let Some(&node_id) = self.layer_group_effect_node_map.get(&(group_id, effect_id)) {
            let _ = client.set_param(node_id, param, value);
        }

        Ok(())
    }
}
