mod editing;
mod input;
mod rendering;

use std::any::Any;
use std::time::Instant;

use imbolc_types::{ChannelConfig, ProcessingStage};
use crate::state::{
    AppState, EffectSlot, EnvConfig, EqConfig, FilterConfig, Instrument, InstrumentId,
    InstrumentSection, LfoConfig, Param, SourceType,
    instrument::{instrument_row_count, instrument_section_for_row, instrument_row_info},
};
use crate::ui::widgets::TextInput;
use crate::ui::{Rect, RenderBuf, Action, InputEvent, Keymap, MouseEvent, PadKeyboard, Pane, PianoKeyboard, PianoRollAction, ToggleResult};
use crate::ui::action_id::ActionId;

/// Local section enum that preserves the pane's view of filter/effects as
/// separate sections, translating to/from InstrumentSection::Processing(i).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Section {
    Source,
    Filter,
    Effects,
    Lfo,
    Envelope,
}

pub struct InstrumentEditPane {
    keymap: Keymap,
    instrument_id: Option<InstrumentId>,
    instrument_name: String,
    source: SourceType,
    source_params: Vec<Param>,
    sample_name: Option<String>,
    filter: Option<FilterConfig>,
    eq: Option<EqConfig>,
    effects: Vec<EffectSlot>,
    lfo: LfoConfig,
    amp_envelope: EnvConfig,
    polyphonic: bool,
    active: bool,
    channel_config: ChannelConfig,
    pub(crate) selected_row: usize,
    pub(crate) scroll_offset: usize,
    editing: bool,
    edit_input: TextInput,
    edit_backup_value: Option<String>,
    piano: PianoKeyboard,
    pad_keyboard: PadKeyboard,
}

