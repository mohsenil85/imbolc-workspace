use std::collections::hash_map::Entry;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::InstrumentId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub tick: u32,
    pub duration: u32,
    pub pitch: u8,
    pub velocity: u8,
    pub probability: f32, // 0.0-1.0, default 1.0 (always play)
}

/// A note stored with position relative to the selection anchor.
/// anchor = (min_tick of selected notes, min_pitch of selected notes)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipboardNote {
    pub tick_offset: u32,  // tick - anchor_tick
    pub pitch_offset: i16, // pitch as i16 - anchor_pitch as i16
    pub duration: u32,
    pub velocity: u8,
    pub probability: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub module_id: InstrumentId,
    pub notes: Vec<Note>,
    pub polyphonic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PianoRollState {
    pub tracks: HashMap<InstrumentId, Track>,
    pub track_order: Vec<InstrumentId>,
    pub bpm: f32,
    pub time_signature: (u8, u8),
    #[serde(skip)]
    pub playing: bool,
    pub looping: bool,
    pub loop_start: u32,
    pub loop_end: u32,
    #[serde(skip)]
    pub playhead: u32,
    pub ticks_per_beat: u32,
    /// Whether note input from piano keys should be recorded to the current track
    #[serde(skip)]
    pub recording: bool,
    /// Swing amount: 0.0 = no swing, 1.0 = max swing (delays offbeat notes)
    pub swing_amount: f32,
}

impl PianoRollState {
    pub fn new() -> Self {
        Self {
            tracks: HashMap::new(),
            track_order: Vec::new(),
            bpm: 120.0,
            time_signature: (4, 4),
            playing: false,
            looping: true,
            loop_start: 0,
            loop_end: 480 * 16, // 4 bars in 4/4
            playhead: 0,
            ticks_per_beat: 480,
            recording: false,
            swing_amount: 0.0,
        }
    }

    pub fn add_track(&mut self, instrument_id: InstrumentId) {
        if let Entry::Vacant(e) = self.tracks.entry(instrument_id) {
            e.insert(Track {
                module_id: instrument_id,
                notes: Vec::new(),
                polyphonic: true,
            });
            self.track_order.push(instrument_id);
        }
    }

    pub fn remove_track(&mut self, instrument_id: InstrumentId) {
        self.tracks.remove(&instrument_id);
        self.track_order.retain(|&id| id != instrument_id);
    }

    /// Get the track at the given index in track_order
    pub fn track_at(&self, index: usize) -> Option<&Track> {
        self.track_order
            .get(index)
            .and_then(|id| self.tracks.get(id))
    }

    /// Get a mutable track at the given index
    pub fn track_at_mut(&mut self, index: usize) -> Option<&mut Track> {
        let id = self.track_order.get(index).copied();
        id.and_then(move |id| self.tracks.get_mut(&id))
    }

    /// Toggle a note at the given position. If a note exists there, remove it; otherwise add one.
    pub fn toggle_note(
        &mut self,
        track_index: usize,
        pitch: u8,
        tick: u32,
        duration: u32,
        velocity: u8,
    ) {
        if let Some(track) = self.track_at_mut(track_index) {
            // Check if a note exists at this pitch/tick
            if let Some(pos) = track
                .notes
                .iter()
                .position(|n| n.pitch == pitch && n.tick == tick)
            {
                track.notes.remove(pos);
            } else {
                let insert_pos = track.notes.partition_point(|n| n.tick < tick);
                track.notes.insert(
                    insert_pos,
                    Note {
                        tick,
                        duration,
                        pitch,
                        velocity,
                        probability: 1.0,
                    },
                );
            }
        }
    }

    /// Find a note at the given pitch and tick (exact match on tick start)
    #[allow(dead_code)]
    pub fn find_note(&self, track_index: usize, pitch: u8, tick: u32) -> Option<&Note> {
        self.track_at(track_index).and_then(|track| {
            track
                .notes
                .iter()
                .find(|n| n.pitch == pitch && n.tick == tick)
        })
    }

    /// Find notes that start within a tick range (for playback)
    #[allow(dead_code)]
    pub fn notes_in_range(&self, track_index: usize, start_tick: u32, end_tick: u32) -> Vec<&Note> {
        if let Some(track) = self.track_at(track_index) {
            track
                .notes
                .iter()
                .filter(|n| n.tick >= start_tick && n.tick < end_tick)
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Advance playhead by a number of ticks, handling loop wrapping
    pub fn advance(&mut self, ticks: u32) {
        if !self.playing {
            return;
        }
        self.playhead += ticks;
        if self.looping && self.playhead >= self.loop_end {
            self.playhead = self.loop_start + (self.playhead - self.loop_end);
        }
    }

    /// Convert a beat number to ticks
    #[allow(dead_code)]
    pub fn beat_to_tick(&self, beat: u32) -> u32 {
        beat * self.ticks_per_beat
    }

    /// Convert ticks to beat number (float)
    pub fn tick_to_beat(&self, tick: u32) -> f32 {
        tick as f32 / self.ticks_per_beat as f32
    }

    /// Total ticks per bar
    pub fn ticks_per_bar(&self) -> u32 {
        self.ticks_per_beat * self.time_signature.0 as u32
    }
}

impl Default for PianoRollState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InstrumentId;

    #[test]
    fn toggle_note_adds_and_removes() {
        let mut pr = PianoRollState::new();
        pr.add_track(InstrumentId::new(1));
        pr.toggle_note(0, 60, 0, 480, 100);
        assert_eq!(pr.track_at(0).unwrap().notes.len(), 1);
        pr.toggle_note(0, 60, 0, 480, 100);
        assert!(pr.track_at(0).unwrap().notes.is_empty());
    }

    #[test]
    fn notes_in_range_filters_by_tick() {
        let mut pr = PianoRollState::new();
        pr.add_track(InstrumentId::new(1));
        pr.toggle_note(0, 60, 0, 480, 100);
        pr.toggle_note(0, 61, 480, 480, 100);
        let notes = pr.notes_in_range(0, 0, 480);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].pitch, 60);
    }

    #[test]
    fn advance_wraps_when_looping() {
        let mut pr = PianoRollState::new();
        pr.playing = true;
        pr.looping = true;
        pr.loop_start = 480;
        pr.loop_end = 960;
        pr.playhead = 900;
        pr.advance(100);
        assert_eq!(pr.playhead, 520);
    }

    #[test]
    fn ticks_per_bar_respects_time_signature() {
        let mut pr = PianoRollState::new();
        pr.time_signature = (3, 4);
        assert_eq!(pr.ticks_per_bar(), pr.ticks_per_beat * 3);
    }

    #[test]
    fn beat_to_tick_uses_ticks_per_beat() {
        let pr = PianoRollState::new();
        assert_eq!(pr.beat_to_tick(2), pr.ticks_per_beat * 2);
    }

    #[test]
    fn notes_stay_sorted_after_toggle() {
        let mut pr = PianoRollState::new();
        pr.add_track(InstrumentId::new(1));
        pr.toggle_note(0, 60, 480, 480, 100);
        pr.toggle_note(0, 62, 0, 480, 100);
        pr.toggle_note(0, 64, 240, 480, 100);
        let track = pr.track_at(0).unwrap();
        let ticks: Vec<u32> = track.notes.iter().map(|n| n.tick).collect();
        assert_eq!(ticks, vec![0, 240, 480]);
    }
}
