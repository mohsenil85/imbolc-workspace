use std::path::PathBuf;

use serde::Deserialize;

use crate::state::music::{JIFlavor, Key, Scale, Tuning};
use crate::state::KeyboardLayout;
use crate::state::MusicalSettings;

const DEFAULT_CONFIG: &str = include_str!("../config.toml");

#[derive(Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    defaults: DefaultsConfig,
    #[serde(default)]
    runtime: RuntimeConfig,
}

#[derive(Deserialize, Default)]
struct DefaultsConfig {
    bpm: Option<u16>,
    key: Option<String>,
    scale: Option<String>,
    tuning_a4: Option<f32>,
    time_signature: Option<[u8; 2]>,
    snap: Option<bool>,
    keyboard_layout: Option<String>,
    bus_count: Option<u8>,
    tuning: Option<String>,
    ji_flavor: Option<String>,
}

#[derive(Deserialize, Default)]
struct RuntimeConfig {
    autosave: Option<bool>,
    autosave_interval_minutes: Option<u64>,
}

pub struct Config {
    defaults: DefaultsConfig,
    runtime: RuntimeConfig,
}

impl Config {
    pub fn load() -> Self {
        let mut base: ConfigFile =
            toml::from_str(DEFAULT_CONFIG).expect("Failed to parse embedded config.toml");

        if let Some(path) = user_config_path() {
            if path.exists() {
                match std::fs::read_to_string(&path) {
                    Ok(contents) => match toml::from_str::<ConfigFile>(&contents) {
                        Ok(user) => {
                            merge_defaults(&mut base.defaults, user.defaults);
                            merge_runtime(&mut base.runtime, user.runtime);
                        }
                        Err(e) => {
                            log::warn!(target: "config", "ignoring malformed config {}: {}", path.display(), e)
                        }
                    },
                    Err(e) => {
                        log::warn!(target: "config", "could not read config {}: {}", path.display(), e)
                    }
                }
            }
        }

        Config {
            defaults: base.defaults,
            runtime: base.runtime,
        }
    }

    pub fn keyboard_layout(&self) -> KeyboardLayout {
        self.defaults
            .keyboard_layout
            .as_deref()
            .and_then(parse_keyboard_layout)
            .unwrap_or_default()
    }

    /// Get the default number of mixing buses for new projects
    pub fn default_bus_count(&self) -> u8 {
        use crate::state::session::DEFAULT_BUS_COUNT;
        self.defaults.bus_count.unwrap_or(DEFAULT_BUS_COUNT)
    }

    pub fn defaults(&self) -> MusicalSettings {
        let fallback = MusicalSettings::default();
        MusicalSettings {
            bpm: self.defaults.bpm.unwrap_or(fallback.bpm),
            key: self
                .defaults
                .key
                .as_deref()
                .and_then(parse_key)
                .unwrap_or(fallback.key),
            scale: self
                .defaults
                .scale
                .as_deref()
                .and_then(parse_scale)
                .unwrap_or(fallback.scale),
            tuning_a4: self.defaults.tuning_a4.unwrap_or(fallback.tuning_a4),
            time_signature: self
                .defaults
                .time_signature
                .map(|ts| (ts[0], ts[1]))
                .unwrap_or(fallback.time_signature),
            snap: self.defaults.snap.unwrap_or(fallback.snap),
            tuning: self
                .defaults
                .tuning
                .as_deref()
                .and_then(parse_tuning)
                .unwrap_or(fallback.tuning),
            ji_flavor: self
                .defaults
                .ji_flavor
                .as_deref()
                .and_then(parse_ji_flavor)
                .unwrap_or(fallback.ji_flavor),
        }
    }

    /// Whether periodic autosave snapshots are enabled.
    pub fn autosave_enabled(&self) -> bool {
        self.runtime.autosave.unwrap_or(true)
    }

    /// Autosave interval in minutes (clamped to 1..10080).
    pub fn autosave_interval_minutes(&self) -> u64 {
        self.runtime
            .autosave_interval_minutes
            .unwrap_or(2)
            .clamp(1, 10_080)
    }
}

fn user_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("imbolc").join("config.toml"))
}

fn merge_defaults(base: &mut DefaultsConfig, user: DefaultsConfig) {
    if user.bpm.is_some() {
        base.bpm = user.bpm;
    }
    if user.key.is_some() {
        base.key = user.key;
    }
    if user.scale.is_some() {
        base.scale = user.scale;
    }
    if user.tuning_a4.is_some() {
        base.tuning_a4 = user.tuning_a4;
    }
    if user.time_signature.is_some() {
        base.time_signature = user.time_signature;
    }
    if user.snap.is_some() {
        base.snap = user.snap;
    }
    if user.keyboard_layout.is_some() {
        base.keyboard_layout = user.keyboard_layout;
    }
    if user.bus_count.is_some() {
        base.bus_count = user.bus_count;
    }
    if user.tuning.is_some() {
        base.tuning = user.tuning;
    }
    if user.ji_flavor.is_some() {
        base.ji_flavor = user.ji_flavor;
    }
}

