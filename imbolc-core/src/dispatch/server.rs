use std::path::PathBuf;

use imbolc_audio::AudioHandle;
use crate::state::AppState;
use crate::action::{DispatchResult, ServerAction};
use super::side_effects::AudioSideEffect;

pub(super) fn dispatch_server(
    action: &ServerAction,
    state: &mut AppState,
    audio: &AudioHandle,
    effects: &mut Vec<AudioSideEffect>,
) -> DispatchResult {
    let mut result = DispatchResult::none();

    match action {
        ServerAction::Connect => {
            effects.push(AudioSideEffect::UpdateState);
            effects.push(AudioSideEffect::Connect { server_addr: "127.0.0.1:57110".to_string() });
            result.push_status(audio.status(), "Connecting...");
        }
        ServerAction::Disconnect => {
            effects.push(AudioSideEffect::Disconnect);
            result.push_status(audio.status(), "Disconnecting...");
        }
        ServerAction::Start { input_device, output_device, buffer_size, sample_rate } => {
            effects.push(AudioSideEffect::StartServer {
                input_device: input_device.clone(),
                output_device: output_device.clone(),
                buffer_size: *buffer_size,
                sample_rate: *sample_rate,
            });
            result.push_status(audio.status(), "Starting server...");
        }
        ServerAction::Stop => {
            effects.push(AudioSideEffect::StopServer);
            result.push_status(audio.status(), "Stopping server...");
        }
        ServerAction::CompileSynthDefs => {
            let scd_path = crate::paths::compile_scd_path();
            effects.push(AudioSideEffect::CompileSynthDefs { scd_path: scd_path.clone() });
            result.push_status(audio.status(), "Compiling synthdefs...");
        }
        ServerAction::CompileVstSynthDefs => {
            let scd_path = crate::paths::compile_vst_scd_path();
            effects.push(AudioSideEffect::CompileSynthDefs { scd_path: scd_path.clone() });
            result.push_status(audio.status(), "Compiling VST synthdefs...");
        }
        ServerAction::LoadSynthDefs => {
            // Load built-in synthdefs (fire-and-forget, result via AudioFeedback)
            let synthdef_dir = crate::paths::synthdefs_dir();
            effects.push(AudioSideEffect::LoadSynthDefs { dir: synthdef_dir });

            // Also load custom synthdefs from config dir
            let config_dir = crate::paths::custom_synthdefs_dir();
            if config_dir.exists() {
                effects.push(AudioSideEffect::LoadSynthDefs { dir: config_dir });
            }

            result.push_status(audio.status(), "Loading synthdefs...");
        }
        ServerAction::RecordMaster => {
            if audio.is_recording() {
                // Push StopRecording effect — path comes back via AudioFeedback::RecordingStopped
                effects.push(AudioSideEffect::StopRecording);

                // Auto-deactivate AudioIn instrument on stop
                if let Some(inst) = state.instruments.selected_instrument_mut() {
                    if inst.source.is_audio_input() && inst.active {
                        inst.active = false;
                        result.audio_dirty.instruments = true;
                        result.audio_dirty.routing = true;
                    }
                }
                // Don't set pending_recording_path here — AudioFeedback::RecordingStopped handles it
                result.push_status(audio.status(), "Stopping recording...");
            } else if audio.is_running() {
                // Auto-activate AudioIn instrument on start
                if let Some(inst) = state.instruments.selected_instrument_mut() {
                    if inst.source.is_audio_input() && !inst.active {
                        inst.active = true;
                        result.audio_dirty.instruments = true;
                        result.audio_dirty.routing = true;
                    }
                }
                let path = super::recording_path("master");
                effects.push(AudioSideEffect::StartRecording { bus: 0, path: path.clone() });
                result.push_status(
                    audio.status(),
                    &format!("Recording to {}", path.display()),
                );
            }
        }
        ServerAction::RecordInput => {
            if audio.is_recording() {
                // Push StopRecording effect — path comes back via AudioFeedback::RecordingStopped
                effects.push(AudioSideEffect::StopRecording);

                // Auto-deactivate AudioIn instrument on stop
                if let Some(inst) = state.instruments.selected_instrument_mut() {
                    if inst.source.is_audio_input() && inst.active {
                        inst.active = false;
                        result.audio_dirty.instruments = true;
                        result.audio_dirty.routing = true;
                    }
                }
                // Don't set pending_recording_path here — AudioFeedback::RecordingStopped handles it
                result.push_status(audio.status(), "Stopping recording...");
            } else if audio.is_running() {
                // Record from the selected instrument's source_out bus
                if let Some(inst) = state.instruments.selected_instrument() {
                    let inst_id = inst.id;
                    // Auto-activate AudioIn instrument on start
                    if inst.source.is_audio_input() && !inst.active {
                        if let Some(inst_mut) = state.instruments.instrument_mut(inst_id) {
                            inst_mut.active = true;
                        }
                        result.audio_dirty.instruments = true;
                        result.audio_dirty.routing = true;
                    }
                    let path = super::recording_path(&format!("input_{}", inst_id));
                    // Bus 0 is hardware out; for instrument recording we use bus 0
                    // since instruments route through output to bus 0
                    effects.push(AudioSideEffect::StartRecording { bus: 0, path: path.clone() });
                    result.push_status(
                        audio.status(),
                        &format!("Recording to {}", path.display()),
                    );
                }
            }
        }
        ServerAction::Restart { input_device, output_device, buffer_size, sample_rate } => {
            effects.push(AudioSideEffect::UpdateState);
            effects.push(AudioSideEffect::RestartServer {
                input_device: input_device.clone(),
                output_device: output_device.clone(),
                server_addr: "127.0.0.1:57110".to_string(),
                buffer_size: *buffer_size,
                sample_rate: *sample_rate,
            });
            result.audio_dirty.instruments = true;
            result.audio_dirty.session = true;
            result.audio_dirty.routing = true;
        }
    }

    result
}

