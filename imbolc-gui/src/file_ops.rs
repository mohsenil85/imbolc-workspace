//! File operations using native dialogs.
//!
//! Uses the `rfd` crate for cross-platform file dialogs.

use std::path::PathBuf;

/// Open a file dialog to select a project file to open.
pub async fn open_project_dialog() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title("Open Imbolc Project")
        .add_filter("Imbolc Project", &["sqlite", "imbolc"])
        .add_filter("All Files", &["*"])
        .pick_file()
        .await
        .map(|f| f.path().to_path_buf())
}

/// Open a file dialog to save a project file.
pub async fn save_project_dialog() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title("Save Imbolc Project")
        .add_filter("Imbolc Project", &["sqlite"])
        .set_file_name("project.sqlite")
        .save_file()
        .await
        .map(|f| f.path().to_path_buf())
}

/// Open a file dialog to import an audio sample.
#[allow(dead_code)]
pub async fn import_sample_dialog() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title("Import Audio Sample")
        .add_filter("Audio Files", &["wav", "aiff", "mp3", "flac", "ogg"])
        .add_filter("All Files", &["*"])
        .pick_file()
        .await
        .map(|f| f.path().to_path_buf())
}

/// Open a file dialog to import multiple audio samples.
#[allow(dead_code)]
pub async fn import_samples_dialog() -> Vec<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title("Import Audio Samples")
        .add_filter("Audio Files", &["wav", "aiff", "mp3", "flac", "ogg"])
        .add_filter("All Files", &["*"])
        .pick_files()
        .await
        .map(|files| files.into_iter().map(|f| f.path().to_path_buf()).collect())
        .unwrap_or_default()
}

/// Open a file dialog to import a custom SynthDef.
#[allow(dead_code)]
pub async fn import_synthdef_dialog() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title("Import Custom SynthDef")
        .add_filter("SuperCollider SynthDef", &["scsyndef", "scd"])
        .add_filter("All Files", &["*"])
        .pick_file()
        .await
        .map(|f| f.path().to_path_buf())
}

/// Open a file dialog to select an impulse response file.
#[allow(dead_code)]
pub async fn import_impulse_response_dialog() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title("Import Impulse Response")
        .add_filter("Audio Files", &["wav", "aiff", "flac"])
        .add_filter("All Files", &["*"])
        .pick_file()
        .await
        .map(|f| f.path().to_path_buf())
}

/// Open a directory dialog to select an export location.
#[allow(dead_code)]
pub async fn select_export_directory_dialog() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title("Select Export Directory")
        .pick_folder()
        .await
        .map(|f| f.path().to_path_buf())
}
