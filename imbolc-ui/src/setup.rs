use crate::audio::devices;
use crate::audio::{self, AudioEngine, AudioHandle};
use crate::ui::StatusEvent;

use std::net::UdpSocket;
use std::time::{Duration, Instant};

/// Wait for scsynth to become ready by polling with `/status` OSC messages.
/// Returns `Ok(())` once scsynth responds, or `Err` if it doesn't respond
/// within the timeout.
fn wait_for_server_ready(addr: &str, timeout: Duration) -> Result<(), String> {
    // Build a minimal OSC /status message using rosc
    let msg = rosc::OscPacket::Message(rosc::OscMessage {
        addr: "/status".to_string(),
        args: vec![],
    });
    let packet =
        rosc::encoder::encode(&msg).map_err(|e| format!("Failed to encode /status: {e}"))?;

    // Bind a temporary UDP socket on any available port
    let socket =
        UdpSocket::bind("127.0.0.1:0").map_err(|e| format!("Failed to bind UDP socket: {e}"))?;
    socket
        .set_read_timeout(Some(Duration::from_millis(200)))
        .map_err(|e| format!("Failed to set read timeout: {e}"))?;

    let start = Instant::now();
    let mut buf = [0u8; 1024];

    while start.elapsed() < timeout {
        // Send /status — if scsynth isn't listening yet, this is silently lost (UDP)
        let _ = socket.send_to(&packet, addr);

        // Wait for any reply (scsynth responds with /status.reply)
        match socket.recv_from(&mut buf) {
            Ok(_) => return Ok(()),
            Err(_) => {
                // Timeout or error — try again after a short sleep
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }

    Err(format!(
        "scsynth did not respond within {:.1}s",
        timeout.as_secs_f64()
    ))
}

/// Auto-start SuperCollider server, connect, and load synthdefs.
/// Returns status events for the UI layer to forward to the server pane.
pub fn auto_start_sc(audio: &mut AudioHandle) -> Vec<StatusEvent> {
    if std::env::var("IMBOLC_NO_AUDIO").is_ok() {
        return Vec::new();
    }

    let mut events = Vec::new();

    // Step 1: Check if synthdefs need compilation
    let scd_path = imbolc_core::paths::compile_scd_path();
    if scd_path.exists() && AudioEngine::synthdefs_need_compilation(&scd_path) {
        events.push(StatusEvent {
            status: audio::ServerStatus::Stopped,
            message: "Compiling synthdefs...".to_string(),
            server_running: None,
        });

        match audio.compile_synthdefs_sync(&scd_path) {
            Ok(msg) => {
                events.push(StatusEvent {
                    status: audio::ServerStatus::Stopped,
                    message: format!("Synthdefs compiled: {}", msg),
                    server_running: None,
                });
            }
            Err(e) => {
                events.push(StatusEvent {
                    status: audio::ServerStatus::Stopped,
                    message: format!("Synthdef compile warning: {}", e),
                    server_running: None,
                });
                // Continue anyway - may have partial synthdefs
            }
        }
    }

    // Step 2: Load saved device preferences and start server
    let config = devices::load_device_config();

    match audio.start_server_with_devices(
        config.input_device.as_deref(),
        config.output_device.as_deref(),
        config.buffer_size.as_samples(),
        config.sample_rate,
        &config.scsynth_args,
    ) {
        Ok(()) => {
            events.push(StatusEvent {
                status: audio::ServerStatus::Running,
                message: "Server started, waiting for scsynth...".to_string(),
                server_running: Some(true),
            });

            // Wait for scsynth to be ready before connecting
            let server_addr = "127.0.0.1:57110";
            match wait_for_server_ready(server_addr, Duration::from_secs(10)) {
                Ok(()) => {}
                Err(e) => {
                    events.push(StatusEvent {
                        status: audio::ServerStatus::Running,
                        message: format!("Warning: {e} — attempting connect anyway"),
                        server_running: None,
                    });
                }
            }

            match audio.connect(server_addr) {
                Ok(()) => {
                    events.push(StatusEvent {
                        status: audio::ServerStatus::Connected,
                        message: "Connected".to_string(),
                        server_running: None,
                    });
                    // Brief pause for scsynth to finish processing /d_loadDir
                    std::thread::sleep(Duration::from_millis(200));
                    // Rebuild routing
                    let _ = audio.rebuild_instrument_routing();
                }
                Err(e) => {
                    events.push(StatusEvent {
                        status: audio::ServerStatus::Running,
                        message: format!("Server running (connect failed: {})", e),
                        server_running: None,
                    });
                }
            }
        }
        Err(_e) => {
            // Server start failed — status remains Stopped
        }
    }

    events
}
