use std::time::Instant;

use imbolc_types::InstrumentId;

use super::VoiceChain;

/// Maximum simultaneous voices per instrument
pub const MAX_VOICES_PER_INSTRUMENT: usize = 64;

/// Manages voice allocation, tracking, stealing, and control bus pooling.
///
/// The allocator owns voice lifecycle data but does not send OSC messages.
/// AudioEngine methods handle the actual SuperCollider communication.
pub struct VoiceAllocator {
    chains: Vec<VoiceChain>,
    next_audio_bus: i32,
    next_control_bus: i32,
    /// Pool of freed control bus triples (freq, gate, vel) available for reuse
    control_bus_pool: Vec<(i32, i32, i32)>,
}

#[allow(dead_code)]
impl VoiceAllocator {
    pub fn new() -> Self {
        Self {
            chains: Vec::new(),
            next_audio_bus: 16,
            next_control_bus: 0,
            control_bus_pool: Vec::new(),
        }
    }

    /// Allocate a control bus triple (freq, gate, vel).
    /// Reuses freed buses from the pool when available.
    pub fn alloc_control_buses(&mut self) -> (i32, i32, i32) {
        if let Some(triple) = self.control_bus_pool.pop() {
            triple
        } else {
            let freq = self.next_control_bus;
            self.next_control_bus += 1;
            let gate = self.next_control_bus;
            self.next_control_bus += 1;
            let vel = self.next_control_bus;
            self.next_control_bus += 1;
            (freq, gate, vel)
        }
    }

    /// Return a control bus triple to the pool for reuse.
    pub fn return_control_buses(&mut self, freq: i32, gate: i32, vel: i32) {
        self.control_bus_pool.push((freq, gate, vel));
    }

    /// Add a voice to the active chain list.
    pub fn add(&mut self, voice: VoiceChain) {
        self.chains.push(voice);
    }

    /// Determine which voices to steal before spawning a new voice.
    /// Returns removed voices that the caller should free via OSC.
    /// Control buses of stolen voices are returned to the pool.
    ///
    /// Handles:
    /// 1. Same-pitch retrigger (always steal matching pitch)
    /// 2. Over-limit stealing (steal lowest-scored candidate)
    pub fn steal_voices(
        &mut self,
        instrument_id: InstrumentId,
        pitch: u8,
    ) -> Vec<VoiceChain> {
        let mut stolen = Vec::new();

        // 1. Same-pitch retrigger: steal any voice (active or released) with the same pitch
        if let Some(pos) = self.chains.iter().position(|v| {
            v.instrument_id == instrument_id && v.pitch == pitch
        }) {
            let voice = self.chains.remove(pos);
            self.control_bus_pool.push(voice.control_buses);
            stolen.push(voice);
        }

        // 2. Count active (non-released) voices for this instrument
        let active_count = self
            .chains
            .iter()
            .filter(|v| v.instrument_id == instrument_id && v.release_state.is_none())
            .count();

        if active_count >= MAX_VOICES_PER_INSTRUMENT {
            if let Some(pos) = self.find_steal_candidate(instrument_id) {
                let voice = self.chains.remove(pos);
                self.control_bus_pool.push(voice.control_buses);
                stolen.push(voice);
            }
        }

        stolen
    }

