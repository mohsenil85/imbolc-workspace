use super::editing::AdjustMode;
use super::InstrumentEditPane;
use crate::state::{AppState, FilterConfig, FilterType, InstrumentSection};
use crate::ui::action_id::{ActionId, InstrumentEditActionId, ModeActionId};
use crate::ui::{
    translate_key, Action, FileSelectAction, InputEvent, InstrumentAction, KeyCode, PaneId,
    SessionAction,
};
use imbolc_types::ProcessingStage;

impl InstrumentEditPane {
    pub(super) fn handle_action_impl(
        &mut self,
        action: ActionId,
        event: &InputEvent,
        state: &AppState,
    ) -> Action {
        if let ActionId::Mode(mode_action) = action {
            return match mode_action {
                // Piano mode actions
                ModeActionId::PianoEscape => {
                    self.perf.piano.deactivate();
                    Action::ExitPerformanceMode
                }
                ModeActionId::PianoOctaveDown => {
                    self.perf.piano.octave_down();
                    Action::None
                }
                ModeActionId::PianoOctaveUp => {
                    self.perf.piano.octave_up();
                    Action::None
                }
                ModeActionId::PianoKey | ModeActionId::PianoSpace => {
                    if let KeyCode::Char(c) = event.key {
                        let c = translate_key(c, state.keyboard_layout);
                        if let Some(pitches) = self.perf.piano.key_to_pitches(c) {
                            // Check if this is a new press or key repeat (sustain)
                            if let Some(new_pitches) = self.perf.piano.key_pressed(
                                c,
                                pitches.clone(),
                                event.timestamp,
                                event.is_repeat,
                            ) {
                                // NEW press - spawn voice(s)
                                if new_pitches.len() == 1 {
                                    return Action::Instrument(InstrumentAction::PlayNote(
                                        new_pitches[0],
                                        100,
                                    ));
                                } else {
                                    return Action::Instrument(InstrumentAction::PlayNotes(
                                        new_pitches,
                                        100,
                                    ));
                                }
                            }
                            // Key repeat - sustain, no action needed
                        }
                    }
                    Action::None
                }
                // Pad layer actions
                ModeActionId::PadEscape => {
                    self.perf.pad.deactivate();
                    Action::ExitPerformanceMode
                }
                ModeActionId::PadKey => {
                    if let KeyCode::Char(c) = event.key {
                        let c = translate_key(c, state.keyboard_layout);
                        if let Some(pad_idx) = self.perf.pad.key_to_pad(c) {
                            return Action::Instrument(InstrumentAction::PlayDrumPad(pad_idx));
                        }
                    }
                    Action::None
                }
                // Text edit layer actions
                ModeActionId::TextConfirm => {
                    let text = self.edit_input.value().to_string();
                    let (section, local_idx) = self.row_info(self.selected_row);
                    match section {
                        InstrumentSection::Source => {
                            let param_idx = if self.source.is_sample() {
                                if local_idx == 0 {
                                    self.editing = false;
                                    self.edit_input.set_focused(false);
                                    return Action::None;
                                }
                                local_idx - 1
                            } else {
                                local_idx
                            };
                            if let Some(param) = self.source_params.get_mut(param_idx) {
                                param.parse_and_set(&text);
                            }
                        }
                        InstrumentSection::Processing(i) => {
                            match self.processing_chain.get_mut(i) {
                                Some(ProcessingStage::Filter(f)) => match local_idx {
                                    1 => {
                                        if let Ok(v) = text.parse::<f32>() {
                                            f.cutoff.value = v.clamp(f.cutoff.min, f.cutoff.max);
                                        }
                                    }
                                    2 => {
                                        if let Ok(v) = text.parse::<f32>() {
                                            f.resonance.value =
                                                v.clamp(f.resonance.min, f.resonance.max);
                                        }
                                    }
                                    idx => {
                                        let extra_idx = idx - 3;
                                        if let Some(param) = f.extra_params.get_mut(extra_idx) {
                                            param.parse_and_set(&text);
                                        }
                                    }
                                },
                                Some(ProcessingStage::Eq(_)) => {} // EQ — no text edit
                                Some(ProcessingStage::Effect(e)) => {
                                    if local_idx > 0 {
                                        let param_idx = local_idx - 1;
                                        if let Some(param) = e.params.get_mut(param_idx) {
                                            param.parse_and_set(&text);
                                        }
                                    }
                                }
                                None => {}
                            }
                        }
                        InstrumentSection::Envelope => {
                            if let Ok(v) = text.parse::<f32>() {
                                let max = if local_idx == 2 { 1.0 } else { 5.0 };
                                let val = v.clamp(0.0, max);
                                match local_idx {
                                    0 => self.amp_envelope.attack = val,
                                    1 => self.amp_envelope.decay = val,
                                    2 => self.amp_envelope.sustain = val,
                                    3 => self.amp_envelope.release = val,
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                    self.editing = false;
                    self.edit_input.set_focused(false);
                    self.edit_backup_value = None;
                    self.emit_update()
                }
                ModeActionId::TextCancel => {
                    // Restore backup value if we have one
                    if let Some(ref backup) = self.edit_backup_value.take() {
                        let (section, local_idx) = self.row_info(self.selected_row);
                        match section {
                            InstrumentSection::Source => {
                                let param_idx = if self.source.is_sample() {
                                    if local_idx == 0 {
                                        0
                                    } else {
                                        local_idx - 1
                                    }
                                } else {
                                    local_idx
                                };
                                if let Some(param) = self.source_params.get_mut(param_idx) {
                                    param.parse_and_set(backup);
                                }
                            }
                            InstrumentSection::Processing(i) => {
                                match self.processing_chain.get_mut(i) {
                                    Some(ProcessingStage::Filter(f)) => match local_idx {
                                        1 => {
                                            if let Ok(v) = backup.parse::<f32>() {
                                                f.cutoff.value =
                                                    v.clamp(f.cutoff.min, f.cutoff.max);
                                            }
                                        }
                                        2 => {
                                            if let Ok(v) = backup.parse::<f32>() {
                                                f.resonance.value =
                                                    v.clamp(f.resonance.min, f.resonance.max);
                                            }
                                        }
                                        idx => {
                                            let extra_idx = idx - 3;
                                            if let Some(param) = f.extra_params.get_mut(extra_idx) {
                                                param.parse_and_set(backup);
                                            }
                                        }
                                    },
                                    Some(ProcessingStage::Eq(_)) => {}
                                    Some(ProcessingStage::Effect(e)) => {
                                        if local_idx > 0 {
                                            let param_idx = local_idx - 1;
                                            if let Some(param) = e.params.get_mut(param_idx) {
                                                param.parse_and_set(backup);
                                            }
                                        }
                                    }
                                    None => {}
                                }
                            }
                            InstrumentSection::Envelope => {
                                if let Ok(v) = backup.parse::<f32>() {
                                    let max = if local_idx == 2 { 1.0 } else { 5.0 };
                                    let val = v.clamp(0.0, max);
                                    match local_idx {
                                        0 => self.amp_envelope.attack = val,
                                        1 => self.amp_envelope.decay = val,
                                        2 => self.amp_envelope.sustain = val,
                                        3 => self.amp_envelope.release = val,
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    self.editing = false;
                    self.edit_input.set_focused(false);
                    self.emit_update()
                }
                ModeActionId::PaletteConfirm | ModeActionId::PaletteCancel => Action::None,
            };
        }

        let ActionId::InstrumentEdit(action) = action else {
            return Action::None;
        };

        match action {
            // Normal pane actions
            InstrumentEditActionId::Done => self.emit_update(),
            InstrumentEditActionId::Next => {
                let total = self.total_rows();
                if total > 0 {
                    self.selected_row = (self.selected_row + 1) % total;
                }
                Action::None
            }
            InstrumentEditActionId::Prev => {
                let total = self.total_rows();
                if total > 0 {
                    self.selected_row = if self.selected_row == 0 {
                        total - 1
                    } else {
                        self.selected_row - 1
                    };
                }
                Action::None
            }
            InstrumentEditActionId::Increase => {
                self.adjust_value(true, false);
                self.emit_update()
            }
            InstrumentEditActionId::Decrease => {
                self.adjust_value(false, false);
                self.emit_update()
            }
            InstrumentEditActionId::IncreaseBig => {
                self.adjust_value(true, true);
                self.emit_update()
            }
            InstrumentEditActionId::DecreaseBig => {
                self.adjust_value(false, true);
                self.emit_update()
            }
            InstrumentEditActionId::IncreaseTiny => {
                self.adjust_value_with_mode(true, AdjustMode::Tiny, state.session.tuning_a4);
                self.emit_update()
            }
            InstrumentEditActionId::DecreaseTiny => {
                self.adjust_value_with_mode(false, AdjustMode::Tiny, state.session.tuning_a4);
                self.emit_update()
            }
            InstrumentEditActionId::IncreaseMusical => {
                self.adjust_value_with_mode(true, AdjustMode::Musical, state.session.tuning_a4);
                self.emit_update()
            }
            InstrumentEditActionId::DecreaseMusical => {
                self.adjust_value_with_mode(false, AdjustMode::Musical, state.session.tuning_a4);
                self.emit_update()
            }
            InstrumentEditActionId::EnterEdit => {
                let (section, local_idx) = self.row_info(self.selected_row);
                // On the sample row, trigger load_sample instead of text edit
                if self.source.is_sample() && section == InstrumentSection::Source && local_idx == 0
                {
                    if let Some(id) = self.instrument_id {
                        return Action::Session(SessionAction::OpenFileBrowser(
                            FileSelectAction::LoadPitchedSample(id),
                        ));
                    }
                    return Action::None;
                }
                // Skip text edit on effect header rows and EQ rows
                if let InstrumentSection::Processing(i) = section {
                    match self.processing_chain.get(i) {
                        Some(ProcessingStage::Effect(_)) if local_idx == 0 => return Action::None,
                        Some(ProcessingStage::Eq(_)) => return Action::None,
                        Some(ProcessingStage::Filter(_)) if local_idx == 0 => return Action::None, // filter type row
                        None => return Action::None,
                        _ => {}
                    }
                }
                self.edit_backup_value = Some(self.current_value_string());
                self.editing = true;
                let current_val = self.current_value_string();
                self.edit_input.set_value(&current_val);
                self.edit_input.select_all();
                self.edit_input.set_focused(true);
                Action::PushLayer("text_edit")
            }
            InstrumentEditActionId::ToggleFilter => {
                // Find first filter in chain
                if let Some(idx) = self.processing_chain.iter().position(|s| s.is_filter()) {
                    // If cursor is on a filter, remove that one; otherwise remove first
                    let remove_idx =
                        if let InstrumentSection::Processing(i) = self.current_section() {
                            if self.processing_chain.get(i).is_some_and(|s| s.is_filter()) {
                                i
                            } else {
                                idx
                            }
                        } else {
                            idx
                        };
                    self.processing_chain.remove(remove_idx);
                    let max = self.total_rows().saturating_sub(1);
                    self.selected_row = self.selected_row.min(max);
                } else {
                    // Insert filter at index 0
                    self.processing_chain.insert(
                        0,
                        ProcessingStage::Filter(FilterConfig::new(FilterType::Lpf)),
                    );
                }
                self.emit_update()
            }
            InstrumentEditActionId::CycleFilterType => {
                // Find filter to cycle: prefer the one the cursor is on, else first in chain
                let filter_idx = if let InstrumentSection::Processing(i) = self.current_section() {
                    if self.processing_chain.get(i).is_some_and(|s| s.is_filter()) {
                        Some(i)
                    } else {
                        self.processing_chain.iter().position(|s| s.is_filter())
                    }
                } else {
                    self.processing_chain.iter().position(|s| s.is_filter())
                };
                if let Some(idx) = filter_idx {
                    if let Some(ProcessingStage::Filter(f)) = self.processing_chain.get_mut(idx) {
                        f.filter_type = match f.filter_type {
                            FilterType::Lpf => FilterType::Hpf,
                            FilterType::Hpf => FilterType::Bpf,
                            FilterType::Bpf => FilterType::Notch,
                            FilterType::Notch => FilterType::Comb,
                            FilterType::Comb => FilterType::Allpass,
                            FilterType::Allpass => FilterType::Vowel,
                            FilterType::Vowel => FilterType::ResDrive,
                            FilterType::ResDrive => FilterType::Lpf,
                        };
                        f.extra_params = f.filter_type.default_extra_params();
                        return self.emit_update();
                    }
                }
                Action::None
            }
            InstrumentEditActionId::AddEffect => {
                Action::Nav(crate::ui::NavAction::PushPane(PaneId::AddEffect))
            }
            InstrumentEditActionId::RemoveEffect => {
                if let InstrumentSection::Processing(i) = self.current_section() {
                    if self.processing_chain.get(i).is_some_and(|s| s.is_effect()) {
                        self.processing_chain.remove(i);
                        let max = self.total_rows().saturating_sub(1);
                        self.selected_row = self.selected_row.min(max);
                        return self.emit_update();
                    }
                }
                Action::None
            }
            InstrumentEditActionId::TogglePoly => {
                self.polyphonic = !self.polyphonic;
                self.emit_update()
            }
            InstrumentEditActionId::ToggleActive => {
                if self.source.is_audio_input() {
                    self.active = !self.active;
                    self.emit_update()
                } else {
                    Action::None
                }
            }
            InstrumentEditActionId::ToggleChannelConfig => {
                if let Some(id) = self.instrument_id {
                    self.channel_config = self.channel_config.toggle();
                    return Action::Instrument(InstrumentAction::ToggleChannelConfig(id));
                }
                Action::None
            }
            InstrumentEditActionId::LoadSample => {
                if self.source.is_sample() {
                    if let Some(id) = self.instrument_id {
                        Action::Session(SessionAction::OpenFileBrowser(
                            FileSelectAction::LoadPitchedSample(id),
                        ))
                    } else {
                        Action::None
                    }
                } else {
                    Action::None
                }
            }
            InstrumentEditActionId::ZeroParam => {
                self.zero_current_param();
                self.emit_update()
            }
            InstrumentEditActionId::ZeroSection => {
                self.zero_current_section();
                self.emit_update()
            }
            InstrumentEditActionId::ResetParam => {
                self.reset_current_param();
                self.emit_update()
            }
            InstrumentEditActionId::ToggleEq => {
                if let Some(idx) = self.processing_chain.iter().position(|s| s.is_eq()) {
                    self.processing_chain.remove(idx);
                    let max = self.total_rows().saturating_sub(1);
                    self.selected_row = self.selected_row.min(max);
                } else {
                    // Insert EQ after last filter, or at 0
                    let insert_pos = self
                        .processing_chain
                        .iter()
                        .rposition(|s| s.is_filter())
                        .map(|i| i + 1)
                        .unwrap_or(0);
                    self.processing_chain.insert(
                        insert_pos,
                        ProcessingStage::Eq(crate::state::EqConfig::default()),
                    );
                }
                self.emit_update()
            }
            InstrumentEditActionId::ToggleLfo => {
                self.lfo.enabled = !self.lfo.enabled;
                self.emit_update()
            }
            InstrumentEditActionId::CycleLfoShape => {
                self.lfo.shape = self.lfo.shape.next();
                self.emit_update()
            }
            InstrumentEditActionId::CycleLfoTarget => {
                self.lfo.target = self.lfo.target.next_lfo_target();
                self.emit_update()
            }
            InstrumentEditActionId::VstParams => {
                let (section, _local_idx) = self.row_info(self.selected_row);
                if section == InstrumentSection::Source && self.source.is_vst() {
                    // Navigate to VST params pane for VST instrument source
                    Action::Nav(crate::ui::NavAction::PushPane(PaneId::VstParams))
                } else if let InstrumentSection::Processing(i) = section {
                    if let Some(ProcessingStage::Effect(effect)) = self.processing_chain.get(i) {
                        if effect.effect_type.is_vst() {
                            if let Some(instrument_id) = self.instrument_id {
                                Action::Instrument(InstrumentAction::OpenVstEffectParams(
                                    instrument_id,
                                    effect.id,
                                ))
                            } else {
                                Action::None
                            }
                        } else {
                            Action::None
                        }
                    } else {
                        Action::None
                    }
                } else if self.source.is_vst() {
                    // Fallback: if source is VST, navigate to source params
                    Action::Nav(crate::ui::NavAction::PushPane(PaneId::VstParams))
                } else {
                    Action::None
                }
            }
            InstrumentEditActionId::NextSection => {
                let current = self.current_section();
                let skip_env = self.source.is_vst();
                let n = self.processing_chain.len();
                let next = match current {
                    InstrumentSection::Source => {
                        if n > 0 {
                            InstrumentSection::Processing(0)
                        } else {
                            InstrumentSection::Lfo
                        }
                    }
                    InstrumentSection::Processing(i) => {
                        if i + 1 < n {
                            InstrumentSection::Processing(i + 1)
                        } else {
                            InstrumentSection::Lfo
                        }
                    }
                    InstrumentSection::Lfo => {
                        if skip_env {
                            InstrumentSection::Source
                        } else {
                            InstrumentSection::Envelope
                        }
                    }
                    InstrumentSection::Envelope => InstrumentSection::Source,
                };
                for i in 0..self.total_rows() {
                    if self.section_for_row(i) == next {
                        self.selected_row = i;
                        break;
                    }
                }
                Action::None
            }
            InstrumentEditActionId::PrevSection => {
                let current = self.current_section();
                let skip_env = self.source.is_vst();
                let n = self.processing_chain.len();
                let prev = match current {
                    InstrumentSection::Source => {
                        if skip_env {
                            InstrumentSection::Lfo
                        } else {
                            InstrumentSection::Envelope
                        }
                    }
                    InstrumentSection::Processing(i) => {
                        if i > 0 {
                            InstrumentSection::Processing(i - 1)
                        } else {
                            InstrumentSection::Source
                        }
                    }
                    InstrumentSection::Lfo => {
                        if n > 0 {
                            InstrumentSection::Processing(n - 1)
                        } else {
                            InstrumentSection::Source
                        }
                    }
                    InstrumentSection::Envelope => InstrumentSection::Lfo,
                };
                for i in 0..self.total_rows() {
                    if self.section_for_row(i) == prev {
                        self.selected_row = i;
                        break;
                    }
                }
                Action::None
            }
            InstrumentEditActionId::MoveStageUp => {
                if let InstrumentSection::Processing(i) = self.current_section() {
                    if i > 0 {
                        let (_, local_idx) = self.row_info(self.selected_row);
                        self.processing_chain.swap(i, i - 1);
                        self.selected_row = self.row_for_processing_stage(i - 1, local_idx);
                        return self.emit_update();
                    }
                }
                Action::None
            }
            InstrumentEditActionId::MoveStageDown => {
                if let InstrumentSection::Processing(i) = self.current_section() {
                    if i + 1 < self.processing_chain.len() {
                        let (_, local_idx) = self.row_info(self.selected_row);
                        self.processing_chain.swap(i, i + 1);
                        self.selected_row = self.row_for_processing_stage(i + 1, local_idx);
                        return self.emit_update();
                    }
                }
                Action::None
            }
            InstrumentEditActionId::ToggleEffectBypass => {
                if let InstrumentSection::Processing(i) = self.current_section() {
                    if let Some(ProcessingStage::Effect(e)) = self.processing_chain.get_mut(i) {
                        e.enabled = !e.enabled;
                        return self.emit_update();
                    }
                }
                Action::None
            }
        }
    }

    pub(super) fn handle_raw_input_impl(&mut self, event: &InputEvent) {
        if self.editing {
            self.edit_input.handle_input(event);
        }
    }

    pub(super) fn handle_mouse_impl(&mut self, event: &crate::ui::MouseEvent) -> Action {
        let total = self.total_rows();
        if total == 0 {
            return Action::None;
        }

        match event.kind {
            crate::ui::MouseEventKind::ScrollUp => {
                self.selected_row = if self.selected_row == 0 {
                    total - 1
                } else {
                    self.selected_row - 1
                };
                Action::None
            }
            crate::ui::MouseEventKind::ScrollDown => {
                self.selected_row = (self.selected_row + 1) % total;
                Action::None
            }
            _ => Action::None,
        }
    }
}
