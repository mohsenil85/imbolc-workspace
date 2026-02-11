use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

use super::action_id::parse_action_id;
use super::keymap::{KeyBinding, KeyPattern, Keymap};
use super::layer::Layer;
use super::KeyCode;

/// Raw TOML structure for the v2 keybindings config file
#[derive(Deserialize)]
struct KeybindingConfig {
    #[allow(dead_code)]
    version: u32,
    layers: HashMap<String, LayerConfig>,
}

#[derive(Deserialize)]
struct LayerConfig {
    #[serde(default = "default_transparent")]
    transparent: bool,
    bindings: Vec<RawBinding>,
}

fn default_transparent() -> bool {
    true
}

/// A single binding entry from TOML
#[derive(Deserialize)]
struct RawBinding {
    key: String,
    action: String,
    description: String,
}

/// Intern a String into a &'static str.
/// These are loaded once at startup and never freed.
fn intern(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// Parse a key notation string into a KeyPattern.
///
/// Supported formats:
/// - `"q"` → Char('q')
/// - `"Up"` → Key(KeyCode::Up)
/// - `"Ctrl+s"` → Ctrl('s')
/// - `"Alt+x"` → Alt('x')
/// - `"Ctrl+Left"` → CtrlKey(KeyCode::Left)
/// - `"Shift+Right"` → ShiftKey(KeyCode::Right)
/// - `"F1"` → Key(KeyCode::F(1))
///
/// Returns `None` for unrecognised key names (e.g. from a malformed user config).
fn parse_key(s: &str) -> Option<KeyPattern> {
    // Check for modifier prefixes
    if let Some(rest) = s.strip_prefix("Ctrl+") {
        if rest.len() == 1 {
            Some(KeyPattern::Ctrl(rest.chars().next().unwrap()))
        } else {
            parse_named_key(rest).map(KeyPattern::CtrlKey)
        }
    } else if let Some(rest) = s.strip_prefix("Alt+") {
        Some(KeyPattern::Alt(rest.chars().next().unwrap()))
    } else if let Some(rest) = s.strip_prefix("Shift+") {
        parse_named_key(rest).map(KeyPattern::ShiftKey)
    } else if s.len() == 1 {
        Some(KeyPattern::Char(s.chars().next().unwrap()))
    } else if s == "Space" {
        Some(KeyPattern::Char(' '))
    } else {
        parse_named_key(s).map(KeyPattern::Key)
    }
}

/// Parse a named key string (e.g., "Up", "Enter", "F1") into a KeyCode.
/// Returns `None` for unrecognised key names.
fn parse_named_key(s: &str) -> Option<KeyCode> {
    match s {
        "Up" => Some(KeyCode::Up),
        "Down" => Some(KeyCode::Down),
        "Left" => Some(KeyCode::Left),
        "Right" => Some(KeyCode::Right),
        "Enter" => Some(KeyCode::Enter),
        "Escape" => Some(KeyCode::Escape),
        "Backspace" => Some(KeyCode::Backspace),
        "Tab" => Some(KeyCode::Tab),
        "Home" => Some(KeyCode::Home),
        "End" => Some(KeyCode::End),
        "PageUp" => Some(KeyCode::PageUp),
        "PageDown" => Some(KeyCode::PageDown),
        "Insert" => Some(KeyCode::Insert),
        "Delete" => Some(KeyCode::Delete),
        _ if s.starts_with('F') => s[1..].parse::<u8>().ok().map(KeyCode::F),
        _ => None,
    }
}

/// Embedded default keybindings TOML
const DEFAULT_KEYBINDINGS: &str = include_str!("../../keybindings.toml");

/// Mode layer names that are not pane layers
const MODE_LAYERS: &[&str] = &[
    "global",
    "piano_mode",
    "pad_mode",
    "text_edit",
    "command_palette",
];

/// Load keybindings: embedded default, optionally merged with user override.
/// Returns (Vec<Layer> for LayerStack, pane keymaps for pane construction).
pub fn load_keybindings() -> (Vec<Layer>, HashMap<String, Keymap>) {
    let mut config: KeybindingConfig =
        toml::from_str(DEFAULT_KEYBINDINGS).expect("Failed to parse embedded keybindings.toml");

    // Try to load user override
    let user_path = user_keybindings_path();
    if let Some(path) = user_path {
        if path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(user_config) = toml::from_str::<KeybindingConfig>(&contents) {
                    merge_config(&mut config, user_config);
                }
            }
        }
    }

    let layers = build_layers(&config.layers);
    let pane_keymaps = build_pane_keymaps(&config.layers);

    (layers, pane_keymaps)
}

