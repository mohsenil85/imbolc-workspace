use serde::{Deserialize, Serialize};

use super::instrument::{LayerGroupMixer, MixerBus};
use super::session::MixerSelection;
use crate::BusId;

/// Maximum number of buses allowed
pub const MAX_BUSES: u8 = 32;

/// Default number of buses for new projects
pub const DEFAULT_BUS_COUNT: u8 = 8;

/// Mixer state: buses, master level/mute, and selection.
/// This is a sub-state of SessionState, extracted for modularity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixerState {
    pub buses: Vec<MixerBus>,
    /// Next bus ID to assign (never reused, always increments)
    #[serde(skip)]
    pub next_bus_id: u8,
    pub master_level: f32,
    pub master_mute: bool,
    #[serde(skip)]
    pub selection: MixerSelection,
    /// Per-layer-group sub-mixers
    #[serde(default)]
    pub layer_group_mixers: Vec<LayerGroupMixer>,
}

impl MixerState {
    pub fn new() -> Self {
        Self::new_with_bus_count(DEFAULT_BUS_COUNT)
    }

    pub fn new_with_bus_count(bus_count: u8) -> Self {
        let bus_count = bus_count.min(MAX_BUSES);
        let buses: Vec<MixerBus> = (1..=bus_count).map(|i| MixerBus::new(BusId::new(i))).collect();
        let next_bus_id = bus_count + 1;
        Self {
            buses,
            next_bus_id,
            master_level: 1.0,
            master_mute: false,
            selection: MixerSelection::default(),
            layer_group_mixers: Vec::new(),
        }
    }

    /// Get a bus by ID
    pub fn bus(&self, id: BusId) -> Option<&MixerBus> {
        self.buses.iter().find(|b| b.id == id)
    }

    /// Get a mutable bus by ID
    pub fn bus_mut(&mut self, id: BusId) -> Option<&mut MixerBus> {
        self.buses.iter_mut().find(|b| b.id == id)
    }

    /// Get an iterator over current bus IDs in order
    pub fn bus_ids(&self) -> impl Iterator<Item = BusId> + '_ {
        self.buses.iter().map(|b| b.id)
    }

    /// Add a new bus. Returns the new bus ID, or None if at max capacity.
    pub fn add_bus(&mut self) -> Option<BusId> {
        if self.buses.len() >= MAX_BUSES as usize {
            return None;
        }
        let id = BusId::new(self.next_bus_id);
        self.next_bus_id = self.next_bus_id.saturating_add(1);
        self.buses.push(MixerBus::new(id));
        Some(id)
    }

    /// Remove a bus by ID. Returns true if the bus was found and removed.
    pub fn remove_bus(&mut self, id: BusId) -> bool {
        if let Some(idx) = self.buses.iter().position(|b| b.id == id) {
            self.buses.remove(idx);
            true
        } else {
            false
        }
    }

    /// Check if any bus is soloed
    pub fn any_bus_solo(&self) -> bool {
        self.buses.iter().any(|b| b.solo)
    }

    /// Compute effective mute for a bus, considering solo state
    pub fn effective_bus_mute(&self, bus: &MixerBus) -> bool {
        if self.any_bus_solo() {
            !bus.solo
        } else {
            bus.mute
        }
    }

    /// Cycle between instrument/layer-group/bus/master sections
    pub fn cycle_section(&mut self) {
        self.selection = match self.selection {
            MixerSelection::Instrument(_) => {
                // Select first layer group if any exist, otherwise skip to buses/master
                if let Some(first) = self.layer_group_mixers.first() {
                    MixerSelection::LayerGroup(first.group_id)
                } else if let Some(first_id) = self.buses.first().map(|b| b.id) {
                    MixerSelection::Bus(first_id)
                } else {
                    MixerSelection::Master
                }
            }
            MixerSelection::LayerGroup(_) => {
                if let Some(first_id) = self.buses.first().map(|b| b.id) {
                    MixerSelection::Bus(first_id)
                } else {
                    MixerSelection::Master
                }
            }
            MixerSelection::Bus(_) => MixerSelection::Master,
            MixerSelection::Master => MixerSelection::Instrument(0),
        };
    }

    /// Recompute next_bus_id from current buses (call after loading from persistence)
    pub fn recompute_next_bus_id(&mut self) {
        self.next_bus_id = self.buses.iter().map(|b| b.id.get()).max().unwrap_or(0).saturating_add(1);
    }

    /// Get a layer group mixer by group ID
    pub fn layer_group_mixer(&self, group_id: u32) -> Option<&LayerGroupMixer> {
        self.layer_group_mixers.iter().find(|g| g.group_id == group_id)
    }

    /// Get a mutable layer group mixer by group ID
    pub fn layer_group_mixer_mut(&mut self, group_id: u32) -> Option<&mut LayerGroupMixer> {
        self.layer_group_mixers.iter_mut().find(|g| g.group_id == group_id)
    }

    /// Add a layer group mixer. Returns false if it already exists.
    pub fn add_layer_group_mixer(&mut self, group_id: u32, bus_ids: &[BusId]) -> bool {
        if self.layer_group_mixer(group_id).is_some() {
            return false;
        }
        self.layer_group_mixers.push(LayerGroupMixer::new(group_id, bus_ids));
        true
    }

    /// Remove a layer group mixer by group ID. Returns true if found and removed.
    pub fn remove_layer_group_mixer(&mut self, group_id: u32) -> bool {
        if let Some(idx) = self.layer_group_mixers.iter().position(|g| g.group_id == group_id) {
            self.layer_group_mixers.remove(idx);
            true
        } else {
            false
        }
    }

    /// Check if any layer group is soloed
    pub fn any_layer_group_solo(&self) -> bool {
        self.layer_group_mixers.iter().any(|g| g.solo)
    }

    /// Compute effective mute for a layer group, considering solo state
    pub fn effective_layer_group_mute(&self, group: &LayerGroupMixer) -> bool {
        if self.any_layer_group_solo() {
            !group.solo
        } else {
            group.mute
        }
    }
}