impl InstrumentEditPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            instrument_id: None,
            instrument_name: String::new(),
            source: SourceType::Saw,
            source_params: Vec::new(),
            sample_name: None,
            filter: None,
            eq: None,
            effects: Vec::new(),
            lfo: LfoConfig::default(),
            amp_envelope: EnvConfig::default(),
            polyphonic: true,
            active: true,
            channel_config: ChannelConfig::default(),
            selected_row: 0,
            scroll_offset: 0,
            editing: false,
            edit_input: TextInput::new(""),
            edit_backup_value: None,
            piano: PianoKeyboard::new(),
            pad_keyboard: PadKeyboard::new(),
        }
    }

    pub fn set_instrument(&mut self, instrument: &Instrument) {
        self.instrument_id = Some(instrument.id);
        self.instrument_name = instrument.name.clone();
        self.source = instrument.source;
        self.source_params = instrument.source_params.clone();
        self.sample_name = instrument.sampler_config.as_ref().and_then(|c| c.sample_name.clone());
        self.filter = instrument.filter().cloned();
        self.eq = instrument.eq().cloned();
        self.effects = instrument.effects().cloned().collect();
        self.lfo = instrument.lfo.clone();
        self.amp_envelope = instrument.amp_envelope.clone();
        self.polyphonic = instrument.polyphonic;
        self.active = instrument.active;
        self.channel_config = instrument.channel_config;
        self.selected_row = 0;
        self.scroll_offset = 0;
    }

    /// Re-sync data from an instrument without resetting cursor position.
    /// Used when returning from a sub-pane (e.g. add_effect) where the same
    /// instrument may have changed.
    fn refresh_instrument(&mut self, instrument: &Instrument) {
        self.instrument_id = Some(instrument.id);
        self.instrument_name = instrument.name.clone();
        self.source = instrument.source;
        self.source_params = instrument.source_params.clone();
        self.sample_name = instrument.sampler_config.as_ref().and_then(|c| c.sample_name.clone());
        self.filter = instrument.filter().cloned();
        self.eq = instrument.eq().cloned();
        self.effects = instrument.effects().cloned().collect();
        self.lfo = instrument.lfo.clone();
        self.amp_envelope = instrument.amp_envelope.clone();
        self.polyphonic = instrument.polyphonic;
        self.active = instrument.active;
        self.channel_config = instrument.channel_config;
        // Clamp selected_row to valid range (effects count may have changed)
        let max = self.total_rows().saturating_sub(1);
        self.selected_row = self.selected_row.min(max);
    }

    #[allow(dead_code)]
    pub fn instrument_id(&self) -> Option<InstrumentId> {
        self.instrument_id
    }

    /// Get current tab as index (for view state - now section based)
    pub fn tab_index(&self) -> u8 {
        match self.current_section() {
            Section::Source => 0,
            Section::Filter => 1,
            Section::Effects => 2,
            Section::Lfo => 3,
            Section::Envelope => 4,
        }
    }

    /// Set tab from index (for view state restoration)
    pub fn set_tab_index(&mut self, idx: u8) {
        let target_section = match idx {
            0 => Section::Source,
            1 => Section::Filter,
            2 => Section::Effects,
            3 => Section::Lfo,
            4 => Section::Envelope,
            _ => Section::Source,
        };
        for i in 0..self.total_rows() {
            if self.section_for_row(i) == target_section {
                self.selected_row = i;
                break;
            }
        }
    }

    /// Apply edits back to an instrument
    #[allow(dead_code)]
    pub fn apply_to(&self, instrument: &mut Instrument) {
        instrument.source = self.source;
        instrument.source_params = self.source_params.clone();
        instrument.processing_chain = self.build_processing_chain();
        instrument.lfo = self.lfo.clone();
        instrument.amp_envelope = self.amp_envelope.clone();
        instrument.polyphonic = self.polyphonic;
        instrument.active = self.active;
    }

    /// Build a processing chain from local filter/eq/effects copies
    fn build_processing_chain(&self) -> Vec<ProcessingStage> {
        let mut chain = Vec::new();
        if let Some(ref f) = self.filter {
            chain.push(ProcessingStage::Filter(f.clone()));
        }
        if let Some(ref e) = self.eq {
            chain.push(ProcessingStage::Eq(e.clone()));
        }
        for effect in &self.effects {
            chain.push(ProcessingStage::Effect(effect.clone()));
        }
        chain
    }

    /// Map InstrumentSection::Processing(i) to the local Section enum
    fn map_section(&self, is: InstrumentSection) -> Section {
        match is {
            InstrumentSection::Source => Section::Source,
            InstrumentSection::Processing(i) => {
                let chain = self.build_processing_chain();
                if let Some(stage) = chain.get(i) {
                    match stage {
                        ProcessingStage::Filter(_) => Section::Filter,
                        ProcessingStage::Eq(_) => Section::Filter, // EQ grouped with filter section conceptually
                        ProcessingStage::Effect(_) => Section::Effects,
                    }
                } else {
                    Section::Effects // placeholder row for empty chain
                }
            }
            InstrumentSection::Lfo => Section::Lfo,
            InstrumentSection::Envelope => Section::Envelope,
        }
    }

    /// Map InstrumentSection row_info to local Section with adjusted local_idx
    fn map_row_info(&self, is: InstrumentSection, local_idx: usize) -> (Section, usize) {
        match is {
            InstrumentSection::Source => (Section::Source, local_idx),
            InstrumentSection::Processing(i) => {
                let chain = self.build_processing_chain();
                if let Some(stage) = chain.get(i) {
                    match stage {
                        ProcessingStage::Filter(_) => (Section::Filter, local_idx),
                        ProcessingStage::Eq(_) => (Section::Filter, local_idx), // not used in practice (EQ is pane-level)
                        ProcessingStage::Effect(_) => {
                            // Calculate local_idx within effects section:
                            // sum rows from all prior Effect stages + local_idx
                            let prior_effect_rows: usize = chain[..i].iter()
                                .filter_map(|s| match s {
                                    ProcessingStage::Effect(e) => Some(1 + e.params.len()),
                                    _ => None,
                                })
                                .sum();
                            (Section::Effects, prior_effect_rows + local_idx)
                        }
                    }
                } else {
                    (Section::Effects, 0) // placeholder
                }
            }
            InstrumentSection::Lfo => (Section::Lfo, local_idx),
            InstrumentSection::Envelope => (Section::Envelope, local_idx),
        }
    }

    /// Total number of selectable rows across all sections
    fn total_rows(&self) -> usize {
        let chain = self.build_processing_chain();
        instrument_row_count(self.source, &self.source_params, &chain)
    }

    /// Calculate non-selectable visual lines (headers + separators)
    fn visual_overhead(&self) -> usize {
        let headers = if self.source.is_vst() { 4 } else { 5 };
        let separators = if self.source.is_vst() { 3 } else { 4 };
        headers + separators
    }

    /// Calculate scroll offset to keep selected_row visible
    fn calc_scroll_offset(selected: usize, total: usize, visible: usize) -> usize {
        if visible == 0 || total <= visible {
            0
        } else if selected >= visible {
            (selected - visible + 1).min(total.saturating_sub(visible))
        } else {
            0
        }
    }

    /// Which section does a given row belong to?
    fn section_for_row(&self, row: usize) -> Section {
        let chain = self.build_processing_chain();
        let is = instrument_section_for_row(row, self.source, &self.source_params, &chain);
        self.map_section(is)
    }

    /// Get section and local index for a row
    fn row_info(&self, row: usize) -> (Section, usize) {
        let chain = self.build_processing_chain();
        let (is, local_idx) = instrument_row_info(row, self.source, &self.source_params, &chain);
        self.map_row_info(is, local_idx)
    }

    fn current_section(&self) -> Section {
        self.section_for_row(self.selected_row)
    }

    /// Decode a local_idx within the Effects section into (effect_index, param_offset).
    /// param_offset == 0 means the effect header row (name/enabled).
    /// param_offset >= 1 means param at index (param_offset - 1).
    fn effect_row_info(&self, local_idx: usize) -> Option<(usize, usize)> {
        let mut offset = 0;
        for (i, effect) in self.effects.iter().enumerate() {
            let rows = 1 + effect.params.len();
            if local_idx < offset + rows {
                return Some((i, local_idx - offset));
            }
            offset += rows;
        }
        None
    }

    pub fn is_editing(&self) -> bool {
        self.editing
    }
}