    /// Find the best steal candidate for a given instrument.
    /// Returns the index of the voice with the lowest score (best target).
    fn find_steal_candidate(&self, instrument_id: InstrumentId) -> Option<usize> {
        let now = Instant::now();

        self.chains
            .iter()
            .enumerate()
            .filter(|(_, v)| v.instrument_id == instrument_id)
            .min_by(|(_, a), (_, b)| {
                let score_a = Self::steal_score(a, now);
                let score_b = Self::steal_score(b, now);
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
    }

    /// Compute a steal priority score for a voice. Lower = better steal target.
    ///
    /// - Released voices: 0–999 (further into release = lower score)
    /// - Active voices: 1000+ (lower velocity = lower; older = lower tiebreaker)
    fn steal_score(voice: &VoiceChain, now: Instant) -> f64 {
        if let Some((released_at, release_dur)) = voice.release_state {
            let elapsed = now.duration_since(released_at).as_secs_f64();
            let progress = if release_dur > 0.0 {
                (elapsed / release_dur as f64).min(1.0)
            } else {
                1.0
            };
            (1.0 - progress) * 999.0
        } else {
            let velocity_score = voice.velocity as f64 * 500.0;
            let age_secs = now.duration_since(voice.spawn_time).as_secs_f64();
            let age_score = 500.0 / (1.0 + age_secs);
            1000.0 + velocity_score + age_score
        }
    }

    /// Mark a voice as released. Returns the index if found, so the caller
    /// can access the voice to send gate=0 via OSC.
    pub fn mark_released(
        &mut self,
        instrument_id: InstrumentId,
        pitch: u8,
        release_time: f32,
    ) -> Option<usize> {
        if let Some(pos) = self
            .chains
            .iter()
            .position(|v| {
                v.instrument_id == instrument_id
                    && v.pitch == pitch
                    && v.release_state.is_none()
            })
        {
            self.chains[pos].release_state = Some((Instant::now(), release_time));
            Some(pos)
        } else {
            None
        }
    }

    /// Drain all voices. Returns removed voices for the caller to free via OSC.
    /// Control buses of all drained voices are returned to the pool.
    pub fn drain_all(&mut self) -> Vec<VoiceChain> {
        let drained: Vec<VoiceChain> = self.chains.drain(..).collect();
        for voice in &drained {
            self.control_bus_pool.push(voice.control_buses);
        }
        drained
    }

    /// Remove and return all voices for a specific instrument.
    /// Control buses of removed voices are returned to the pool.
    pub fn drain_instrument(&mut self, instrument_id: InstrumentId) -> Vec<VoiceChain> {
        let mut drained = Vec::new();
        let mut i = 0;
        while i < self.chains.len() {
            if self.chains[i].instrument_id == instrument_id {
                let voice = self.chains.remove(i);
                self.control_bus_pool.push(voice.control_buses);
                drained.push(voice);
            } else {
                i += 1;
            }
        }
        drained
    }

    /// Remove voices whose release envelope has fully expired.
    /// Returns expired voices so the caller can unregister their nodes.
    /// Control buses of expired voices are returned to the pool.
    pub fn cleanup_expired(&mut self) -> Vec<VoiceChain> {
        let now = Instant::now();
        let mut expired = Vec::new();
        let mut i = 0;
        while i < self.chains.len() {
            let is_expired = if let Some((released_at, release_dur)) = self.chains[i].release_state {
                now.duration_since(released_at).as_secs_f32() >= release_dur + 1.5
            } else {
                false
            };
            if is_expired {
                let voice = self.chains.remove(i);
                self.control_bus_pool.push(voice.control_buses);
                expired.push(voice);
            } else {
                i += 1;
            }
        }
        expired
    }

    /// Remove a voice by its group_id. Returns it if found.
    /// Control buses are returned to the pool.
    pub fn remove_by_group_id(&mut self, group_id: i32) -> Option<VoiceChain> {
        if let Some(pos) = self.chains.iter().position(|v| v.group_id == group_id) {
            let voice = self.chains.remove(pos);
            self.control_bus_pool.push(voice.control_buses);
            Some(voice)
        } else {
            None
        }
    }

    /// Iterate over all voices for a given instrument.
    pub fn voices_for_instrument(&self, instrument_id: InstrumentId) -> impl Iterator<Item = &VoiceChain> {
        self.chains.iter().filter(move |v| v.instrument_id == instrument_id)
    }

    /// Access all voice chains (read-only).
    pub fn chains(&self) -> &[VoiceChain] {
        &self.chains
    }

    /// Number of control bus triples in the reuse pool.
    pub fn control_bus_pool_size(&self) -> usize {
        self.control_bus_pool.len()
    }

    /// Sync bus watermarks from the bus allocator after a routing rebuild.
    pub fn sync_bus_watermarks(&mut self, audio_bus: i32, control_bus: i32) {
        self.next_audio_bus = audio_bus;
        // Only advance control bus if allocator is ahead (don't regress past pool allocations)
        if control_bus > self.next_control_bus {
            self.next_control_bus = control_bus;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_voice(inst_id: InstrumentId, pitch: u8, group_id: i32, buses: (i32, i32, i32)) -> VoiceChain {
        VoiceChain {
            instrument_id: inst_id,
            pitch,
            velocity: 0.8,
            group_id,
            midi_node_id: group_id + 1,
            source_node: group_id + 2,
            spawn_time: Instant::now(),
            release_state: None,
            control_buses: buses,
        }
    }

    fn make_expired_voice(inst_id: InstrumentId, pitch: u8, group_id: i32, buses: (i32, i32, i32)) -> VoiceChain {
        VoiceChain {
            instrument_id: inst_id,
            pitch,
            velocity: 0.8,
            group_id,
            midi_node_id: group_id + 1,
            source_node: group_id + 2,
            spawn_time: Instant::now() - Duration::from_secs(10),
            release_state: Some((Instant::now() - Duration::from_secs(5), 0.5)),
            control_buses: buses,
        }
    }

    #[test]
    fn test_control_buses_returned_on_cleanup_expired() {
        let mut alloc = VoiceAllocator::new();
        let buses = alloc.alloc_control_buses();
        alloc.add(make_expired_voice(1, 60, 100, buses));
        // Also add a live voice that should NOT be cleaned up
        let buses2 = alloc.alloc_control_buses();
        alloc.add(make_voice(1, 72, 200, buses2));

        assert_eq!(alloc.control_bus_pool_size(), 0);
        let expired = alloc.cleanup_expired();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].group_id, 100);
        assert_eq!(alloc.control_bus_pool_size(), 1);
        assert_eq!(alloc.chains().len(), 1);
    }

    #[test]
    fn test_control_buses_returned_on_drain_all() {
        let mut alloc = VoiceAllocator::new();
        let buses1 = alloc.alloc_control_buses();
        let buses2 = alloc.alloc_control_buses();
        alloc.add(make_voice(1, 60, 100, buses1));
        alloc.add(make_voice(1, 72, 200, buses2));

        assert_eq!(alloc.control_bus_pool_size(), 0);
        let drained = alloc.drain_all();
        assert_eq!(drained.len(), 2);
        assert_eq!(alloc.control_bus_pool_size(), 2);
        assert!(alloc.chains().is_empty());
    }

    #[test]
    fn test_control_buses_returned_on_steal() {
        let mut alloc = VoiceAllocator::new();
        let buses = alloc.alloc_control_buses();
        alloc.add(make_voice(1, 60, 100, buses));

        assert_eq!(alloc.control_bus_pool_size(), 0);
        // Same-pitch retrigger should steal and return buses
        let stolen = alloc.steal_voices(1, 60);
        assert_eq!(stolen.len(), 1);
        assert_eq!(alloc.control_bus_pool_size(), 1);
    }

    #[test]
    fn test_remove_by_group_id() {
        let mut alloc = VoiceAllocator::new();
        let buses = alloc.alloc_control_buses();
        alloc.add(make_voice(1, 60, 100, buses));
        let buses2 = alloc.alloc_control_buses();
        alloc.add(make_voice(1, 72, 200, buses2));

        assert_eq!(alloc.control_bus_pool_size(), 0);
        let removed = alloc.remove_by_group_id(100);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().pitch, 60);
        assert_eq!(alloc.control_bus_pool_size(), 1);
        assert_eq!(alloc.chains().len(), 1);
    }

    #[test]
    fn test_remove_by_group_id_not_found() {
        let mut alloc = VoiceAllocator::new();
        let buses = alloc.alloc_control_buses();
        alloc.add(make_voice(1, 60, 100, buses));

        let removed = alloc.remove_by_group_id(999);
        assert!(removed.is_none());
        assert_eq!(alloc.control_bus_pool_size(), 0);
        assert_eq!(alloc.chains().len(), 1);
    }

    #[test]
    fn test_control_buses_returned_on_drain_instrument() {
        let mut alloc = VoiceAllocator::new();
        let buses1 = alloc.alloc_control_buses();
        let buses2 = alloc.alloc_control_buses();
        let buses3 = alloc.alloc_control_buses();
        alloc.add(make_voice(1, 60, 100, buses1));
        alloc.add(make_voice(2, 72, 200, buses2)); // different instrument
        alloc.add(make_voice(1, 84, 300, buses3));

        assert_eq!(alloc.control_bus_pool_size(), 0);
        let drained = alloc.drain_instrument(1);
        assert_eq!(drained.len(), 2);
        assert_eq!(alloc.control_bus_pool_size(), 2);
        assert_eq!(alloc.chains().len(), 1); // instrument 2 voice remains
    }
}
