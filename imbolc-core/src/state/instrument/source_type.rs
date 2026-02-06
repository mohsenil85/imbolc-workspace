pub use imbolc_types::SourceType;

use crate::state::custom_synthdef::CustomSynthDefRegistry;
use crate::state::param::{Param, ParamValue};
use crate::state::vst_plugin::VstPluginRegistry;

/// Extension trait for SourceType methods that require registry access
pub trait SourceTypeExt {
    /// Get display name, with custom synthdef name lookup
    fn display_name(&self, registry: &CustomSynthDefRegistry) -> String;
    /// Get display name with VST plugin registry lookup
    fn display_name_vst(&self, custom_registry: &CustomSynthDefRegistry, vst_registry: &VstPluginRegistry) -> String;
    /// Get short name with custom synthdef lookup
    fn short_name_with_registry(&self, registry: &CustomSynthDefRegistry) -> String;
    /// Get short name with VST plugin registry lookup
    fn short_name_vst(&self, custom_registry: &CustomSynthDefRegistry, vst_registry: &VstPluginRegistry) -> String;
    /// Get the SuperCollider synthdef name with custom synthdef lookup
    fn synth_def_name_with_registry(&self, registry: &CustomSynthDefRegistry) -> String;
    /// Get default params with custom synthdef lookup
    fn default_params_with_registry(&self, registry: &CustomSynthDefRegistry) -> Vec<Param>;
    /// All source types including custom ones from registry
    fn all_with_custom(registry: &CustomSynthDefRegistry) -> Vec<SourceType>;
}

impl SourceTypeExt for SourceType {
    fn display_name(&self, registry: &CustomSynthDefRegistry) -> String {
        match self {
            SourceType::Custom(id) => registry
                .get(*id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "Custom".to_string()),
            _ => self.name().to_string(),
        }
    }

    fn display_name_vst(&self, custom_registry: &CustomSynthDefRegistry, vst_registry: &VstPluginRegistry) -> String {
        match self {
            SourceType::Vst(id) => vst_registry
                .get(*id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "VST".to_string()),
            _ => self.display_name(custom_registry),
        }
    }

    fn short_name_with_registry(&self, registry: &CustomSynthDefRegistry) -> String {
        match self {
            SourceType::Custom(id) => registry
                .get(*id)
                .map(|s| s.synthdef_name.clone())
                .unwrap_or_else(|| "custom".to_string()),
            _ => self.short_name().to_string(),
        }
    }

    fn short_name_vst(&self, custom_registry: &CustomSynthDefRegistry, vst_registry: &VstPluginRegistry) -> String {
        match self {
            SourceType::Vst(id) => vst_registry
                .get(*id)
                .map(|p| p.name.to_lowercase())
                .unwrap_or_else(|| "vst".to_string()),
            _ => self.short_name_with_registry(custom_registry),
        }
    }

    fn synth_def_name_with_registry(&self, registry: &CustomSynthDefRegistry) -> String {
        match self {
            SourceType::Custom(id) => registry
                .get(*id)
                .map(|s| s.synthdef_name.clone())
                .unwrap_or_else(|| "imbolc_saw".to_string()),
            SourceType::Vst(_) => "imbolc_vst_instrument".to_string(),
            _ => self.synth_def_name().to_string(),
        }
    }

    fn default_params_with_registry(&self, registry: &CustomSynthDefRegistry) -> Vec<Param> {
        match self {
            SourceType::Custom(id) => registry
                .get(*id)
                .map(|s| {
                    s.params
                        .iter()
                        .map(|p| Param {
                            name: p.name.clone(),
                            value: ParamValue::Float(p.default),
                            min: p.min,
                            max: p.max,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            _ => self.default_params(),
        }
    }

    fn all_with_custom(registry: &CustomSynthDefRegistry) -> Vec<SourceType> {
        let mut types = SourceType::all();
        for synthdef in &registry.synthdefs {
            types.push(SourceType::Custom(synthdef.id));
        }
        types
    }
}
