//! # imbolc-types
//!
//! Shared type definitions for the Imbolc DAW ecosystem.
//! This crate contains data structures used across imbolc, imbolc-core, and imbolc-net.
//!
//! ## Status
//!
//! Types are being migrated from imbolc-core incrementally.
//! See plans/imbolc-net.md for the full extraction plan.

mod param;
pub mod state;
pub mod action;
mod audio;
pub mod dispatch;

pub use audio::{AudioFeedback, ExportKind, ServerStatus};
pub use param::{Param, ParamValue, adjust_freq_semitone, adjust_musical_step, is_freq_param};
pub use action::*;
pub use dispatch::Dispatcher;

// Re-export all state types at crate root for convenience
pub use state::*;

/// Unique identifier for an instrument.
pub type InstrumentId = u32;

/// Newtype for bus identifiers. Bus IDs are always >= 1 (allocated by MixerState).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
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
pub type EffectId = u32;

/// Unique identifier for a custom SynthDef.
pub type CustomSynthDefId = u32;

/// Unique identifier for a VST plugin.
pub type VstPluginId = u32;
