use std::path::PathBuf;
use std::process::Command;
use serde::{Deserialize, Serialize};

/// Audio buffer size options for scsynth
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BufferSize {
    B64 = 64,
    B128 = 128,
    B256 = 256,
    #[default]
    B512 = 512,
    B1024 = 1024,
    B2048 = 2048,
}

impl BufferSize {
    pub const ALL: [BufferSize; 6] = [
        BufferSize::B64,
        BufferSize::B128,
        BufferSize::B256,
        BufferSize::B512,
        BufferSize::B1024,
        BufferSize::B2048,
    ];

    pub fn as_samples(&self) -> u32 {
        *self as u32
    }

    /// Calculate latency in milliseconds for a given sample rate
    pub fn latency_ms(&self, sample_rate: u32) -> f32 {
        (self.as_samples() as f32 / sample_rate as f32) * 1000.0
    }
}

/// An audio device discovered on the system
#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub name: String,
    pub input_channels: Option<u32>,
    pub output_channels: Option<u32>,
    pub sample_rate: Option<u32>,
    #[allow(dead_code)]
    pub is_default_input: bool,
    #[allow(dead_code)]
    pub is_default_output: bool,
}

/// User-selected device configuration
#[derive(Debug, Clone)]
pub struct AudioDeviceConfig {
    pub input_device: Option<String>,  // None = system default
    pub output_device: Option<String>, // None = system default
    pub buffer_size: BufferSize,       // default 512
    pub sample_rate: u32,              // default 44100
}

impl Default for AudioDeviceConfig {
    fn default() -> Self {
        Self {
            input_device: None,
            output_device: None,
            buffer_size: BufferSize::default(),
            sample_rate: 44100,
        }
    }
}

/// Enumerate audio devices via system_profiler (macOS)
pub fn enumerate_devices() -> Vec<AudioDevice> {
    let output = match Command::new("system_profiler")
        .args(["SPAudioDataType", "-json"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    if !output.status.success() {
        return Vec::new();
    }

    let json_str = match std::str::from_utf8(&output.stdout) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let parsed: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut devices = Vec::new();

    // system_profiler returns: { "SPAudioDataType": [ { "_items": [...] } ] }
    let items = parsed
        .get("SPAudioDataType")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("_items"))
        .and_then(|v| v.as_array());

    // If nested _items structure doesn't work, try flat array
    let items = items.or_else(|| {
        parsed
            .get("SPAudioDataType")
            .and_then(|v| v.as_array())
    });

    if let Some(items) = items {
        for item in items {
            let name = item
                .get("_name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            if name.is_empty() {
                continue;
            }

            let input_channels = item
                .get("coreaudio_input_source")
                .and_then(|v| v.as_str())
                .and_then(|s| {
                    // Format: "X Channels" or just a number
                    s.split_whitespace().next().and_then(|n| n.parse().ok())
                })
                .or_else(|| {
                    item.get("coreaudio_device_input")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32)
                });

            let output_channels = item
                .get("coreaudio_output_source")
                .and_then(|v| v.as_str())
                .and_then(|s| {
                    s.split_whitespace().next().and_then(|n| n.parse().ok())
                })
                .or_else(|| {
                    item.get("coreaudio_device_output")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32)
                });

            let sample_rate = item
                .get("coreaudio_device_srate")
                .and_then(|v| {
                    v.as_u64().map(|n| n as u32).or_else(|| {
                        v.as_str().and_then(|s| s.parse().ok())
                    })
                });

            let is_default_output = item
                .get("coreaudio_default_audio_output_device")
                .and_then(|v| v.as_str())
                .map(|v| v == "spaudio_yes")
                .unwrap_or(false);

            let is_default_input = item
                .get("coreaudio_default_audio_input_device")
                .and_then(|v| v.as_str())
                .map(|v| v == "spaudio_yes")
                .unwrap_or(false);

            devices.push(AudioDevice {
                name,
                input_channels,
                output_channels,
                sample_rate,
                is_default_input,
                is_default_output,
            });
        }
    }

    // Filter out devices that crash scsynth (iPhone/iPad continuity devices)
    devices.retain(|d| !is_blacklisted_device(&d.name));

    devices
}

/// Devices known to crash scsynth during audio initialization.
/// These are typically iOS continuity devices that expose incompatible
/// CoreAudio stream formats.
fn is_blacklisted_device(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("iphone") || lower.contains("ipad")
}

/// Resolve the default output and input device names.
/// Used to always pass explicit `-H` to scsynth so it never probes
/// problematic devices.
pub fn default_device_names() -> (Option<String>, Option<String>) {
    let devices = enumerate_devices();
    let output = devices.iter()
        .find(|d| d.is_default_output)
        .map(|d| d.name.clone());
    let input = devices.iter()
        .find(|d| d.is_default_input)
        .map(|d| d.name.clone());
    (output, input)
}

fn config_path() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home)
            .join(".config")
            .join("imbolc")
            .join("audio_devices.json")
    } else {
        PathBuf::from("audio_devices.json")
    }
}

/// Load device config from ~/.config/imbolc/audio_devices.json
pub fn load_device_config() -> AudioDeviceConfig {
    let path = config_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return AudioDeviceConfig::default(),
    };
    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return AudioDeviceConfig::default(),
    };

    // Parse buffer size with backward compatibility
    let buffer_size = parsed
        .get("buffer_size")
        .and_then(|v| v.as_u64())
        .and_then(|bs| match bs {
            64 => Some(BufferSize::B64),
            128 => Some(BufferSize::B128),
            256 => Some(BufferSize::B256),
            512 => Some(BufferSize::B512),
            1024 => Some(BufferSize::B1024),
            2048 => Some(BufferSize::B2048),
            _ => None,
        })
        .unwrap_or_default();

    let sample_rate = parsed
        .get("sample_rate")
        .and_then(|v| v.as_u64())
        .map(|sr| sr as u32)
        .unwrap_or(44100);

    AudioDeviceConfig {
        input_device: parsed
            .get("input_device")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        output_device: parsed
            .get("output_device")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        buffer_size,
        sample_rate,
    }
}

/// Save device config to ~/.config/imbolc/audio_devices.json
pub fn save_device_config(config: &AudioDeviceConfig) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let obj = serde_json::json!({
        "input_device": config.input_device,
        "output_device": config.output_device,
        "buffer_size": config.buffer_size.as_samples(),
        "sample_rate": config.sample_rate,
    });
    let _ = std::fs::write(&path, serde_json::to_string_pretty(&obj).unwrap_or_default());
}
