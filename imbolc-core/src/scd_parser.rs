//! Parser for SuperCollider SynthDef source files (.scd)
//!
//! Extracts synthdef name and parameters from .scd files using regex.

use regex::Regex;

/// Parsed result from an .scd file
pub struct ParsedSynthDef {
    pub name: String,
    pub params: Vec<(String, f32)>, // (name, default)
}

/// Internal params to filter out (not user-editable)
const INTERNAL_PARAMS: &[&str] = &[
    "out", "freq_in", "gate_in", "vel_in", "attack", "decay", "sustain", "release",
];

/// Parse a SynthDef .scd file and extract name and parameters
pub fn parse_scd_file(content: &str) -> Result<ParsedSynthDef, String> {
    // Find SynthDef name: SynthDef(\name, ... or SynthDef("name", ...
    let name_re =
        Regex::new(r#"SynthDef\s*\(\s*[\\"](\w+)"#).map_err(|e| format!("Regex error: {}", e))?;

    let name = name_re
        .captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or("Could not find SynthDef name")?;

    // Find args: { |arg1=val1, arg2=val2, ...|
    let args_re = Regex::new(r"\{\s*\|([^|]+)\|").map_err(|e| format!("Regex error: {}", e))?;

    let args_str = args_re
        .captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .ok_or("Could not find SynthDef arguments")?;

    // Parse individual args: name=default or just name
    let param_re =
        Regex::new(r"(\w+)\s*=\s*\(?\s*(-?[\d.]+)").map_err(|e| format!("Regex error: {}", e))?;

    let params: Vec<(String, f32)> = param_re
        .captures_iter(args_str)
        .filter_map(|c| {
            let name = c.get(1)?.as_str().to_string();
            let default: f32 = c.get(2)?.as_str().parse().ok()?;
            // Filter out internal params
            if INTERNAL_PARAMS.contains(&name.as_str()) {
                None
            } else {
                Some((name, default))
            }
        })
        .collect();

    Ok(ParsedSynthDef { name, params })
}

/// Infer min/max from param name and default value
pub fn infer_param_range(name: &str, default: f32) -> (f32, f32) {
    let name_lower = name.to_lowercase();
    match name_lower.as_str() {
        n if n.contains("freq") || n.contains("cutoff") => (20.0, 20000.0),
        n if n.contains("amp") || n.contains("level") || n.contains("mix") || n.contains("wet") => {
            (0.0, 1.0)
        }
        n if n.contains("rate") => (0.1, 10.0),
        n if n.contains("time") || n.contains("delay") => (0.0, 2.0),
        n if n.contains("pan") => (-1.0, 1.0),
        n if n.contains("resonance") || n.contains("res") || n.contains("q") => (0.0, 1.0),
        n if n.contains("detune") => (-12.0, 12.0),
        n if n.contains("phase") => (0.0, 1.0),
        n if n.contains("feedback") || n.contains("fb") => (0.0, 1.0),
        _ => {
            // Generic: ±10x default, or 0-1 if default is in that range
            if (0.0..=1.0).contains(&default) {
                (0.0, 1.0)
            } else if default > 0.0 {
                (default * 0.1, default * 10.0)
            } else {
                (default * 10.0, default.abs() * 10.0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_synthdef() {
        let content = r#"
SynthDef(\my_synth, {
    |out=0, freq_in=(-1), gate_in=(-1), my_param=0.5, cutoff=1000|
    var sig = SinOsc.ar(440);
    Out.ar(out, sig);
});
"#;
        let result = parse_scd_file(content).unwrap();
        assert_eq!(result.name, "my_synth");
        assert_eq!(result.params.len(), 2); // my_param and cutoff (out, freq_in, gate_in filtered)
        assert_eq!(result.params[0].0, "my_param");
        assert_eq!(result.params[0].1, 0.5);
        assert_eq!(result.params[1].0, "cutoff");
        assert_eq!(result.params[1].1, 1000.0);
    }

    #[test]
    fn test_parse_string_name() {
        let content = r#"
SynthDef("test_synth", {
    |out=0, gain=0.8|
    Out.ar(out, SinOsc.ar(440) * gain);
});
"#;
        let result = parse_scd_file(content).unwrap();
        assert_eq!(result.name, "test_synth");
        assert_eq!(result.params.len(), 1);
        assert_eq!(result.params[0].0, "gain");
    }

    #[test]
    fn test_infer_range_freq() {
        let (min, max) = infer_param_range("cutoff_freq", 1000.0);
        assert_eq!(min, 20.0);
        assert_eq!(max, 20000.0);
    }

    #[test]
    fn test_infer_range_amp() {
        let (min, max) = infer_param_range("amp", 0.5);
        assert_eq!(min, 0.0);
        assert_eq!(max, 1.0);
    }

    #[test]
    fn test_infer_range_pan() {
        let (min, max) = infer_param_range("pan", 0.0);
        assert_eq!(min, -1.0);
        assert_eq!(max, 1.0);
    }

    #[test]
    fn infer_range_unknown_param() {
        // Default 0.5 is in 0-1 range, so should get (0.0, 1.0)
        let (min, max) = infer_param_range("some_random_param", 0.5);
        assert_eq!(min, 0.0);
        assert_eq!(max, 1.0);
    }

    #[test]
    fn parse_empty_string_fails() {
        let result = parse_scd_file("");
        assert!(result.is_err());
    }
}
