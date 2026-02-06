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
            next_id: 0,
        }
    }

    pub fn add(&mut self, mut plugin: VstPlugin) -> VstPluginId {
        let id = self.next_id;
        self.next_id += 1;
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
