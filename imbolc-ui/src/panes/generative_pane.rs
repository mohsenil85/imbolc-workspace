use std::any::Any;

use crate::state::AppState;
use crate::ui::action_id::{ActionId, GenerativeActionId};
use crate::ui::{
    Action, Color, GenerativeAction, InputEvent, Keymap, Pane, Rect, RenderBuf, Style,
};
use imbolc_types::state::generative::{EuclideanConfig, GenerativeAlgorithm};

/// Focus sections in the generative pane
const SECTION_MACROS: usize = 0;
const SECTION_CONSTRAINTS: usize = 1;
const SECTION_VOICES: usize = 2;
const SECTION_COUNT: usize = 3;

/// Macro param indices
const MACRO_DENSITY: usize = 0;
const MACRO_CHAOS: usize = 1;
const MACRO_ENERGY: usize = 2;
const MACRO_MOTION: usize = 3;
const MACRO_COUNT: usize = 4;

/// Constraint param indices
const CONSTRAINT_SCALE_LOCK: usize = 0;
const CONSTRAINT_PITCH_MIN: usize = 1;
const CONSTRAINT_PITCH_MAX: usize = 2;
const CONSTRAINT_MAX_NOTES: usize = 3;
const CONSTRAINT_HUMANIZE_TIME: usize = 4;
const CONSTRAINT_HUMANIZE_VEL: usize = 5;
const CONSTRAINT_COUNT: usize = 6;

/// Voice param indices (per-voice, depends on algorithm)
const VOICE_ENABLED: usize = 0;
const VOICE_MUTED: usize = 1;
const VOICE_TARGET: usize = 2;
const VOICE_ALGORITHM: usize = 3;
const VOICE_RATE: usize = 4;
// Algorithm-specific start at 5
const VOICE_ALG_PARAM_START: usize = 5;

pub struct GenerativePane {
    keymap: Keymap,
    focus_section: usize,
    selected_param: usize,
    selected_voice: usize,
}

impl GenerativePane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            focus_section: SECTION_MACROS,
            selected_param: 0,
            selected_voice: 0,
        }
    }

    fn param_count(&self, state: &AppState) -> usize {
        match self.focus_section {
            SECTION_MACROS => MACRO_COUNT,
            SECTION_CONSTRAINTS => CONSTRAINT_COUNT,
            SECTION_VOICES => {
                let gen = &state.session.generative;
                if let Some(voice) = gen.voices.get(self.selected_voice) {
                    match &voice.algorithm {
                        GenerativeAlgorithm::Euclidean(_) => VOICE_ALG_PARAM_START + 4, // pulses, steps, rotation, pitch_mode
                        GenerativeAlgorithm::Markov(_) => VOICE_ALG_PARAM_START + 3,    // rest_prob, duration_mode, randomize
                        GenerativeAlgorithm::LSystem(_) => VOICE_ALG_PARAM_START + 3,   // axiom, iterations, step_interval
                    }
                } else {
                    0
                }
            }
            _ => 0,
        }
    }
}

