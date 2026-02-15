//! Pure state-mutation reducer for generative engine actions.

use crate::action::GenerativeAction;
use crate::state::generative::{GenVoice, GenVoiceId, GenerativeAlgorithm};
use crate::state::session::SessionState;

/// Apply a GenerativeAction to session state. Returns true if handled.
pub fn reduce(action: &GenerativeAction, session: &mut SessionState) -> bool {
    let gen = &mut session.generative;

    match action {
        // Engine toggles
        GenerativeAction::ToggleEnabled => {
            gen.enabled = !gen.enabled;
        }
        GenerativeAction::ToggleCapture => {
            gen.capture_enabled = !gen.capture_enabled;
        }
        GenerativeAction::ClearCapture => {
            gen.captured_events.clear();
        }
        GenerativeAction::CommitCapture => {
            // CommitCapture is handled by dispatch (needs piano roll access).
            // Reducer is a no-op; dispatch will clear captured_events after commit.
            return true;
        }

        // Voice CRUD
        GenerativeAction::AddVoice(algorithm) => {
            let id = GenVoiceId::new(gen.next_voice_id);
            gen.next_voice_id += 1;
            gen.voices.push(GenVoice::new(id, algorithm.clone()));
            // Auto-enable engine when first voice is added
            gen.enabled = true;
        }
        GenerativeAction::RemoveVoice(id) => {
            gen.voices.retain(|v| v.id != *id);
        }
        GenerativeAction::ToggleVoice(id) => {
            if let Some(v) = voice_mut(gen, *id) {
                v.enabled = !v.enabled;
            }
        }
        GenerativeAction::MuteVoice(id) => {
            if let Some(v) = voice_mut(gen, *id) {
                v.muted = !v.muted;
            }
        }
        GenerativeAction::SetVoiceTarget(id, target) => {
            if let Some(v) = voice_mut(gen, *id) {
                v.target_instrument = *target;
            }
        }
        GenerativeAction::SetVoiceAlgorithm(id, algorithm) => {
            if let Some(v) = voice_mut(gen, *id) {
                v.algorithm = algorithm.clone();
            }
        }

        // Macros
        GenerativeAction::AdjustDensity(delta) => {
            gen.macros.density = (gen.macros.density + delta).clamp(0.0, 1.0);
        }
        GenerativeAction::AdjustChaos(delta) => {
            gen.macros.chaos = (gen.macros.chaos + delta).clamp(0.0, 1.0);
        }
        GenerativeAction::AdjustEnergy(delta) => {
            gen.macros.energy = (gen.macros.energy + delta).clamp(0.0, 1.0);
        }
        GenerativeAction::AdjustMotion(delta) => {
            gen.macros.motion = (gen.macros.motion + delta).clamp(0.0, 1.0);
        }

        // Constraints
        GenerativeAction::ToggleScaleLock => {
            gen.constraints.scale_lock = !gen.constraints.scale_lock;
        }
        GenerativeAction::AdjustPitchMin(delta) => {
            let new_val = (gen.constraints.pitch_min as i16 + *delta as i16).clamp(0, 127) as u8;
            gen.constraints.pitch_min = new_val.min(gen.constraints.pitch_max);
        }
        GenerativeAction::AdjustPitchMax(delta) => {
            let new_val = (gen.constraints.pitch_max as i16 + *delta as i16).clamp(0, 127) as u8;
            gen.constraints.pitch_max = new_val.max(gen.constraints.pitch_min);
        }
        GenerativeAction::AdjustMaxNotesPerBeat(delta) => {
            gen.constraints.max_notes_per_beat =
                (gen.constraints.max_notes_per_beat as i16 + *delta as i16).clamp(0, 32) as u8;
        }
        GenerativeAction::AdjustHumanizeTiming(delta) => {
            gen.constraints.humanize_timing =
                (gen.constraints.humanize_timing + delta).clamp(0.0, 1.0);
        }
        GenerativeAction::AdjustHumanizeVelocity(delta) => {
            gen.constraints.humanize_velocity =
                (gen.constraints.humanize_velocity + delta).clamp(0.0, 1.0);
        }

        // Euclidean params
        GenerativeAction::SetEuclideanPulses(id, pulses) => {
            if let Some(v) = voice_mut(gen, *id) {
                if let GenerativeAlgorithm::Euclidean(ref mut cfg) = v.algorithm {
                    cfg.pulses = (*pulses).min(cfg.steps);
                }
            }
        }
        GenerativeAction::SetEuclideanSteps(id, steps) => {
            if let Some(v) = voice_mut(gen, *id) {
                if let GenerativeAlgorithm::Euclidean(ref mut cfg) = v.algorithm {
                    cfg.steps = (*steps).max(1).min(64);
                    cfg.pulses = cfg.pulses.min(cfg.steps);
                    cfg.rotation = cfg.rotation.min(cfg.steps.saturating_sub(1));
                }
            }
        }
        GenerativeAction::SetEuclideanRotation(id, rotation) => {
            if let Some(v) = voice_mut(gen, *id) {
                if let GenerativeAlgorithm::Euclidean(ref mut cfg) = v.algorithm {
                    cfg.rotation = (*rotation).min(cfg.steps.saturating_sub(1));
                }
            }
        }
        GenerativeAction::CycleEuclideanPitchMode(id) => {
            if let Some(v) = voice_mut(gen, *id) {
                if let GenerativeAlgorithm::Euclidean(ref mut cfg) = v.algorithm {
                    cfg.pitch_mode = cfg.pitch_mode.cycle();
                }
            }
        }
        GenerativeAction::CycleVoiceRate(id) => {
            if let Some(v) = voice_mut(gen, *id) {
                let rate = v.algorithm.rate_mut();
                *rate = rate.cycle();
            }
        }
        GenerativeAction::CycleVoiceRateReverse(id) => {
            if let Some(v) = voice_mut(gen, *id) {
                let rate = v.algorithm.rate_mut();
                *rate = rate.cycle_reverse();
            }
        }

        // Markov params
        GenerativeAction::SetMarkovTransition(id, from, to, weight) => {
            if let Some(v) = voice_mut(gen, *id) {
                if let GenerativeAlgorithm::Markov(ref mut cfg) = v.algorithm {
                    let from = *from as usize;
                    let to = *to as usize;
                    if from < 12 && to < 12 {
                        cfg.transition_matrix[from][to] = weight.max(0.0);
                        cfg.normalize_row(from);
                    }
                }
            }
        }
        GenerativeAction::AdjustMarkovRestProb(id, delta) => {
            if let Some(v) = voice_mut(gen, *id) {
                if let GenerativeAlgorithm::Markov(ref mut cfg) = v.algorithm {
                    cfg.rest_probability = (cfg.rest_probability + delta).clamp(0.0, 1.0);
                }
            }
        }
        GenerativeAction::CycleMarkovDurationMode(id) => {
            if let Some(v) = voice_mut(gen, *id) {
                if let GenerativeAlgorithm::Markov(ref mut cfg) = v.algorithm {
                    cfg.duration_mode = cfg.duration_mode.cycle();
                }
            }
        }
        GenerativeAction::RandomizeMarkovMatrix(id) => {
            let mut rng = gen.next_voice_id as u64 * 6364136223846793005 + 1;
            if let Some(v) = voice_mut(gen, *id) {
                if let GenerativeAlgorithm::Markov(ref mut cfg) = v.algorithm {
                    cfg.randomize(&mut rng);
                }
            }
        }

        // L-System params
        GenerativeAction::SetLSystemAxiom(id, axiom) => {
            if let Some(v) = voice_mut(gen, *id) {
                if let GenerativeAlgorithm::LSystem(ref mut cfg) = v.algorithm {
                    cfg.axiom = axiom.clone();
                }
            }
        }
        GenerativeAction::SetLSystemIterations(id, iterations) => {
            if let Some(v) = voice_mut(gen, *id) {
                if let GenerativeAlgorithm::LSystem(ref mut cfg) = v.algorithm {
                    cfg.iterations = (*iterations).min(6);
                }
            }
        }
        GenerativeAction::AdjustLSystemStepInterval(id, delta) => {
            if let Some(v) = voice_mut(gen, *id) {
                if let GenerativeAlgorithm::LSystem(ref mut cfg) = v.algorithm {
                    cfg.step_interval = (cfg.step_interval as i16 + *delta as i16).clamp(-12, 12) as i8;
                }
            }
        }
        GenerativeAction::AddLSystemRule(id, symbol, replacement) => {
            if let Some(v) = voice_mut(gen, *id) {
                if let GenerativeAlgorithm::LSystem(ref mut cfg) = v.algorithm {
                    cfg.rules.push((*symbol, replacement.clone()));
                }
            }
        }
        GenerativeAction::RemoveLSystemRule(id, index) => {
            if let Some(v) = voice_mut(gen, *id) {
                if let GenerativeAlgorithm::LSystem(ref mut cfg) = v.algorithm {
                    if *index < cfg.rules.len() {
                        cfg.rules.remove(*index);
                    }
                }
            }
        }
    }

    true
}

