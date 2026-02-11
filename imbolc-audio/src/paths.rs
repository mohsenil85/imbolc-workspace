use std::path::PathBuf;

/// Resolve the built-in synthdefs directory.
///
/// Fallback chain:
/// 1. `IMBOLC_SYNTHDEFS_DIR` env var (runtime override)
/// 2. `CARGO_MANIFEST_DIR/../imbolc-core/synthdefs` (compile-time, resolves to imbolc-core/)
/// 3. `./synthdefs` relative to CWD (backward compat)
pub fn synthdefs_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("IMBOLC_SYNTHDEFS_DIR") {
        return PathBuf::from(dir);
    }

    let compile_time = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../imbolc-core/synthdefs"
    ));
    if compile_time.exists() {
        return compile_time;
    }

    PathBuf::from("synthdefs")
}

/// User-local directory for custom synthdefs (`~/.config/imbolc/synthdefs/`).
pub fn custom_synthdefs_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home)
            .join(".config")
            .join("imbolc")
            .join("synthdefs")
    } else {
        PathBuf::from("synthdefs")
    }
}
