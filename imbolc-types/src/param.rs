use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub value: ParamValue,
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParamValue {
    Float(f32),
    Int(i32),
    Bool(bool),
}

impl ParamValue {
    pub fn to_f32(&self) -> f32 {
        match self {
            ParamValue::Float(v) => *v,
            ParamValue::Int(v) => *v as f32,
            ParamValue::Bool(v) => if *v { 1.0 } else { 0.0 },
        }
    }
}

// --- Param adjustment algorithms (extracted from UI panes) ---

/// Check if a parameter name represents a frequency-type parameter
pub fn is_freq_param(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("freq") || lower.contains("cutoff") || lower.contains("formant") || lower.contains("bw")
}

/// Move a frequency value by one semitone up or down
pub fn adjust_freq_semitone(value: f32, increase: bool, tuning_a4: f32, min: f32, max: f32) -> f32 {
    let midi = 69.0 + 12.0 * (value / tuning_a4).ln() / 2.0_f32.ln();
    let new_midi = if increase { midi.round() + 1.0 } else { midi.round() - 1.0 };
    (tuning_a4 * 2.0_f32.powf((new_midi - 69.0) / 12.0)).clamp(min, max)
}

/// Snap to nearest "nice" step based on param range
pub fn adjust_musical_step(value: f32, increase: bool, min: f32, max: f32) -> f32 {
    let range = max - min;
    let step = if range <= 1.0 {
        0.1
    } else if range <= 10.0 {
        0.5
    } else if range <= 100.0 {
        1.0
    } else {
        10.0
    };
    let snapped = (value / step).round() * step;
    let new_val = if increase { snapped + step } else { snapped - step };
    new_val.clamp(min, max)
}

impl Param {
    /// Adjust the parameter value by a fraction of its range
    pub fn adjust(&mut self, increase: bool, fraction: f32) {
        let range = self.max - self.min;
        match &mut self.value {
            ParamValue::Float(ref mut v) => {
                let delta = range * fraction;
                if increase { *v = (*v + delta).min(self.max); }
                else { *v = (*v - delta).max(self.min); }
            }
            ParamValue::Int(ref mut v) => {
                let delta = ((range * fraction) as i32).max(1);
                if increase { *v = (*v + delta).min(self.max as i32); }
                else { *v = (*v - delta).max(self.min as i32); }
            }
            ParamValue::Bool(ref mut v) => { *v = !*v; }
        }
    }

    /// Adjust the parameter using musical-step logic (semitones for freq params, nice steps otherwise)
    pub fn adjust_musical(&mut self, increase: bool, tuning_a4: f32) {
        match &mut self.value {
            ParamValue::Float(ref mut v) => {
                if is_freq_param(&self.name) {
                    *v = adjust_freq_semitone(*v, increase, tuning_a4, self.min, self.max);
                } else {
                    *v = adjust_musical_step(*v, increase, self.min, self.max);
                }
            }
            ParamValue::Int(ref mut v) => {
                let range = self.max - self.min;
                let step = if range <= 10.0 { 1 } else if range <= 100.0 { 5 } else { 10 };
                if increase { *v = (*v + step).min(self.max as i32); }
                else { *v = (*v - step).max(self.min as i32); }
            }
            ParamValue::Bool(ref mut v) => { *v = !*v; }
        }
    }

    /// Set the parameter to its minimum (zero) value
    pub fn zero(&mut self) {
        match &mut self.value {
            ParamValue::Float(ref mut v) => *v = self.min,
            ParamValue::Int(ref mut v) => *v = self.min as i32,
            ParamValue::Bool(ref mut v) => *v = false,
        }
    }