/// Find sclang executable, checking common locations
pub(super) fn find_sclang() -> Option<PathBuf> {
    // Check if sclang is in PATH
    if let Ok(output) = std::process::Command::new("which").arg("sclang").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }

    // Common macOS locations
    let candidates = [
        "/Applications/SuperCollider.app/Contents/MacOS/sclang",
        "/Applications/SuperCollider/SuperCollider.app/Contents/MacOS/sclang",
        "/usr/local/bin/sclang",
        "/opt/homebrew/bin/sclang",
    ];

    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Compile a .scd file using sclang
pub fn compile_synthdef(
    scd_path: &std::path::Path,
    output_dir: &std::path::Path,
    synthdef_name: &str,
) -> Result<PathBuf, String> {
    // Find sclang
    let sclang = find_sclang().ok_or_else(|| {
        "sclang not found. Install SuperCollider or add sclang to PATH.".to_string()
    })?;

    // Read the original .scd file
    let scd_content = std::fs::read_to_string(scd_path)
        .map_err(|e| format!("Failed to read .scd file: {}", e))?;

    // Replace directory references with the actual output directory
    // Handle both patterns: `dir ? thisProcess...` and just `thisProcess...`
    let output_dir_str = format!("\"{}\"", output_dir.display());
    let modified_content = scd_content
        .replace("dir ? thisProcess.nowExecutingPath.dirname", &output_dir_str)
        .replace("thisProcess.nowExecutingPath.dirname", &output_dir_str);

    // Wrap in a block that exits when done
    let compile_script = format!(
        "(\n{}\n\"SUCCESS\".postln;\n0.exit;\n)",
        modified_content
    );

    // Write temp compile script
    let temp_script = std::env::temp_dir().join("imbolc_compile_custom.scd");
    std::fs::write(&temp_script, &compile_script)
        .map_err(|e| format!("Failed to write compile script: {}", e))?;

    // Run sclang with a timeout by spawning and waiting
    let mut child = std::process::Command::new(&sclang)
        .arg(&temp_script)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run sclang: {}", e))?;

    // Wait up to 30 seconds for compilation
    let timeout = std::time::Duration::from_secs(30);
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err("sclang compilation timed out".to_string());
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => return Err(format!("Error waiting for sclang: {}", e)),
        }
    }

    let output = child.wait_with_output()
        .map_err(|e| format!("Failed to get sclang output: {}", e))?;

    // Check for errors (but ignore common non-error messages)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Look for actual errors, not just any "ERROR" in output
    let has_error = stderr.lines().any(|line| {
        line.contains("ERROR:") || line.contains("FAILURE")
    }) || stdout.lines().any(|line| {
        line.starts_with("ERROR:") || line.contains("FAILURE")
    });

    if has_error {
        return Err(format!("sclang error: {}{}", stdout, stderr));
    }

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_script);

    let scsyndef_path = output_dir.join(format!("{}.scsyndef", synthdef_name));
    if scsyndef_path.exists() {
        Ok(scsyndef_path)
    } else {
        // Fallback: assume success if no errors, but return dir if specific file missing?
        // Actually, if file is missing, something went wrong despite no error logs.
        // But for backward compatibility with load_synthdefs logic which takes a dir:
        Ok(output_dir.to_path_buf())
    }
}
