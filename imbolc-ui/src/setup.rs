use crate::audio::devices;
use crate::audio::{self, AudioHandle};
use crate::ui::StatusEvent;

/// Auto-start SuperCollider server, connect, and load synthdefs.
/// Returns status events for the UI layer to forward to the server pane.
pub fn auto_start_sc(
    audio: &mut AudioHandle,
) -> Vec<StatusEvent> {
    if std::env::var("IMBOLC_NO_AUDIO").is_ok() {
        return Vec::new();
    }

    let mut events = Vec::new();

    // Load saved device preferences
    let config = devices::load_device_config();

    match audio.start_server_with_devices(
        config.input_device.as_deref(),
        config.output_device.as_deref(),
    ) {
        Ok(()) => {
            events.push(StatusEvent {
                status: audio::ServerStatus::Running,
                message: "Server started".to_string(),
                server_running: Some(true),
            });
            match audio.connect("127.0.0.1:57110") {
                Ok(()) => {
                    events.push(StatusEvent {
                        status: audio::ServerStatus::Connected,
                        message: "Connected".to_string(),
                        server_running: None,
                    });
                    // Wait for scsynth to finish processing /d_loadDir
                    std::thread::sleep(std::time::Duration::from_millis(200));
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