    /// Parse a string and set the value, clamping to bounds. Returns true on success.
    pub fn parse_and_set(&mut self, text: &str) -> bool {
        match &mut self.value {
            ParamValue::Float(ref mut v) => {
                if let Ok(parsed) = text.parse::<f32>() {
                    *v = parsed.clamp(self.min, self.max);
                    true
                } else {
                    false
                }
            }
            ParamValue::Int(ref mut v) => {
                if let Ok(parsed) = text.parse::<i32>() {
                    *v = parsed.clamp(self.min as i32, self.max as i32);
                    true
                } else {
                    false
                }
            }
            ParamValue::Bool(ref mut v) => {
                match text.to_lowercase().as_str() {
                    "true" | "1" | "yes" | "on" => { *v = true; true }
                    "false" | "0" | "no" | "off" => { *v = false; true }
                    _ => false,
                }
            }
        }
    }

    /// Adjust by a raw delta scaled to 2% of range. Returns `Some(new_value)` for Float params.
    pub fn adjust_delta(&mut self, delta: f32) -> Option<f32> {
        let range = self.max - self.min;
        match &mut self.value {
            ParamValue::Float(v) => {
                *v = (*v + delta * range * 0.02).clamp(self.min, self.max);
                Some(*v)
            }
            ParamValue::Int(v) => {
                *v = (*v + (delta * range * 0.02) as i32).clamp(self.min as i32, self.max as i32);
                None
            }
            ParamValue::Bool(b) => {
                *b = !*b;
                None
            }
        }
    }

