//! Instrument collection state.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::drum_sequencer::DrumSequencerState;
use super::instrument::{Instrument, SourceType};
use crate::InstrumentId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentState {
    pub instruments: Vec<Instrument>,
    pub selected: Option<usize>,
    pub next_id: InstrumentId,
    #[serde(default = "default_sampler_buffer_id")]
    pub next_sampler_buffer_id: u32,
    /// Set by dispatch when editing an instrument; read by InstrumentEditPane on_enter
    #[serde(skip)]
    pub editing_instrument_id: Option<InstrumentId>,
    /// Counter for allocating layer group IDs
    pub next_layer_group_id: u32,
    /// Index from InstrumentId → Vec position for O(1) lookups.
    #[serde(skip)]
    id_index: HashMap<InstrumentId, usize>,
}

impl InstrumentState {
    pub fn new() -> Self {
        Self {
            instruments: Vec::new(),
            selected: None,
            next_id: InstrumentId::new(0),
            next_sampler_buffer_id: 20000,
            editing_instrument_id: None,
            next_layer_group_id: 0,
            id_index: HashMap::new(),
        }
    }

    /// Rebuild the id → index lookup table from the Vec.
    ///
    /// Call after any operation that replaces `instruments` wholesale
    /// (e.g. undo/redo, persistence load, network state replacement).
    pub fn rebuild_index(&mut self) {
        self.id_index.clear();
        for (i, inst) in self.instruments.iter().enumerate() {
            self.id_index.insert(inst.id, i);
        }
    }

    pub fn add_instrument(&mut self, source: SourceType) -> InstrumentId {
        let id = self.next_id;
        self.next_id = InstrumentId::new(self.next_id.get() + 1);
        let instrument = Instrument::new(id, source);
        self.instruments.push(instrument);
        self.selected = Some(self.instruments.len() - 1);
        self.id_index.insert(id, self.instruments.len() - 1);

        id
    }

    pub fn remove_instrument(&mut self, id: InstrumentId) {
        // Capture layer group before removal for singleton cleanup
        let old_group = self.instrument(id).and_then(|i| i.layer_group);

        if let Some(pos) = self.instruments.iter().position(|s| s.id == id) {
            self.instruments.remove(pos);

            if let Some(sel) = self.selected {
                if sel >= self.instruments.len() {
                    self.selected = if self.instruments.is_empty() {
                        None
                    } else {
                        Some(self.instruments.len() - 1)
                    };
                }
            }

            // Rebuild index since positions shifted
            self.rebuild_index();
        }

        // If old group now has only 1 member, clear it (group of 1 is meaningless)
        if let Some(g) = old_group {
            let remaining: Vec<InstrumentId> = self
                .instruments
                .iter()
                .filter(|i| i.layer_group == Some(g))
                .map(|i| i.id)
                .collect();
            if remaining.len() == 1 {
                if let Some(inst) = self.instrument_mut(remaining[0]) {
                    inst.layer_group = None;
                }
            }
        }
    }

    pub fn instrument(&self, id: InstrumentId) -> Option<&Instrument> {
        // Use index for O(1) lookup, fall back to linear scan if index is stale
        if let Some(&idx) = self.id_index.get(&id) {
            if let Some(inst) = self.instruments.get(idx) {
                if inst.id == id {
                    return Some(inst);
                }
            }
        }
        self.instruments.iter().find(|s| s.id == id)
    }

    pub fn instrument_mut(&mut self, id: InstrumentId) -> Option<&mut Instrument> {
        // Use index for O(1) lookup, fall back to linear scan if index is stale
        if let Some(&idx) = self.id_index.get(&id) {
            if let Some(inst) = self.instruments.get(idx) {
                if inst.id == id {
                    return self.instruments.get_mut(idx);
                }
            }
        }
        self.instruments.iter_mut().find(|s| s.id == id)
    }

    pub fn selected_instrument(&self) -> Option<&Instrument> {
        self.selected.and_then(|idx| self.instruments.get(idx))
    }

    #[allow(dead_code)]
    pub fn selected_instrument_mut(&mut self) -> Option<&mut Instrument> {
        self.selected.and_then(|idx| self.instruments.get_mut(idx))
    }

    pub fn select_next(&mut self) {
        if self.instruments.is_empty() {
            self.selected = None;
            return;
        }
        self.selected = match self.selected {
            None => Some(0),
            Some(idx) if idx < self.instruments.len() - 1 => Some(idx + 1),
            Some(idx) => Some(idx),
        };
    }

    pub fn select_prev(&mut self) {
        if self.instruments.is_empty() {
            self.selected = None;
            return;
        }
        self.selected = match self.selected {
            None => Some(0),
            Some(0) => Some(0),
            Some(idx) => Some(idx - 1),
        };
    }

