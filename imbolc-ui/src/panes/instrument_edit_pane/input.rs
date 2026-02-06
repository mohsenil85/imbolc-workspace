use super::editing::AdjustMode;
use super::{InstrumentEditPane, Section};
use crate::state::{
    AppState, FilterConfig, FilterType,
};
use crate::ui::{Action, FileSelectAction, InputEvent, InstrumentAction, KeyCode, SessionAction, translate_key};
use crate::ui::action_id::{ActionId, InstrumentEditActionId, ModeActionId};

impl InstrumentEditPane {
    pub(super) fn handle_action_impl(&mut self, action: ActionId, event: &InputEvent, state: &AppState) -> Action {
        match action {
            // Piano mode actions
            ActionId::Mode(ModeActionId::PianoEscape) => {
                self.piano.deactivate();
                Action::ExitPerformanceMode
            }
            ActionId::Mode(ModeActionId::PianoOctaveDown) => { self.piano.octave_down(); Action::None }
            ActionId::Mode(ModeActionId::PianoOctaveUp) => { self.piano.octave_up(); Action::None }
            ActionId::Mode(ModeActionId::PianoKey) | ActionId::Mode(ModeActionId::PianoSpace) => {
                if let KeyCode::Char(c) = event.key {
                    let c = translate_key(c, state.keyboard_layout);
                    if let Some(pitches) = self.piano.key_to_pitches(c) {
                        if pitches.len() == 1 {
                            return Action::Instrument(InstrumentAction::PlayNote(pitches[0], 100));
                        } else {
                            return Action::Instrument(InstrumentAction::PlayNotes(pitches.clone(), 100));
                        }
                    }
                }
                Action::None
            }
            // Pad layer actions
            ActionId::Mode(ModeActionId::PadEscape) => {
                self.pad_keyboard.deactivate();
                Action::ExitPerformanceMode
            }
            ActionId::Mode(ModeActionId::PadKey) => {
                if let KeyCode::Char(c) = event.key {
                    let c = translate_key(c, state.keyboard_layout);
                    if let Some(pad_idx) = self.pad_keyboard.key_to_pad(c) {
                        return Action::Instrument(InstrumentAction::PlayDrumPad(pad_idx));
                    }
                }
                Action::None
            }
            // Text edit layer actions
            ActionId::Mode(ModeActionId::TextConfirm) => {
                let text = self.edit_input.value().to_string();
                let (section, local_idx) = self.row_info(self.selected_row);
                match section {
                    Section::Source => {
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
                    Section::Filter => {
                        if let Some(ref mut f) = self.filter {
                            match local_idx {
                                1 => if let Ok(v) = text.parse::<f32>() { f.cutoff.value = v.clamp(f.cutoff.min, f.cutoff.max); },
                                2 => if let Ok(v) = text.parse::<f32>() { f.resonance.value = v.clamp(f.resonance.min, f.resonance.max); },
                                idx => {
                                    let extra_idx = idx - 3;
                                    if let Some(param) = f.extra_params.get_mut(extra_idx) {
                                        param.parse_and_set(&text);
                                    }
                                }
                            }
                        }
                    }
                    Section::Effects => {
                        if let Some((effect_idx, param_offset)) = self.effect_row_info(local_idx) {
                            if param_offset > 0 {
                                let param_idx = param_offset - 1;
                                if let Some(effect) = self.effects.get_mut(effect_idx) {
                                    if let Some(param) = effect.params.get_mut(param_idx) {
                                        param.parse_and_set(&text);
                                    }
                                }
                            }
                        }
                    }
                    Section::Envelope => {
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
            ActionId::Mode(ModeActionId::TextCancel) => {
                // Restore backup value if we have one
                if let Some(ref backup) = self.edit_backup_value.take() {
                    let (section, local_idx) = self.row_info(self.selected_row);
                    match section {
                        Section::Source => {
                            let param_idx = if self.source.is_sample() {
                                if local_idx == 0 { 0 } else { local_idx - 1 }
                            } else {
                                local_idx
                            };
                            if let Some(param) = self.source_params.get_mut(param_idx) {
                                param.parse_and_set(backup);
                            }
                        }
                        Section::Filter => {
                            if let Some(ref mut f) = self.filter {
                                match local_idx {
                                    1 => if let Ok(v) = backup.parse::<f32>() { f.cutoff.value = v.clamp(f.cutoff.min, f.cutoff.max); },
                                    2 => if let Ok(v) = backup.parse::<f32>() { f.resonance.value = v.clamp(f.resonance.min, f.resonance.max); },
                                    idx => {
                                        let extra_idx = idx - 3;
                                        if let Some(param) = f.extra_params.get_mut(extra_idx) {
                                            param.parse_and_set(backup);
                                        }
                                    }
                                }
                            }
                        }
                        Section::Effects => {
                            if let Some((effect_idx, param_offset)) = self.effect_row_info(local_idx) {
                                if param_offset > 0 {
                                    let param_idx = param_offset - 1;
                                    if let Some(effect) = self.effects.get_mut(effect_idx) {
                                        if let Some(param) = effect.params.get_mut(param_idx) {
                                            param.parse_and_set(backup);
                                        }
                                    }
                                }
                            }
                        }
                        Section::Envelope => {
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
            // Normal pane actions
            ActionId::InstrumentEdit(InstrumentEditActionId::Done) => {
                self.emit_update()
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::Next) => {
                let total = self.total_rows();
                if total > 0 {
                    self.selected_row = (self.selected_row + 1) % total;
                }
                Action::None
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::Prev) => {
                let total = self.total_rows();
                if total > 0 {
                    self.selected_row = if self.selected_row == 0 { total - 1 } else { self.selected_row - 1 };
                }
                Action::None
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::Increase) => {
                self.adjust_value(true, false);
                self.emit_update()
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::Decrease) => {
                self.adjust_value(false, false);
                self.emit_update()
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::IncreaseBig) => {
                self.adjust_value(true, true);
                self.emit_update()
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::DecreaseBig) => {
                self.adjust_value(false, true);
                self.emit_update()
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::IncreaseTiny) => {
                self.adjust_value_with_mode(true, AdjustMode::Tiny, state.session.tuning_a4);
                self.emit_update()
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::DecreaseTiny) => {
                self.adjust_value_with_mode(false, AdjustMode::Tiny, state.session.tuning_a4);
                self.emit_update()
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::IncreaseMusical) => {
                self.adjust_value_with_mode(true, AdjustMode::Musical, state.session.tuning_a4);
                self.emit_update()
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::DecreaseMusical) => {
                self.adjust_value_with_mode(false, AdjustMode::Musical, state.session.tuning_a4);
                self.emit_update()
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::EnterEdit) => {
                let (section, local_idx) = self.row_info(self.selected_row);
                // On the sample row, trigger load_sample instead of text edit
                if self.source.is_sample() {
                    if section == Section::Source && local_idx == 0 {
                        if let Some(id) = self.instrument_id {
                            return Action::Session(SessionAction::OpenFileBrowser(FileSelectAction::LoadPitchedSample(id)));
                        }
                        return Action::None;
                    }
                }
                // Skip text edit on effect header rows
                if section == Section::Effects {
                    if let Some((_, param_offset)) = self.effect_row_info(local_idx) {
                        if param_offset == 0 { return Action::None; }
                    } else {
                        return Action::None;
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
            ActionId::InstrumentEdit(InstrumentEditActionId::ToggleFilter) => {
                if self.filter.is_some() {
                    self.filter = None;
                } else {
                    self.filter = Some(FilterConfig::new(FilterType::Lpf));
                }
                self.emit_update()
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::CycleFilterType) => {
                if let Some(ref mut f) = self.filter {
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
                Action::None
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::AddEffect) => {
                Action::Nav(crate::ui::NavAction::PushPane("add_effect"))
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::RemoveEffect) => {
                let (section, local_idx) = self.row_info(self.selected_row);
                if section == Section::Effects && !self.effects.is_empty() {
                    if let Some((effect_idx, _)) = self.effect_row_info(local_idx) {
                        self.effects.remove(effect_idx);
                        // Clamp selected_row after removal
                        let max = self.total_rows().saturating_sub(1);
                        self.selected_row = self.selected_row.min(max);
                        return self.emit_update();
                    }
                }
                Action::None
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::TogglePoly) => {
                self.polyphonic = !self.polyphonic;
                self.emit_update()
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::ToggleActive) => {
                if self.source.is_audio_input() {
                    self.active = !self.active;
                    self.emit_update()
                } else {
                    Action::None
                }
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::LoadSample) => {
                if self.source.is_sample() {
                    if let Some(id) = self.instrument_id {
                        Action::Session(SessionAction::OpenFileBrowser(FileSelectAction::LoadPitchedSample(id)))
                    } else {
                        Action::None
                    }
                } else {
                    Action::None
                }
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::ZeroParam) => {
                self.zero_current_param();
                self.emit_update()
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::ZeroSection) => {
                self.zero_current_section();
                self.emit_update()
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::ToggleEq) => {
                if self.eq.is_some() {
                    self.eq = None;
                } else {
                    self.eq = Some(crate::state::EqConfig::default());
                }
                self.emit_update()
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::ToggleLfo) => {
                self.lfo.enabled = !self.lfo.enabled;
                self.emit_update()
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::CycleLfoShape) => {
                self.lfo.shape = self.lfo.shape.next();
                self.emit_update()
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::CycleLfoTarget) => {
                self.lfo.target = self.lfo.target.next();
                self.emit_update()
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::VstParams) => {
                let (section, local_idx) = self.row_info(self.selected_row);
                if section == Section::Source && self.source.is_vst() {
                    // Navigate to VST params pane for VST instrument source
                    Action::Nav(crate::ui::NavAction::PushPane("vst_params"))
                } else if section == Section::Effects {
                    let idx = self.effect_row_info(local_idx).map(|(i, _)| i).unwrap_or(0);
                    if let Some(effect) = self.effects.get(idx) {
                        if effect.effect_type.is_vst() {
                            if let Some(instrument_id) = self.instrument_id {
                                Action::Instrument(InstrumentAction::OpenVstEffectParams(instrument_id, effect.id))
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
                    Action::Nav(crate::ui::NavAction::PushPane("vst_params"))
                } else {
                    Action::None
                }
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::NextSection) => {
                // Jump to first row of next section
                let current = self.current_section();
                let skip_env = self.source.is_vst();
                let next = match current {
                    Section::Source => Section::Filter,
                    Section::Filter => Section::Effects,
                    Section::Effects => Section::Lfo,
                    Section::Lfo => if skip_env { Section::Source } else { Section::Envelope },
                    Section::Envelope => Section::Source,
                };
                for i in 0..self.total_rows() {
                    if self.section_for_row(i) == next {
                        self.selected_row = i;
                        break;
                    }
                }
                Action::None
            }
            ActionId::InstrumentEdit(InstrumentEditActionId::PrevSection) => {
                // Jump to first row of previous section
                let current = self.current_section();
                let skip_env = self.source.is_vst();
                let prev = match current {
                    Section::Source => if skip_env { Section::Lfo } else { Section::Envelope },
                    Section::Filter => Section::Source,
                    Section::Effects => Section::Filter,
                    Section::Lfo => Section::Effects,
                    Section::Envelope => Section::Lfo,
                };
                for i in 0..self.total_rows() {
                    if self.section_for_row(i) == prev {
                        self.selected_row = i;
                        break;
                    }
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    pub(super) fn handle_raw_input_impl(&mut self, event: &InputEvent) {
        if self.editing {
            self.edit_input.handle_input(event);
        }
    }

    pub(super) fn handle_mouse_impl(&mut self, event: &crate::ui::MouseEvent) -> Action {
        let total = self.total_rows();
        if total == 0 { return Action::None; }

        match event.kind {
            crate::ui::MouseEventKind::ScrollUp => {
                self.selected_row = if self.selected_row == 0 { total - 1 } else { self.selected_row - 1 };
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
