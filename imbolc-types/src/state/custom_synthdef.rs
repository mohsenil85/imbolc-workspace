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
    pub name: String,              // Display name (derived from synthdef name)
    pub synthdef_name: String,     // SuperCollider name (e.g., "my_bass")
    pub source_path: PathBuf,      // Original .scd file path
    pub params: Vec<ParamSpec>,    // Extracted parameters
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
            next_id: 0,
        }
    }

    pub fn add(&mut self, mut synthdef: CustomSynthDef) -> CustomSynthDefId {
        let id = self.next_id;
        self.next_id += 1;
        synthdef.id = id;
        self.synthdefs.push(synthdef);
        id
    }

    pub fn get(&self, id: CustomSynthDefId) -> Option<&CustomSynthDef> {
        self.synthdefs.iter().find(|s| s.id == id)
    }
}
