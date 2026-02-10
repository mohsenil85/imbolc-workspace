use crate::state::AutomationTarget;
use crate::state::instrument::SourceType;
use crate::state::instrument_state::InstrumentState;
use crate::state::session::SessionState;
use super::{save_project, load_project, temp_db_path};

#[test]
fn save_and_load_round_trip_arrangement() {
    use crate::state::arrangement::PlayMode;
    use crate::state::piano_roll::Note;

    let mut session = SessionState::new();
    let mut instruments = InstrumentState::new();
    let inst_id = instruments.add_instrument(SourceType::Saw);

    // Create clips with notes
    let clip_id = session.arrangement.add_clip("Melody".to_string(), inst_id, 480);
    if let Some(clip) = session.arrangement.clip_mut(clip_id) {
        clip.notes.push(Note { tick: 0, pitch: 60, velocity: 100, duration: 120, probability: 1.0 });
        clip.notes.push(Note { tick: 120, pitch: 64, velocity: 80, duration: 120, probability: 0.8 });
    }

    let clip_id2 = session.arrangement.add_clip("Bass".to_string(), inst_id, 960);
    if let Some(clip) = session.arrangement.clip_mut(clip_id2) {
        clip.notes.push(Note { tick: 0, pitch: 36, velocity: 127, duration: 480, probability: 1.0 });
    }

    // Place clips on timeline
    let pid1 = session.arrangement.add_placement(clip_id, inst_id, 0);
    let pid2 = session.arrangement.add_placement(clip_id, inst_id, 480);
    let pid3 = session.arrangement.add_placement(clip_id2, inst_id, 960);

    // Resize one placement
    session.arrangement.resize_placement(pid2, Some(240));

    // Set UI state
    session.arrangement.play_mode = PlayMode::Song;
    session.arrangement.view_start_tick = 100;
    session.arrangement.ticks_per_col = 60;
    session.arrangement.cursor_tick = 480;
    session.arrangement.selected_lane = 0;
    session.arrangement.selected_placement = Some(1);

    // Add piano roll track (required for persistence)
    session.piano_roll.add_track(inst_id);

    let path = temp_db_path();
    save_project(&path, &session, &instruments).expect("save_project");
    let (loaded_session, _loaded_instruments) = load_project(&path).expect("load_project");

    let arr = &loaded_session.arrangement;

    // Clips
    assert_eq!(arr.clips.len(), 2);
    let lc1 = arr.clip(clip_id).expect("clip 1 missing");
    assert_eq!(lc1.name, "Melody");
    assert_eq!(lc1.instrument_id, inst_id);
    assert_eq!(lc1.length_ticks, 480);
    assert_eq!(lc1.notes.len(), 2);
    assert_eq!(lc1.notes[0].pitch, 60);
    assert_eq!(lc1.notes[1].pitch, 64);
    assert!((lc1.notes[1].probability - 0.8).abs() < 0.01);

    let lc2 = arr.clip(clip_id2).expect("clip 2 missing");
    assert_eq!(lc2.name, "Bass");
    assert_eq!(lc2.length_ticks, 960);
    assert_eq!(lc2.notes.len(), 1);

    // Placements
    assert_eq!(arr.placements.len(), 3);
    let lp1 = arr.placements.iter().find(|p| p.id == pid1).expect("placement 1");
    assert_eq!(lp1.start_tick, 0);
    assert_eq!(lp1.length_override, None);

    let lp2 = arr.placements.iter().find(|p| p.id == pid2).expect("placement 2");
    assert_eq!(lp2.start_tick, 480);
    assert_eq!(lp2.length_override, Some(240));

    let lp3 = arr.placements.iter().find(|p| p.id == pid3).expect("placement 3");
    assert_eq!(lp3.clip_id, clip_id2);
    assert_eq!(lp3.start_tick, 960);

    // Settings
    assert_eq!(arr.play_mode, PlayMode::Song);
    assert_eq!(arr.view_start_tick, 100);
    assert_eq!(arr.ticks_per_col, 60);
    assert_eq!(arr.cursor_tick, 480);
    assert_eq!(arr.selected_lane, 0);
    assert_eq!(arr.selected_placement, Some(1));

    std::fs::remove_file(&path).ok();
}

#[test]
fn round_trip_arrangement_clips() {
    use crate::state::arrangement::PlayMode;
    use crate::state::piano_roll::Note;

    let mut session = SessionState::new();
    let mut instruments = InstrumentState::new();
    let inst_id = instruments.add_instrument(SourceType::Saw);
    session.piano_roll.add_track(inst_id);

    let clip_id = session.arrangement.add_clip("Loop".to_string(), inst_id, 960);
    if let Some(clip) = session.arrangement.clip_mut(clip_id) {
        clip.notes.push(Note { tick: 0, pitch: 48, velocity: 100, duration: 240, probability: 1.0 });
        clip.notes.push(Note { tick: 480, pitch: 52, velocity: 80, duration: 240, probability: 0.5 });
    }

    let _pid = session.arrangement.add_placement(clip_id, inst_id, 0);
    session.arrangement.add_placement(clip_id, inst_id, 960);

    session.arrangement.play_mode = PlayMode::Song;

    let path = temp_db_path();
    save_project(&path, &session, &instruments).expect("save");
    let (loaded, _) = load_project(&path).expect("load");

    let arr = &loaded.arrangement;
    assert_eq!(arr.clips.len(), 1);
    let clip = arr.clip(clip_id).unwrap();
    assert_eq!(clip.name, "Loop");
    assert_eq!(clip.notes.len(), 2);
    assert!((clip.notes[1].probability - 0.5).abs() < 0.01);
    assert_eq!(arr.placements.len(), 2);
    assert_eq!(arr.play_mode, PlayMode::Song);

    std::fs::remove_file(&path).ok();
}

#[test]
fn round_trip_automation_with_curves() {
    let mut session = SessionState::new();
    let instruments = InstrumentState::new();

    let lane_id = session.automation.add_lane(AutomationTarget::bpm());
    let lane = session.automation.lane_mut(lane_id).unwrap();
    lane.add_point(0, 0.0);
    if let Some(p) = lane.point_at_mut(0) {
        p.curve = crate::state::automation::CurveType::Exponential;
    }
    lane.add_point(480, 0.5);
    if let Some(p) = lane.point_at_mut(480) {
        p.curve = crate::state::automation::CurveType::SCurve;
    }
    lane.add_point(960, 1.0);

    let path = temp_db_path();
    save_project(&path, &session, &instruments).expect("save");
    let (loaded, _) = load_project(&path).expect("load");

    let loaded_lane = loaded.automation.lane(lane_id).expect("lane missing");
    assert_eq!(loaded_lane.points.len(), 3);
    assert_eq!(loaded_lane.points[0].curve, crate::state::automation::CurveType::Exponential);
    assert_eq!(loaded_lane.points[1].curve, crate::state::automation::CurveType::SCurve);

    // Verify interpolation still works after reload
    let val = loaded_lane.value_at(240);
    assert!(val.is_some());

    std::fs::remove_file(&path).ok();
}