impl Default for GenerativePane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for GenerativePane {
    fn id(&self) -> &'static str {
        "generative"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, state: &AppState) -> Action {
        let gen = &state.session.generative;

        match action {
            ActionId::Generative(GenerativeActionId::PrevParam) => {
                self.selected_param = self.selected_param.saturating_sub(1);
                Action::None
            }
            ActionId::Generative(GenerativeActionId::NextParam) => {
                let max = self.param_count(state).saturating_sub(1);
                self.selected_param = (self.selected_param + 1).min(max);
                Action::None
            }
            ActionId::Generative(GenerativeActionId::PrevSection) => {
                self.focus_section = self.focus_section.saturating_sub(1);
                self.selected_param = 0;
                Action::None
            }
            ActionId::Generative(GenerativeActionId::NextSection) => {
                self.focus_section = (self.focus_section + 1).min(SECTION_COUNT - 1);
                self.selected_param = 0;
                Action::None
            }
            ActionId::Generative(GenerativeActionId::PrevVoice) => {
                self.selected_voice = self.selected_voice.saturating_sub(1);
                self.selected_param = 0;
                Action::None
            }
            ActionId::Generative(GenerativeActionId::NextVoice) => {
                if !gen.voices.is_empty() {
                    self.selected_voice =
                        (self.selected_voice + 1).min(gen.voices.len() - 1);
                    self.selected_param = 0;
                }
                Action::None
            }
            ActionId::Generative(GenerativeActionId::ToggleEngine) => {
                Action::Generative(GenerativeAction::ToggleEnabled)
            }
            ActionId::Generative(GenerativeActionId::ToggleCapture) => {
                Action::Generative(GenerativeAction::ToggleCapture)
            }
            ActionId::Generative(GenerativeActionId::CommitCapture) => {
                Action::Generative(GenerativeAction::CommitCapture)
            }
            ActionId::Generative(GenerativeActionId::ClearCapture) => {
                Action::Generative(GenerativeAction::ClearCapture)
            }
            ActionId::Generative(GenerativeActionId::AddVoice) => {
                Action::Generative(GenerativeAction::AddVoice(
                    GenerativeAlgorithm::Euclidean(EuclideanConfig::default()),
                ))
            }
            ActionId::Generative(GenerativeActionId::RemoveVoice) => {
                if let Some(voice) = gen.voices.get(self.selected_voice) {
                    let id = voice.id;
                    if self.selected_voice > 0 && self.selected_voice >= gen.voices.len() - 1 {
                        self.selected_voice -= 1;
                    }
                    Action::Generative(GenerativeAction::RemoveVoice(id))
                } else {
                    Action::None
                }
            }
            ActionId::Generative(GenerativeActionId::ToggleVoice) => {
                if let Some(voice) = gen.voices.get(self.selected_voice) {
                    Action::Generative(GenerativeAction::ToggleVoice(voice.id))
                } else {
                    Action::None
                }
            }
            ActionId::Generative(GenerativeActionId::MuteVoice) => {
                if let Some(voice) = gen.voices.get(self.selected_voice) {
                    Action::Generative(GenerativeAction::MuteVoice(voice.id))
                } else {
                    Action::None
                }
            }
            ActionId::Generative(GenerativeActionId::CycleAlgorithm) => {
                if let Some(voice) = gen.voices.get(self.selected_voice) {
                    let new_alg = match &voice.algorithm {
                        GenerativeAlgorithm::Euclidean(_) => {
                            GenerativeAlgorithm::Markov(Default::default())
                        }
                        GenerativeAlgorithm::Markov(_) => {
                            GenerativeAlgorithm::LSystem(Default::default())
                        }
                        GenerativeAlgorithm::LSystem(_) => {
                            GenerativeAlgorithm::Euclidean(Default::default())
                        }
                    };
                    self.selected_param = 0;
                    Action::Generative(GenerativeAction::SetVoiceAlgorithm(voice.id, new_alg))
                } else {
                    Action::None
                }
            }
            ActionId::Generative(GenerativeActionId::CycleTarget) => {
                if let Some(voice) = gen.voices.get(self.selected_voice) {
                    let instruments = &state.instruments.instruments;
                    if instruments.is_empty() {
                        return Action::None;
                    }
                    let next = match voice.target_instrument {
                        None => Some(instruments[0].id),
                        Some(current) => {
                            let pos = instruments.iter().position(|i| i.id == current);
                            match pos {
                                Some(idx) if idx + 1 < instruments.len() => {
                                    Some(instruments[idx + 1].id)
                                }
                                _ => None,
                            }
                        }
                    };
                    Action::Generative(GenerativeAction::SetVoiceTarget(voice.id, next))
                } else {
                    Action::None
                }
            }
            ActionId::Generative(GenerativeActionId::CycleRate) => {
                if let Some(voice) = gen.voices.get(self.selected_voice) {
                    Action::Generative(GenerativeAction::CycleVoiceRate(voice.id))
                } else {
                    Action::None
                }
            }
            ActionId::Generative(GenerativeActionId::CycleRateReverse) => {
                if let Some(voice) = gen.voices.get(self.selected_voice) {
                    Action::Generative(GenerativeAction::CycleVoiceRateReverse(voice.id))
                } else {
                    Action::None
                }
            }
            ActionId::Generative(GenerativeActionId::CyclePitchMode) => {
                if let Some(voice) = gen.voices.get(self.selected_voice) {
                    Action::Generative(GenerativeAction::CycleEuclideanPitchMode(voice.id))
                } else {
                    Action::None
                }
            }
            ActionId::Generative(GenerativeActionId::Toggle) => {
                self.handle_toggle(state)
            }
            ActionId::Generative(GenerativeActionId::Increase) => {
                self.handle_adjust(state, true)
            }
            ActionId::Generative(GenerativeActionId::Decrease) => {
                self.handle_adjust(state, false)
            }
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let gen = &state.session.generative;

        // Title bar
        let title_style = Style::new().fg(Color::new(180, 140, 255));
        let enabled_str = if gen.enabled { "ON" } else { "OFF" };
        let capture_str = if gen.capture_enabled { "REC" } else { "OFF" };
        let title = format!(
            " Generative Engine [{}]  Capture: {}  Events: {} ",
            enabled_str,
            capture_str,
            gen.captured_events.len()
        );
        buf.draw_line(
            Rect::new(area.x, area.y, area.width, 1),
            &[(&title, title_style)],
        );

        if area.height < 6 {
            return;
        }

        let content_y = area.y + 2;
        let content_h = area.height.saturating_sub(3);

        // Three sections side by side
        let col_w = area.width / 3;
        let macros_area = Rect::new(area.x, content_y, col_w.min(30), content_h);
        let constraints_area = Rect::new(area.x + col_w, content_y, col_w.min(30), content_h);
        let voices_area = Rect::new(area.x + col_w * 2, content_y, area.width - col_w * 2, content_h);

        self.render_macros(macros_area, buf, state);
        self.render_constraints(constraints_area, buf, state);
        self.render_voices(voices_area, buf, state);

        // Section headers with selection indicator
        let sections = ["Macros", "Constraints", "Voices"];
        let header_y = area.y + 1;
        for (i, (name, x)) in sections
            .iter()
            .zip([area.x + 1, area.x + col_w + 1, area.x + col_w * 2 + 1])
            .enumerate()
        {
            let style = if i == self.focus_section {
                Style::new().fg(Color::new(100, 220, 255))
            } else {
                Style::new().fg(Color::DARK_GRAY)
            };
            buf.draw_line(Rect::new(x, header_y, name.len() as u16, 1), &[(name, style)]);
        }
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl GenerativePane {
    fn handle_toggle(&self, state: &AppState) -> Action {
        let gen = &state.session.generative;
        match self.focus_section {
            SECTION_MACROS => Action::None, // macros don't toggle
            SECTION_CONSTRAINTS => {
                if self.selected_param == CONSTRAINT_SCALE_LOCK {
                    Action::Generative(GenerativeAction::ToggleScaleLock)
                } else {
                    Action::None
                }
            }
            SECTION_VOICES => {
                if let Some(voice) = gen.voices.get(self.selected_voice) {
                    match self.selected_param {
                        VOICE_ENABLED => {
                            Action::Generative(GenerativeAction::ToggleVoice(voice.id))
                        }
                        VOICE_MUTED => {
                            Action::Generative(GenerativeAction::MuteVoice(voice.id))
                        }
                        _ => Action::None,
                    }
                } else {
                    Action::None
                }
            }
            _ => Action::None,
        }
    }

    fn handle_adjust(&self, state: &AppState, increase: bool) -> Action {
        let gen = &state.session.generative;
        let delta = if increase { 1.0 } else { -1.0 };
        let small_delta = delta * 0.05;

        match self.focus_section {
            SECTION_MACROS => match self.selected_param {
                MACRO_DENSITY => {
                    Action::Generative(GenerativeAction::AdjustDensity(small_delta))
                }
                MACRO_CHAOS => Action::Generative(GenerativeAction::AdjustChaos(small_delta)),
                MACRO_ENERGY => {
                    Action::Generative(GenerativeAction::AdjustEnergy(small_delta))
                }
                MACRO_MOTION => {
                    Action::Generative(GenerativeAction::AdjustMotion(small_delta))
                }
                _ => Action::None,
            },
            SECTION_CONSTRAINTS => match self.selected_param {
                CONSTRAINT_SCALE_LOCK => {
                    Action::Generative(GenerativeAction::ToggleScaleLock)
                }
                CONSTRAINT_PITCH_MIN => {
                    Action::Generative(GenerativeAction::AdjustPitchMin(delta as i8))
                }
                CONSTRAINT_PITCH_MAX => {
                    Action::Generative(GenerativeAction::AdjustPitchMax(delta as i8))
                }
                CONSTRAINT_MAX_NOTES => {
                    Action::Generative(GenerativeAction::AdjustMaxNotesPerBeat(delta as i8))
                }
                CONSTRAINT_HUMANIZE_TIME => {
                    Action::Generative(GenerativeAction::AdjustHumanizeTiming(small_delta))
                }
                CONSTRAINT_HUMANIZE_VEL => {
                    Action::Generative(GenerativeAction::AdjustHumanizeVelocity(small_delta))
                }
                _ => Action::None,
            },
            SECTION_VOICES => {
                if let Some(voice) = gen.voices.get(self.selected_voice) {
                    let id = voice.id;
                    match self.selected_param {
                        VOICE_ENABLED => Action::Generative(GenerativeAction::ToggleVoice(id)),
                        VOICE_MUTED => Action::Generative(GenerativeAction::MuteVoice(id)),
                        VOICE_TARGET => {
                            // cycle target same as CycleTarget action
                            let instruments = &state.instruments.instruments;
                            if instruments.is_empty() {
                                return Action::None;
                            }
                            let next = if increase {
                                match voice.target_instrument {
                                    None => Some(instruments[0].id),
                                    Some(current) => {
                                        let pos =
                                            instruments.iter().position(|i| i.id == current);
                                        match pos {
                                            Some(idx) if idx + 1 < instruments.len() => {
                                                Some(instruments[idx + 1].id)
                                            }
                                            _ => None,
                                        }
                                    }
                                }
                            } else {
                                match voice.target_instrument {
                                    None => instruments.last().map(|i| i.id),
                                    Some(current) => {
                                        let pos =
                                            instruments.iter().position(|i| i.id == current);
                                        match pos {
                                            Some(0) => None,
                                            Some(idx) => Some(instruments[idx - 1].id),
                                            _ => None,
                                        }
                                    }
                                }
                            };
                            Action::Generative(GenerativeAction::SetVoiceTarget(id, next))
                        }
                        VOICE_ALGORITHM => {
                            let new_alg = if increase {
                                match &voice.algorithm {
                                    GenerativeAlgorithm::Euclidean(_) => {
                                        GenerativeAlgorithm::Markov(Default::default())
                                    }
                                    GenerativeAlgorithm::Markov(_) => {
                                        GenerativeAlgorithm::LSystem(Default::default())
                                    }
                                    GenerativeAlgorithm::LSystem(_) => {
                                        GenerativeAlgorithm::Euclidean(Default::default())
                                    }
                                }
                            } else {
                                match &voice.algorithm {
                                    GenerativeAlgorithm::Euclidean(_) => {
                                        GenerativeAlgorithm::LSystem(Default::default())
                                    }
                                    GenerativeAlgorithm::Markov(_) => {
                                        GenerativeAlgorithm::Euclidean(Default::default())
                                    }
                                    GenerativeAlgorithm::LSystem(_) => {
                                        GenerativeAlgorithm::Markov(Default::default())
                                    }
                                }
                            };
                            Action::Generative(GenerativeAction::SetVoiceAlgorithm(id, new_alg))
                        }
                        VOICE_RATE => {
                            if increase {
                                Action::Generative(GenerativeAction::CycleVoiceRate(id))
                            } else {
                                Action::Generative(GenerativeAction::CycleVoiceRateReverse(id))
                            }
                        }
                        p if p >= VOICE_ALG_PARAM_START => {
                            self.handle_alg_adjust(voice, p - VOICE_ALG_PARAM_START, increase)
                        }
                        _ => Action::None,
                    }
                } else {
                    Action::None
                }
            }
            _ => Action::None,
        }
    }

    fn handle_alg_adjust(
        &self,
        voice: &imbolc_types::state::generative::GenVoice,
        alg_param: usize,
        increase: bool,
    ) -> Action {
        let id = voice.id;
        let delta = if increase { 1i8 } else { -1i8 };

        match &voice.algorithm {
            GenerativeAlgorithm::Euclidean(cfg) => match alg_param {
                0 => {
                    // pulses
                    let new = (cfg.pulses as i16 + delta as i16).clamp(0, cfg.steps as i16) as u8;
                    Action::Generative(GenerativeAction::SetEuclideanPulses(id, new))
                }
                1 => {
                    // steps
                    let new = (cfg.steps as i16 + delta as i16).clamp(1, 32) as u8;
                    Action::Generative(GenerativeAction::SetEuclideanSteps(id, new))
                }
                2 => {
                    // rotation
                    let new =
                        (cfg.rotation as i16 + delta as i16).clamp(0, cfg.steps as i16 - 1) as u8;
                    Action::Generative(GenerativeAction::SetEuclideanRotation(id, new))
                }
                3 => {
                    // pitch mode
                    Action::Generative(GenerativeAction::CycleEuclideanPitchMode(id))
                }
                _ => Action::None,
            },
            GenerativeAlgorithm::Markov(_cfg) => match alg_param {
                0 => {
                    // rest probability
                    let d = if increase { 0.05 } else { -0.05 };
                    Action::Generative(GenerativeAction::AdjustMarkovRestProb(id, d))
                }
                1 => {
                    // duration mode
                    Action::Generative(GenerativeAction::CycleMarkovDurationMode(id))
                }
                2 => {
                    // randomize matrix
                    if increase {
                        Action::Generative(GenerativeAction::RandomizeMarkovMatrix(id))
                    } else {
                        Action::None
                    }
                }
                _ => Action::None,
            },
            GenerativeAlgorithm::LSystem(cfg) => match alg_param {
                0 => {
                    // iterations
                    let new = (cfg.iterations as i16 + delta as i16).clamp(1, 6) as u8;
                    Action::Generative(GenerativeAction::SetLSystemIterations(id, new))
                }
                1 => {
                    // step interval
                    let new = (cfg.step_interval as i16 + delta as i16).clamp(-12, 12) as i8;
                    Action::Generative(GenerativeAction::AdjustLSystemStepInterval(id, new))
                }
                2 => {
                    // note duration (velocity reuse for now)
                    Action::None
                }
                _ => Action::None,
            },
        }
    }

    fn render_macros(&self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let gen = &state.session.generative;
        let macros = &gen.macros;
        let is_focused = self.focus_section == SECTION_MACROS;

        let params: [(&str, f32); MACRO_COUNT] = [
            ("Density", macros.density),
            ("Chaos", macros.chaos),
            ("Energy", macros.energy),
            ("Motion", macros.motion),
        ];

        for (i, (name, value)) in params.iter().enumerate() {
            if i as u16 >= area.height {
                break;
            }
            let y = area.y + i as u16;
            let selected = is_focused && self.selected_param == i;
            let label_style = if selected {
                Style::new().fg(Color::new(100, 220, 255))
            } else {
                Style::new().fg(Color::WHITE)
            };
            let bar = render_bar(*value, 12);
            let val_str = format!("{}: {} {:.0}%", name, bar, value * 100.0);
            buf.draw_line(Rect::new(area.x + 1, y, val_str.len() as u16, 1), &[(&val_str, label_style)]);
        }
    }

    fn render_constraints(&self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let gen = &state.session.generative;
        let c = &gen.constraints;
        let is_focused = self.focus_section == SECTION_CONSTRAINTS;

        let params: Vec<(String, usize)> = vec![
            (format!("Scale Lock: {}", if c.scale_lock { "ON" } else { "OFF" }), CONSTRAINT_SCALE_LOCK),
            (format!("Pitch Min:  {}", note_name(c.pitch_min)), CONSTRAINT_PITCH_MIN),
            (format!("Pitch Max:  {}", note_name(c.pitch_max)), CONSTRAINT_PITCH_MAX),
            (format!("Max Notes:  {}", if c.max_notes_per_beat == 0 { "Inf".to_string() } else { c.max_notes_per_beat.to_string() }), CONSTRAINT_MAX_NOTES),
            (format!("Hum. Time:  {:.0}%", c.humanize_timing * 100.0), CONSTRAINT_HUMANIZE_TIME),
            (format!("Hum. Vel:   {:.0}%", c.humanize_velocity * 100.0), CONSTRAINT_HUMANIZE_VEL),
        ];

        for (text, idx) in &params {
            let row = *idx;
            if row as u16 >= area.height {
                break;
            }
            let y = area.y + row as u16;
            let selected = is_focused && self.selected_param == *idx;
            let style = if selected {
                Style::new().fg(Color::new(100, 220, 255))
            } else {
                Style::new().fg(Color::WHITE)
            };
            buf.draw_line(Rect::new(area.x + 1, y, text.len() as u16, 1), &[(text, style)]);
        }
    }

    fn render_voices(&self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let gen = &state.session.generative;
        let is_focused = self.focus_section == SECTION_VOICES;

        if gen.voices.is_empty() {
            let msg = "(no voices - press 'a' to add)";
            let style = Style::new().fg(Color::DARK_GRAY);
            buf.draw_line(Rect::new(area.x + 1, area.y, msg.len() as u16, 1), &[(msg, style)]);
            return;
        }

        // Show list of voices first, then details of selected
        let mut y = area.y;
        for (i, voice) in gen.voices.iter().enumerate() {
            if y >= area.y + area.height {
                break;
            }
            let is_selected_voice = i == self.selected_voice;
            let marker = if is_selected_voice { ">" } else { " " };
            let enabled = if voice.enabled { "+" } else { "-" };
            let muted = if voice.muted { "M" } else { " " };
            let target_name = voice
                .target_instrument
                .and_then(|id| state.instruments.instrument(id))
                .map(|i| i.name.as_str())
                .unwrap_or("--");
            let summary = format!(
                "{} {} {} [{}] {} -> {}  {}",
                marker,
                enabled,
                muted,
                voice.algorithm.short_name(),
                voice.algorithm.rate().name(),
                target_name,
                voice.name,
            );
            let style = if is_selected_voice && is_focused {
                Style::new().fg(Color::new(100, 220, 255))
            } else if is_selected_voice {
                Style::new().fg(Color::WHITE)
            } else {
                Style::new().fg(Color::DARK_GRAY)
            };
            let max_w = (area.width.saturating_sub(1)) as usize;
            let truncated: String = summary.chars().take(max_w).collect();
            buf.draw_line(
                Rect::new(area.x + 1, y, truncated.len() as u16, 1),
                &[(&truncated, style)],
            );
            y += 1;
        }

        // Detail section for selected voice
        y += 1; // blank line
        if let Some(voice) = gen.voices.get(self.selected_voice) {
            let detail_params = build_voice_detail_params(voice, state);
            for (i, (label, _value_str)) in detail_params.iter().enumerate() {
                if y >= area.y + area.height {
                    break;
                }
                let selected = is_focused && self.selected_param == i;
                let style = if selected {
                    Style::new().fg(Color::new(100, 220, 255))
                } else {
                    Style::new().fg(Color::WHITE)
                };
                let max_w = (area.width.saturating_sub(1)) as usize;
                let text: String = label.chars().take(max_w).collect();
                buf.draw_line(Rect::new(area.x + 1, y, text.len() as u16, 1), &[(&text, style)]);
                y += 1;
            }
        }
    }
}

fn build_voice_detail_params(
    voice: &imbolc_types::state::generative::GenVoice,
    state: &AppState,
) -> Vec<(String, String)> {
    let target_name = voice
        .target_instrument
        .and_then(|id| state.instruments.instrument(id))
        .map(|i| i.name.clone())
        .unwrap_or_else(|| "--".to_string());

    let mut params = vec![
        (
            format!("Enabled:   {}", if voice.enabled { "ON" } else { "OFF" }),
            String::new(),
        ),
        (
            format!("Muted:     {}", if voice.muted { "YES" } else { "NO" }),
            String::new(),
        ),
        (format!("Target:    {}", target_name), String::new()),
        (
            format!("Algorithm: {}", voice.algorithm.name()),
            String::new(),
        ),
        (
            format!("Rate:      {}", voice.algorithm.rate().name()),
            String::new(),
        ),
    ];

    match &voice.algorithm {
        GenerativeAlgorithm::Euclidean(cfg) => {
            params.push((format!("Pulses:    {}/{}", cfg.pulses, cfg.steps), String::new()));
            params.push((format!("Steps:     {}", cfg.steps), String::new()));
            params.push((format!("Rotation:  {}", cfg.rotation), String::new()));
            params.push((format!("Pitch:     {}", cfg.pitch_mode.name()), String::new()));
        }
        GenerativeAlgorithm::Markov(cfg) => {
            params.push((
                format!("Rest Prob: {:.0}%", cfg.rest_probability * 100.0),
                String::new(),
            ));
            params.push((format!("Duration:  {}", cfg.duration_mode.name()), String::new()));
            params.push(("Randomize: [+]".to_string(), String::new()));
        }
        GenerativeAlgorithm::LSystem(cfg) => {
            params.push((format!("Iterations: {}", cfg.iterations), String::new()));
            params.push((format!("Step Int:   {}", cfg.step_interval), String::new()));
            params.push((format!("Axiom:      {}", &cfg.axiom), String::new()));
        }
    }

    params
}

fn render_bar(value: f32, width: usize) -> String {
    let filled = (value * width as f32).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty))
}

