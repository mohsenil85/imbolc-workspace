//! Project metadata state.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::session::MusicalSettings;

/// Project metadata (path, dirty flag, defaults).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    /// Current project file path (None = untitled/new project)
    #[serde(skip)]
    pub path: Option<PathBuf>,
    /// Whether state has changed since last save/load
    #[serde(skip)]
    pub dirty: bool,
    /// Musical defaults used when creating new projects
    pub default_settings: MusicalSettings,
}

impl Default for ProjectMeta {
    fn default() -> Self {
        Self {
            path: None,
            dirty: false,
            default_settings: MusicalSettings::default(),
        }
    }
}

impl ProjectMeta {
    pub fn new_with_defaults(defaults: MusicalSettings) -> Self {
        Self {
            path: None,
            dirty: false,
            default_settings: defaults,
        }
    }
}
