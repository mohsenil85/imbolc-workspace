use super::{InstrumentEditPane, Section};
use crate::state::param::{adjust_freq_semitone, adjust_musical_step};
use crate::ui::{Action, InstrumentAction, InstrumentUpdate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AdjustMode {
    Tiny,
    Normal,
    Big,
    Musical,
}

impl InstrumentEditPane {
    pub(super) fn adjust_value(&mut self, increase: bool, big: bool) {
        let mode = if big { AdjustMode::Big } else { AdjustMode::Normal };
        self.adjust_value_with_mode(increase, mode, 440.0);
    }

    pub(super) fn adjust_value_with_mode(&mut self, increase: bool, mode: AdjustMode, tuning_a4: f32) {
        let (section, local_idx) = self.row_info(self.selected_row);
        let fraction = match mode {
            AdjustMode::Tiny => 0.01,
            AdjustMode::Normal => 0.05,
            AdjustMode::Big => 0.10,
            AdjustMode::Musical => 0.05, // fallback, overridden per-section
        };

        match section {
            Section::Source => {
                let param_idx = if self.source.is_sample() {
                    if local_idx == 0 { return; } // sample name row — not adjustable
                    local_idx - 1
                } else {
                    local_idx
                };
                if let Some(param) = self.source_params.get_mut(param_idx) {
                    if mode == AdjustMode::Musical {
                        param.adjust_musical(increase, tuning_a4);
                    } else {
                        param.adjust(increase, fraction);
                    }
                }
            }
            Section::Filter => {
                if let Some(ref mut f) = self.filter {
                    match local_idx {
                        0 => {} // type - use 't' to cycle
                        1 => {
                            if mode == AdjustMode::Musical {
                                f.cutoff.value = adjust_freq_semitone(f.cutoff.value, increase, tuning_a4, f.cutoff.min, f.cutoff.max);
                            } else {
                                let range = f.cutoff.max - f.cutoff.min;
                                let delta = range * fraction;
                                if increase { f.cutoff.value = (f.cutoff.value + delta).min(f.cutoff.max); }
                                else { f.cutoff.value = (f.cutoff.value - delta).max(f.cutoff.min); }
                            }
                        }
                        2 => {
                            if mode == AdjustMode::Musical {
                                f.resonance.value = adjust_musical_step(f.resonance.value, increase, f.resonance.min, f.resonance.max);
                            } else {
                                let range = f.resonance.max - f.resonance.min;
                                let delta = range * fraction;
                                if increase { f.resonance.value = (f.resonance.value + delta).min(f.resonance.max); }
                                else { f.resonance.value = (f.resonance.value - delta).max(f.resonance.min); }
                            }
                        }
                        idx => {
                            // Extra filter params (local_idx >= 3)
                            let extra_idx = idx - 3;
                            if let Some(param) = f.extra_params.get_mut(extra_idx) {
                                if mode == AdjustMode::Musical {
                                    param.adjust_musical(increase, tuning_a4);
                                } else {
                                    param.adjust(increase, fraction);
                                }
                            }
                        }
                    }
                }
            }
            Section::Effects => {
                if let Some((effect_idx, param_offset)) = self.effect_row_info(local_idx) {
                    if param_offset == 0 { return; } // header row — not adjustable
                    let param_idx = param_offset - 1;
                    if let Some(effect) = self.effects.get_mut(effect_idx) {
                        if let Some(param) = effect.params.get_mut(param_idx) {
                            if mode == AdjustMode::Musical {
                                param.adjust_musical(increase, tuning_a4);
                            } else {
                                param.adjust(increase, fraction);
                            }
                        }
                    }
                }
            }
            Section::Lfo => {
                match local_idx {
                    0 => {} // enabled - use 'l' to toggle
                    1 => {
                        // rate: 0.1 to 32 Hz
                        let delta = match mode {
                            AdjustMode::Tiny => 0.1,
                            AdjustMode::Musical => 1.0,
                            AdjustMode::Big => 2.0,
                            AdjustMode::Normal => 0.5,
                        };
                        if increase { self.lfo.rate = (self.lfo.rate + delta).min(32.0); }
                        else { self.lfo.rate = (self.lfo.rate - delta).max(0.1); }
                    }
                    2 => {
                        // depth: 0 to 1
                        let delta = match mode {
                            AdjustMode::Tiny => 0.01,
                            AdjustMode::Musical => 0.1,
                            _ => fraction,
                        };
                        if increase { self.lfo.depth = (self.lfo.depth + delta).min(1.0); }
                        else { self.lfo.depth = (self.lfo.depth - delta).max(0.0); }
                    }
                    3 => {} // shape/target - use 's'/'m' to cycle
                    _ => {}
                }
            }
            Section::Envelope => {
                let delta = match mode {
                    AdjustMode::Tiny => 0.01,
                    AdjustMode::Musical => 0.1,
                    AdjustMode::Normal => 0.05,
                    AdjustMode::Big => 0.1,
                };
                let val = match local_idx {
                    0 => &mut self.amp_envelope.attack,
                    1 => &mut self.amp_envelope.decay,
                    2 => &mut self.amp_envelope.sustain,
                    3 => &mut self.amp_envelope.release,
                    _ => return,
                };
                if increase { *val = (*val + delta).min(if local_idx == 2 { 1.0 } else { 5.0 }); }
                else { *val = (*val - delta).max(0.0); }
            }
        }
    }

    pub(super) fn emit_update(&self) -> Action {
        if let Some(id) = self.instrument_id {
            Action::Instrument(InstrumentAction::Update(Box::new(InstrumentUpdate {
                id,
                source: self.source,
                source_params: self.source_params.clone(),
                processing_chain: self.build_processing_chain(),
                lfo: self.lfo.clone(),
                amp_envelope: self.amp_envelope.clone(),
                polyphonic: self.polyphonic,
                active: self.active,
            })))
        } else {
            Action::None
        }
    }

    /// Set current parameter to its minimum (zero) value
    pub(super) fn zero_current_param(&mut self) {
        let (section, local_idx) = self.row_info(self.selected_row);

        match section {
            Section::Source => {
                let param_idx = if self.source.is_sample() {
                    if local_idx == 0 { return; }
                    local_idx - 1
                } else {
                    local_idx
                };
                if let Some(param) = self.source_params.get_mut(param_idx) {
                    param.zero();
                }
            }
            Section::Filter => {
                if let Some(ref mut f) = self.filter {
                    match local_idx {
                        0 => {} // type - can't zero
                        1 => f.cutoff.value = f.cutoff.min,
                        2 => f.resonance.value = f.resonance.min,
                        idx => {
                            let extra_idx = idx - 3;
                            if let Some(param) = f.extra_params.get_mut(extra_idx) {
                                param.zero();
                            }
                        }
                    }
                }
            }
            Section::Effects => {
                if let Some((effect_idx, param_offset)) = self.effect_row_info(local_idx) {
                    if param_offset == 0 { return; } // header row
                    let param_idx = param_offset - 1;
                    if let Some(effect) = self.effects.get_mut(effect_idx) {
                        if let Some(param) = effect.params.get_mut(param_idx) {
                            param.zero();
                        }
                    }
                }
            }
            Section::Lfo => {
                match local_idx {
                    0 => self.lfo.enabled = false,
                    1 => self.lfo.rate = 0.1,
                    2 => self.lfo.depth = 0.0,
                    3 => {} // shape/target - can't zero
                    _ => {}
                }
            }
            Section::Envelope => {
                match local_idx {
                    0 => self.amp_envelope.attack = 0.0,
                    1 => self.amp_envelope.decay = 0.0,
                    2 => self.amp_envelope.sustain = 0.0,
                    3 => self.amp_envelope.release = 0.0,
                    _ => {}
                }
            }
        }
    }

    /// Reset current parameter to its default value
    pub(super) fn reset_current_param(&mut self) {
        let (section, local_idx) = self.row_info(self.selected_row);

        match section {
            Section::Source => {
                let param_idx = if self.source.is_sample() {
                    if local_idx == 0 { return; }
                    local_idx - 1
                } else {
                    local_idx
                };
                let defaults = self.source.default_params();
                if let Some(param) = self.source_params.get_mut(param_idx) {
                    if let Some(default) = defaults.iter().find(|p| p.name == param.name) {
                        param.value = default.value.clone();
                    }
                }
            }
            Section::Filter => {
                if let Some(ref mut f) = self.filter {
                    match local_idx {
                        0 => {} // type - can't reset
                        1 => f.cutoff.value = 1000.0, // FilterConfig::new default
                        2 => f.resonance.value = 0.5,
                        idx => {
                            let extra_idx = idx - 3;
                            let defaults = f.filter_type.default_extra_params();
                            if let Some(param) = f.extra_params.get_mut(extra_idx) {
                                if let Some(default) = defaults.get(extra_idx) {
                                    param.value = default.value.clone();
                                }
                            }
                        }
                    }
                }
            }
            Section::Effects => {
                if let Some((effect_idx, param_offset)) = self.effect_row_info(local_idx) {
                    if param_offset == 0 { return; } // header row
                    let param_idx = param_offset - 1;
                    if let Some(effect) = self.effects.get_mut(effect_idx) {
                        let defaults = effect.effect_type.default_params();
                        if let Some(param) = effect.params.get_mut(param_idx) {
                            if let Some(default) = defaults.get(param_idx) {
                                param.value = default.value.clone();
                            }
                        }
                    }
                }
            }
            Section::Lfo => {
                use crate::state::LfoConfig;
                let defaults = LfoConfig::default();
                match local_idx {
                    0 => self.lfo.enabled = defaults.enabled,
                    1 => self.lfo.rate = defaults.rate,
                    2 => self.lfo.depth = defaults.depth,
                    3 => {} // shape/target - cycle, not value reset
                    _ => {}
                }
            }
            Section::Envelope => {
                let defaults = self.source.default_envelope();
                match local_idx {
                    0 => self.amp_envelope.attack = defaults.attack,
                    1 => self.amp_envelope.decay = defaults.decay,
                    2 => self.amp_envelope.sustain = defaults.sustain,
                    3 => self.amp_envelope.release = defaults.release,
                    _ => {}
                }
            }
        }
    }

    /// Set all parameters in the current section to their minimum values
    pub(super) fn zero_current_section(&mut self) {
        let section = self.current_section();

        match section {
            Section::Source => {
                for param in &mut self.source_params {
                    param.zero();
                }
            }
            Section::Filter => {
                if let Some(ref mut f) = self.filter {
                    f.cutoff.value = f.cutoff.min;
                    f.resonance.value = f.resonance.min;
                    for param in &mut f.extra_params {
                        param.zero();
                    }
                }
            }
            Section::Effects => {
                for effect in &mut self.effects {
                    for param in &mut effect.params {
                        param.zero();
                    }
                }
            }
            Section::Lfo => {
                self.lfo.enabled = false;
                self.lfo.rate = 0.1;
                self.lfo.depth = 0.0;
            }
            Section::Envelope => {
                self.amp_envelope.attack = 0.0;
                self.amp_envelope.decay = 0.0;
                self.amp_envelope.sustain = 0.0;
                self.amp_envelope.release = 0.0;
            }
        }
    }

    /// Get current parameter value as a string for pre-filling text edit
    pub(super) fn current_value_string(&self) -> String {
        let (section, local_idx) = self.row_info(self.selected_row);
        match section {
            Section::Source => {
                let param_idx = if self.source.is_sample() {
                    if local_idx == 0 { return String::new(); }
                    local_idx - 1
                } else {
                    local_idx
                };
                self.source_params.get(param_idx)
                    .map(|p| p.value_string())
                    .unwrap_or_default()
            }
            Section::Filter => {
                if let Some(ref f) = self.filter {
                    match local_idx {
                        1 => format!("{:.2}", f.cutoff.value),
                        2 => format!("{:.2}", f.resonance.value),
                        idx => {
                            let extra_idx = idx - 3;
                            f.extra_params.get(extra_idx)
                                .map(|p| p.value_string())
                                .unwrap_or_default()
                        }
                    }
                } else {
                    String::new()
                }
            }
            Section::Effects => {
                if let Some((effect_idx, param_offset)) = self.effect_row_info(local_idx) {
                    if param_offset == 0 { return String::new(); }
                    let param_idx = param_offset - 1;
                    self.effects.get(effect_idx)
                        .and_then(|e| e.params.get(param_idx))
                        .map(|p| p.value_string())
                        .unwrap_or_default()
                } else { String::new() }
            }
            Section::Envelope => {
                match local_idx {
                    0 => format!("{:.2}", self.amp_envelope.attack),
                    1 => format!("{:.2}", self.amp_envelope.decay),
                    2 => format!("{:.2}", self.amp_envelope.sustain),
                    3 => format!("{:.2}", self.amp_envelope.release),
                    _ => String::new(),
                }
            }
            _ => String::new(),
        }
    }
}

