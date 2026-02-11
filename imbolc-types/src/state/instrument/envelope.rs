use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvConfig {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.0,
            release: 0.3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_config_default() {
        let env = EnvConfig::default();
        assert_eq!(env.attack, 0.01);
        assert_eq!(env.decay, 0.1);
        assert_eq!(env.sustain, 0.0);
        assert_eq!(env.release, 0.3);
    }

    #[test]
    fn env_config_clone() {
        let env = EnvConfig {
            attack: 0.05,
            decay: 0.2,
            sustain: 0.8,
            release: 0.5,
        };
        let cloned = env.clone();
        assert_eq!(cloned.attack, env.attack);
        assert_eq!(cloned.decay, env.decay);
        assert_eq!(cloned.sustain, env.sustain);
        assert_eq!(cloned.release, env.release);
    }
}
