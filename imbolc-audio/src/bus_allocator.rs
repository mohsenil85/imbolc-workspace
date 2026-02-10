use std::collections::HashMap;

use imbolc_types::InstrumentId;

type ModuleId = InstrumentId;

/// Manages audio and control bus allocation for module routing
#[derive(Debug, Clone)]
pub struct BusAllocator {
    /// Audio bus allocations: (module_id, port_name) -> bus_index
    audio_buses: HashMap<(ModuleId, String), i32>,
    /// Control bus allocations: (module_id, port_name) -> bus_index
    control_buses: HashMap<(ModuleId, String), i32>,
    /// Next available audio bus (starts at 16 to avoid hardware outputs)
    pub next_audio_bus: i32,
    /// Next available control bus
    pub next_control_bus: i32,
}

impl BusAllocator {
    /// Audio buses 0-15 are reserved for hardware I/O
    const AUDIO_BUS_START: i32 = 16;
    /// Control buses start at 0
    const CONTROL_BUS_START: i32 = 0;

    pub fn new() -> Self {
        Self {
            audio_buses: HashMap::new(),
            control_buses: HashMap::new(),
            next_audio_bus: Self::AUDIO_BUS_START,
            next_control_bus: Self::CONTROL_BUS_START,
        }
    }

    /// Get or allocate an audio bus for a module's output port.
    /// Returns stereo bus index (allocates 2 channels).
    pub fn get_or_alloc_audio_bus(&mut self, module_id: ModuleId, port_name: &str) -> i32 {
        self.get_or_alloc_audio_bus_with_channels(module_id, port_name, 2)
    }

    /// Get or allocate an audio bus for a module's output port with specified channel count.
    /// Returns bus index (allocates `channels` channels).
    pub fn get_or_alloc_audio_bus_with_channels(
        &mut self,
        module_id: ModuleId,
        port_name: &str,
        channels: usize,
    ) -> i32 {
        let key = (module_id, port_name.to_string());
        if let Some(&bus) = self.audio_buses.get(&key) {
            return bus;
        }

        let bus = self.next_audio_bus;
        self.next_audio_bus += channels as i32;
        self.audio_buses.insert(key, bus);
        bus
    }

    /// Get or allocate a control bus for a module's output port.
    pub fn get_or_alloc_control_bus(&mut self, module_id: ModuleId, port_name: &str) -> i32 {
        let key = (module_id, port_name.to_string());
        if let Some(&bus) = self.control_buses.get(&key) {
            return bus;
        }

        let bus = self.next_control_bus;
        self.next_control_bus += 1;
        self.control_buses.insert(key, bus);
        bus
    }

    /// Get an existing audio bus without allocating
    pub fn get_audio_bus(&self, module_id: ModuleId, port_name: &str) -> Option<i32> {
        self.audio_buses.get(&(module_id, port_name.to_string())).copied()
    }

    /// Get an existing control bus without allocating
    #[allow(dead_code)]
    pub fn get_control_bus(&self, module_id: ModuleId, port_name: &str) -> Option<i32> {
        self.control_buses.get(&(module_id, port_name.to_string())).copied()
    }

    /// Free all buses allocated for a module
    #[allow(dead_code)]
    pub fn free_module_buses(&mut self, module_id: ModuleId) {
        self.audio_buses.retain(|(id, _), _| *id != module_id);
        self.control_buses.retain(|(id, _), _| *id != module_id);
    }

    /// Reset all allocations (used when rebuilding routing)
    pub fn reset(&mut self) {
        self.audio_buses.clear();
        self.control_buses.clear();
        self.next_audio_bus = Self::AUDIO_BUS_START;
        self.next_control_bus = Self::CONTROL_BUS_START;
    }
}

impl Default for BusAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u32) -> InstrumentId {
        InstrumentId::new(n)
    }

    #[test]
    fn test_audio_bus_allocation() {
        let mut alloc = BusAllocator::new();

        let bus1 = alloc.get_or_alloc_audio_bus(id(1), "out");
        assert_eq!(bus1, 16); // First bus starts at 16

        let bus2 = alloc.get_or_alloc_audio_bus(id(2), "out");
        assert_eq!(bus2, 18); // Next stereo pair

        // Same module/port returns same bus
        let bus1_again = alloc.get_or_alloc_audio_bus(id(1), "out");
        assert_eq!(bus1_again, 16);
    }

    #[test]
    fn test_control_bus_allocation() {
        let mut alloc = BusAllocator::new();

        let bus1 = alloc.get_or_alloc_control_bus(id(1), "freq");
        assert_eq!(bus1, 0);

        let bus2 = alloc.get_or_alloc_control_bus(id(1), "gate");
        assert_eq!(bus2, 1);

        let bus3 = alloc.get_or_alloc_control_bus(id(2), "out");
        assert_eq!(bus3, 2);
    }

    #[test]
    fn test_free_module_buses() {
        let mut alloc = BusAllocator::new();

        alloc.get_or_alloc_audio_bus(id(1), "out");
        alloc.get_or_alloc_control_bus(id(1), "freq");
        alloc.get_or_alloc_audio_bus(id(2), "out");

        alloc.free_module_buses(id(1));

        assert!(alloc.get_audio_bus(id(1), "out").is_none());
        assert!(alloc.get_control_bus(id(1), "freq").is_none());
        assert!(alloc.get_audio_bus(id(2), "out").is_some());
    }

    #[test]
    fn test_reset() {
        let mut alloc = BusAllocator::new();

        alloc.get_or_alloc_audio_bus(id(1), "out");
        alloc.get_or_alloc_control_bus(id(1), "freq");

        alloc.reset();

        // After reset, new allocations start fresh
        let bus = alloc.get_or_alloc_audio_bus(id(1), "out");
        assert_eq!(bus, 16);
    }

    #[test]
    fn test_mono_bus_allocation() {
        let mut alloc = BusAllocator::new();

        // Allocate mono bus
        let bus1 = alloc.get_or_alloc_audio_bus_with_channels(id(1), "out", 1);
        assert_eq!(bus1, 16);

        // Next mono bus is adjacent
        let bus2 = alloc.get_or_alloc_audio_bus_with_channels(id(2), "out", 1);
        assert_eq!(bus2, 17);

        // Stereo bus follows
        let bus3 = alloc.get_or_alloc_audio_bus(id(3), "out");
        assert_eq!(bus3, 18);

        // Same module/port returns same bus
        let bus1_again = alloc.get_or_alloc_audio_bus_with_channels(id(1), "out", 1);
        assert_eq!(bus1_again, 16);
    }
}
