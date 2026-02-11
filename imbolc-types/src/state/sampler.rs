//! Sampler types for pitched sample playback.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

pub type BufferId = u32;
pub type SliceId = u32;

/// A loaded sample buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleBuffer {
    pub id: BufferId,
    pub path: String,
    pub name: String,
    pub num_frames: u32,
    pub sample_rate: u32,
    pub num_channels: u8,
    pub duration_secs: f32,
    /// SuperCollider buffer number (assigned when loaded)
    pub sc_bufnum: Option<i32>,
}

impl SampleBuffer {
    pub fn new(id: BufferId, path: String, name: String) -> Self {
        Self {
            id,
            path,
            name,
            num_frames: 0,
            sample_rate: 44100,
            num_channels: 2,
            duration_secs: 0.0,
            sc_bufnum: None,
        }
    }

    /// Update buffer info after loading
    pub fn set_info(&mut self, num_frames: u32, sample_rate: u32, num_channels: u8) {
        self.num_frames = num_frames;
        self.sample_rate = sample_rate;
        self.num_channels = num_channels;
        self.duration_secs = num_frames as f32 / sample_rate as f32;
    }
}

/// A slice within a sample buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slice {
    pub id: SliceId,
    /// Start position as a fraction of the buffer (0.0-1.0)
    pub start: f32,
    /// End position as a fraction of the buffer (0.0-1.0)
    pub end: f32,
    pub name: String,
    /// MIDI note this slice maps to (for chromatic/mapped mode)
    pub root_note: u8,
}

impl Slice {
    pub fn new(id: SliceId, start: f32, end: f32) -> Self {
        Self {
            id,
            start: start.clamp(0.0, 1.0),
            end: end.clamp(0.0, 1.0),
            name: format!("Slice {}", id),
            root_note: 60, // Middle C
        }
    }

    /// Create a full-buffer slice
    pub fn full(id: SliceId) -> Self {
        Self::new(id, 0.0, 1.0)
    }

    /// Duration as a fraction of the buffer
    pub fn duration(&self) -> f32 {
        (self.end - self.start).abs()
    }
}

/// Sampler configuration for an instrument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplerConfig {
    pub buffer_id: Option<BufferId>,
    pub sample_name: Option<String>,
    pub slices: Vec<Slice>,
    pub selected_slice: usize,
    pub loop_mode: bool,
    /// Whether to change playback rate based on MIDI note (pitch tracking)
    pub pitch_tracking: bool,
    /// Next slice ID for auto-increment
    pub next_slice_id: SliceId,
}

impl SamplerConfig {
    pub fn new() -> Self {
        // Create a default full-buffer slice
        let mut config = Self {
            buffer_id: None,
            sample_name: None,
            slices: Vec::new(),
            selected_slice: 0,
            loop_mode: false,
            pitch_tracking: true,
            next_slice_id: 0,
        };
        // Add initial full-buffer slice
        config.add_slice(0.0, 1.0);
        config
    }

    pub fn add_slice(&mut self, start: f32, end: f32) -> SliceId {
        let id = self.next_slice_id;
        self.next_slice_id += 1;
        self.slices.push(Slice::new(id, start, end));
        id
    }

    pub fn remove_slice(&mut self, id: SliceId) {
        if let Some(pos) = self.slices.iter().position(|s| s.id == id) {
            self.slices.remove(pos);
            // Adjust selected slice if needed
            if self.selected_slice >= self.slices.len() && !self.slices.is_empty() {
                self.selected_slice = self.slices.len() - 1;
            }
        }
    }

    pub fn selected_slice(&self) -> Option<&Slice> {
        self.slices.get(self.selected_slice)
    }

    pub fn selected_slice_mut(&mut self) -> Option<&mut Slice> {
        self.slices.get_mut(self.selected_slice)
    }

    pub fn select_next_slice(&mut self) {
        if !self.slices.is_empty() {
            self.selected_slice = (self.selected_slice + 1) % self.slices.len();
        }
    }

    pub fn select_prev_slice(&mut self) {
        if !self.slices.is_empty() {
            self.selected_slice = if self.selected_slice == 0 {
                self.slices.len() - 1
            } else {
                self.selected_slice - 1
            };
        }
    }

    /// Get the next slice ID (for persistence)
    pub fn next_slice_id(&self) -> SliceId {
        self.next_slice_id
    }

    /// Set the next slice ID (for persistence)
    pub fn set_next_slice_id(&mut self, id: SliceId) {
        self.next_slice_id = id;
    }

    /// Find which slice to play for a given MIDI note (in mapped mode)
    pub fn slice_for_note(&self, note: u8) -> Option<&Slice> {
        // Simple mapping: notes 0-127 map to slices modulo slice count
        if self.slices.is_empty() {
            return None;
        }
        // Find slice with matching root_note, or fall back to modulo mapping
        self.slices
            .iter()
            .find(|s| s.root_note == note)
            .or_else(|| self.slices.get(note as usize % self.slices.len()))
    }
}

impl Default for SamplerConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Global sample buffer registry
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SampleRegistry {
    pub buffers: Vec<SampleBuffer>,
    pub next_buffer_id: BufferId,
}

impl SampleRegistry {
    pub fn new() -> Self {
        Self {
            buffers: Vec::new(),
            next_buffer_id: 0,
        }
    }

    pub fn add_buffer(&mut self, path: String, name: String) -> BufferId {
        let id = self.next_buffer_id;
        self.next_buffer_id += 1;
        self.buffers.push(SampleBuffer::new(id, path, name));
        id
    }

    pub fn remove_buffer(&mut self, id: BufferId) {
        self.buffers.retain(|b| b.id != id);
    }

    pub fn buffer(&self, id: BufferId) -> Option<&SampleBuffer> {
        self.buffers.iter().find(|b| b.id == id)
    }

    pub fn buffer_mut(&mut self, id: BufferId) -> Option<&mut SampleBuffer> {
        self.buffers.iter_mut().find(|b| b.id == id)
    }

    pub fn buffer_by_path(&self, path: &str) -> Option<&SampleBuffer> {
        self.buffers.iter().find(|b| b.path == path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_buffer() {
        let mut buf = SampleBuffer::new(0, "/path/to/sample.wav".to_string(), "sample".to_string());
        assert_eq!(buf.duration_secs, 0.0);

        buf.set_info(44100, 44100, 2);
        assert!((buf.duration_secs - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_slice() {
        let slice = Slice::new(0, 0.25, 0.75);
        assert!((slice.duration() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_sampler_config() {
        let mut config = SamplerConfig::new();
        assert_eq!(config.slices.len(), 1); // Default full slice

        let id = config.add_slice(0.0, 0.5);
        assert_eq!(config.slices.len(), 2);

        config.remove_slice(id);
        assert_eq!(config.slices.len(), 1);
    }

    #[test]
    fn test_sample_registry() {
        let mut registry = SampleRegistry::new();
        let id = registry.add_buffer("/path/sample.wav".to_string(), "sample".to_string());

        assert!(registry.buffer(id).is_some());
        assert!(registry.buffer_by_path("/path/sample.wav").is_some());

        registry.remove_buffer(id);
        assert!(registry.buffer(id).is_none());
    }
}
