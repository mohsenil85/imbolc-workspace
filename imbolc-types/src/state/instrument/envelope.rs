use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvConfig {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self { attack: 0.01, decay: 0.1, sustain: 0.0, release: 0.3 }
    }
}