fn voice_mut(gen: &mut crate::state::generative::GenerativeState, id: GenVoiceId) -> Option<&mut GenVoice> {
    gen.voices.iter_mut().find(|v| v.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::generative::*;

    fn make_session() -> SessionState {
        SessionState::new()
    }

    #[test]
    fn toggle_enabled() {
        let mut session = make_session();
        assert!(!session.generative.enabled);
        reduce(&GenerativeAction::ToggleEnabled, &mut session);
        assert!(session.generative.enabled);
        reduce(&GenerativeAction::ToggleEnabled, &mut session);
        assert!(!session.generative.enabled);
    }

    #[test]
    fn toggle_capture() {
        let mut session = make_session();
        assert!(!session.generative.capture_enabled);
        reduce(&GenerativeAction::ToggleCapture, &mut session);
        assert!(session.generative.capture_enabled);
    }

    #[test]
    fn add_and_remove_voice() {
        let mut session = make_session();
        assert!(!session.generative.enabled);
        reduce(
            &GenerativeAction::AddVoice(GenerativeAlgorithm::Euclidean(EuclideanConfig::default())),
            &mut session,
        );
        assert_eq!(session.generative.voices.len(), 1);
        let id = session.generative.voices[0].id;
        assert!(session.generative.voices[0].enabled);
        // Engine auto-enables on AddVoice
        assert!(session.generative.enabled);

        reduce(&GenerativeAction::RemoveVoice(id), &mut session);
        assert!(session.generative.voices.is_empty());
    }

    #[test]
    fn toggle_and_mute_voice() {
        let mut session = make_session();
        reduce(
            &GenerativeAction::AddVoice(GenerativeAlgorithm::Euclidean(EuclideanConfig::default())),
            &mut session,
        );
        let id = session.generative.voices[0].id;

        reduce(&GenerativeAction::ToggleVoice(id), &mut session);
        assert!(!session.generative.voices[0].enabled);

        reduce(&GenerativeAction::MuteVoice(id), &mut session);
        assert!(session.generative.voices[0].muted);
    }

    #[test]
    fn adjust_macros_clamped() {
        let mut session = make_session();
        reduce(&GenerativeAction::AdjustDensity(2.0), &mut session);
        assert_eq!(session.generative.macros.density, 1.0);
        reduce(&GenerativeAction::AdjustDensity(-3.0), &mut session);
        assert_eq!(session.generative.macros.density, 0.0);
    }

    #[test]
    fn constraints_scale_lock_toggle() {
        let mut session = make_session();
        assert!(session.generative.constraints.scale_lock);
        reduce(&GenerativeAction::ToggleScaleLock, &mut session);
        assert!(!session.generative.constraints.scale_lock);
    }

    #[test]
    fn pitch_min_max_clamped() {
        let mut session = make_session();
        // pitch_min starts at 36, pitch_max at 96
        reduce(&GenerativeAction::AdjustPitchMin(100), &mut session);
        // Should clamp to pitch_max (96)
        assert_eq!(session.generative.constraints.pitch_min, 96);

        reduce(&GenerativeAction::AdjustPitchMax(-100), &mut session);
        // Should clamp to pitch_min (96)
        assert_eq!(session.generative.constraints.pitch_max, 96);
    }

    #[test]
    fn euclidean_set_pulses() {
        let mut session = make_session();
        reduce(
            &GenerativeAction::AddVoice(GenerativeAlgorithm::Euclidean(EuclideanConfig::default())),
            &mut session,
        );
        let id = session.generative.voices[0].id;

        reduce(&GenerativeAction::SetEuclideanPulses(id, 3), &mut session);
        if let GenerativeAlgorithm::Euclidean(cfg) = &session.generative.voices[0].algorithm {
            assert_eq!(cfg.pulses, 3);
        } else {
            panic!("Expected Euclidean");
        }
    }

    #[test]
    fn euclidean_pulses_clamped_to_steps() {
        let mut session = make_session();
        reduce(
            &GenerativeAction::AddVoice(GenerativeAlgorithm::Euclidean(EuclideanConfig::default())),
            &mut session,
        );
        let id = session.generative.voices[0].id;
        // Default steps=8, try to set pulses=20
        reduce(&GenerativeAction::SetEuclideanPulses(id, 20), &mut session);
        if let GenerativeAlgorithm::Euclidean(cfg) = &session.generative.voices[0].algorithm {
            assert_eq!(cfg.pulses, 8); // clamped to steps
        }
    }

    #[test]
    fn cycle_voice_rate() {
        let mut session = make_session();
        reduce(
            &GenerativeAction::AddVoice(GenerativeAlgorithm::Euclidean(EuclideanConfig::default())),
            &mut session,
        );
        let id = session.generative.voices[0].id;
        let original_rate = *session.generative.voices[0].algorithm.rate();
        reduce(&GenerativeAction::CycleVoiceRate(id), &mut session);
        assert_ne!(*session.generative.voices[0].algorithm.rate(), original_rate);
    }

    #[test]
    fn markov_rest_prob_clamped() {
        let mut session = make_session();
        reduce(
            &GenerativeAction::AddVoice(GenerativeAlgorithm::Markov(MarkovConfig::default())),
            &mut session,
        );
        let id = session.generative.voices[0].id;
        reduce(&GenerativeAction::AdjustMarkovRestProb(id, 2.0), &mut session);
        if let GenerativeAlgorithm::Markov(cfg) = &session.generative.voices[0].algorithm {
            assert_eq!(cfg.rest_probability, 1.0);
        }
    }

    #[test]
    fn lsystem_iterations_capped() {
        let mut session = make_session();
        reduce(
            &GenerativeAction::AddVoice(GenerativeAlgorithm::LSystem(LSystemConfig::default())),
            &mut session,
        );
        let id = session.generative.voices[0].id;
        reduce(&GenerativeAction::SetLSystemIterations(id, 20), &mut session);
        if let GenerativeAlgorithm::LSystem(cfg) = &session.generative.voices[0].algorithm {
            assert_eq!(cfg.iterations, 6);
        }
    }

    #[test]
    fn clear_capture() {
        let mut session = make_session();
        use crate::InstrumentId;
        session.generative.captured_events.push(CapturedGenEvent {
            instrument_id: InstrumentId::new(1),
            pitch: 60,
            velocity: 100,
            duration_ticks: 240,
            tick: 0,
        });
        assert_eq!(session.generative.captured_events.len(), 1);
        reduce(&GenerativeAction::ClearCapture, &mut session);
        assert!(session.generative.captured_events.is_empty());
    }

    #[test]
    fn set_voice_target() {
        let mut session = make_session();
        reduce(
            &GenerativeAction::AddVoice(GenerativeAlgorithm::Euclidean(EuclideanConfig::default())),
            &mut session,
        );
        let id = session.generative.voices[0].id;
        let inst_id = crate::InstrumentId::new(42);
        reduce(&GenerativeAction::SetVoiceTarget(id, Some(inst_id)), &mut session);
        assert_eq!(session.generative.voices[0].target_instrument, Some(inst_id));
    }
}