impl Default for MixerState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bus_lookup_by_id() {
        let mixer = MixerState::new();
        assert!(mixer.bus(BusId::new(1)).is_some());
        assert_eq!(mixer.bus(BusId::new(1)).unwrap().id, BusId::new(1));
        assert!(mixer.bus(BusId::new(8)).is_some());
        assert_eq!(mixer.bus(BusId::new(8)).unwrap().id, BusId::new(8));
        // Non-existent IDs return None
        assert!(mixer.bus(BusId::new(9)).is_none());
        assert!(mixer.bus(BusId::new(100)).is_none());
    }

    #[test]
    fn add_bus_increments_id() {
        let mut mixer = MixerState::new();
        assert_eq!(mixer.buses.len(), DEFAULT_BUS_COUNT as usize);
        let new_id = mixer.add_bus().unwrap();
        assert_eq!(new_id, BusId::new(DEFAULT_BUS_COUNT + 1));
        assert_eq!(mixer.buses.len(), DEFAULT_BUS_COUNT as usize + 1);
        assert!(mixer.bus(new_id).is_some());
    }

    #[test]
    fn add_bus_respects_max_limit() {
        let mut mixer = MixerState::new_with_bus_count(MAX_BUSES);
        assert_eq!(mixer.buses.len(), MAX_BUSES as usize);
        assert!(mixer.add_bus().is_none());
        assert_eq!(mixer.buses.len(), MAX_BUSES as usize);
    }

    #[test]
    fn remove_bus() {
        let mut mixer = MixerState::new();
        assert!(mixer.bus(BusId::new(3)).is_some());
        assert!(mixer.remove_bus(BusId::new(3)));
        assert!(mixer.bus(BusId::new(3)).is_none());
        // Removing non-existent bus returns false
        assert!(!mixer.remove_bus(BusId::new(3)));
        assert!(!mixer.remove_bus(BusId::new(100)));
    }

    #[test]
    fn bus_ids_iterator() {
        let mixer = MixerState::new();
        let ids: Vec<BusId> = mixer.bus_ids().collect();
        let expected: Vec<BusId> = (1..=DEFAULT_BUS_COUNT).map(BusId::new).collect();
        assert_eq!(ids, expected);
    }

    #[test]
    fn recompute_next_bus_id() {
        let mut mixer = MixerState::new();
        mixer.next_bus_id = 0; // Simulate loading from persistence
        mixer.recompute_next_bus_id();
        assert_eq!(mixer.next_bus_id, DEFAULT_BUS_COUNT + 1);
    }

    #[test]
    fn effective_bus_mute_no_solo() {
        let mixer = MixerState::new();
        let bus = mixer.bus(BusId::new(1)).unwrap();
        assert!(!mixer.effective_bus_mute(bus));

        let mut bus_copy = bus.clone();
        bus_copy.mute = true;
        assert!(mixer.effective_bus_mute(&bus_copy));
    }

    #[test]
    fn effective_bus_mute_with_solo() {
        let mut mixer = MixerState::new();
        mixer.bus_mut(BusId::new(1)).unwrap().solo = true;
        // Bus 1 is soloed — should not be muted
        assert!(!mixer.effective_bus_mute(mixer.bus(BusId::new(1)).unwrap()));
        // Bus 2 is not soloed — should be muted
        assert!(mixer.effective_bus_mute(mixer.bus(BusId::new(2)).unwrap()));
    }

    #[test]
    fn cycle_section_full_cycle() {
        let mut mixer = MixerState::new();
        assert!(matches!(mixer.selection, MixerSelection::Instrument(0)));
        mixer.cycle_section();
        assert!(matches!(mixer.selection, MixerSelection::Bus(id) if id == BusId::new(1)));
        mixer.cycle_section();
        assert!(matches!(mixer.selection, MixerSelection::Master));
        mixer.cycle_section();
        assert!(matches!(mixer.selection, MixerSelection::Instrument(0)));
    }

    #[test]
    fn any_bus_solo() {
        let mut mixer = MixerState::new();
        assert!(!mixer.any_bus_solo());
        mixer.bus_mut(BusId::new(3)).unwrap().solo = true;
        assert!(mixer.any_bus_solo());
    }

    // ========================================================================
    // Bus effect CRUD tests
    // ========================================================================

    #[test]
    fn bus_add_effect() {
        use crate::state::instrument::EffectType;
        let mut bus = MixerBus::new(BusId::new(1));
        let id = bus.add_effect(EffectType::Reverb);
        assert_eq!(id, 0);
        assert_eq!(bus.effects.len(), 1);
        assert_eq!(bus.effects[0].effect_type, EffectType::Reverb);
        assert_eq!(bus.next_effect_id, 1);
    }

    #[test]
    fn bus_add_multiple_effects() {
        use crate::state::instrument::EffectType;
        let mut bus = MixerBus::new(BusId::new(1));
        let id0 = bus.add_effect(EffectType::Reverb);
        let id1 = bus.add_effect(EffectType::Delay);
        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
        assert_eq!(bus.effects.len(), 2);
        assert_eq!(bus.next_effect_id, 2);
    }

    #[test]
    fn bus_effect_by_id() {
        use crate::state::instrument::EffectType;
        let mut bus = MixerBus::new(BusId::new(1));
        let id = bus.add_effect(EffectType::Reverb);
        assert!(bus.effect_by_id(id).is_some());
        assert_eq!(bus.effect_by_id(id).unwrap().effect_type, EffectType::Reverb);
        assert!(bus.effect_by_id(999).is_none());
    }

    #[test]
    fn bus_remove_effect() {
        use crate::state::instrument::EffectType;
        let mut bus = MixerBus::new(BusId::new(1));
        let id = bus.add_effect(EffectType::Reverb);
        assert!(bus.remove_effect(id));
        assert!(bus.effects.is_empty());
        assert!(!bus.remove_effect(id)); // already removed
    }

    #[test]
    fn bus_move_effect() {
        use crate::state::instrument::EffectType;
        let mut bus = MixerBus::new(BusId::new(1));
        let id0 = bus.add_effect(EffectType::Reverb);
        let id1 = bus.add_effect(EffectType::Delay);
        // Move first effect down
        assert!(bus.move_effect(id0, 1));
        assert_eq!(bus.effects[0].id, id1);
        assert_eq!(bus.effects[1].id, id0);
        // Move beyond bounds fails
        assert!(!bus.move_effect(id0, 1));
    }

    #[test]
    fn bus_recalculate_next_effect_id() {
        use crate::state::instrument::EffectType;
        let mut bus = MixerBus::new(BusId::new(1));
        bus.add_effect(EffectType::Reverb);
        bus.add_effect(EffectType::Delay);
        bus.next_effect_id = 0; // simulate loading
        bus.recalculate_next_effect_id();
        assert_eq!(bus.next_effect_id, 2);
    }

    // ========================================================================
    // LayerGroupMixer effect CRUD tests
    // ========================================================================

    #[test]
    fn layer_group_add_effect() {
        use crate::state::instrument::{EffectType, LayerGroupMixer};
        let mut gm = LayerGroupMixer::new(1, &[BusId::new(1), BusId::new(2)]);
        let id = gm.add_effect(EffectType::TapeComp);
        assert_eq!(id, 0);
        assert_eq!(gm.effects.len(), 1);
        assert_eq!(gm.effects[0].effect_type, EffectType::TapeComp);
    }

    #[test]
    fn layer_group_remove_effect() {
        use crate::state::instrument::{EffectType, LayerGroupMixer};
        let mut gm = LayerGroupMixer::new(1, &[]);
        let id = gm.add_effect(EffectType::Limiter);
        assert!(gm.remove_effect(id));
        assert!(gm.effects.is_empty());
    }

    #[test]
    fn layer_group_move_effect() {
        use crate::state::instrument::{EffectType, LayerGroupMixer};
        let mut gm = LayerGroupMixer::new(1, &[]);
        let id0 = gm.add_effect(EffectType::Reverb);
        let id1 = gm.add_effect(EffectType::Delay);
        assert!(gm.move_effect(id0, 1));
        assert_eq!(gm.effects[0].id, id1);
        assert_eq!(gm.effects[1].id, id0);
    }
}
