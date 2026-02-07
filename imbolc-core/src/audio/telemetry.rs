//! Audio thread telemetry for latency and jitter monitoring.
//!
//! Provides lock-free metrics collection for tick durations without
//! allocations in the hot path.

use std::time::Duration;

/// Ring buffer size for tick duration samples.
const TICK_BUFFER_SIZE: usize = 256;

/// Audio thread telemetry for performance monitoring.
///
/// Collects tick duration metrics in a fixed-size ring buffer.
/// All operations are allocation-free for use in the realtime audio thread.
pub struct AudioTelemetry {
    /// Ring buffer of tick durations in microseconds
    tick_durations_us: [u32; TICK_BUFFER_SIZE],
    /// Current write index in the ring buffer
    tick_idx: usize,
    /// Maximum tick duration observed in current window
    max_tick_us: u32,
    /// Count of ticks that exceeded the target budget
    overrun_count: u64,
    /// Number of samples collected (saturates at TICK_BUFFER_SIZE)
    sample_count: usize,
}

impl Default for AudioTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioTelemetry {
    /// Create a new telemetry collector.
    pub fn new() -> Self {
        Self {
            tick_durations_us: [0; TICK_BUFFER_SIZE],
            tick_idx: 0,
            max_tick_us: 0,
            overrun_count: 0,
            sample_count: 0,
        }
    }

    /// Record a tick duration. Call this at the end of each audio thread tick.
    ///
    /// # Arguments
    /// * `duration` - Time taken for the tick
    /// * `budget_us` - Target budget in microseconds (e.g., 500 for 0.5ms ticks)
    #[inline]
    pub fn record(&mut self, duration: Duration, budget_us: u32) {
        let us = duration.as_micros().min(u32::MAX as u128) as u32;

        self.tick_durations_us[self.tick_idx] = us;
        self.tick_idx = (self.tick_idx + 1) % TICK_BUFFER_SIZE;

        if self.sample_count < TICK_BUFFER_SIZE {
            self.sample_count += 1;
        }

        if us > self.max_tick_us {
            self.max_tick_us = us;
        }

        if us > budget_us {
            self.overrun_count += 1;
        }
    }

    /// Get a summary of collected metrics and reset the max for the next window.
    ///
    /// Returns (avg_tick_us, max_tick_us, p95_tick_us, overruns).
    pub fn take_summary(&mut self) -> (u32, u32, u32, u64) {
        if self.sample_count == 0 {
            return (0, 0, 0, 0);
        }

        // Calculate average
        let sum: u64 = self.tick_durations_us[..self.sample_count]
            .iter()
            .map(|&x| x as u64)
            .sum();
        let avg = (sum / self.sample_count as u64) as u32;

        // Calculate p95 (95th percentile)
        let mut sorted = self.tick_durations_us;
        sorted[..self.sample_count].sort_unstable();
        let p95_idx = (self.sample_count * 95 / 100).max(1) - 1;
        let p95 = sorted[p95_idx.min(self.sample_count - 1)];

        let max = self.max_tick_us;
        let overruns = self.overrun_count;

        // Reset max for next window (keep overrun count cumulative)
        self.max_tick_us = 0;

        (avg, max, p95, overruns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_basic() {
        let mut t = AudioTelemetry::new();

        // Record some samples
        t.record(Duration::from_micros(100), 500);
        t.record(Duration::from_micros(200), 500);
        t.record(Duration::from_micros(300), 500);

        let (avg, max, _p95, overruns) = t.take_summary();
        assert_eq!(avg, 200); // (100+200+300)/3
        assert_eq!(max, 300);
        assert_eq!(overruns, 0);
    }

    #[test]
    fn test_telemetry_overruns() {
        let mut t = AudioTelemetry::new();

        t.record(Duration::from_micros(400), 500);
        t.record(Duration::from_micros(600), 500); // overrun
        t.record(Duration::from_micros(800), 500); // overrun

        let (_avg, _max, _p95, overruns) = t.take_summary();
        assert_eq!(overruns, 2);
    }
}
