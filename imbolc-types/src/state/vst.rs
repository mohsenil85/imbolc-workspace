//! VST plugin types.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::VstPluginId;

/// Whether a VST plugin is an instrument or effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VstPluginKind {
    Instrument,
    Effect,
}

/// Specification for a VST parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VstParamSpec {
    pub index: u32,      // VST param index (0-based)
    pub name: String,
    pub default: f32,    // 0.0-1.0 normalized
    pub label: Option<String>, // unit string from plugin, e.g. "Hz", "dB"
}

/// A registered VST plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VstPlugin {
    pub id: VstPluginId,
    pub name: String,           // display name (from filename)
    pub plugin_path: PathBuf,   // path to .vst3/.vst bundle
    pub kind: VstPluginKind,
    pub params: Vec<VstParamSpec>,
}

/// Registry of all VST plugins
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VstPluginRegistry {
    pub plugins: Vec<VstPlugin>,
    pub next_id: VstPluginId,
}

impl VstPluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            next_id: VstPluginId::new(0),
        }
    }

    pub fn add(&mut self, mut plugin: VstPlugin) -> VstPluginId {
        let id = self.next_id;
        self.next_id = VstPluginId::new(self.next_id.get() + 1);
        plugin.id = id;
        self.plugins.push(plugin);
        id
    }

    pub fn get(&self, id: VstPluginId) -> Option<&VstPlugin> {
        self.plugins.iter().find(|p| p.id == id)
    }

    pub fn remove(&mut self, id: VstPluginId) {
        self.plugins.retain(|p| p.id != id);
    }

    pub fn by_name(&self, name: &str) -> Option<&VstPlugin> {
        self.plugins.iter().find(|p| p.name == name)
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn instruments(&self) -> impl Iterator<Item = &VstPlugin> {
        self.plugins.iter().filter(|p| p.kind == VstPluginKind::Instrument)
    }

    pub fn effects(&self) -> impl Iterator<Item = &VstPlugin> {
        self.plugins.iter().filter(|p| p.kind == VstPluginKind::Effect)
    }

    pub fn get_mut(&mut self, id: VstPluginId) -> Option<&mut VstPlugin> {
        self.plugins.iter_mut().find(|p| p.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_plugin(name: &str, kind: VstPluginKind) -> VstPlugin {
        VstPlugin {
            id: VstPluginId::new(0),
            name: name.to_string(),
            plugin_path: PathBuf::from("/tmp/test.vst3"),
            kind,
            params: vec![],
        }
    }

    #[test]
    fn registry_new_empty() {
        let reg = VstPluginRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.next_id, VstPluginId::new(0));
    }

    #[test]
    fn registry_add_assigns_id() {
        let mut reg = VstPluginRegistry::new();
        let id0 = reg.add(make_plugin("A", VstPluginKind::Instrument));
        let id1 = reg.add(make_plugin("B", VstPluginKind::Effect));
        assert_eq!(id0, VstPluginId::new(0));
        assert_eq!(id1, VstPluginId::new(1));
    }

    #[test]
    fn registry_get_by_id() {
        let mut reg = VstPluginRegistry::new();
        let id = reg.add(make_plugin("Test", VstPluginKind::Instrument));
        assert!(reg.get(id).is_some());
        assert_eq!(reg.get(id).unwrap().name, "Test");
    }

    #[test]
    fn registry_remove() {
        let mut reg = VstPluginRegistry::new();
        let id = reg.add(make_plugin("X", VstPluginKind::Effect));
        reg.remove(id);
        assert!(reg.get(id).is_none());
    }

    #[test]
    fn registry_by_name() {
        let mut reg = VstPluginRegistry::new();
        reg.add(make_plugin("Synth1", VstPluginKind::Instrument));
        assert!(reg.by_name("Synth1").is_some());
    }

    #[test]
    fn registry_by_name_missing() {
        let reg = VstPluginRegistry::new();
        assert!(reg.by_name("Nonexistent").is_none());
    }

    #[test]
    fn registry_is_empty() {
        let mut reg = VstPluginRegistry::new();
        assert!(reg.is_empty());
        reg.add(make_plugin("A", VstPluginKind::Instrument));
        assert!(!reg.is_empty());
    }

    #[test]
    fn registry_len() {
        let mut reg = VstPluginRegistry::new();
        assert_eq!(reg.len(), 0);
        reg.add(make_plugin("A", VstPluginKind::Instrument));
        reg.add(make_plugin("B", VstPluginKind::Effect));
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn registry_instruments_filter() {
        let mut reg = VstPluginRegistry::new();
        reg.add(make_plugin("Inst", VstPluginKind::Instrument));
        reg.add(make_plugin("FX", VstPluginKind::Effect));
        let insts: Vec<_> = reg.instruments().collect();
        assert_eq!(insts.len(), 1);
        assert_eq!(insts[0].name, "Inst");
    }

    #[test]
    fn registry_effects_filter() {
        let mut reg = VstPluginRegistry::new();
        reg.add(make_plugin("Inst", VstPluginKind::Instrument));
        reg.add(make_plugin("FX", VstPluginKind::Effect));
        let effs: Vec<_> = reg.effects().collect();
        assert_eq!(effs.len(), 1);
        assert_eq!(effs[0].name, "FX");
    }
}
