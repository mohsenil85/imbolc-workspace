//! Per-track groove settings: swing, humanization, and timing offset.

use serde::{Deserialize, Serialize};

/// Swing grid subdivision - which notes are affected by swing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SwingGrid {
    /// Affects odd 8th notes (beats 2, 4, 6, 8...)
    #[default]
    Eighths,
    /// Affects odd 16th notes
    Sixteenths,
    /// Both 8th and 16th note subdivisions
    Both,
}

impl SwingGrid {
    /// Get all swing grid options for cycling.
    pub fn all() -> &'static [SwingGrid] {
        &[SwingGrid::Eighths, SwingGrid::Sixteenths, SwingGrid::Both]
    }

    /// Get the next swing grid option (for cycling).
    pub fn next(self) -> SwingGrid {
        match self {
            SwingGrid::Eighths => SwingGrid::Sixteenths,
            SwingGrid::Sixteenths => SwingGrid::Both,
            SwingGrid::Both => SwingGrid::Eighths,
        }
    }

    /// Human-readable name for display.
    pub fn name(self) -> &'static str {
        match self {
            SwingGrid::Eighths => "8ths",
            SwingGrid::Sixteenths => "16ths",
            SwingGrid::Both => "Both",
        }
    }
}

/// Per-track groove configuration.
///
/// All fields except `timing_offset_ms` are optional overrides.
/// When `None`, the global value is used.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct GrooveConfig {
    /// Swing amount override (0.0-1.0). None = use global.
    pub swing_amount: Option<f32>,
    /// Swing grid override. None = use global (Eighths).
    pub swing_grid: Option<SwingGrid>,
    /// Velocity humanization override (0.0-1.0). None = use global.
    pub humanize_velocity: Option<f32>,
    /// Timing humanization override (0.0-1.0). None = use global.
    pub humanize_timing: Option<f32>,
    /// Timing offset in ms (-50.0 to +50.0). Negative = rush, Positive = drag.
    pub timing_offset_ms: f32,
}

impl GrooveConfig {
    /// Create a new groove config with all defaults (use global settings).
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if this config has any per-track overrides.
    pub fn has_overrides(&self) -> bool {
        self.swing_amount.is_some()
            || self.swing_grid.is_some()
            || self.humanize_velocity.is_some()
            || self.humanize_timing.is_some()
            || self.timing_offset_ms != 0.0
    }

    /// Reset all overrides to use global settings.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Get effective swing amount, falling back to global.
    pub fn effective_swing(&self, global_swing: f32) -> f32 {
        self.swing_amount.unwrap_or(global_swing)
    }

    /// Get effective swing grid, falling back to global.
    pub fn effective_swing_grid(&self, global_grid: SwingGrid) -> SwingGrid {
        self.swing_grid.unwrap_or(global_grid)
    }

    /// Get effective velocity humanization, falling back to global.
    pub fn effective_humanize_velocity(&self, global_humanize: f32) -> f32 {
        self.humanize_velocity.unwrap_or(global_humanize)
    }

    /// Get effective timing humanization, falling back to global.
    pub fn effective_humanize_timing(&self, global_humanize: f32) -> f32 {
        self.humanize_timing.unwrap_or(global_humanize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_no_overrides() {
        let config = GrooveConfig::default();
        assert!(!config.has_overrides());
        assert_eq!(config.timing_offset_ms, 0.0);
    }

    #[test]
    fn effective_values_use_global_when_none() {
        let config = GrooveConfig::default();
        assert_eq!(config.effective_swing(0.5), 0.5);
        assert_eq!(config.effective_swing_grid(SwingGrid::Sixteenths), SwingGrid::Sixteenths);
        assert_eq!(config.effective_humanize_velocity(0.3), 0.3);
        assert_eq!(config.effective_humanize_timing(0.2), 0.2);
    }

    #[test]
    fn effective_values_use_override_when_set() {
        let config = GrooveConfig {
            swing_amount: Some(0.7),
            swing_grid: Some(SwingGrid::Both),
            humanize_velocity: Some(0.4),
            humanize_timing: Some(0.1),
            timing_offset_ms: 5.0,
        };
        assert!(config.has_overrides());
        assert_eq!(config.effective_swing(0.5), 0.7);
        assert_eq!(config.effective_swing_grid(SwingGrid::Eighths), SwingGrid::Both);
        assert_eq!(config.effective_humanize_velocity(0.3), 0.4);
        assert_eq!(config.effective_humanize_timing(0.2), 0.1);
    }

    #[test]
    fn swing_grid_cycles() {
        assert_eq!(SwingGrid::Eighths.next(), SwingGrid::Sixteenths);
        assert_eq!(SwingGrid::Sixteenths.next(), SwingGrid::Both);
        assert_eq!(SwingGrid::Both.next(), SwingGrid::Eighths);
    }

    #[test]
    fn reset_clears_overrides() {
        let mut config = GrooveConfig {
            swing_amount: Some(0.7),
            timing_offset_ms: 10.0,
            ..Default::default()
        };
        assert!(config.has_overrides());
        config.reset();
        assert!(!config.has_overrides());
    }
}
