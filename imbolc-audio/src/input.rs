//! Audio input capture using cpal.
//!
//! Provides real-time audio input capture for multi-track recording.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, Stream, StreamConfig};

use imbolc_types::InstrumentId;

/// Ring buffer for audio capture
pub struct RingBuffer {
    buffer: Vec<f32>,
    write_pos: usize,
    capacity: usize,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            write_pos: 0,
            capacity,
        }
    }

    /// Write samples to the ring buffer
    pub fn write(&mut self, samples: &[f32]) {
        for &sample in samples {
            self.buffer[self.write_pos] = sample;
            self.write_pos = (self.write_pos + 1) % self.capacity;
        }
    }

    /// Drain all samples and return them
    pub fn drain(&mut self) -> Vec<f32> {
        let result = self.buffer.clone();
        self.buffer.fill(0.0);
        self.write_pos = 0;
        result
    }

    /// Get recent samples for waveform preview (returns normalized peaks)
    pub fn get_peaks(&self, num_peaks: usize) -> Vec<f32> {
        if num_peaks == 0 || self.buffer.is_empty() {
            return vec![];
        }
        let chunk_size = self.capacity / num_peaks;
        if chunk_size == 0 {
            return vec![0.0; num_peaks];
        }
        let mut peaks = Vec::with_capacity(num_peaks);
        for i in 0..num_peaks {
            let start = i * chunk_size;
            let end = ((i + 1) * chunk_size).min(self.capacity);
            let peak = self.buffer[start..end]
                .iter()
                .map(|s| s.abs())
                .fold(0.0_f32, f32::max);
            peaks.push(peak);
        }
        peaks
    }
}

/// State for an active track recording
pub struct TrackRecording {
    pub instrument_id: InstrumentId,
    pub path: PathBuf,
    pub samples: Vec<f32>,
    pub start_tick: u32,
    pub sample_rate: u32,
    pub channels: u16,
    pub started_at: Instant,
}

impl TrackRecording {
    pub fn new(
        instrument_id: InstrumentId,
        path: PathBuf,
        start_tick: u32,
        sample_rate: u32,
        channels: u16,
    ) -> Self {
        Self {
            instrument_id,
            path,
            samples: Vec::new(),
            start_tick,
            sample_rate,
            channels,
            started_at: Instant::now(),
        }
    }

    /// Calculate duration in ticks based on samples collected and BPM
    pub fn duration_ticks(&self, bpm: f32, ticks_per_beat: u32) -> u32 {
        if self.samples.is_empty() || self.sample_rate == 0 {
            return 0;
        }
        let duration_secs = self.samples.len() as f32 / (self.sample_rate as f32 * self.channels as f32);
        let beats = duration_secs * (bpm / 60.0);
        (beats * ticks_per_beat as f32) as u32
    }
}

/// Manages audio input capture via cpal
pub struct AudioInputManager {
    host: Host,
    device: Option<Device>,
    stream: Option<Stream>,
    ring_buffer: Arc<Mutex<RingBuffer>>,
    sample_rate: u32,
    channels: u16,
    active_recordings: Vec<TrackRecording>,
}

impl AudioInputManager {
    /// Create a new audio input manager
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host.default_input_device();

        let (sample_rate, channels) = if let Some(ref dev) = device {
            let config = dev
                .default_input_config()
                .map_err(|e| format!("Failed to get input config: {}", e))?;
            (config.sample_rate().0, config.channels())
        } else {
            (44100, 2)
        };

        // 10 seconds buffer at sample rate
        let buffer_size = (sample_rate as usize) * (channels as usize) * 10;

