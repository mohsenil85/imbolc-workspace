use std::time::Instant;

use crate::state::InstrumentId;

use super::VoiceChain;

/// Maximum simultaneous voices per instrument
pub const MAX_VOICES_PER_INSTRUMENT: usize = 16;

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
            stolen.push(self.chains.remove(pos));
        }

        // 2. Count active (non-released) voices for this instrument
        let active_count = self
            .chains
            .iter()
            .filter(|v| v.instrument_id == instrument_id && v.release_state.is_none())
            .count();

        if active_count >= MAX_VOICES_PER_INSTRUMENT {
            if let Some(pos) = self.find_steal_candidate(instrument_id) {
                stolen.push(self.chains.remove(pos));
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

    /// Drain all voices. Returns an iterator over removed voices
    /// for the caller to free via OSC.
    pub fn drain_all(&mut self) -> std::vec::Drain<'_, VoiceChain> {
        self.chains.drain(..)
    }

    /// Remove and return all voices for a specific instrument.
    pub fn drain_instrument(&mut self, instrument_id: InstrumentId) -> Vec<VoiceChain> {
        let mut drained = Vec::new();
        let mut i = 0;
        while i < self.chains.len() {
            if self.chains[i].instrument_id == instrument_id {
                drained.push(self.chains.remove(i));
            } else {
                i += 1;
            }
        }
        drained
    }

    /// Remove voices whose release envelope has fully expired.
    pub fn cleanup_expired(&mut self) {
        let now = Instant::now();
        self.chains.retain(|v| {
            if let Some((released_at, release_dur)) = v.release_state {
                now.duration_since(released_at).as_secs_f32() < release_dur + 1.5
            } else {
                true
            }
        });
    }

    /// Iterate over all voices for a given instrument.
    pub fn voices_for_instrument(&self, instrument_id: InstrumentId) -> impl Iterator<Item = &VoiceChain> {
        self.chains.iter().filter(move |v| v.instrument_id == instrument_id)
    }

    /// Access all voice chains (read-only).
    pub fn chains(&self) -> &[VoiceChain] {
        &self.chains
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