impl Pane for InstrumentEditPane {
    fn id(&self) -> &'static str {
        "instrument_edit"
    }

    fn handle_action(&mut self, action: ActionId, event: &InputEvent, state: &AppState) -> Action {
        self.handle_action_impl(action, event, state)
    }

    fn handle_raw_input(&mut self, event: &InputEvent, _state: &AppState) -> Action {
        self.handle_raw_input_impl(event);
        Action::None
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        self.render_impl(area, buf, state);
    }

    fn handle_mouse(&mut self, event: &MouseEvent, _area: Rect, _state: &AppState) -> Action {
        self.handle_mouse_impl(event)
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn tick(&mut self, state: &AppState) -> Vec<Action> {
        if !self.piano.is_active() || !self.piano.has_active_keys() {
            return vec![];
        }
        let now = Instant::now();
        let released = self.piano.check_releases(now);
        if released.is_empty() {
            return vec![];
        }
        // Get the currently selected instrument ID
        let instrument_id = state.instruments.selected_instrument()
            .map(|inst| inst.id)
            .unwrap_or(0);
        // Flatten all released pitches (handles chords)
        released.into_iter()
            .map(|(_, pitches)| {
                if pitches.len() == 1 {
                    Action::PianoRoll(PianoRollAction::ReleaseNote {
                        pitch: pitches[0],
                        instrument_id,
                    })
                } else {
                    Action::PianoRoll(PianoRollAction::ReleaseNotes {
                        pitches,
                        instrument_id,
                    })
                }
            })
            .collect()
    }

    fn on_enter(&mut self, state: &AppState) {
        if let Some(inst) = state.instruments.selected_instrument() {
            if self.instrument_id == Some(inst.id) {
                self.refresh_instrument(inst);
            } else {
                self.set_instrument(inst);
            }
        }
    }

    fn toggle_performance_mode(&mut self, state: &AppState) -> ToggleResult {
        if self.pad_keyboard.is_active() {
            self.pad_keyboard.deactivate();
            ToggleResult::Deactivated
        } else if self.piano.is_active() {
            self.piano.handle_escape();
            if self.piano.is_active() {
                ToggleResult::CycledLayout
            } else {
                ToggleResult::Deactivated
            }
        } else if state.instruments.selected_instrument()
            .map_or(false, |s| s.source.is_kit())
        {
            self.pad_keyboard.activate();
            ToggleResult::ActivatedPad
        } else {
            self.piano.activate();
            ToggleResult::ActivatedPiano
        }
    }

    fn activate_piano(&mut self) {
        if !self.piano.is_active() { self.piano.activate(); }
        self.pad_keyboard.deactivate();
    }

    fn activate_pad(&mut self) {
        if !self.pad_keyboard.is_active() { self.pad_keyboard.activate(); }
        self.piano.deactivate();
    }

    fn deactivate_performance(&mut self) {
        self.piano.release_all();
        self.piano.deactivate();
        self.pad_keyboard.deactivate();
    }

    fn supports_performance_mode(&self) -> bool { true }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Default for InstrumentEditPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}
