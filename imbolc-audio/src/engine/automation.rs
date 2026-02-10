use super::backend::{AudioBackend, BackendMessage, RawArg, build_n_set_message};
use super::AudioEngine;
use imbolc_types::{AutomationTarget, InstrumentState, SessionState};
use imbolc_types::{BusParameter, GlobalParameter, InstrumentParameter, ParameterTarget};

impl AudioEngine {
    /// Apply an automation value to a target parameter
    /// This updates the appropriate synth node in real-time
    pub fn apply_automation(&self, target: &AutomationTarget, value: f32, state: &mut InstrumentState, session: &SessionState) -> Result<(), String> {
        if !self.is_running {
            return Ok(());
        }
        let backend = self.backend.as_ref().ok_or("Not connected")?;

        match target {
            AutomationTarget::Instrument(instrument_id, param) => {
                self.apply_instrument_automation(&**backend, *instrument_id, param, value, state, session)?;
            }
            AutomationTarget::Bus(bus_id, BusParameter::Level) => {
                if let Some(&node_id) = self.bus_node_map.get(bus_id) {
                    backend.set_param(node_id, "level", value)
                        .map_err(|e| e.to_string())?;
                }
            }
            AutomationTarget::Global(GlobalParameter::Bpm) => {
                // Handled in playback.rs, not here
            }
            AutomationTarget::Global(GlobalParameter::TimeSignature) => {
                // Global time signature changes are handled via session state sync
                // No direct OSC action needed here
            }
        }

        Ok(())
    }

    /// Apply automation to an instrument parameter
    fn apply_instrument_automation(
        &self,
        backend: &dyn AudioBackend,
        instrument_id: u32,
        param: &InstrumentParameter,
        value: f32,
        state: &mut InstrumentState,
        session: &SessionState,
    ) -> Result<(), String> {
        match param {
            InstrumentParameter::Standard(pt) => {
                self.apply_parameter_target(backend, instrument_id, pt, value, state, session)
            }
        }
    }

