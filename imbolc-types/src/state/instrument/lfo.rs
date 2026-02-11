use serde::{Deserialize, Serialize};

use crate::ParameterTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LfoShape {
    Sine,
    Square,
    Saw,
    Triangle,
}

impl LfoShape {
    pub fn name(&self) -> &'static str {
        match self {
            LfoShape::Sine => "Sine",
            LfoShape::Square => "Square",
            LfoShape::Saw => "Saw",
            LfoShape::Triangle => "Triangle",
        }
    }

    pub fn index(&self) -> i32 {
        match self {
            LfoShape::Sine => 0,
            LfoShape::Square => 1,
            LfoShape::Saw => 2,
            LfoShape::Triangle => 3,
        }
    }

    #[allow(dead_code)]
    pub fn all() -> Vec<LfoShape> {
        vec![
            LfoShape::Sine,
            LfoShape::Square,
            LfoShape::Saw,
            LfoShape::Triangle,
        ]
    }

    pub fn next(&self) -> LfoShape {
        match self {
            LfoShape::Sine => LfoShape::Square,
            LfoShape::Square => LfoShape::Saw,
            LfoShape::Saw => LfoShape::Triangle,
            LfoShape::Triangle => LfoShape::Sine,
        }
    }

    pub fn from_name(name: &str) -> Option<LfoShape> {
        match name {
            "Sine" => Some(LfoShape::Sine),
            "Square" => Some(LfoShape::Square),
            "Saw" => Some(LfoShape::Saw),
            "Triangle" => Some(LfoShape::Triangle),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfoConfig {
    pub enabled: bool,
    pub rate: f32,
    pub depth: f32,
    pub shape: LfoShape,
    pub target: ParameterTarget,
}

impl Default for LfoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rate: 2.0,
            depth: 0.5,
            shape: LfoShape::Sine,
            target: ParameterTarget::FilterCutoff,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lfo_shape_name() {
        assert_eq!(LfoShape::Sine.name(), "Sine");
        assert_eq!(LfoShape::Square.name(), "Square");
        assert_eq!(LfoShape::Saw.name(), "Saw");
        assert_eq!(LfoShape::Triangle.name(), "Triangle");
    }

    #[test]
    fn lfo_shape_index() {
        assert_eq!(LfoShape::Sine.index(), 0);
        assert_eq!(LfoShape::Square.index(), 1);
        assert_eq!(LfoShape::Saw.index(), 2);
        assert_eq!(LfoShape::Triangle.index(), 3);
    }

    #[test]
    fn lfo_shape_all() {
        assert_eq!(LfoShape::all().len(), 4);
    }

    #[test]
    fn lfo_shape_next_cycles() {
        assert_eq!(LfoShape::Sine.next(), LfoShape::Square);
        assert_eq!(LfoShape::Square.next(), LfoShape::Saw);
        assert_eq!(LfoShape::Saw.next(), LfoShape::Triangle);
        assert_eq!(LfoShape::Triangle.next(), LfoShape::Sine);
    }

    #[test]
    fn lfo_shape_from_name() {
        assert_eq!(LfoShape::from_name("Sine"), Some(LfoShape::Sine));
        assert_eq!(LfoShape::from_name("Square"), Some(LfoShape::Square));
        assert_eq!(LfoShape::from_name("unknown"), None);
    }

    #[test]
    fn lfo_shape_from_name_case_sensitive() {
        assert_eq!(LfoShape::from_name("sine"), None);
    }

    #[test]
    fn lfo_config_default() {
        let cfg = LfoConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.rate, 2.0);
        assert_eq!(cfg.depth, 0.5);
        assert_eq!(cfg.shape, LfoShape::Sine);
    }

    #[test]
    fn lfo_shape_index_matches_position() {
        for (i, shape) in LfoShape::all().iter().enumerate() {
            assert_eq!(shape.index(), i as i32);
        }
    }
}
