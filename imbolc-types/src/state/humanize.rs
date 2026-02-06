use serde::{Deserialize, Serialize};

/// Global humanization settings for velocity and timing jitter.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct HumanizeSettings {
    /// Velocity jitter amount (0.0-1.0)
    pub velocity: f32,
    /// Timing jitter amount (0.0-1.0)
    pub timing: f32,
}

impl HumanizeSettings {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_zero() {
        let settings = HumanizeSettings::default();
        assert_eq!(settings.velocity, 0.0);
        assert_eq!(settings.timing, 0.0);
    }
}