    /// Check if any instrument is soloed
    pub fn any_instrument_solo(&self) -> bool {
        self.instruments.iter().any(|s| s.solo)
    }

    pub fn selected_drum_sequencer(&self) -> Option<&DrumSequencerState> {
        self.selected_instrument()
            .and_then(|s| s.drum_sequencer.as_ref())
    }

    pub fn selected_drum_sequencer_mut(&mut self) -> Option<&mut DrumSequencerState> {
        self.selected
            .and_then(|idx| self.instruments.get_mut(idx))
            .and_then(|s| s.drum_sequencer.as_mut())
    }

    /// Allocate a new unique layer group ID
    pub fn next_layer_group(&mut self) -> u32 {
        let id = self.next_layer_group_id;
        self.next_layer_group_id += 1;
        id
    }

    /// Returns sorted unique group IDs from instruments that have a layer_group set.
    pub fn active_layer_groups(&self) -> Vec<u32> {
        let mut groups: Vec<u32> = self
            .instruments
            .iter()
            .filter_map(|i| i.layer_group)
            .collect();
        groups.sort_unstable();
        groups.dedup();
        groups
    }

    /// Returns all instrument IDs in the same layer group as `id` (including `id` itself).
    /// If the instrument has no layer group, returns just `vec![id]`.
    pub fn layer_group_members(&self, id: InstrumentId) -> Vec<InstrumentId> {
        let group = self.instrument(id).and_then(|inst| inst.layer_group);
        match group {
            Some(g) => self
                .instruments
                .iter()
                .filter(|inst| inst.layer_group == Some(g))
                .map(|inst| inst.id)
                .collect(),
            None => vec![id],
        }
    }
}

impl Default for InstrumentState {
    fn default() -> Self {
        Self::new()
    }
}

