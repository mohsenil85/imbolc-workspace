//! # imbolc-types
//!
//! Shared type definitions for the Imbolc DAW ecosystem.
//! This crate contains data structures used across imbolc, imbolc-core, and imbolc-net.
//!
//! ## Status
//!
//! Types are being migrated from imbolc-core incrementally.
//! See plans/imbolc-net.md for the full extraction plan.

pub mod action;
mod audio;
mod param;
pub mod reduce;
pub mod state;
pub mod tuning;

pub use action::*;
pub use audio::{AudioFeedback, ExportKind, ServerStatus};
pub use param::{adjust_freq_semitone, adjust_musical_step, is_freq_param, Param, ParamValue};

// Re-export all state types at crate root for convenience
pub use state::*;

/// Unique identifier for an instrument.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct InstrumentId(u32);

impl InstrumentId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
    pub fn get(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for InstrumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Newtype for bus identifiers. Bus IDs are always >= 1 (allocated by MixerState).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct BusId(u8);

impl BusId {
    /// Create a BusId. Panics if id == 0.
    pub fn new(id: u8) -> Self {
        assert!(id > 0, "BusId cannot be zero");
        Self(id)
    }

    /// Extract the raw u8 value.
    pub fn get(self) -> u8 {
        self.0
    }
}

impl std::fmt::Display for BusId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an effect slot within an instrument.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
pub struct EffectId(u32);

impl EffectId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
    pub fn get(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for EffectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a custom SynthDef.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
pub struct CustomSynthDefId(u32);

impl CustomSynthDefId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
    pub fn get(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for CustomSynthDefId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a VST plugin.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
pub struct VstPluginId(u32);

impl VstPluginId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
    pub fn get(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for VstPluginId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Index of a parameter within an effect's parameter list (0-based).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct ParamIndex(usize);

impl ParamIndex {
    pub fn new(idx: usize) -> Self {
        Self(idx)
    }
    pub fn get(self) -> usize {
        self.0
    }
}

impl std::fmt::Display for ParamIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
