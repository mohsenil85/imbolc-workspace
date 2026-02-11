//! App runtime coordinator.
//!
//! Decomposes the monolithic event loop into focused subsystems:
//! - `input` — event polling, layer resolution, global handler, pane dispatch
//! - `audio_sync` — apply pending audio effects
//! - `feedback` — I/O, audio, and MIDI feedback draining
//! - `render` — FPS throttle, meter/visualization updates, frame rendering

mod audio_sync;
mod feedback;
mod input;
mod render;

use std::sync::mpsc::Receiver;
use std::time::Instant;

use crate::action::{AudioEffect, IoFeedback};
use crate::audio::AudioHandle;
use crate::config;
use crate::dispatch::LocalDispatcher;
use crate::global_actions::{apply_status_events, InstrumentSelectMode};
use imbolc_core::interaction_log::InteractionLog;
use crate::midi;
use crate::setup;
use crate::state::{self, AppState};
use crate::ui::{Frame, LayerStack, PaneId, PaneManager, RatatuiBackend, keybindings};

/// Top-level runtime that owns all application state and drives the event loop.
pub struct AppRuntime {
    pub(crate) dispatcher: LocalDispatcher,
    pub(crate) audio: AudioHandle,
    pub(crate) panes: PaneManager,
    pub(crate) layer_stack: LayerStack,
    pub(crate) app_frame: Frame,
    pub(crate) midi_input: midi::MidiInputManager,
    pub(crate) io_rx: Receiver<IoFeedback>,
    pub(crate) recent_projects: state::recent_projects::RecentProjects,

    // Interaction logging
    pub(crate) ui_log: Option<InteractionLog>,

    // Per-frame state
    pub(crate) select_mode: InstrumentSelectMode,
    pub(crate) pending_audio_effects: Vec<AudioEffect>,
    pub(crate) needs_full_sync: bool,
    pub(crate) quit_after_save: bool,
    pub(crate) render_needed: bool,
    pub(crate) last_render_time: Instant,
    pub(crate) last_area: ratatui::layout::Rect,
}

impl AppRuntime {
    pub fn new() -> Self {
        let (io_tx, io_rx) = std::sync::mpsc::channel::<IoFeedback>();
        let config = config::Config::load();
        let mut state = AppState::new_with_defaults(config.defaults());
        state.keyboard_layout = config.keyboard_layout();

        let (layers, mut keymaps) = keybindings::load_keybindings();
        let mut panes = crate::register_all_panes(&mut keymaps);

        let mut layer_stack = LayerStack::new(layers);
        layer_stack.push("global");
        if state.instruments.instruments.is_empty() {
            panes.switch_to(PaneId::Add, &state);
        }
        layer_stack.set_pane_layer(panes.active().id());

        let mut audio = AudioHandle::new();
        audio.sync_state(&state);

        let mut dispatcher = LocalDispatcher::new(state, io_tx);
        let mut app_frame = Frame::new();

        // Initialize MIDI input
        let mut midi_input = midi::MidiInputManager::new();
        midi_input.refresh_ports();
        if !midi_input.list_ports().is_empty() {
            let _ = midi_input.connect(0);
        }
        dispatcher.state_mut().midi.port_names =
            midi_input.list_ports().iter().map(|p| p.name.clone()).collect();
        dispatcher.state_mut().midi.connected_port =
            midi_input.connected_port_name().map(|s| s.to_string());

        let recent_projects = state::recent_projects::RecentProjects::load();
        let mut pending_audio_effects: Vec<AudioEffect> = Vec::new();
        let mut needs_full_sync = false;

        // CLI argument: optional project path (skip flags like --verbose)
        let project_arg = std::env::args().skip(1).find(|a| !a.starts_with('-'));
        if let Some(arg) = project_arg {
            let load_path = std::path::PathBuf::from(&arg);
            if load_path.exists() {
                if let Ok((session, instruments)) =
                    state::persistence::load_project(&load_path)
                {
                    let name = load_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("untitled")
                        .to_string();
                    let st = dispatcher.state_mut();
                    st.session = session;
                    st.instruments = instruments;
                    st.instruments.rebuild_index();
                    st.project.path = Some(load_path);
                    st.project.dirty = false;
                    app_frame.set_project_name(name);
                    pending_audio_effects.extend(AudioEffect::all());
                    needs_full_sync = true;

                    if dispatcher.state().instruments.instruments.is_empty() {
                        panes.switch_to(PaneId::Add, dispatcher.state());
                    } else {
                        panes.switch_to(PaneId::InstrumentEdit, dispatcher.state());
                    }
                    layer_stack.set_pane_layer(panes.active().id());
                }
            } else {
                let name = load_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("untitled")
                    .to_string();
                dispatcher.state_mut().project.path = Some(load_path);
                app_frame.set_project_name(name);
            }
        }

        // Auto-start SuperCollider and apply status events
        {
            let startup_events = setup::auto_start_sc(&mut audio);
            apply_status_events(&startup_events, &mut panes, &mut app_frame);
        }

        Self {
            dispatcher,
            audio,
            panes,
            layer_stack,
            app_frame,
            midi_input,
            io_rx,
            recent_projects,
            ui_log: InteractionLog::ui(),
            select_mode: InstrumentSelectMode::Normal,
            pending_audio_effects,
            needs_full_sync,
            quit_after_save: false,
            render_needed: true,
            last_render_time: Instant::now(),
            last_area: ratatui::layout::Rect::new(0, 0, 80, 24),
        }
    }

    /// Main event loop.
    pub fn run(&mut self, backend: &mut RatatuiBackend) -> std::io::Result<()> {
        loop {
            self.layer_stack.set_pane_layer(self.panes.active().id());
            self.dispatcher.set_active_pane(self.panes.active().id());

            if self.process_events(backend)? {
                break;
            }

            self.process_tick();
            self.apply_pending_effects();
            self.drain_io_feedback();

            if self.quit_after_save && !self.dispatcher.state().project.dirty {
                break;
            }

            self.drain_audio_feedback();
            self.drain_midi_events();
            self.maybe_render(backend)?;
        }
        Ok(())
    }
}

/// Public entry point for standalone mode.
pub fn run(backend: &mut RatatuiBackend) -> std::io::Result<()> {
    let mut runtime = AppRuntime::new();

    // Propagate keyboard enhancement flag to all piano keyboards
    if backend.keyboard_enhancement_enabled() {
        use crate::panes::{InstrumentPane, InstrumentEditPane, PianoRollPane};
        if let Some(p) = runtime.panes.get_pane_mut::<InstrumentPane>("instrument") {
            p.set_enhanced_keyboard(true);
        }
        if let Some(p) = runtime.panes.get_pane_mut::<InstrumentEditPane>("instrument_edit") {
            p.set_enhanced_keyboard(true);
        }
        if let Some(p) = runtime.panes.get_pane_mut::<PianoRollPane>("piano_roll") {
            p.set_enhanced_keyboard(true);
        }
    }

    runtime.run(backend)
}
