pub use imbolc_types::{EffectId, EffectSlot, EffectType};

use crate::state::vst_plugin::VstPluginRegistry;

/// Extension trait for EffectType methods that require registry access
pub trait EffectTypeExt {
    /// Get display name with VST plugin registry lookup
    fn display_name(&self, vst_registry: &VstPluginRegistry) -> String;
}

impl EffectTypeExt for EffectType {
    fn display_name(&self, vst_registry: &VstPluginRegistry) -> String {
        match self {
            EffectType::Vst(id) => vst_registry
                .get(*id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "VST".to_string()),
            _ => self.name().to_string(),
        }
    }
}