    /// Get the current value as a display string
    pub fn value_string(&self) -> String {
        match &self.value {
            ParamValue::Float(v) => format!("{:.2}", v),
            ParamValue::Int(v) => format!("{}", v),
            ParamValue::Bool(v) => format!("{}", v),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn float_param(name: &str, value: f32, min: f32, max: f32) -> Param {
        Param { name: name.to_string(), value: ParamValue::Float(value), min, max }
    }

    fn int_param(name: &str, value: i32, min: f32, max: f32) -> Param {
        Param { name: name.to_string(), value: ParamValue::Int(value), min, max }
    }

    fn bool_param(name: &str, value: bool) -> Param {
        Param { name: name.to_string(), value: ParamValue::Bool(value), min: 0.0, max: 1.0 }
    }

    #[test]
    fn is_freq_param_matches() {
        assert!(is_freq_param("freq"));
        assert!(is_freq_param("Cutoff"));
        assert!(is_freq_param("formant_freq"));
        assert!(is_freq_param("bw"));
        assert!(!is_freq_param("gain"));
        assert!(!is_freq_param("level"));
    }

    #[test]
    fn adjust_freq_semitone_up_down() {
        let a4 = 440.0;
        let up = adjust_freq_semitone(a4, true, 440.0, 20.0, 20000.0);
        // A#4 ≈ 466.16
        assert!((up - 466.16).abs() < 0.1);

        let down = adjust_freq_semitone(a4, false, 440.0, 20.0, 20000.0);
        // G#4 ≈ 415.30
        assert!((down - 415.30).abs() < 0.1);
    }

    #[test]
    fn adjust_freq_semitone_clamps() {
        let result = adjust_freq_semitone(20.0, false, 440.0, 20.0, 20000.0);
        assert!(result >= 20.0);
    }

    #[test]
    fn adjust_musical_step_small_range() {
        // range=1.0 → step=0.1
        let result = adjust_musical_step(0.5, true, 0.0, 1.0);
        assert!((result - 0.6).abs() < 0.01);
    }

    #[test]
    fn adjust_musical_step_clamps() {
        let result = adjust_musical_step(1.0, true, 0.0, 1.0);
        assert!((result - 1.0).abs() < 0.01);
    }

    #[test]
    fn param_adjust_float() {
        let mut p = float_param("gain", 0.5, 0.0, 1.0);
        p.adjust(true, 0.1);
        assert!((p.value.to_f32() - 0.6).abs() < 0.01);

        p.adjust(false, 0.1);
        assert!((p.value.to_f32() - 0.5).abs() < 0.01);
    }

    #[test]
    fn param_adjust_int() {
        let mut p = int_param("steps", 5, 0.0, 10.0);
        p.adjust(true, 0.15);
        assert_eq!(p.value.to_f32() as i32, 6); // delta = (10*0.15)=1.5 → 1(max(1)) → 5+1=6
    }

    #[test]
    fn param_adjust_bool() {
        let mut p = bool_param("enabled", false);
        p.adjust(true, 0.1);
        assert_eq!(p.value.to_f32(), 1.0);
    }

    #[test]
    fn param_adjust_musical_freq() {
        let mut p = float_param("freq", 440.0, 20.0, 20000.0);
        p.adjust_musical(true, 440.0);
        assert!((p.value.to_f32() - 466.16).abs() < 0.1);
    }

    #[test]
    fn param_adjust_musical_non_freq() {
        let mut p = float_param("gain", 0.5, 0.0, 1.0);
        p.adjust_musical(true, 440.0);
        // range=1.0 → step=0.1, snapped=0.5, result=0.6
        assert!((p.value.to_f32() - 0.6).abs() < 0.01);
    }

    #[test]
    fn param_zero() {
        let mut p = float_param("gain", 0.8, 0.0, 1.0);
        p.zero();
        assert_eq!(p.value.to_f32(), 0.0);

        let mut p = int_param("steps", 5, 1.0, 10.0);
        p.zero();
        assert_eq!(p.value.to_f32(), 1.0);

        let mut p = bool_param("on", true);
        p.zero();
        assert_eq!(p.value.to_f32(), 0.0);
    }

    #[test]
    fn param_parse_and_set() {
        let mut p = float_param("gain", 0.5, 0.0, 1.0);
        assert!(p.parse_and_set("0.75"));
        assert!((p.value.to_f32() - 0.75).abs() < 0.001);

        // Clamps to max
        assert!(p.parse_and_set("5.0"));
        assert!((p.value.to_f32() - 1.0).abs() < 0.001);

        // Invalid
        assert!(!p.parse_and_set("abc"));
    }

    #[test]
    fn param_parse_and_set_int() {
        let mut p = int_param("steps", 5, 0.0, 10.0);
        assert!(p.parse_and_set("7"));
        assert_eq!(p.value.to_f32() as i32, 7);
    }

    #[test]
    fn param_parse_and_set_bool() {
        let mut p = bool_param("on", false);
        assert!(p.parse_and_set("true"));
        assert_eq!(p.value.to_f32(), 1.0);
        assert!(p.parse_and_set("off"));
        assert_eq!(p.value.to_f32(), 0.0);
        assert!(!p.parse_and_set("maybe"));
    }

    #[test]
    fn adjust_delta_float() {
        let mut p = float_param("gain", 0.5, 0.0, 1.0);
        let result = p.adjust_delta(1.0);
        // delta=1.0, range=1.0, step=0.02 → 0.5 + 0.02 = 0.52
        assert!(result.is_some());
        assert!((result.unwrap() - 0.52).abs() < 0.001);
        assert!((p.value.to_f32() - 0.52).abs() < 0.001);
    }

    #[test]
    fn adjust_delta_float_clamps() {
        let mut p = float_param("gain", 0.99, 0.0, 1.0);
        let result = p.adjust_delta(5.0);
        assert_eq!(result, Some(1.0));
    }

    #[test]
    fn adjust_delta_int() {
        let mut p = int_param("steps", 5, 0.0, 100.0);
        let result = p.adjust_delta(1.0);
        // delta=1.0, range=100, step=2.0 → 5 + 2 = 7
        assert!(result.is_none());
        assert_eq!(p.value.to_f32() as i32, 7);
    }

    #[test]
    fn adjust_delta_bool() {
        let mut p = bool_param("on", false);
        let result = p.adjust_delta(1.0);
        assert!(result.is_none());
        assert_eq!(p.value.to_f32(), 1.0);
    }

    #[test]
    fn param_value_string() {
        let p = float_param("x", 0.123, 0.0, 1.0);
        assert_eq!(p.value_string(), "0.12");

        let p = int_param("x", 42, 0.0, 100.0);
        assert_eq!(p.value_string(), "42");

        let p = bool_param("x", true);
        assert_eq!(p.value_string(), "true");
    }
}
