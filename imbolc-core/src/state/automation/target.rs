// Re-export AutomationTarget from imbolc-types
pub use imbolc_types::AutomationTarget;

// Keep the context-aware methods that need Instrument and VstPluginRegistry
use crate::state::instrument::{Instrument, SourceType};
use crate::state::vst_plugin::VstPluginRegistry;

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
                targets.push(AutomationTarget::EffectParam(id, effect.id, param_idx));
            }
        }

        // SampleRate + SampleAmp: only for sample-based sources
        if matches!(inst.source, SourceType::PitchedSampler | SourceType::Kit) {
            targets.push(AutomationTarget::SampleRate(id));
            targets.push(AutomationTarget::SampleAmp(id));
        }

        // VstParam: only for VST source instruments
        if let SourceType::Vst(vst_id) = inst.source {
            if let Some(plugin) = vst_registry.get(vst_id) {
                for param in &plugin.params {
                    targets.push(AutomationTarget::VstParam(id, param.index));
                }
            }
        }

        // EqBandParam: only when EQ is enabled (12 bands x 3 params = 36 targets)
        if inst.eq.is_some() {
            for band in 0..12 {
                for param in 0..3 {
                    targets.push(AutomationTarget::EqBandParam(id, band, param));
                }
            }
        }

        targets
    }

    fn name_with_context(&self, inst: Option<&Instrument>, vst_registry: &VstPluginRegistry) -> String {
        match self {
            AutomationTarget::EffectParam(_, effect_id, param_idx) => {
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
            AutomationTarget::VstParam(_, param_index) => {
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
