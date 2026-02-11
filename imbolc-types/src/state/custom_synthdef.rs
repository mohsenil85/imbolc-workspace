//! Custom SynthDef registry types.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::CustomSynthDefId;

/// Specification for a parameter extracted from .scd file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamSpec {
    pub name: String,
    pub default: f32,
    pub min: f32,
    pub max: f32,
}

/// A user-imported custom SynthDef
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSynthDef {
    pub id: CustomSynthDefId,
    pub name: String,           // Display name (derived from synthdef name)
    pub synthdef_name: String,  // SuperCollider name (e.g., "my_bass")
    pub source_path: PathBuf,   // Original .scd file path
    pub params: Vec<ParamSpec>, // Extracted parameters
}

/// Registry of all custom synthdefs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CustomSynthDefRegistry {
    pub synthdefs: Vec<CustomSynthDef>,
    pub next_id: CustomSynthDefId,
}

impl CustomSynthDefRegistry {
    pub fn new() -> Self {
        Self {
            synthdefs: Vec::new(),
            next_id: CustomSynthDefId::new(0),
        }
    }

    pub fn add(&mut self, mut synthdef: CustomSynthDef) -> CustomSynthDefId {
        let id = self.next_id;
        self.next_id = CustomSynthDefId::new(self.next_id.get() + 1);
        synthdef.id = id;
        self.synthdefs.push(synthdef);
        id
    }

    pub fn get(&self, id: CustomSynthDefId) -> Option<&CustomSynthDef> {
        self.synthdefs.iter().find(|s| s.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_synthdef(name: &str) -> CustomSynthDef {
        CustomSynthDef {
            id: CustomSynthDefId::new(0),
            name: name.to_string(),
            synthdef_name: format!("imbolc_{}", name),
            source_path: PathBuf::from("/tmp/test.scd"),
            params: vec![],
        }
    }

    #[test]
    fn registry_new_empty() {
        let reg = CustomSynthDefRegistry::new();
        assert!(reg.synthdefs.is_empty());
        assert_eq!(reg.next_id, CustomSynthDefId::new(0));
    }

    #[test]
    fn registry_add_assigns_id() {
        let mut reg = CustomSynthDefRegistry::new();
        let id0 = reg.add(make_synthdef("first"));
        let id1 = reg.add(make_synthdef("second"));
        assert_eq!(id0, CustomSynthDefId::new(0));
        assert_eq!(id1, CustomSynthDefId::new(1));
    }

    #[test]
    fn registry_get_finds_by_id() {
        let mut reg = CustomSynthDefRegistry::new();
        let id = reg.add(make_synthdef("test"));
        assert!(reg.get(id).is_some());
        assert_eq!(reg.get(id).unwrap().name, "test");
    }

    #[test]
    fn registry_get_missing_returns_none() {
        let reg = CustomSynthDefRegistry::new();
        assert!(reg.get(CustomSynthDefId::new(99)).is_none());
    }
}