fn merge_runtime(base: &mut RuntimeConfig, user: RuntimeConfig) {
    if user.autosave.is_some() {
        base.autosave = user.autosave;
    }
    if user.autosave_interval_minutes.is_some() {
        base.autosave_interval_minutes = user.autosave_interval_minutes;
    }
}

fn parse_key(s: &str) -> Option<Key> {
    match s {
        "C" => Some(Key::C),
        "C#" | "Cs" => Some(Key::Cs),
        "D" => Some(Key::D),
        "D#" | "Ds" => Some(Key::Ds),
        "E" => Some(Key::E),
        "F" => Some(Key::F),
        "F#" | "Fs" => Some(Key::Fs),
        "G" => Some(Key::G),
        "G#" | "Gs" => Some(Key::Gs),
        "A" => Some(Key::A),
        "A#" | "As" => Some(Key::As),
        "B" => Some(Key::B),
        _ => None,
    }
}

fn parse_keyboard_layout(s: &str) -> Option<KeyboardLayout> {
    match s.to_lowercase().as_str() {
        "qwerty" => Some(KeyboardLayout::Qwerty),
        "colemak" => Some(KeyboardLayout::Colemak),
        _ => None,
    }
}

fn parse_scale(s: &str) -> Option<Scale> {
    match s {
        "Major" => Some(Scale::Major),
        "Minor" => Some(Scale::Minor),
        "Dorian" => Some(Scale::Dorian),
        "Phrygian" => Some(Scale::Phrygian),
        "Lydian" => Some(Scale::Lydian),
        "Mixolydian" => Some(Scale::Mixolydian),
        "Aeolian" => Some(Scale::Aeolian),
        "Locrian" => Some(Scale::Locrian),
        "Pentatonic" => Some(Scale::Pentatonic),
        "Blues" => Some(Scale::Blues),
        "Chromatic" => Some(Scale::Chromatic),
        _ => None,
    }
}

fn parse_tuning(s: &str) -> Option<Tuning> {
    match s {
        "EqualTemperament" | "ET" | "12-TET" => Some(Tuning::EqualTemperament),
        "ScaleJI" => Some(Tuning::ScaleJI),
        "ChordJI" => Some(Tuning::ChordJI),
        "AdaptiveJI" => Some(Tuning::AdaptiveJI),
        "GlobalJI" => Some(Tuning::GlobalJI),
        _ => None,
    }
}

fn parse_ji_flavor(s: &str) -> Option<JIFlavor> {
    match s {
        "FiveLimit" | "5-Limit" | "5L" => Some(JIFlavor::FiveLimit),
        "SevenLimit" | "7-Limit" | "7L" => Some(JIFlavor::SevenLimit),
        "Pythagorean" => Some(JIFlavor::Pythagorean),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_embedded_config() {
        let config = Config::load();
        let defaults = config.defaults();
        assert_eq!(defaults.bpm, 120);
        assert_eq!(defaults.key, Key::C);
        assert_eq!(defaults.scale, Scale::Major);
        assert!((defaults.tuning_a4 - 432.0).abs() < f32::EPSILON);
        assert_eq!(defaults.time_signature, (4, 4));
        assert!(!defaults.snap);
        assert_eq!(config.keyboard_layout(), KeyboardLayout::Colemak);
        assert!(config.autosave_enabled());
        assert_eq!(config.autosave_interval_minutes(), 2);
    }

    #[test]
    fn test_parse_keys() {
        assert_eq!(parse_key("C"), Some(Key::C));
        assert_eq!(parse_key("C#"), Some(Key::Cs));
        assert_eq!(parse_key("Fs"), Some(Key::Fs));
        assert_eq!(parse_key("F#"), Some(Key::Fs));
        assert_eq!(parse_key("X"), None);
    }

    #[test]
    fn test_parse_scales() {
        assert_eq!(parse_scale("Major"), Some(Scale::Major));
        assert_eq!(parse_scale("Minor"), Some(Scale::Minor));
        assert_eq!(parse_scale("Blues"), Some(Scale::Blues));
        assert_eq!(parse_scale("Nope"), None);
    }

    #[test]
    fn test_parse_keyboard_layout() {
        assert_eq!(
            parse_keyboard_layout("qwerty"),
            Some(KeyboardLayout::Qwerty)
        );
        assert_eq!(
            parse_keyboard_layout("COLEMAK"),
            Some(KeyboardLayout::Colemak)
        );
        assert_eq!(parse_keyboard_layout("unknown"), None);
    }
}
