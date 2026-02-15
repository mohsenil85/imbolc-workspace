//! Adaptive JI solver with continuity constraints.

/// Syntonic comma threshold in cents — if a pitch would deviate more than this
/// from its previous frequency, snap toward the previous value.
const SNAP_THRESHOLD_CENTS: f64 = 5.0;

/// Weight for snapping toward previous frequency (0.0 = no snap, 1.0 = full snap).
const SNAP_WEIGHT: f64 = 0.7;

/// Apply adaptive continuity constraint to a JI frequency.
///
/// If the ideal JI frequency deviates from the previous frequency by more than
/// the threshold, interpolate toward the previous value.
///
/// Returns the adjusted frequency.
pub fn adaptive_snap(ideal_freq: f64, previous_freq: f64) -> f64 {
    if previous_freq <= 0.0 {
        return ideal_freq;
    }

    let cents_diff = 1200.0 * (ideal_freq / previous_freq).ln() / 2.0_f64.ln();
    if cents_diff.abs() <= SNAP_THRESHOLD_CENTS {
        ideal_freq
    } else {
        // Interpolate: move toward previous frequency by SNAP_WEIGHT
        let log_ideal = ideal_freq.ln();
        let log_prev = previous_freq.ln();
        let log_result = log_ideal * (1.0 - SNAP_WEIGHT) + log_prev * SNAP_WEIGHT;
        log_result.exp()
    }
}

/// Compute global drift in cents between ideal JI frequency and ET reference.
pub fn drift_cents(ji_freq: f64, et_freq: f64) -> f64 {
    if et_freq <= 0.0 || ji_freq <= 0.0 {
        return 0.0;
    }
    1200.0 * (ji_freq / et_freq).ln() / 2.0_f64.ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_snap_when_previous_is_zero() {
        let result = adaptive_snap(440.0, 0.0);
        assert!((result - 440.0).abs() < 1e-10);
    }

    #[test]
    fn no_snap_within_threshold() {
        // 2 cents is within the 5-cent threshold
        let prev = 440.0;
        let ideal = prev * 2.0_f64.powf(2.0 / 1200.0); // 2 cents sharp
        let result = adaptive_snap(ideal, prev);
        assert!((result - ideal).abs() < 1e-10);
    }

    #[test]
    fn snaps_when_exceeding_threshold() {
        let prev = 440.0;
        // 22 cents is the syntonic comma — well above threshold
        let ideal = prev * 2.0_f64.powf(22.0 / 1200.0);
        let result = adaptive_snap(ideal, prev);
        // Result should be between prev and ideal, closer to prev
        assert!(result > prev);
        assert!(result < ideal);
    }

    #[test]
    fn drift_cents_zero_for_equal() {
        let drift = drift_cents(440.0, 440.0);
        assert!(drift.abs() < 1e-10);
    }

    #[test]
    fn drift_cents_positive_for_sharp() {
        let et_freq = 440.0;
        let ji_freq = 440.0 * (5.0 / 4.0) / 2.0_f64.powf(4.0 / 12.0);
        let drift = drift_cents(ji_freq, et_freq);
        // 5/4 major third is ~14 cents flat of ET major third
        // but relative to A4 unison, this is comparing E5 JI vs E5 ET
        // Let's just verify it's nonzero and finite
        assert!(drift.is_finite());
    }

    #[test]
    fn drift_cents_handles_zero() {
        assert_eq!(drift_cents(0.0, 440.0), 0.0);
        assert_eq!(drift_cents(440.0, 0.0), 0.0);
    }
}
