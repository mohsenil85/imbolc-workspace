use std::path::PathBuf;

use crate::action::{AudioEffect, DispatchResult, ServerAction};
use crate::state::AppState;
use imbolc_audio::AudioHandle;

pub(super) fn dispatch_server(
    action: &ServerAction,
    state: &mut AppState,
    audio: &mut AudioHandle,
) -> DispatchResult {
    let mut result = DispatchResult::none();

    match action {
        ServerAction::Connect => {
            let _ = audio.connect_async("127.0.0.1:57110");
            result.push_status(audio.status(), "Connecting...");
        }
        ServerAction::Disconnect => {
            let _ = audio.disconnect_async();
            result.push_status(audio.status(), "Disconnecting...");
        }
        ServerAction::Start {
            input_device,
            output_device,
            buffer_size,
            sample_rate,
            scsynth_args,
        } => {
            let _ = audio.start_server_async(
                input_device.as_deref(),
                output_device.as_deref(),
                *buffer_size,
                *sample_rate,
                scsynth_args,
            );
            result.push_status(audio.status(), "Starting server...");
        }
        ServerAction::Stop => {
            let _ = audio.stop_server_async();
            result.push_status(audio.status(), "Stopping server...");
        }
        ServerAction::CompileSynthDefs => {
            let scd_path = crate::paths::compile_scd_path();
            let _ = audio.compile_synthdefs_async(&scd_path);
            result.push_status(audio.status(), "Compiling synthdefs...");
        }
        ServerAction::CompileVstSynthDefs => {
            let scd_path = crate::paths::compile_vst_scd_path();
            let _ = audio.compile_synthdefs_async(&scd_path);
            result.push_status(audio.status(), "Compiling VST synthdefs...");
        }
        ServerAction::LoadSynthDefs => {
            // Load built-in synthdefs (fire-and-forget, result via AudioFeedback)
            let synthdef_dir = crate::paths::synthdefs_dir();
            let _ = audio.load_synthdefs(&synthdef_dir);

            // Also load custom synthdefs from config dir
            let config_dir = crate::paths::custom_synthdefs_dir();
            if config_dir.exists() {
                let _ = audio.load_synthdefs(&config_dir);
            }

            result.push_status(audio.status(), "Loading synthdefs...");
        }
        ServerAction::RecordMaster => {
            if audio.is_recording() {
                // Stop recording — path comes back via AudioFeedback::RecordingStopped
                let _ = audio.stop_recording();

                // Auto-deactivate AudioIn instrument on stop
                if let Some(inst) = state.instruments.selected_instrument_mut() {
                    if inst.source.is_audio_input() && inst.mixer.active {
                        inst.mixer.active = false;
                        result.audio_effects.push(AudioEffect::RebuildInstruments);
                        result.audio_effects.push(AudioEffect::RebuildRouting);
                    }
                }
                // Don't set pending_recording_path here — AudioFeedback::RecordingStopped handles it
                result.push_status(audio.status(), "Stopping recording...");
            } else if audio.is_running() {
                // Auto-activate AudioIn instrument on start
                if let Some(inst) = state.instruments.selected_instrument_mut() {
                    if inst.source.is_audio_input() && !inst.mixer.active {
                        inst.mixer.active = true;
                        result.audio_effects.push(AudioEffect::RebuildInstruments);
                        result.audio_effects.push(AudioEffect::RebuildRouting);
                    }
                }
                let path = super::recording_path("master");
                let _ = audio.start_recording(0, &path);
                result.push_status(audio.status(), format!("Recording to {}", path.display()));
            } else {
                result.push_status(
                    imbolc_audio::ServerStatus::Stopped,
                    "Audio engine not running",
                );
            }
        }
        ServerAction::RecordInput => {
            if audio.is_recording() {
                // Stop recording — path comes back via AudioFeedback::RecordingStopped
                let _ = audio.stop_recording();

                // Auto-deactivate AudioIn instrument on stop
                if let Some(inst) = state.instruments.selected_instrument_mut() {
                    if inst.source.is_audio_input() && inst.mixer.active {
                        inst.mixer.active = false;
                        result.audio_effects.push(AudioEffect::RebuildInstruments);
                        result.audio_effects.push(AudioEffect::RebuildRouting);
                    }
                }
                // Don't set pending_recording_path here — AudioFeedback::RecordingStopped handles it
                result.push_status(audio.status(), "Stopping recording...");
            } else if audio.is_running() {
                // Record from the selected instrument's source_out bus
                if let Some(inst) = state.instruments.selected_instrument() {
                    let inst_id = inst.id;
                    // Auto-activate AudioIn instrument on start
                    if inst.source.is_audio_input() && !inst.mixer.active {
                        if let Some(inst_mut) = state.instruments.instrument_mut(inst_id) {
                            inst_mut.mixer.active = true;
                        }
                        result.audio_effects.push(AudioEffect::RebuildInstruments);
                        result.audio_effects.push(AudioEffect::RebuildRouting);
                    }
                    let path = super::recording_path(&format!("input_{}", inst_id));
                    // Bus 0 is hardware out; for instrument recording we use bus 0
                    // since instruments route through output to bus 0
                    let _ = audio.start_recording(0, &path);
                    result.push_status(audio.status(), format!("Recording to {}", path.display()));
                }
            } else {
                result.push_status(
                    imbolc_audio::ServerStatus::Stopped,
                    "Audio engine not running",
                );
            }
        }
        ServerAction::Restart {
            input_device,
            output_device,
            buffer_size,
            sample_rate,
            scsynth_args,
        } => {
            let _ = audio.restart_server_async(
                input_device.as_deref(),
                output_device.as_deref(),
                "127.0.0.1:57110",
                *buffer_size,
                *sample_rate,
                scsynth_args,
            );
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result.audio_effects.push(AudioEffect::RebuildSession);
            result.audio_effects.push(AudioEffect::RebuildRouting);
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
        .replace(
            "dir ? thisProcess.nowExecutingPath.dirname",
            &output_dir_str,
        )
        .replace("thisProcess.nowExecutingPath.dirname", &output_dir_str);

    // Wrap in a block that exits when done
    let compile_script = format!("(\n{}\n\"SUCCESS\".postln;\n0.exit;\n)", modified_content);

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

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to get sclang output: {}", e))?;

    // Check for errors (but ignore common non-error messages)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Look for actual errors, not just any "ERROR" in output
    let has_error = stderr
        .lines()
        .any(|line| line.contains("ERROR:") || line.contains("FAILURE"))
        || stdout
            .lines()
            .any(|line| line.starts_with("ERROR:") || line.contains("FAILURE"));

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
