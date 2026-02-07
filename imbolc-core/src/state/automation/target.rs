// Re-export AutomationTarget from imbolc-types
pub use imbolc_types::AutomationTarget;

// Keep the context-aware methods that need Instrument and VstPluginRegistry
use crate::state::instrument::{Instrument, SourceType};
use crate::state::vst_plugin::VstPluginRegistry;
use imbolc_types::ParameterTarget;

/// Extension trait for context-aware AutomationTarget methods that depend on complex types.
pub trait AutomationTargetExt {
    /// Get context-aware automation targets for an instrument.
    /// Includes the static 10 targets plus context-dependent ones based on
    /// the instrument's effects, source type, VST plugins, and EQ.
    fn targets_for_instrument_context(inst: &Instrument, vst_registry: &VstPluginRegistry) -> Vec<AutomationTarget>;

    /// Get a context-aware display name using instrument data for richer labels.
    /// Falls back to `name()` for targets that don't benefit from context.
    fn name_with_context(&self, inst: Option<&Instrument>, vst_registry: &VstPluginRegistry) -> String;
}

impl AutomationTargetExt for AutomationTarget {
    fn targets_for_instrument_context(inst: &Instrument, vst_registry: &VstPluginRegistry) -> Vec<AutomationTarget> {
        let id = inst.id;
        let mut targets = AutomationTarget::targets_for_instrument(id);

        // EffectParam: one target per param for each non-VST effect
        for effect in &inst.effects {
            if effect.effect_type.is_vst() {
                continue;
            }
            for (param_idx, _param) in effect.params.iter().enumerate() {
                targets.push(AutomationTarget::effect_param(id, effect.id, param_idx));
            }
            // Effect bypass
            targets.push(AutomationTarget::effect_bypass(id, effect.id));
        }

        // SampleRate + SampleAmp: only for sample-based sources
        if matches!(inst.source, SourceType::PitchedSampler | SourceType::Kit) {
            targets.push(AutomationTarget::sample_rate(id));
            targets.push(AutomationTarget::sample_amp(id));
        }

        // VstParam: only for VST source instruments
        if let SourceType::Vst(vst_id) = inst.source {
            if let Some(plugin) = vst_registry.get(vst_id) {
                for param in &plugin.params {
                    targets.push(AutomationTarget::vst_param(id, param.index));
                }
            }
        }

        // EqBandParam: only when EQ is enabled (12 bands x 3 params = 36 targets)
        if inst.eq.is_some() {
            for band in 0..12 {
                targets.push(AutomationTarget::eq_band_freq(id, band));
                targets.push(AutomationTarget::eq_band_gain(id, band));
                targets.push(AutomationTarget::eq_band_q(id, band));
            }
        }

        targets
    }

    fn name_with_context(&self, inst: Option<&Instrument>, vst_registry: &VstPluginRegistry) -> String {
        match self.parameter_target() {
            Some(ParameterTarget::EffectParam(effect_id, param_idx)) => {
                if let Some(inst) = inst {
                    if let Some(effect) = inst.effect_by_id(*effect_id) {
                        let effect_name = effect.effect_type.name();
                        if let Some(param) = effect.params.get(*param_idx) {
                            return format!("{} > {}", effect_name, param.name);
                        }
                    }
                }
                self.name()
            }
            Some(ParameterTarget::VstParam(param_index)) => {
                if let Some(inst) = inst {
                    if let SourceType::Vst(vst_id) = inst.source {
                        if let Some(plugin) = vst_registry.get(vst_id) {
                            if let Some(param) = plugin.params.iter().find(|p| p.index == *param_index) {
                                return format!("VST: {}", param.name);
                            }
                        }
                    }
                }
                self.name()
            }
            _ => self.name(),
        }
    }
}