    /// Apply automation for a ParameterTarget on a specific instrument
    fn apply_parameter_target(
        &self,
        backend: &dyn AudioBackend,
        instrument_id: u32,
        param: &ParameterTarget,
        value: f32,
        state: &mut InstrumentState,
        session: &SessionState,
    ) -> Result<(), String> {
        match param {
            ParameterTarget::Level => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    let effective_level = value * session.mixer.master_level;
                    backend.set_param(nodes.output, "level", effective_level)
                        .map_err(|e| e.to_string())?;
                }
            }
            ParameterTarget::Pan => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    backend.set_param(nodes.output, "pan", value)
                        .map_err(|e| e.to_string())?;
                }
            }
            ParameterTarget::FilterCutoff => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(filter_node) = nodes.filter {
                        backend.set_param(filter_node, "cutoff", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            ParameterTarget::FilterResonance => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(filter_node) = nodes.filter {
                        backend.set_param(filter_node, "resonance", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            ParameterTarget::FilterBypass => {
                let bypassed = value >= 0.5;
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    if let Some(filter) = inst.filter_mut() {
                        filter.enabled = !bypassed;
                    }
                }
            }
            ParameterTarget::EffectParam(effect_id, param_idx) => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(&effect_node) = nodes.effects.get(effect_id) {
                        if let Some(instrument) = state.instrument(instrument_id) {
                            if let Some(effect) = instrument.effect_by_id(*effect_id) {
                                if let Some(param) = effect.params.get(*param_idx) {
                                    backend.set_param(effect_node, &param.name, value)
                                        .map_err(|e| e.to_string())?;
                                }
                            }
                        }
                    }
                }
            }
            ParameterTarget::EffectBypass(effect_id) => {
                let bypassed = value >= 0.5;
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    if let Some(effect) = inst.effect_by_id_mut(*effect_id) {
                        effect.enabled = !bypassed;
                    }
                }
            }
            ParameterTarget::SampleRate => {
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == instrument_id {
                        backend.set_param(voice.source_node, "rate", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            ParameterTarget::SampleAmp => {
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == instrument_id {
                        backend.set_param(voice.source_node, "amp", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            ParameterTarget::LfoRate => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(lfo_node) = nodes.lfo {
                        backend.set_param(lfo_node, "rate", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            ParameterTarget::LfoDepth => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(lfo_node) = nodes.lfo {
                        backend.set_param(lfo_node, "depth", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            ParameterTarget::Attack => {
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    inst.amp_envelope.attack = value;
                }
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == instrument_id {
                        backend.set_param(voice.source_node, "attack", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            ParameterTarget::Decay => {
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    inst.amp_envelope.decay = value;
                }
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == instrument_id {
                        backend.set_param(voice.source_node, "decay", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            ParameterTarget::Sustain => {
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    inst.amp_envelope.sustain = value;
                }
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == instrument_id {
                        backend.set_param(voice.source_node, "sustain", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            ParameterTarget::Release => {
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    inst.amp_envelope.release = value;
                }
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == instrument_id {
                        backend.set_param(voice.source_node, "release", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            ParameterTarget::SendLevel(send_idx) => {
                if let Some(inst) = state.instrument(instrument_id) {
                    if let Some(send) = inst.sends.get(*send_idx) {
                        if let Some(&node_id) = self.send_node_map.get(&(instrument_id, send.bus_id)) {
                            backend.set_param(node_id, "level", value)
                                .map_err(|e| e.to_string())?;
                        }
                    }
                }
            }
            ParameterTarget::VstParam(param_index) => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(source_node) = nodes.source {
                        backend.send_unit_cmd(
                            source_node,
                            super::VST_UGEN_INDEX,
                            "/set",
                            vec![RawArg::Int(*param_index as i32), RawArg::Float(value)],
                        ).map_err(|e| e.to_string())?;
                    }
                }
            }
            ParameterTarget::EqBandFreq(band) => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(eq_node) = nodes.eq {
                        let param_name = format!("b{}_freq", band);
                        backend.set_param(eq_node, &param_name, value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            ParameterTarget::EqBandGain(band) => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(eq_node) = nodes.eq {
                        let param_name = format!("b{}_gain", band);
                        backend.set_param(eq_node, &param_name, value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            ParameterTarget::EqBandQ(band) => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(eq_node) = nodes.eq {
                        let param_name = format!("b{}_q", band);
                        // Q is inverted in SC
                        let sc_value = 1.0 / value;
                        backend.set_param(eq_node, &param_name, sc_value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            // Track groove settings: state-only, no OSC messages needed
            ParameterTarget::Swing => {
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    inst.groove.swing_amount = Some(value);
                }
            }
            ParameterTarget::HumanizeVelocity => {
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    inst.groove.humanize_velocity = Some(value);
                }
            }
            ParameterTarget::HumanizeTiming => {
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    inst.groove.humanize_timing = Some(value);
                }
            }
            ParameterTarget::TimingOffset => {
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    inst.groove.timing_offset_ms = value;
                }
            }
            ParameterTarget::TimeSignature => {
                // Decode time signature from normalized value and update state
                let target = AutomationTarget::track_time_signature(instrument_id);
                if let Some(discrete) = target.normalized_to_discrete(value) {
                    if let imbolc_types::DiscreteValue::TimeSignature(num, denom) = discrete {
                        if let Some(inst) = state.instrument_mut(instrument_id) {
                            inst.groove.time_signature = Some((num, denom));
                        }
                    }
                }
            }
            // Voice synthesis params - state-only for now (future SynthDef support)
            ParameterTarget::Pitch
            | ParameterTarget::PulseWidth
            | ParameterTarget::Detune
            | ParameterTarget::FmIndex
            | ParameterTarget::WavetablePosition
            | ParameterTarget::FormantFreq
            | ParameterTarget::SyncRatio
            | ParameterTarget::Pressure
            | ParameterTarget::Embouchure
            | ParameterTarget::GrainSize
            | ParameterTarget::GrainDensity
            | ParameterTarget::FbFeedback
            | ParameterTarget::RingModDepth
            | ParameterTarget::ChaosParam
            | ParameterTarget::AdditiveRolloff
            | ParameterTarget::MembraneTension
            | ParameterTarget::StretchRatio
            | ParameterTarget::PitchShift
            | ParameterTarget::DelayTime
            | ParameterTarget::DelayFeedback
            | ParameterTarget::ReverbMix
            | ParameterTarget::GateRate => {
                // These are state-only updates for now
                // Future: map to appropriate SynthDef params
            }
        }

        Ok(())
    }

    /// Build backend messages for an automation target without sending them.
    /// Also applies any required state-side mutations (e.g. envelope values).
    /// Returns messages to be batched into a single bundle.
    pub fn collect_automation_messages(
        &self,
        target: &AutomationTarget,
        value: f32,
        state: &mut InstrumentState,
        session: &SessionState,
    ) -> Vec<BackendMessage> {
        let mut msgs = Vec::new();

        match target {
            AutomationTarget::Instrument(instrument_id, param) => {
                self.collect_instrument_messages(&mut msgs, *instrument_id, param, value, state, session);
            }
            AutomationTarget::Bus(bus_id, BusParameter::Level) => {
                if let Some(&node_id) = self.bus_node_map.get(bus_id) {
                    msgs.push(build_n_set_message(node_id, "level", value));
                }
            }
            AutomationTarget::Global(GlobalParameter::Bpm) => {
                // Handled in playback.rs
            }
            AutomationTarget::Global(GlobalParameter::TimeSignature) => {
                // State-only update
            }
        }

        msgs
    }

    /// Collect messages for an instrument parameter
    fn collect_instrument_messages(
        &self,
        msgs: &mut Vec<BackendMessage>,
        instrument_id: u32,
        param: &InstrumentParameter,
        value: f32,
        state: &mut InstrumentState,
        session: &SessionState,
    ) {
        match param {
            InstrumentParameter::Standard(pt) => {
                self.collect_parameter_target_messages(msgs, instrument_id, pt, value, state, session);
            }
        }
    }

    /// Collect messages for a ParameterTarget
    fn collect_parameter_target_messages(
        &self,
        msgs: &mut Vec<BackendMessage>,
        instrument_id: u32,
        param: &ParameterTarget,
        value: f32,
        state: &mut InstrumentState,
        session: &SessionState,
    ) {
        match param {
            ParameterTarget::Level => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    let effective_level = value * session.mixer.master_level;
                    msgs.push(build_n_set_message(nodes.output, "level", effective_level));
                }
            }
            ParameterTarget::Pan => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    msgs.push(build_n_set_message(nodes.output, "pan", value));
                }
            }
            ParameterTarget::FilterCutoff => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(filter_node) = nodes.filter {
                        msgs.push(build_n_set_message(filter_node, "cutoff", value));
                    }
                }
            }
            ParameterTarget::FilterResonance => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(filter_node) = nodes.filter {
                        msgs.push(build_n_set_message(filter_node, "resonance", value));
                    }
                }
            }
            ParameterTarget::FilterBypass => {
                let bypassed = value >= 0.5;
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    if let Some(filter) = inst.filter_mut() {
                        filter.enabled = !bypassed;
                    }
                }
            }
            ParameterTarget::EffectParam(effect_id, param_idx) => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(&effect_node) = nodes.effects.get(effect_id) {
                        if let Some(instrument) = state.instrument(instrument_id) {
                            if let Some(effect) = instrument.effect_by_id(*effect_id) {
                                if let Some(param) = effect.params.get(*param_idx) {
                                    msgs.push(build_n_set_message(effect_node, &param.name, value));
                                }
                            }
                        }
                    }
                }
            }
            ParameterTarget::EffectBypass(effect_id) => {
                let bypassed = value >= 0.5;
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    if let Some(effect) = inst.effect_by_id_mut(*effect_id) {
                        effect.enabled = !bypassed;
                    }
                }
            }
            ParameterTarget::SampleRate => {
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == instrument_id {
                        msgs.push(build_n_set_message(voice.source_node, "rate", value));
                    }
                }
            }
            ParameterTarget::SampleAmp => {
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == instrument_id {
                        msgs.push(build_n_set_message(voice.source_node, "amp", value));
                    }
                }
            }
            ParameterTarget::LfoRate => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(lfo_node) = nodes.lfo {
                        msgs.push(build_n_set_message(lfo_node, "rate", value));
                    }
                }
            }
            ParameterTarget::LfoDepth => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(lfo_node) = nodes.lfo {
                        msgs.push(build_n_set_message(lfo_node, "depth", value));
                    }
                }
            }
            ParameterTarget::Attack => {
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    inst.amp_envelope.attack = value;
                }
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == instrument_id {
                        msgs.push(build_n_set_message(voice.source_node, "attack", value));
                    }
                }
            }
            ParameterTarget::Decay => {
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    inst.amp_envelope.decay = value;
                }
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == instrument_id {
                        msgs.push(build_n_set_message(voice.source_node, "decay", value));
                    }
                }
            }
            ParameterTarget::Sustain => {
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    inst.amp_envelope.sustain = value;
                }
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == instrument_id {
                        msgs.push(build_n_set_message(voice.source_node, "sustain", value));
                    }
                }
            }
            ParameterTarget::Release => {
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    inst.amp_envelope.release = value;
                }
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == instrument_id {
                        msgs.push(build_n_set_message(voice.source_node, "release", value));
                    }
                }
            }
            ParameterTarget::SendLevel(send_idx) => {
                if let Some(inst) = state.instrument(instrument_id) {
                    if let Some(send) = inst.sends.get(*send_idx) {
                        if let Some(&node_id) = self.send_node_map.get(&(instrument_id, send.bus_id)) {
                            msgs.push(build_n_set_message(node_id, "level", value));
                        }
                    }
                }
            }
            ParameterTarget::VstParam(param_index) => {
                // /u_cmd doesn't use /n_set — fall back to direct send via apply_automation
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(source_node) = nodes.source {
                        if let Some(backend) = self.backend.as_ref() {
                            let _ = backend.send_unit_cmd(
                                source_node,
                                super::VST_UGEN_INDEX,
                                "/set",
                                vec![RawArg::Int(*param_index as i32), RawArg::Float(value)],
                            );
                        }
                    }
                }
            }
            ParameterTarget::EqBandFreq(band) => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(eq_node) = nodes.eq {
                        let param_name = format!("b{}_freq", band);
                        msgs.push(build_n_set_message(eq_node, &param_name, value));
                    }
                }
            }
            ParameterTarget::EqBandGain(band) => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(eq_node) = nodes.eq {
                        let param_name = format!("b{}_gain", band);
                        msgs.push(build_n_set_message(eq_node, &param_name, value));
                    }
                }
            }
            ParameterTarget::EqBandQ(band) => {
                if let Some(nodes) = self.node_map.get(&instrument_id) {
                    if let Some(eq_node) = nodes.eq {
                        let param_name = format!("b{}_q", band);
                        let sc_value = 1.0 / value;
                        msgs.push(build_n_set_message(eq_node, &param_name, sc_value));
                    }
                }
            }
            // Track groove settings: state-only, no OSC messages needed
            ParameterTarget::Swing => {
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    inst.groove.swing_amount = Some(value);
                }
            }
            ParameterTarget::HumanizeVelocity => {
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    inst.groove.humanize_velocity = Some(value);
                }
            }
            ParameterTarget::HumanizeTiming => {
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    inst.groove.humanize_timing = Some(value);
                }
            }
            ParameterTarget::TimingOffset => {
                if let Some(inst) = state.instrument_mut(instrument_id) {
                    inst.groove.timing_offset_ms = value;
                }
            }
            ParameterTarget::TimeSignature => {
                let target = AutomationTarget::track_time_signature(instrument_id);
                if let Some(discrete) = target.normalized_to_discrete(value) {
                    if let imbolc_types::DiscreteValue::TimeSignature(num, denom) = discrete {
                        if let Some(inst) = state.instrument_mut(instrument_id) {
                            inst.groove.time_signature = Some((num, denom));
                        }
                    }
                }
            }
            // Voice synthesis params - state-only for now
            ParameterTarget::Pitch
            | ParameterTarget::PulseWidth
            | ParameterTarget::Detune
            | ParameterTarget::FmIndex
            | ParameterTarget::WavetablePosition
            | ParameterTarget::FormantFreq
            | ParameterTarget::SyncRatio
            | ParameterTarget::Pressure
            | ParameterTarget::Embouchure
            | ParameterTarget::GrainSize
            | ParameterTarget::GrainDensity
            | ParameterTarget::FbFeedback
            | ParameterTarget::RingModDepth
            | ParameterTarget::ChaosParam
            | ParameterTarget::AdditiveRolloff
            | ParameterTarget::MembraneTension
            | ParameterTarget::StretchRatio
            | ParameterTarget::PitchShift
            | ParameterTarget::DelayTime
            | ParameterTarget::DelayFeedback
            | ParameterTarget::ReverbMix
            | ParameterTarget::GateRate => {
                // State-only updates for now
            }
        }
    }

    /// Send a batch of automation messages as a single timestamped bundle.
    pub fn send_automation_bundle(&self, messages: Vec<BackendMessage>, offset_secs: f64) -> Result<(), String> {
        self.queue_timed_bundle(messages, offset_secs)
    }
}
