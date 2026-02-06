//! Content loading for DocsPane
//!
//! Handles loading documentation from embedded resources and user overlay.

use std::collections::HashMap;

/// Topic entry for the browser
#[derive(Debug, Clone)]
pub struct TopicEntry {
    pub title: String,
    pub path: String,
}

// Embedded documentation content
const SOURCES_TOML: &str = include_str!("../../docs/sources.toml");
const INDEX_TOML: &str = include_str!("../../docs/index.toml");

// Embedded markdown files
const DOC_OSCILLATORS: &str = include_str!("../../docs/sources/oscillators.md");
const DOC_FM: &str = include_str!("../../docs/sources/fm.md");
const DOC_PHYSICAL: &str = include_str!("../../docs/sources/physical.md");
const DOC_DRUMS: &str = include_str!("../../docs/sources/drums.md");
const DOC_SAMPLERS: &str = include_str!("../../docs/sources/samplers.md");
const DOC_ROUTING: &str = include_str!("../../docs/sources/routing.md");

/// Load the sources.toml mapping (source short_name -> doc path)
pub fn load_sources_map() -> HashMap<String, String> {
    let mut map = HashMap::new();

    // Parse the embedded TOML
    if let Ok(value) = SOURCES_TOML.parse::<toml::Table>() {
        if let Some(sources) = value.get("sources").and_then(|v| v.as_table()) {
            for (key, val) in sources {
                if let Some(path) = val.as_str() {
                    map.insert(key.clone(), path.to_string());
                }
            }
        }
    }

    // Try to load user overlay from ~/.config/imbolc/docs/sources.toml
    if let Some(config_dir) = dirs::config_dir() {
        let user_sources = config_dir.join("imbolc").join("docs").join("sources.toml");
        if let Ok(content) = std::fs::read_to_string(&user_sources) {
            if let Ok(value) = content.parse::<toml::Table>() {
                if let Some(sources) = value.get("sources").and_then(|v| v.as_table()) {
                    for (key, val) in sources {
                        if let Some(path) = val.as_str() {
                            map.insert(key.clone(), path.to_string());
                        }
                    }
                }
            }
        }
    }

    map
}

/// Load the topic index for the browser
pub fn load_topic_index() -> Vec<TopicEntry> {
    let mut topics = Vec::new();

    // Parse the embedded TOML
    if let Ok(value) = INDEX_TOML.parse::<toml::Table>() {
        if let Some(topic_array) = value.get("topics").and_then(|v| v.as_array()) {
            for item in topic_array {
                if let Some(table) = item.as_table() {
                    let title = table
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Untitled")
                        .to_string();
                    let path = table
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    topics.push(TopicEntry { title, path });
                }
            }
        }
    }

    topics
}

/// Load a documentation file by path
///
/// First checks user overlay in ~/.config/imbolc/docs/, then falls back to embedded.
pub fn load_doc(path: &str) -> Option<String> {
    // Try user overlay first
    if let Some(config_dir) = dirs::config_dir() {
        let user_path = config_dir.join("imbolc").join("docs").join(path);
        if let Ok(content) = std::fs::read_to_string(&user_path) {
            return Some(content);
        }
    }

    // Fall back to embedded
    match path {
        "sources/oscillators.md" => Some(DOC_OSCILLATORS.to_string()),
        "sources/fm.md" => Some(DOC_FM.to_string()),
        "sources/physical.md" => Some(DOC_PHYSICAL.to_string()),
        "sources/drums.md" => Some(DOC_DRUMS.to_string()),
        "sources/samplers.md" => Some(DOC_SAMPLERS.to_string()),
        "sources/routing.md" => Some(DOC_ROUTING.to_string()),
        _ => None,
    }
}
