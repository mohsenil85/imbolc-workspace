use super::backend::{BackendMessage, RawArg, build_n_set_message};
use super::AudioEngine;
use crate::state::{AutomationTarget, InstrumentState, SessionState};

impl AudioEngine {
    /// Apply an automation value to a target parameter
    /// This updates the appropriate synth node in real-time
    pub fn apply_automation(&self, target: &AutomationTarget, value: f32, state: &mut InstrumentState, session: &SessionState) -> Result<(), String> {
        if !self.is_running {
            return Ok(());
        }
        let backend = self.backend.as_ref().ok_or("Not connected")?;

        match target {
            AutomationTarget::InstrumentLevel(instrument_id) => {
                if let Some(nodes) = self.node_map.get(instrument_id) {
                    let effective_level = value * session.mixer.master_level;
                    backend.set_param(nodes.output, "level", effective_level)
                        .map_err(|e| e.to_string())?;
                }
            }
            AutomationTarget::InstrumentPan(instrument_id) => {
                if let Some(nodes) = self.node_map.get(instrument_id) {
                    backend.set_param(nodes.output, "pan", value)
                        .map_err(|e| e.to_string())?;
                }
            }
            AutomationTarget::FilterCutoff(instrument_id) => {
                if let Some(nodes) = self.node_map.get(instrument_id) {
                    if let Some(filter_node) = nodes.filter {
                        backend.set_param(filter_node, "cutoff", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            AutomationTarget::FilterResonance(instrument_id) => {
                if let Some(nodes) = self.node_map.get(instrument_id) {
                    if let Some(filter_node) = nodes.filter {
                        backend.set_param(filter_node, "resonance", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            AutomationTarget::EffectParam(instrument_id, effect_id, param_idx) => {
                if let Some(nodes) = self.node_map.get(instrument_id) {
                    if let Some(&effect_node) = nodes.effects.get(effect_id) {
                        if let Some(instrument) = state.instrument(*instrument_id) {
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
            AutomationTarget::SampleRate(instrument_id) => {
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == *instrument_id {
                        backend.set_param(voice.source_node, "rate", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            AutomationTarget::SampleAmp(instrument_id) => {
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == *instrument_id {
                        backend.set_param(voice.source_node, "amp", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            AutomationTarget::LfoRate(instrument_id) => {
                if let Some(nodes) = self.node_map.get(instrument_id) {
                    if let Some(lfo_node) = nodes.lfo {
                        backend.set_param(lfo_node, "rate", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            AutomationTarget::LfoDepth(instrument_id) => {
                if let Some(nodes) = self.node_map.get(instrument_id) {
                    if let Some(lfo_node) = nodes.lfo {
                        backend.set_param(lfo_node, "depth", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            AutomationTarget::EnvelopeAttack(instrument_id) => {
                if let Some(inst) = state.instrument_mut(*instrument_id) {
                    inst.amp_envelope.attack = value;
                }
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == *instrument_id {
                        backend.set_param(voice.source_node, "attack", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            AutomationTarget::EnvelopeDecay(instrument_id) => {
                if let Some(inst) = state.instrument_mut(*instrument_id) {
                    inst.amp_envelope.decay = value;
                }
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == *instrument_id {
                        backend.set_param(voice.source_node, "decay", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            AutomationTarget::EnvelopeSustain(instrument_id) => {
                if let Some(inst) = state.instrument_mut(*instrument_id) {
                    inst.amp_envelope.sustain = value;
                }
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == *instrument_id {
                        backend.set_param(voice.source_node, "sustain", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            AutomationTarget::EnvelopeRelease(instrument_id) => {
                if let Some(inst) = state.instrument_mut(*instrument_id) {
                    inst.amp_envelope.release = value;
                }
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == *instrument_id {
                        backend.set_param(voice.source_node, "release", value)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            AutomationTarget::SendLevel(instrument_id, send_idx) => {
                if let Some(inst) = state.instrument(*instrument_id) {
                    if let Some(send) = inst.sends.get(*send_idx) {
                        if let Some(&node_id) = self.send_node_map.get(&(*instrument_id, send.bus_id)) {
                            backend.set_param(node_id, "level", value)
                                .map_err(|e| e.to_string())?;
                        }
                    }
                }
            }
            AutomationTarget::BusLevel(bus_id) => {
                if let Some(&node_id) = self.bus_node_map.get(bus_id) {
                    backend.set_param(node_id, "level", value)
                        .map_err(|e| e.to_string())?;
                }
            }
            AutomationTarget::Bpm => {
                // Handled in playback.rs, not here
            }
            AutomationTarget::VstParam(instrument_id, param_index) => {
                if let Some(nodes) = self.node_map.get(instrument_id) {
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
            AutomationTarget::EqBandParam(instrument_id, band, param) => {
                if let Some(nodes) = self.node_map.get(instrument_id) {
                    if let Some(eq_node) = nodes.eq {
                        let param_name = match param {
                            0 => format!("b{}_freq", band),
                            1 => format!("b{}_gain", band),
                            _ => format!("b{}_q", band),
                        };
                        let sc_value = if *param == 2 { 1.0 / value } else { value };
                        backend.set_param(eq_node, &param_name, sc_value)
                            .map_err(|e| e.to_string())?;
                    }
                }
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
            AutomationTarget::InstrumentLevel(instrument_id) => {
                if let Some(nodes) = self.node_map.get(instrument_id) {
                    let effective_level = value * session.mixer.master_level;
                    msgs.push(build_n_set_message(nodes.output, "level", effective_level));
                }
            }
            AutomationTarget::InstrumentPan(instrument_id) => {
                if let Some(nodes) = self.node_map.get(instrument_id) {
                    msgs.push(build_n_set_message(nodes.output, "pan", value));
                }
            }
            AutomationTarget::FilterCutoff(instrument_id) => {
                if let Some(nodes) = self.node_map.get(instrument_id) {
                    if let Some(filter_node) = nodes.filter {
                        msgs.push(build_n_set_message(filter_node, "cutoff", value));
                    }
                }
            }
            AutomationTarget::FilterResonance(instrument_id) => {
                if let Some(nodes) = self.node_map.get(instrument_id) {
                    if let Some(filter_node) = nodes.filter {
                        msgs.push(build_n_set_message(filter_node, "resonance", value));
                    }
                }
            }
            AutomationTarget::EffectParam(instrument_id, effect_id, param_idx) => {
                if let Some(nodes) = self.node_map.get(instrument_id) {
                    if let Some(&effect_node) = nodes.effects.get(effect_id) {
                        if let Some(instrument) = state.instrument(*instrument_id) {
                            if let Some(effect) = instrument.effect_by_id(*effect_id) {
                                if let Some(param) = effect.params.get(*param_idx) {
                                    msgs.push(build_n_set_message(effect_node, &param.name, value));
                                }
                            }
                        }
                    }
                }
            }
            AutomationTarget::SampleRate(instrument_id) => {
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == *instrument_id {
                        msgs.push(build_n_set_message(voice.source_node, "rate", value));
                    }
                }
            }
            AutomationTarget::SampleAmp(instrument_id) => {
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == *instrument_id {
                        msgs.push(build_n_set_message(voice.source_node, "amp", value));
                    }
                }
            }
            AutomationTarget::LfoRate(instrument_id) => {
                if let Some(nodes) = self.node_map.get(instrument_id) {
                    if let Some(lfo_node) = nodes.lfo {
                        msgs.push(build_n_set_message(lfo_node, "rate", value));
                    }
                }
            }
            AutomationTarget::LfoDepth(instrument_id) => {
                if let Some(nodes) = self.node_map.get(instrument_id) {
                    if let Some(lfo_node) = nodes.lfo {
                        msgs.push(build_n_set_message(lfo_node, "depth", value));
                    }
                }
            }
            AutomationTarget::EnvelopeAttack(instrument_id) => {
                if let Some(inst) = state.instrument_mut(*instrument_id) {
                    inst.amp_envelope.attack = value;
                }
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == *instrument_id {
                        msgs.push(build_n_set_message(voice.source_node, "attack", value));
                    }
                }
            }
            AutomationTarget::EnvelopeDecay(instrument_id) => {
                if let Some(inst) = state.instrument_mut(*instrument_id) {
                    inst.amp_envelope.decay = value;
                }
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == *instrument_id {
                        msgs.push(build_n_set_message(voice.source_node, "decay", value));
                    }
                }
            }
            AutomationTarget::EnvelopeSustain(instrument_id) => {
                if let Some(inst) = state.instrument_mut(*instrument_id) {
                    inst.amp_envelope.sustain = value;
                }
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == *instrument_id {
                        msgs.push(build_n_set_message(voice.source_node, "sustain", value));
                    }
                }
            }
            AutomationTarget::EnvelopeRelease(instrument_id) => {
                if let Some(inst) = state.instrument_mut(*instrument_id) {
                    inst.amp_envelope.release = value;
                }
                for voice in self.voice_allocator.chains() {
                    if voice.instrument_id == *instrument_id {
                        msgs.push(build_n_set_message(voice.source_node, "release", value));
                    }
                }
            }
            AutomationTarget::SendLevel(instrument_id, send_idx) => {
                if let Some(inst) = state.instrument(*instrument_id) {
                    if let Some(send) = inst.sends.get(*send_idx) {
                        if let Some(&node_id) = self.send_node_map.get(&(*instrument_id, send.bus_id)) {
                            msgs.push(build_n_set_message(node_id, "level", value));
                        }
                    }
                }
            }
            AutomationTarget::BusLevel(bus_id) => {
                if let Some(&node_id) = self.bus_node_map.get(bus_id) {
                    msgs.push(build_n_set_message(node_id, "level", value));
                }
            }
            AutomationTarget::Bpm => {
                // Handled in playback.rs, not here
            }
            AutomationTarget::VstParam(instrument_id, param_index) => {
                // /u_cmd doesn't use /n_set — fall back to direct send via apply_automation
                if let Some(nodes) = self.node_map.get(instrument_id) {
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
            AutomationTarget::EqBandParam(instrument_id, band, param) => {
                if let Some(nodes) = self.node_map.get(instrument_id) {
                    if let Some(eq_node) = nodes.eq {
                        let param_name = match param {
                            0 => format!("b{}_freq", band),
                            1 => format!("b{}_gain", band),
                            _ => format!("b{}_q", band),
                        };
                        let sc_value = if *param == 2 { 1.0 / value } else { value };
                        msgs.push(build_n_set_message(eq_node, &param_name, sc_value));
                    }
                }
            }
        }

        msgs
    }

    /// Send a batch of automation messages as a single timestamped bundle.
    pub fn send_automation_bundle(&self, messages: Vec<BackendMessage>, offset_secs: f64) -> Result<(), String> {
        if messages.is_empty() {
            return Ok(());
        }
        let backend = self.backend.as_ref().ok_or("Not connected")?;
        backend.send_bundle(messages, offset_secs).map_err(|e| e.to_string())
    }
}
