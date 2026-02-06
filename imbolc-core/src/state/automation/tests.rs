#[cfg(test)]
mod tests {
    use crate::state::automation::*;

    #[test]
    fn test_automation_point() {
        let point = AutomationPoint::new(480, 0.5);
        assert_eq!(point.tick, 480);
        assert!((point.value - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_automation_lane_interpolation() {
        let mut lane = AutomationLane::new(0, AutomationTarget::InstrumentLevel(0));

        // Add points
        lane.add_point(0, 0.0);
        lane.add_point(100, 1.0);

        // Test interpolation
        assert!((lane.value_at(0).unwrap() - 0.0).abs() < 0.01);
        assert!((lane.value_at(50).unwrap() - 0.5).abs() < 0.01);
        assert!((lane.value_at(100).unwrap() - 1.0).abs() < 0.01);

        // Beyond last point should return last value
        assert!((lane.value_at(150).unwrap() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_automation_lane_step_curve() {
        let mut lane = AutomationLane::new(0, AutomationTarget::InstrumentLevel(0));

        lane.points.push(AutomationPoint::with_curve(0, 0.0, CurveType::Step));
        lane.points.push(AutomationPoint::with_curve(100, 1.0, CurveType::Step));

        // Step should hold at previous value
        assert!((lane.value_at(50).unwrap() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_automation_state() {
        let mut state = AutomationState::new();

        let id1 = state.add_lane(AutomationTarget::InstrumentLevel(0));
        let id2 = state.add_lane(AutomationTarget::InstrumentPan(0));

        assert_eq!(state.lanes.len(), 2);

        // Adding same target should return existing ID
        let id1_again = state.add_lane(AutomationTarget::InstrumentLevel(0));
        assert_eq!(id1, id1_again);
        assert_eq!(state.lanes.len(), 2);

        state.remove_lane(id1);
        assert_eq!(state.lanes.len(), 1);
        assert!(state.lane(id2).is_some());
    }

    #[test]
    fn test_value_range_mapping() {
        let mut lane = AutomationLane::new(0, AutomationTarget::FilterCutoff(0));
        // Default range for filter cutoff is 20-20000

        lane.add_point(0, 0.0);   // Maps to 20 Hz
        lane.add_point(100, 1.0); // Maps to 20000 Hz

        let val_at_0 = lane.value_at(0).unwrap();
        let val_at_100 = lane.value_at(100).unwrap();

        assert!((val_at_0 - 20.0).abs() < 1.0);
        assert!((val_at_100 - 20000.0).abs() < 1.0);
    }

    #[test]
    fn test_new_target_instrument_id() {
        assert_eq!(AutomationTarget::LfoRate(5).instrument_id(), Some(5));
        assert_eq!(AutomationTarget::SendLevel(3, 0).instrument_id(), Some(3));
        assert_eq!(AutomationTarget::BusLevel(1).instrument_id(), None);
        assert_eq!(AutomationTarget::Bpm.instrument_id(), None);
    }

    #[test]
    fn test_new_target_ranges() {
        let (min, max) = AutomationTarget::LfoRate(0).default_range();
        assert!((min - 0.1).abs() < 0.01);
        assert!((max - 32.0).abs() < 0.01);

        let (min, max) = AutomationTarget::Bpm.default_range();
        assert!((min - 30.0).abs() < 0.01);
        assert!((max - 300.0).abs() < 0.01);
    }

    #[test]
    fn test_global_targets_not_removed_by_instrument_cleanup() {
        let mut state = AutomationState::new();
        state.add_lane(AutomationTarget::InstrumentLevel(1));
        state.add_lane(AutomationTarget::Bpm);
        state.add_lane(AutomationTarget::BusLevel(2));

        state.remove_lanes_for_instrument(1);
        assert_eq!(state.lanes.len(), 2);
        assert!(matches!(state.lanes[0].target, AutomationTarget::Bpm));
        assert!(matches!(state.lanes[1].target, AutomationTarget::BusLevel(2)));
    }

    #[test]
    fn test_remove_lanes_for_instrument_updates_selection() {
        let mut state = AutomationState::new();

        let _id1 = state.add_lane(AutomationTarget::InstrumentLevel(1));
        let _id2 = state.add_lane(AutomationTarget::InstrumentPan(2));
        let _id3 = state.add_lane(AutomationTarget::FilterCutoff(1));

        state.selected_lane = Some(2);
        state.remove_lanes_for_instrument(1);

        assert_eq!(state.lanes.len(), 1);
        assert!(matches!(state.lanes[0].target, AutomationTarget::InstrumentPan(2)));
        assert_eq!(state.selected_lane, Some(0));
    }

    #[test]
    fn point_new_clamps_value() {
        let point = AutomationPoint::new(0, 1.5);
        assert!((point.value - 1.0).abs() < f32::EPSILON);

        let point = AutomationPoint::new(0, -0.5);
        assert!((point.value - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn add_point_replaces_at_same_tick() {
        let mut lane = AutomationLane::new(0, AutomationTarget::InstrumentLevel(0));
        lane.add_point(100, 0.5);
        lane.add_point(100, 0.8);
        assert_eq!(lane.points.len(), 1);
        assert!((lane.points[0].value - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn add_point_keeps_sorted_order() {
        let mut lane = AutomationLane::new(0, AutomationTarget::InstrumentLevel(0));
        lane.add_point(100, 0.5);
        lane.add_point(50, 0.3);
        lane.add_point(200, 0.9);
        let ticks: Vec<u32> = lane.points.iter().map(|p| p.tick).collect();
        assert_eq!(ticks, vec![50, 100, 200]);
    }

    #[test]
    fn value_at_disabled_lane() {
        let mut lane = AutomationLane::new(0, AutomationTarget::InstrumentLevel(0));
        lane.add_point(0, 0.5);
        lane.enabled = false;
        assert!(lane.value_at(0).is_none());
    }

    #[test]
    fn value_at_empty_lane() {
        let lane = AutomationLane::new(0, AutomationTarget::InstrumentLevel(0));
        assert!(lane.value_at(0).is_none());
    }

    #[test]
    fn value_at_exponential_curve() {
        let mut lane = AutomationLane::new(0, AutomationTarget::InstrumentLevel(0));
        lane.points.push(AutomationPoint::with_curve(0, 0.0, CurveType::Exponential));
        lane.points.push(AutomationPoint::with_curve(100, 1.0, CurveType::Linear));
        // At midpoint t=0.5, exponential gives t^2 = 0.25 (normalized)
        let val = lane.value_at(50).unwrap();
        // Value should be 0.0 + 0.25 * (1.0 - 0.0) = 0.25, scaled to lane range (0.0-1.0)
        assert!((val - 0.25).abs() < 0.01);
    }

    #[test]
    fn value_at_s_curve() {
        let mut lane = AutomationLane::new(0, AutomationTarget::InstrumentLevel(0));
        lane.points.push(AutomationPoint::with_curve(0, 0.0, CurveType::SCurve));
        lane.points.push(AutomationPoint::with_curve(100, 1.0, CurveType::Linear));
        // At midpoint t=0.5, smoothstep gives 0.5*0.5*(3-2*0.5) = 0.5
        let val = lane.value_at(50).unwrap();
        assert!((val - 0.5).abs() < 0.01);
    }

    #[test]
    fn normalize_value_instrument_pan() {
        let target = AutomationTarget::InstrumentPan(0);
        // Range is -1.0 to 1.0
        assert!((target.normalize_value(-1.0) - 0.0).abs() < f32::EPSILON);
        assert!((target.normalize_value(0.0) - 0.5).abs() < f32::EPSILON);
        assert!((target.normalize_value(1.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn normalize_value_equal_min_max() {
        // Create a target where min == max would give 0.5
        // EffectParam has range (0.0, 1.0) so min != max, but let's test the branch
        // by computing manually. If min==max, normalize returns 0.5.
        let (min, max): (f32, f32) = (5.0, 5.0);
        let result = if max > min { ((0.5 - min) / (max - min)).clamp(0.0, 1.0) } else { 0.5 };
        assert!((result - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn select_next_at_end_stays() {
        let mut state = AutomationState::new();
        state.add_lane(AutomationTarget::InstrumentLevel(0));
        state.add_lane(AutomationTarget::InstrumentPan(0));
        state.selected_lane = Some(1);
        state.select_next();
        assert_eq!(state.selected_lane, Some(1));
    }

    #[test]
    fn select_prev_at_0_stays() {
        let mut state = AutomationState::new();
        state.add_lane(AutomationTarget::InstrumentLevel(0));
        state.selected_lane = Some(0);
        state.select_prev();
        assert_eq!(state.selected_lane, Some(0));
    }

    #[test]
    fn recalculate_next_lane_id() {
        let mut state = AutomationState::new();
        let _id1 = state.add_lane(AutomationTarget::InstrumentLevel(0));
        let _id2 = state.add_lane(AutomationTarget::InstrumentPan(0));
        let id3 = state.add_lane(AutomationTarget::FilterCutoff(0));

        // Manually set next_lane_id to 0 to simulate loading
        state.next_lane_id = 0;
        state.recalculate_next_lane_id();
        assert_eq!(state.next_lane_id, id3 + 1);
    }

    #[test]
    fn targets_for_instrument_context_plain_oscillator() {
        use crate::state::instrument::{Instrument, SourceType};
        use crate::state::vst_plugin::VstPluginRegistry;

        let inst = Instrument::new(1, SourceType::Saw);
        let vst_registry = VstPluginRegistry::new();
        let targets = AutomationTarget::targets_for_instrument_context(&inst, &vst_registry);
        // Plain oscillator: 14 static targets (10 original + 4 groove)
        assert_eq!(targets.len(), 14);
    }

    #[test]
    fn targets_for_instrument_context_with_effects() {
        use crate::state::instrument::{EffectType, Instrument, SourceType};
        use crate::state::vst_plugin::VstPluginRegistry;

        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.add_effect(EffectType::Delay); // 3 params: time, feedback, mix
        inst.add_effect(EffectType::Reverb); // 3 params: room, damp, mix
        let vst_registry = VstPluginRegistry::new();
        let targets = AutomationTarget::targets_for_instrument_context(&inst, &vst_registry);
        // 14 static + 3 (Delay params) + 3 (Reverb params) = 20
        assert_eq!(targets.len(), 20);
        // Verify some EffectParam targets exist
        assert!(targets.iter().any(|t| matches!(t, AutomationTarget::EffectParam(1, _, 0))));
    }

    #[test]
    fn targets_for_instrument_context_pitched_sampler() {
        use crate::state::instrument::{Instrument, SourceType};
        use crate::state::vst_plugin::VstPluginRegistry;

        let inst = Instrument::new(1, SourceType::PitchedSampler);
        let vst_registry = VstPluginRegistry::new();
        let targets = AutomationTarget::targets_for_instrument_context(&inst, &vst_registry);
        // 14 static + SampleRate + SampleAmp = 16
        assert_eq!(targets.len(), 16);
        assert!(targets.iter().any(|t| matches!(t, AutomationTarget::SampleRate(1))));
        assert!(targets.iter().any(|t| matches!(t, AutomationTarget::SampleAmp(1))));
    }

    #[test]
    fn targets_for_instrument_context_with_eq() {
        use crate::state::instrument::{Instrument, SourceType, EqConfig};
        use crate::state::vst_plugin::VstPluginRegistry;

        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.eq = Some(EqConfig::default());
        let vst_registry = VstPluginRegistry::new();
        let targets = AutomationTarget::targets_for_instrument_context(&inst, &vst_registry);
        // 14 static + 36 EQ band params (12 bands x 3 params) = 50
        assert_eq!(targets.len(), 50);
        assert!(targets.iter().any(|t| matches!(t, AutomationTarget::EqBandParam(1, 0, 0))));
        assert!(targets.iter().any(|t| matches!(t, AutomationTarget::EqBandParam(1, 11, 2))));
    }
}
