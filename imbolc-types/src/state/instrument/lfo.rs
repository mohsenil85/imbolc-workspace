use serde::{Serialize, Deserialize};

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
        vec![LfoShape::Sine, LfoShape::Square, LfoShape::Saw, LfoShape::Triangle]
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
