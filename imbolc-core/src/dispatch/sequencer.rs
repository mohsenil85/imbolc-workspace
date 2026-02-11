use crate::action::{
    AudioEffect, ChopperAction, DispatchResult, NavIntent, PaneId, SequencerAction,
};
use crate::state::drum_sequencer::{euclidean_rhythm, DrumPattern, DrumStep};
use crate::state::sampler::Slice;
use crate::state::{AppState, ClipboardContents};
use imbolc_audio::AudioHandle;

use super::helpers::compute_waveform_peaks;

pub(super) fn dispatch_sequencer(
    action: &SequencerAction,
    state: &mut AppState,
    audio: &mut AudioHandle,
) -> DispatchResult {
    match action {
        SequencerAction::ToggleStep(pad_idx, step_idx) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if let Some(step) = seq
                    .pattern_mut()
                    .steps
                    .get_mut(*pad_idx)
                    .and_then(|s| s.get_mut(*step_idx))
                {
                    step.active = !step.active;
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::AdjustVelocity(pad_idx, step_idx, delta) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if let Some(step) = seq
                    .pattern_mut()
                    .steps
                    .get_mut(*pad_idx)
                    .and_then(|s| s.get_mut(*step_idx))
                {
                    step.velocity = (step.velocity as i16 + *delta as i16).clamp(1, 127) as u8;
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::ClearPad(pad_idx) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                for step in seq
                    .pattern_mut()
                    .steps
                    .get_mut(*pad_idx)
                    .iter_mut()
                    .flat_map(|s| s.iter_mut())
                {
                    step.active = false;
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::ClearPattern => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                let len = seq.pattern().length;
                *seq.pattern_mut() = DrumPattern::new(len);
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::CyclePatternLength => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                let lengths = [8, 16, 32, 64];
                let current = seq.pattern().length;
                let idx = lengths.iter().position(|&l| l == current).unwrap_or(0);
                let new_len = lengths[(idx + 1) % lengths.len()];
                let old_pattern = seq.pattern().clone();
                let mut new_pattern = DrumPattern::new(new_len);
                for (pad_idx, old_steps) in old_pattern.steps.iter().enumerate() {
                    for (step_idx, step) in old_steps.iter().enumerate() {
                        if step_idx < new_len {
                            new_pattern.steps[pad_idx][step_idx] = step.clone();
                        }
                    }
                }
                *seq.pattern_mut() = new_pattern;
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::NextPattern => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                seq.current_pattern = (seq.current_pattern + 1) % seq.patterns.len();
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::PrevPattern => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                seq.current_pattern = if seq.current_pattern == 0 {
                    seq.patterns.len() - 1
                } else {
                    seq.current_pattern - 1
                };
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::AdjustPadLevel(pad_idx, delta) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if let Some(pad) = seq.pads.get_mut(*pad_idx) {
                    pad.level = (pad.level + delta).clamp(0.0, 1.0);
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::PlayStop => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                seq.playing = !seq.playing;
                if !seq.playing {
                    seq.current_step = 0;
                    seq.step_accumulator = 0.0;
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::LoadSample(pad_idx) => DispatchResult::with_nav(
            NavIntent::OpenFileBrowser(crate::action::FileSelectAction::LoadDrumSample(*pad_idx)),
        ),
        SequencerAction::AdjustSwing(delta) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                seq.swing_amount = (seq.swing_amount + delta).clamp(0.0, 1.0);
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::ApplyEuclidean {
            pad,
            pulses,
            steps,
            rotation,
        } => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                let pattern_len = seq.pattern().length;
                let rhythm = euclidean_rhythm(*pulses, *steps, *rotation);
                if let Some(pad_steps) = seq.pattern_mut().steps.get_mut(*pad) {
                    for (i, step) in pad_steps.iter_mut().enumerate() {
                        step.active = rhythm.get(i % rhythm.len()).copied().unwrap_or(false);
                    }
                    // If steps param differs from pattern length, only write up to pattern_len
                    let _ = pattern_len; // used above via rhythm indexing
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::AdjustProbability(pad_idx, step_idx, delta) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if let Some(step) = seq
                    .pattern_mut()
                    .steps
                    .get_mut(*pad_idx)
                    .and_then(|s| s.get_mut(*step_idx))
                {
                    step.probability = (step.probability + delta).clamp(0.0, 1.0);
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::LoadSampleResult(pad_idx, path) => {
            let path_str = path.to_string_lossy().to_string();
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            // Allocate from global counter to avoid ID collisions after instrument deletion
            let buffer_id = state.instruments.next_sampler_buffer_id;
            state.instruments.next_sampler_buffer_id += 1;

            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if audio.is_running() {
                    let _ = audio.load_sample(buffer_id, &path_str);
                }

                if let Some(pad) = seq.pads.get_mut(*pad_idx) {
                    pad.buffer_id = Some(buffer_id);
                    pad.path = Some(path_str);
                    pad.name = name;
                }
            }

            let mut result = DispatchResult::with_nav(NavIntent::Pop);
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::ToggleChain => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                seq.chain_enabled = !seq.chain_enabled;
                seq.chain_position = 0;
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::AddChainStep(pattern_index) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if *pattern_index < seq.patterns.len() {
                    seq.chain.push(*pattern_index);
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::RemoveChainStep(position) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if *position < seq.chain.len() {
                    seq.chain.remove(*position);
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::MoveChainStep(from, to) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                let from = *from;
                let to = *to;
                if from < seq.chain.len() && to < seq.chain.len() {
                    let item = seq.chain.remove(from);
                    seq.chain.insert(to, item);
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::ToggleReverse(pad_idx) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if let Some(pad) = seq.pads.get_mut(*pad_idx) {
                    pad.reverse = !pad.reverse;
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::AdjustPadPitch(pad_idx, delta) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if let Some(pad) = seq.pads.get_mut(*pad_idx) {
                    pad.pitch = (pad.pitch as i16 + *delta as i16).clamp(-24, 24) as i8;
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::AdjustStepPitch(pad_idx, step_idx, delta) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if let Some(step) = seq
                    .pattern_mut()
                    .steps
                    .get_mut(*pad_idx)
                    .and_then(|s| s.get_mut(*step_idx))
                {
                    step.pitch_offset =
                        (step.pitch_offset as i16 + *delta as i16).clamp(-24, 24) as i8;
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::DeleteStepsInRegion {
            start_pad,
            end_pad,
            start_step,
            end_step,
        } => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                let pattern = &mut seq.patterns[seq.current_pattern];
                for pad in *start_pad..=*end_pad {
                    for step in *start_step..=*end_step {
                        if pad < pattern.steps.len() && step < pattern.steps[pad].len() {
                            pattern.steps[pad][step] = DrumStep::default();
                        }
                    }
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::CopySteps {
            start_pad,
            end_pad,
            start_step,
            end_step,
        } => {
            if let Some(seq) = state.instruments.selected_drum_sequencer() {
                let pattern = seq.pattern();
                let mut steps = Vec::new();
                for pad_idx in *start_pad..=*end_pad {
                    for step_idx in *start_step..=*end_step {
                        if pad_idx < pattern.steps.len() && step_idx < pattern.steps[pad_idx].len()
                        {
                            let step = &pattern.steps[pad_idx][step_idx];
                            if step.active {
                                steps.push((
                                    pad_idx - start_pad,
                                    step_idx - start_step,
                                    step.clone(),
                                ));
                            }
                        }
                    }
                }
                if !steps.is_empty() {
                    state.clipboard.contents = Some(ClipboardContents::DrumSteps { steps });
                }
            }
            DispatchResult::none()
        }
        SequencerAction::PasteSteps {
            anchor_pad,
            anchor_step,
            steps,
        } => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                let pattern = &mut seq.patterns[seq.current_pattern];
                for (pad_offset, step_offset, step_data) in steps {
                    let pad = anchor_pad + pad_offset;
                    let step = anchor_step + step_offset;
                    if pad < pattern.steps.len() && step < pattern.steps[pad].len() {
                        pattern.steps[pad][step] = step_data.clone();
                    }
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::SetPadInstrument(pad_idx, instrument_id, freq) => {
            // Get target instrument name first (before mutable borrow)
            let target_name = state
                .instruments
                .instrument(*instrument_id)
                .map(|i| i.name.clone());

            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if let Some(pad) = seq.pads.get_mut(*pad_idx) {
                    // Clear sample source when switching to instrument
                    pad.buffer_id = None;
                    pad.path = None;
                    // Set instrument trigger
                    pad.instrument_id = Some(*instrument_id);
                    pad.trigger_freq = *freq;
                    // Update name to target instrument name
                    if let Some(name) = target_name {
                        pad.name = name;
                    }
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::ClearPadInstrument(pad_idx) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if let Some(pad) = seq.pads.get_mut(*pad_idx) {
                    pad.instrument_id = None;
                    pad.trigger_freq = 440.0;
                    // Keep other fields (name, etc.) as-is
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::SetPadTriggerFreq(pad_idx, freq) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if let Some(pad) = seq.pads.get_mut(*pad_idx) {
                    pad.trigger_freq = *freq;
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        SequencerAction::OpenInstrumentPicker(pad_idx) => {
            // Store which pad we're editing, then open the picker
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                seq.editing_pad = Some(*pad_idx);
            }
            DispatchResult::with_nav(NavIntent::PushTo(PaneId::InstrumentPicker))
        }
        SequencerAction::CycleStepResolution => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                seq.step_resolution = seq.step_resolution.cycle_next();
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
    }
}

pub(super) fn dispatch_chopper(
    action: &ChopperAction,
    state: &mut AppState,
    audio: &mut AudioHandle,
) -> DispatchResult {
    match action {
        ChopperAction::LoadSample => DispatchResult::with_nav(NavIntent::OpenFileBrowser(
            crate::action::FileSelectAction::LoadChopperSample,
        )),
        ChopperAction::LoadSampleResult(path) => {
            let path_str = path.to_string_lossy().to_string();
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            // Compute waveform peaks from WAV file
            let (peaks, duration_secs) = compute_waveform_peaks(&path_str);

            // Allocate from global counter to avoid ID collisions after instrument deletion
            let buffer_id = state.instruments.next_sampler_buffer_id;
            state.instruments.next_sampler_buffer_id += 1;

            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if audio.is_running() {
                    let _ = audio.load_sample(buffer_id, &path_str);
                }

                let initial_slice = Slice::full(0);
                seq.chopper = Some(crate::state::drum_sequencer::ChopperState {
                    buffer_id: Some(buffer_id),
                    path: Some(path_str),
                    name,
                    slices: vec![initial_slice],
                    selected_slice: 0,
                    next_slice_id: 1,
                    waveform_peaks: peaks,
                    duration_secs,
                });
            }

            let mut result =
                DispatchResult::with_nav(NavIntent::ConditionalPop(PaneId::FileBrowser));
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        ChopperAction::AddSlice(cursor_pos) => {
            let cursor_pos = *cursor_pos;
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if let Some(chopper) = &mut seq.chopper {
                    // Find which slice contains cursor_pos
                    if let Some(idx) = chopper
                        .slices
                        .iter()
                        .position(|s| s.start <= cursor_pos && s.end > cursor_pos)
                    {
                        let old_end = chopper.slices[idx].end;
                        chopper.slices[idx].end = cursor_pos;

                        let new_id = chopper.next_slice_id;
                        chopper.next_slice_id += 1;
                        let new_slice = Slice::new(new_id, cursor_pos, old_end);
                        chopper.slices.insert(idx + 1, new_slice);
                        chopper.selected_slice = idx + 1;
                    }
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        ChopperAction::RemoveSlice => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if let Some(chopper) = &mut seq.chopper {
                    if chopper.slices.len() > 1 {
                        let idx = chopper.selected_slice;
                        let removed = chopper.slices.remove(idx);
                        if idx > 0 {
                            // Extend previous slice's end to cover gap
                            chopper.slices[idx - 1].end = removed.end;
                            chopper.selected_slice = idx - 1;
                        } else if !chopper.slices.is_empty() {
                            // Extend next slice's start to cover gap
                            chopper.slices[0].start = removed.start;
                            chopper.selected_slice = 0;
                        }
                    }
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        ChopperAction::AssignToPad(pad_idx) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                let assign_data = seq.chopper.as_ref().and_then(|c| {
                    c.slices
                        .get(c.selected_slice)
                        .map(|s| (c.buffer_id, s.start, s.end))
                });
                if let Some((buffer_id, start, end)) = assign_data {
                    if let Some(pad) = seq.pads.get_mut(*pad_idx) {
                        pad.buffer_id = buffer_id;
                        pad.slice_start = start;
                        pad.slice_end = end;
                        // Copy name from chopper
                        if let Some(chopper) = &seq.chopper {
                            pad.name = format!("{} {}", chopper.name, chopper.selected_slice + 1);
                            pad.path = chopper.path.clone();
                        }
                    }
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        ChopperAction::AutoSlice(n) => {
            let n = *n;
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if let Some(chopper) = &mut seq.chopper {
                    chopper.slices.clear();
                    for i in 0..n {
                        let start = i as f32 / n as f32;
                        let end = (i + 1) as f32 / n as f32;
                        let id = chopper.next_slice_id;
                        chopper.next_slice_id += 1;
                        chopper.slices.push(Slice::new(id, start, end));
                    }
                    chopper.selected_slice = 0;
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        ChopperAction::PreviewSlice => {
            if let Some(instrument) = state.instruments.selected_instrument() {
                if let Some(seq) = instrument.drum_sequencer() {
                    if let Some(chopper) = &seq.chopper {
                        if let Some(slice) = chopper.slices.get(chopper.selected_slice) {
                            if let Some(buffer_id) = chopper.buffer_id {
                                if audio.is_running() {
                                    let _ = audio.play_drum_hit_to_instrument(
                                        buffer_id,
                                        0.8,
                                        instrument.id,
                                        slice.start,
                                        slice.end,
                                        1.0,
                                        0.0,
                                    );
                                } else {
                                    return DispatchResult::with_status(
                                        imbolc_audio::ServerStatus::Stopped,
                                        "Audio engine not running",
                                    );
                                }
                            }
                        }
                    }
                }
            }
            DispatchResult::none()
        }
        ChopperAction::SelectSlice(delta) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if let Some(chopper) = &mut seq.chopper {
                    if !chopper.slices.is_empty() {
                        let len = chopper.slices.len() as i8;
                        let new_idx =
                            (chopper.selected_slice as i8 + delta).rem_euclid(len) as usize;
                        chopper.selected_slice = new_idx;
                    }
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        ChopperAction::NudgeSliceStart(delta) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if let Some(chopper) = &mut seq.chopper {
                    if let Some(slice) = chopper.slices.get_mut(chopper.selected_slice) {
                        slice.start = (slice.start + delta).clamp(0.0, slice.end - 0.001);
                    }
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        ChopperAction::NudgeSliceEnd(delta) => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if let Some(chopper) = &mut seq.chopper {
                    if let Some(slice) = chopper.slices.get_mut(chopper.selected_slice) {
                        slice.end = (slice.end + delta).clamp(slice.start + 0.001, 1.0);
                    }
                }
            }
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        ChopperAction::CommitAll => {
            if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
                if let Some(chopper) = &seq.chopper {
                    let assignments: Vec<_> = chopper
                        .slices
                        .iter()
                        .enumerate()
                        .take(crate::state::drum_sequencer::NUM_PADS)
                        .map(|(i, s)| {
                            (
                                i,
                                chopper.buffer_id,
                                s.start,
                                s.end,
                                chopper.name.clone(),
                                chopper.path.clone(),
                            )
                        })
                        .collect();
                    for (i, buffer_id, start, end, name, path) in assignments {
                        if let Some(pad) = seq.pads.get_mut(i) {
                            pad.buffer_id = buffer_id;
                            pad.slice_start = start;
                            pad.slice_end = end;
                            pad.name = format!("{} {}", name, i + 1);
                            pad.path = path;
                        }
                    }
                }
            }
            let mut result = DispatchResult::with_nav(NavIntent::Pop);
            result.audio_effects.push(AudioEffect::RebuildInstruments);
            result
        }
        ChopperAction::MoveCursor(_) => {
            // Cursor tracked locally in pane
            DispatchResult::none()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use imbolc_audio::AudioHandle;

    fn setup() -> (AppState, AudioHandle) {
        let mut state = AppState::new();
        state.add_instrument(crate::state::SourceType::Kit);
        state.instruments.selected = Some(0);
        (state, AudioHandle::new())
    }

    #[test]
    fn toggle_step() {
        let (mut state, mut audio) = setup();
        dispatch_sequencer(&SequencerAction::ToggleStep(0, 0), &mut state, &mut audio);
        let seq = state.instruments.selected_drum_sequencer().unwrap();
        assert!(seq.pattern().steps[0][0].active);

        dispatch_sequencer(&SequencerAction::ToggleStep(0, 0), &mut state, &mut audio);
        let seq = state.instruments.selected_drum_sequencer().unwrap();
        assert!(!seq.pattern().steps[0][0].active);
    }

    #[test]
    fn adjust_velocity_clamps() {
        let (mut state, mut audio) = setup();
        // Default velocity is 100
        dispatch_sequencer(
            &SequencerAction::AdjustVelocity(0, 0, 50),
            &mut state,
            &mut audio,
        );
        let vel = state
            .instruments
            .selected_drum_sequencer()
            .unwrap()
            .pattern()
            .steps[0][0]
            .velocity;
        assert_eq!(vel, 127);

        dispatch_sequencer(
            &SequencerAction::AdjustVelocity(0, 0, -127),
            &mut state,
            &mut audio,
        );
        let vel = state
            .instruments
            .selected_drum_sequencer()
            .unwrap()
            .pattern()
            .steps[0][0]
            .velocity;
        assert_eq!(vel, 1);
    }

    #[test]
    fn clear_pad() {
        let (mut state, mut audio) = setup();
        dispatch_sequencer(&SequencerAction::ToggleStep(0, 0), &mut state, &mut audio);
        dispatch_sequencer(&SequencerAction::ToggleStep(0, 1), &mut state, &mut audio);
        dispatch_sequencer(&SequencerAction::ClearPad(0), &mut state, &mut audio);
        let seq = state.instruments.selected_drum_sequencer().unwrap();
        assert!(seq.pattern().steps[0].iter().all(|s| !s.active));
    }

    #[test]
    fn clear_pattern() {
        let (mut state, mut audio) = setup();
        dispatch_sequencer(&SequencerAction::ToggleStep(0, 0), &mut state, &mut audio);
        dispatch_sequencer(&SequencerAction::ClearPattern, &mut state, &mut audio);
        let seq = state.instruments.selected_drum_sequencer().unwrap();
        assert!(seq
            .pattern()
            .steps
            .iter()
            .all(|pad| pad.iter().all(|s| !s.active)));
    }

    #[test]
    fn cycle_pattern_length() {
        let (mut state, mut audio) = setup();
        // Default: 16
        let expected_lengths = [32, 64, 8, 16];
        for expected in expected_lengths {
            dispatch_sequencer(&SequencerAction::CyclePatternLength, &mut state, &mut audio);
            let len = state
                .instruments
                .selected_drum_sequencer()
                .unwrap()
                .pattern()
                .length;
            assert_eq!(len, expected);
        }
    }

    #[test]
    fn next_prev_pattern_wraps() {
        let (mut state, mut audio) = setup();
        let n_patterns = state
            .instruments
            .selected_drum_sequencer()
            .unwrap()
            .patterns
            .len();
        for _ in 0..n_patterns {
            dispatch_sequencer(&SequencerAction::NextPattern, &mut state, &mut audio);
        }
        assert_eq!(
            state
                .instruments
                .selected_drum_sequencer()
                .unwrap()
                .current_pattern,
            0
        );

        dispatch_sequencer(&SequencerAction::PrevPattern, &mut state, &mut audio);
        assert_eq!(
            state
                .instruments
                .selected_drum_sequencer()
                .unwrap()
                .current_pattern,
            n_patterns - 1
        );
    }

    #[test]
    fn adjust_pad_level_clamps() {
        let (mut state, mut audio) = setup();
        dispatch_sequencer(
            &SequencerAction::AdjustPadLevel(0, 2.0),
            &mut state,
            &mut audio,
        );
        let level = state.instruments.selected_drum_sequencer().unwrap().pads[0].level;
        assert!((level - 1.0).abs() < f32::EPSILON);

        dispatch_sequencer(
            &SequencerAction::AdjustPadLevel(0, -5.0),
            &mut state,
            &mut audio,
        );
        let level = state.instruments.selected_drum_sequencer().unwrap().pads[0].level;
        assert!((level - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn play_stop_toggles_and_resets() {
        let (mut state, mut audio) = setup();
        dispatch_sequencer(&SequencerAction::PlayStop, &mut state, &mut audio);
        assert!(state.instruments.selected_drum_sequencer().unwrap().playing);

        dispatch_sequencer(&SequencerAction::PlayStop, &mut state, &mut audio);
        let seq = state.instruments.selected_drum_sequencer().unwrap();
        assert!(!seq.playing);
        assert_eq!(seq.current_step, 0);
    }

    #[test]
    fn adjust_swing_clamps() {
        let (mut state, mut audio) = setup();
        dispatch_sequencer(&SequencerAction::AdjustSwing(2.0), &mut state, &mut audio);
        assert!(
            (state
                .instruments
                .selected_drum_sequencer()
                .unwrap()
                .swing_amount
                - 1.0)
                .abs()
                < f32::EPSILON
        );
        dispatch_sequencer(&SequencerAction::AdjustSwing(-5.0), &mut state, &mut audio);
        assert!(
            (state
                .instruments
                .selected_drum_sequencer()
                .unwrap()
                .swing_amount
                - 0.0)
                .abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn apply_euclidean() {
        let (mut state, mut audio) = setup();
        // Use steps=16 to match default pattern length, so rhythm doesn't repeat
        dispatch_sequencer(
            &SequencerAction::ApplyEuclidean {
                pad: 0,
                pulses: 4,
                steps: 16,
                rotation: 0,
            },
            &mut state,
            &mut audio,
        );
        let seq = state.instruments.selected_drum_sequencer().unwrap();
        let active_count = seq.pattern().steps[0].iter().filter(|s| s.active).count();
        assert_eq!(active_count, 4);
    }

    #[test]
    fn adjust_probability_clamps() {
        let (mut state, mut audio) = setup();
        dispatch_sequencer(
            &SequencerAction::AdjustProbability(0, 0, -2.0),
            &mut state,
            &mut audio,
        );
        let prob = state
            .instruments
            .selected_drum_sequencer()
            .unwrap()
            .pattern()
            .steps[0][0]
            .probability;
        assert!((prob - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn adjust_pad_pitch_clamps() {
        let (mut state, mut audio) = setup();
        dispatch_sequencer(
            &SequencerAction::AdjustPadPitch(0, 50),
            &mut state,
            &mut audio,
        );
        assert_eq!(
            state.instruments.selected_drum_sequencer().unwrap().pads[0].pitch,
            24
        );
        dispatch_sequencer(
            &SequencerAction::AdjustPadPitch(0, -100),
            &mut state,
            &mut audio,
        );
        assert_eq!(
            state.instruments.selected_drum_sequencer().unwrap().pads[0].pitch,
            -24
        );
    }

    #[test]
    fn adjust_step_pitch_clamps() {
        let (mut state, mut audio) = setup();
        dispatch_sequencer(
            &SequencerAction::AdjustStepPitch(0, 0, 50),
            &mut state,
            &mut audio,
        );
        assert_eq!(
            state
                .instruments
                .selected_drum_sequencer()
                .unwrap()
                .pattern()
                .steps[0][0]
                .pitch_offset,
            24
        );
        dispatch_sequencer(
            &SequencerAction::AdjustStepPitch(0, 0, -100),
            &mut state,
            &mut audio,
        );
        assert_eq!(
            state
                .instruments
                .selected_drum_sequencer()
                .unwrap()
                .pattern()
                .steps[0][0]
                .pitch_offset,
            -24
        );
    }

    #[test]
    fn chain_operations() {
        let (mut state, mut audio) = setup();
        dispatch_sequencer(&SequencerAction::ToggleChain, &mut state, &mut audio);
        assert!(
            state
                .instruments
                .selected_drum_sequencer()
                .unwrap()
                .chain_enabled
        );

        dispatch_sequencer(&SequencerAction::AddChainStep(0), &mut state, &mut audio);
        dispatch_sequencer(&SequencerAction::AddChainStep(1), &mut state, &mut audio);
        assert_eq!(
            state
                .instruments
                .selected_drum_sequencer()
                .unwrap()
                .chain
                .len(),
            2
        );

        dispatch_sequencer(
            &SequencerAction::MoveChainStep(0, 1),
            &mut state,
            &mut audio,
        );
        assert_eq!(
            state.instruments.selected_drum_sequencer().unwrap().chain,
            vec![1, 0]
        );

        dispatch_sequencer(&SequencerAction::RemoveChainStep(0), &mut state, &mut audio);
        assert_eq!(
            state.instruments.selected_drum_sequencer().unwrap().chain,
            vec![0]
        );

        // Out of bounds add — should be no-op (pattern index 99 doesn't exist)
        dispatch_sequencer(&SequencerAction::AddChainStep(99), &mut state, &mut audio);
        assert_eq!(
            state
                .instruments
                .selected_drum_sequencer()
                .unwrap()
                .chain
                .len(),
            1
        );
    }
}