fn user_keybindings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("imbolc").join("keybindings.toml"))
}

/// Merge user config into the base config.
/// User layer entries fully replace the default layer entries.
fn merge_config(base: &mut KeybindingConfig, user: KeybindingConfig) {
    for (layer_id, layer_config) in user.layers {
        base.layers.insert(layer_id, layer_config);
    }
}

fn build_bindings(layer_name: &str, raw: &[RawBinding]) -> Vec<KeyBinding> {
    raw.iter()
        .filter_map(|b| {
            let pattern = match parse_key(&b.key) {
                Some(p) => p,
                None => {
                    log::warn!(target: "ui::keybindings", "ignoring unknown key '{}' in keybindings", b.key);
                    return None;
                }
            };
            match parse_action_id(layer_name, &b.action) {
                Some(action_id) => Some(KeyBinding {
                    pattern,
                    action: action_id,
                    description: intern(b.description.clone()),
                }),
                None => {
                    log::warn!(target: "ui::keybindings", "ignoring unknown action '{}' in layer '{}'", b.action, layer_name);
                    None
                }
            }
        })
        .collect()
}

fn build_layers(layers: &HashMap<String, LayerConfig>) -> Vec<Layer> {
    layers
        .iter()
        .map(|(name, config)| Layer {
            name: intern(name.clone()),
            keymap: Keymap::from_bindings(build_bindings(name, &config.bindings)),
            transparent: config.transparent,
        })
        .collect()
}

/// Build pane keymaps (excluding mode layers) for pane construction.
fn build_pane_keymaps(layers: &HashMap<String, LayerConfig>) -> HashMap<String, Keymap> {
    layers
        .iter()
        .filter(|(name, _)| !MODE_LAYERS.contains(&name.as_str()))
        .map(|(name, config)| {
            (
                name.clone(),
                Keymap::from_bindings(build_bindings(name, &config.bindings)),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key_char() {
        assert_eq!(parse_key("q"), Some(KeyPattern::Char('q')));
        assert_eq!(parse_key("+"), Some(KeyPattern::Char('+')));
    }

    #[test]
    fn test_parse_key_named() {
        assert_eq!(parse_key("Up"), Some(KeyPattern::Key(KeyCode::Up)));
        assert_eq!(parse_key("Enter"), Some(KeyPattern::Key(KeyCode::Enter)));
        assert_eq!(parse_key("Space"), Some(KeyPattern::Char(' ')));
    }

    #[test]
    fn test_parse_key_modifiers() {
        assert_eq!(parse_key("Ctrl+s"), Some(KeyPattern::Ctrl('s')));
        assert_eq!(parse_key("Alt+x"), Some(KeyPattern::Alt('x')));
        assert_eq!(
            parse_key("Ctrl+Left"),
            Some(KeyPattern::CtrlKey(KeyCode::Left))
        );
        assert_eq!(
            parse_key("Shift+Right"),
            Some(KeyPattern::ShiftKey(KeyCode::Right))
        );
    }

    #[test]
    fn test_parse_key_f_keys() {
        assert_eq!(parse_key("F1"), Some(KeyPattern::Key(KeyCode::F(1))));
        assert_eq!(parse_key("F12"), Some(KeyPattern::Key(KeyCode::F(12))));
    }

    #[test]
    fn test_parse_key_unknown() {
        assert_eq!(parse_key("Bogus"), None);
        assert_eq!(parse_key("Ctrl+Bogus"), None);
        assert_eq!(parse_key("Shift+Bogus"), None);
    }

    #[test]
    fn test_load_embedded_keybindings() {
        let (layers, pane_keymaps) = load_keybindings();
        // Should have layers
        assert!(layers.len() > 5);
        // Should have pane keymaps
        assert!(pane_keymaps.contains_key("instrument"));
        assert!(pane_keymaps.contains_key("mixer"));
        assert!(pane_keymaps.contains_key("piano_roll"));
    }
}
