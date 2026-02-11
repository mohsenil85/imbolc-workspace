use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const MAX_ENTRIES: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub path: PathBuf,
    pub name: String,
    #[serde(with = "system_time_serde")]
    pub last_opened: SystemTime,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecentProjects {
    pub entries: Vec<RecentProject>,
}

impl RecentProjects {
    pub fn load() -> Self {
        let path = Self::storage_path();
        match std::fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let path = Self::storage_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }

    pub fn add(&mut self, path: &Path, name: &str) {
        // Remove existing entry with the same path
        self.entries.retain(|e| e.path != path);

        // Insert at front
        self.entries.insert(
            0,
            RecentProject {
                path: path.to_path_buf(),
                name: name.to_string(),
                last_opened: SystemTime::now(),
            },
        );

        // Trim to max
        self.entries.truncate(MAX_ENTRIES);
    }

    pub fn remove(&mut self, path: &Path) {
        self.entries.retain(|e| e.path != path);
    }

    fn storage_path() -> PathBuf {
        if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home)
                .join(".config")
                .join("imbolc")
                .join("recent.json")
        } else {
            PathBuf::from("recent.json")
        }
    }
}

mod system_time_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_promotes_to_front() {
        let mut recent = RecentProjects::default();
        recent.add(Path::new("/a.sqlite"), "a");
        recent.add(Path::new("/b.sqlite"), "b");
        assert_eq!(recent.entries[0].name, "b");
        assert_eq!(recent.entries[1].name, "a");

        // Re-adding "a" promotes it
        recent.add(Path::new("/a.sqlite"), "a");
        assert_eq!(recent.entries[0].name, "a");
        assert_eq!(recent.entries[1].name, "b");
        assert_eq!(recent.entries.len(), 2);
    }

    #[test]
    fn test_remove() {
        let mut recent = RecentProjects::default();
        recent.add(Path::new("/a.sqlite"), "a");
        recent.add(Path::new("/b.sqlite"), "b");
        recent.remove(Path::new("/a.sqlite"));
        assert_eq!(recent.entries.len(), 1);
        assert_eq!(recent.entries[0].name, "b");
    }

    #[test]
    fn test_max_entries() {
        let mut recent = RecentProjects::default();
        for i in 0..25 {
            recent.add(&PathBuf::from(format!("/{}.sqlite", i)), &format!("{}", i));
        }
        assert_eq!(recent.entries.len(), MAX_ENTRIES);
        assert_eq!(recent.entries[0].name, "24");
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut recent = RecentProjects::default();
        recent.add(Path::new("/test.sqlite"), "test");
        let json = serde_json::to_string(&recent).unwrap();
        let loaded: RecentProjects = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].name, "test");
    }
}