fn note_name(midi: u8) -> String {
    let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = (midi as i16 / 12) - 1;
    let note = midi % 12;
    format!("{}{}", names[note as usize], octave)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::input::KeyCode;

    fn test_state() -> AppState {
        AppState::new()
    }

    fn test_event() -> InputEvent {
        InputEvent::new(KeyCode::Down, crate::ui::input::Modifiers::none())
    }

    #[test]
    fn pane_id() {
        let pane = GenerativePane::default();
        assert_eq!(pane.id(), "generative");
    }

    #[test]
    fn navigate_sections() {
        let mut pane = GenerativePane::default();
        let state = test_state();
        let event = test_event();

        assert_eq!(pane.focus_section, SECTION_MACROS);

        pane.handle_action(
            ActionId::Generative(GenerativeActionId::NextSection),
            &event,
            &state,
        );
        assert_eq!(pane.focus_section, SECTION_CONSTRAINTS);

        pane.handle_action(
            ActionId::Generative(GenerativeActionId::NextSection),
            &event,
            &state,
        );
        assert_eq!(pane.focus_section, SECTION_VOICES);

        // Can't go past last
        pane.handle_action(
            ActionId::Generative(GenerativeActionId::NextSection),
            &event,
            &state,
        );
        assert_eq!(pane.focus_section, SECTION_VOICES);

        pane.handle_action(
            ActionId::Generative(GenerativeActionId::PrevSection),
            &event,
            &state,
        );
        assert_eq!(pane.focus_section, SECTION_CONSTRAINTS);
    }

    #[test]
    fn navigate_params() {
        let mut pane = GenerativePane::default();
        let state = test_state();
        let event = test_event();

        // Macros section has 4 params
        assert_eq!(pane.selected_param, 0);

        pane.handle_action(
            ActionId::Generative(GenerativeActionId::NextParam),
            &event,
            &state,
        );
        assert_eq!(pane.selected_param, 1);

        pane.handle_action(
            ActionId::Generative(GenerativeActionId::PrevParam),
            &event,
            &state,
        );
        assert_eq!(pane.selected_param, 0);

        // Can't go below 0
        pane.handle_action(
            ActionId::Generative(GenerativeActionId::PrevParam),
            &event,
            &state,
        );
        assert_eq!(pane.selected_param, 0);
    }

    #[test]
    fn toggle_engine() {
        let mut pane = GenerativePane::default();
        let state = test_state();
        let event = test_event();

        let action = pane.handle_action(
            ActionId::Generative(GenerativeActionId::ToggleEngine),
            &event,
            &state,
        );
        assert!(matches!(
            action,
            Action::Generative(GenerativeAction::ToggleEnabled)
        ));
    }

    #[test]
    fn toggle_capture() {
        let mut pane = GenerativePane::default();
        let state = test_state();
        let event = test_event();

        let action = pane.handle_action(
            ActionId::Generative(GenerativeActionId::ToggleCapture),
            &event,
            &state,
        );
        assert!(matches!(
            action,
            Action::Generative(GenerativeAction::ToggleCapture)
        ));
    }

    #[test]
    fn add_voice() {
        let mut pane = GenerativePane::default();
        let state = test_state();
        let event = test_event();

        let action = pane.handle_action(
            ActionId::Generative(GenerativeActionId::AddVoice),
            &event,
            &state,
        );
        assert!(matches!(
            action,
            Action::Generative(GenerativeAction::AddVoice(_))
        ));
    }

    #[test]
    fn adjust_density() {
        let mut pane = GenerativePane::default();
        let state = test_state();
        let event = test_event();

        // Section macros, param 0 = density
        pane.focus_section = SECTION_MACROS;
        pane.selected_param = MACRO_DENSITY;

        let action = pane.handle_action(
            ActionId::Generative(GenerativeActionId::Increase),
            &event,
            &state,
        );
        assert!(matches!(
            action,
            Action::Generative(GenerativeAction::AdjustDensity(_))
        ));
    }

    #[test]
    fn toggle_scale_lock() {
        let mut pane = GenerativePane::default();
        let state = test_state();
        let event = test_event();

        pane.focus_section = SECTION_CONSTRAINTS;
        pane.selected_param = CONSTRAINT_SCALE_LOCK;

        let action = pane.handle_action(
            ActionId::Generative(GenerativeActionId::Toggle),
            &event,
            &state,
        );
        assert!(matches!(
            action,
            Action::Generative(GenerativeAction::ToggleScaleLock)
        ));
    }

    #[test]
    fn note_name_display() {
        assert_eq!(note_name(60), "C4");
        assert_eq!(note_name(69), "A4");
        assert_eq!(note_name(36), "C2");
        assert_eq!(note_name(96), "C7");
    }

    #[test]
    fn render_bar_display() {
        let bar = render_bar(0.5, 10);
        assert_eq!(bar.chars().count(), 10);
    }
}
