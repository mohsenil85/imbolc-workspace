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

/// Unique identifier for an effect slot within an instrument.
pub type EffectId = u32;

/// Unique identifier for a custom SynthDef.
pub type CustomSynthDefId = u32;

/// Unique identifier for a VST plugin.
pub type VstPluginId = u32;

/// Unique identifier for a sample buffer.
pub type BufferId = i32;