fn default_sampler_buffer_id() -> u32 {
    20000
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instrument_state_creation() {
        let state = InstrumentState::new();
        assert_eq!(state.instruments.len(), 0);
        assert_eq!(state.selected, None);
    }

    #[test]
    fn test_add_instrument() {
        let mut state = InstrumentState::new();
        let id1 = state.add_instrument(SourceType::Saw);
        let id2 = state.add_instrument(SourceType::Sin);

        assert_eq!(state.instruments.len(), 2);
        assert_eq!(state.instruments[0].id, id1);
        assert_eq!(state.instruments[1].id, id2);
        assert_eq!(state.selected, Some(1)); // selects newly added
    }

    #[test]
    fn test_remove_instrument() {
        let mut state = InstrumentState::new();
        let id1 = state.add_instrument(SourceType::Saw);
        let id2 = state.add_instrument(SourceType::Sin);
        let _id3 = state.add_instrument(SourceType::Sqr);

        state.remove_instrument(id2);

        assert_eq!(state.instruments.len(), 2);
        assert_eq!(state.instruments[0].id, id1);
    }

    #[test]
    fn test_remove_last_instrument() {
        let mut state = InstrumentState::new();
        let id1 = state.add_instrument(SourceType::Saw);
        let id2 = state.add_instrument(SourceType::Sin);

        state.selected = Some(1);
        state.remove_instrument(id2);

        assert_eq!(state.selected, Some(0));
        assert_eq!(state.instruments[0].id, id1);
    }

    #[test]
    fn test_remove_all_instruments() {
        let mut state = InstrumentState::new();
        let id1 = state.add_instrument(SourceType::Saw);

        state.remove_instrument(id1);
        assert_eq!(state.selected, None);
        assert!(state.instruments.is_empty());
    }

    #[test]
    fn test_select_navigation() {
        let mut state = InstrumentState::new();
        state.add_instrument(SourceType::Saw);
        state.add_instrument(SourceType::Sin);
        state.add_instrument(SourceType::Sqr);

        assert_eq!(state.selected, Some(2)); // selects last added
        state.select_prev();
        assert_eq!(state.selected, Some(1));
        state.select_prev();
        assert_eq!(state.selected, Some(0));
        state.select_prev();
        assert_eq!(state.selected, Some(0)); // stay at start
        state.select_next();
        assert_eq!(state.selected, Some(1));
        state.select_next();
        assert_eq!(state.selected, Some(2));
        state.select_next();
        assert_eq!(state.selected, Some(2)); // stay at end
    }

    #[test]
    fn layer_group_members_returns_all_in_group() {
        let mut state = InstrumentState::new();
        let id1 = state.add_instrument(SourceType::Saw);
        let id2 = state.add_instrument(SourceType::Sin);
        let _id3 = state.add_instrument(SourceType::Sqr);

        let group = state.next_layer_group();
        state.instrument_mut(id1).unwrap().layer_group = Some(group);
        state.instrument_mut(id2).unwrap().layer_group = Some(group);

        let members = state.layer_group_members(id1);
        assert_eq!(members.len(), 2);
        assert!(members.contains(&id1));
        assert!(members.contains(&id2));
    }

    #[test]
    fn layer_group_members_solo_no_group() {
        let mut state = InstrumentState::new();
        let id = state.add_instrument(SourceType::Saw);
        let members = state.layer_group_members(id);
        assert_eq!(members, vec![id]);
    }

    #[test]
    fn remove_instrument_clears_singleton_group() {
        let mut state = InstrumentState::new();
        let id1 = state.add_instrument(SourceType::Saw);
        let id2 = state.add_instrument(SourceType::Sin);

        let group = state.next_layer_group();
        state.instrument_mut(id1).unwrap().layer_group = Some(group);
        state.instrument_mut(id2).unwrap().layer_group = Some(group);

        state.remove_instrument(id1);

        // id2 should have layer_group cleared (group of 1 is meaningless)
        assert_eq!(state.instrument(id2).unwrap().layer_group, None);
    }

    #[test]
    fn select_next_wraps_at_boundary() {
        let mut state = InstrumentState::new();
        state.add_instrument(SourceType::Saw);
        state.add_instrument(SourceType::Sin);
        state.selected = Some(1);
        state.select_next();
        assert_eq!(state.selected, Some(1)); // stays at end, does not wrap
    }

    #[test]
    fn select_prev_wraps_at_boundary() {
        let mut state = InstrumentState::new();
        state.add_instrument(SourceType::Saw);
        state.add_instrument(SourceType::Sin);
        state.selected = Some(0);
        state.select_prev();
        assert_eq!(state.selected, Some(0)); // stays at start, does not wrap
    }

    #[test]
    fn index_correct_after_add() {
        let mut state = InstrumentState::new();
        let id1 = state.add_instrument(SourceType::Saw);
        let id2 = state.add_instrument(SourceType::Sin);
        let id3 = state.add_instrument(SourceType::Sqr);

        assert!(state.instrument(id1).is_some());
        assert!(state.instrument(id2).is_some());
        assert!(state.instrument(id3).is_some());
        assert_eq!(state.instrument(id1).unwrap().id, id1);
        assert_eq!(state.instrument(id2).unwrap().id, id2);
        assert_eq!(state.instrument(id3).unwrap().id, id3);
    }

    #[test]
    fn index_correct_after_remove() {
        let mut state = InstrumentState::new();
        let id1 = state.add_instrument(SourceType::Saw);
        let id2 = state.add_instrument(SourceType::Sin);
        let id3 = state.add_instrument(SourceType::Sqr);

        state.remove_instrument(id2);

        assert!(state.instrument(id1).is_some());
        assert!(state.instrument(id2).is_none());
        assert!(state.instrument(id3).is_some());
        assert_eq!(state.instrument(id1).unwrap().id, id1);
        assert_eq!(state.instrument(id3).unwrap().id, id3);
    }

    #[test]
    fn fallback_works_when_index_empty() {
        let mut state = InstrumentState::new();
        let id1 = state.add_instrument(SourceType::Saw);
        let id2 = state.add_instrument(SourceType::Sin);

        // Clear the index to simulate a deserialized state without rebuild_index
        state.id_index.clear();

        // Should still find instruments via linear fallback
        assert_eq!(state.instrument(id1).unwrap().id, id1);
        assert_eq!(state.instrument(id2).unwrap().id, id2);
        assert_eq!(state.instrument_mut(id1).unwrap().id, id1);
    }

    #[test]
    fn rebuild_index_restores_lookups() {
        let mut state = InstrumentState::new();
        let id1 = state.add_instrument(SourceType::Saw);
        let id2 = state.add_instrument(SourceType::Sin);

        // Simulate whole-struct replacement by clearing and rebuilding
        state.id_index.clear();
        state.rebuild_index();

        assert_eq!(state.instrument(id1).unwrap().id, id1);
        assert_eq!(state.instrument(id2).unwrap().id, id2);
    }

    #[test]
    fn clone_preserves_index() {
        let mut state = InstrumentState::new();
        let id1 = state.add_instrument(SourceType::Saw);
        let id2 = state.add_instrument(SourceType::Sin);

        let cloned = state.clone();
        assert_eq!(cloned.instrument(id1).unwrap().id, id1);
        assert_eq!(cloned.instrument(id2).unwrap().id, id2);
    }
}