        Ok(Self {
            host,
            device,
            stream: None,
            ring_buffer: Arc::new(Mutex::new(RingBuffer::new(buffer_size))),
            sample_rate,
            channels,
            active_recordings: Vec::new(),
        })
    }

    /// Check if audio input is available
    pub fn has_input_device(&self) -> bool {
        self.device.is_some()
    }

    /// Get available input devices
    pub fn available_devices(&self) -> Vec<String> {
        self.host
            .input_devices()
            .map(|devices| {
                devices
                    .filter_map(|d| d.name().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Start capturing audio from the default input device
    pub fn start_capture(&mut self) -> Result<(), String> {
        if self.stream.is_some() {
            return Ok(()); // Already capturing
        }

        let device = self
            .device
            .as_ref()
            .ok_or_else(|| "No input device available".to_string())?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get input config: {}", e))?;

        let ring_buffer = Arc::clone(&self.ring_buffer);
        let channels = config.channels();
        let sample_rate = config.sample_rate().0;
        let stream_config: StreamConfig = config.into();

        let stream = device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut buf) = ring_buffer.lock() {
                        buf.write(data);
                    }
                },
                |err| {
                    log::error!("Audio input error: {}", err);
                },
                None,
            )
            .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start input stream: {}", e))?;

        self.stream = Some(stream);
        self.channels = channels;
        self.sample_rate = sample_rate;

        Ok(())
    }

    /// Stop capturing audio
    pub fn stop_capture(&mut self) {
        self.stream = None;
    }

    /// Check if currently capturing
    pub fn is_capturing(&self) -> bool {
        self.stream.is_some()
    }

    /// Start recording to a track
    pub fn start_track_recording(
        &mut self,
        instrument_id: InstrumentId,
        path: PathBuf,
        start_tick: u32,
    ) -> Result<(), String> {
        // Start capture if not already running
        if !self.is_capturing() {
            self.start_capture()?;
        }

        // Create new recording state
        let recording = TrackRecording::new(
            instrument_id,
            path,
            start_tick,
            self.sample_rate,
            self.channels,
        );

        self.active_recordings.push(recording);
        Ok(())
    }

    /// Stop recording for a specific track and write to file
    pub fn stop_track_recording(&mut self, instrument_id: InstrumentId) -> Option<PathBuf> {
        let idx = self
            .active_recordings
            .iter()
            .position(|r| r.instrument_id == instrument_id)?;

        let mut recording = self.active_recordings.remove(idx);

        // Drain samples from ring buffer to recording
        if let Ok(mut buf) = self.ring_buffer.lock() {
            recording.samples.extend(buf.drain());
        }

        // Write to WAV file
        if let Err(e) = self.write_wav(&recording.path, &recording.samples) {
            log::error!("Failed to write WAV: {}", e);
            return None;
        }

        // Stop capture if no more active recordings
        if self.active_recordings.is_empty() {
            self.stop_capture();
        }

        Some(recording.path)
    }

    /// Write samples to a WAV file
    fn write_wav(&self, path: &Path, samples: &[f32]) -> Result<(), String> {
        let spec = hound::WavSpec {
            channels: self.channels,
            sample_rate: self.sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = hound::WavWriter::create(path, spec)
            .map_err(|e| format!("Failed to create WAV writer: {}", e))?;

        for &sample in samples {
            writer
                .write_sample(sample)
                .map_err(|e| format!("Failed to write sample: {}", e))?;
        }

        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize WAV: {}", e))?;

        Ok(())
    }

    /// Get waveform peaks for preview (useful for track display)
    pub fn get_peaks(&self, num_peaks: usize) -> Vec<f32> {
        if let Ok(buf) = self.ring_buffer.lock() {
            buf.get_peaks(num_peaks)
        } else {
            vec![]
        }
    }

    /// Check if a track is currently recording
    pub fn is_track_recording(&self, instrument_id: InstrumentId) -> bool {
        self.active_recordings
            .iter()
            .any(|r| r.instrument_id == instrument_id)
    }

    /// Get the current sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get the number of channels
    pub fn channels(&self) -> u16 {
        self.channels
    }
}

impl Default for AudioInputManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            host: cpal::default_host(),
            device: None,
            stream: None,
            ring_buffer: Arc::new(Mutex::new(RingBuffer::new(0))),
            sample_rate: 44100,
            channels: 2,
            active_recordings: Vec::new(),
        })
    }
}
