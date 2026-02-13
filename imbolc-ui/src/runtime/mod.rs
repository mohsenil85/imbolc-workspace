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

use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use crate::action::{AudioEffect, IoFeedback};
use crate::audio::AudioHandle;
use crate::config;
use crate::dispatch::LocalDispatcher;
use crate::global_actions::{apply_status_events, InstrumentSelectMode};
use crate::midi;
use crate::panes::{ConfirmPane, PendingAction};
use crate::setup;
use crate::state::{self, AppState};
use crate::ui::{keybindings, Frame, LayerStack, PaneId, PaneManager, RatatuiBackend};
use imbolc_core::interaction_log::InteractionLog;

fn autosave_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("imbolc")
        .join(".imbolc.autosave")
}

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
    pub(crate) autosave_enabled: bool,
    pub(crate) autosave_interval: Duration,
    pub(crate) autosave_path: PathBuf,
    pub(crate) autosave_id: u64,
    pub(crate) autosave_in_progress: bool,
    pub(crate) last_autosave_at: Instant,
}

impl AppRuntime {
    pub fn new() -> Self {
        let (io_tx, io_rx) = std::sync::mpsc::channel::<IoFeedback>();
        let config = config::Config::load();
        let autosave_enabled = config.autosave_enabled();
        let autosave_interval_minutes = config.autosave_interval_minutes();
        let autosave_interval = Duration::from_secs(autosave_interval_minutes.saturating_mul(60));
        let autosave_path = autosave_path();
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
        app_frame.set_autosave_config(autosave_enabled, autosave_interval_minutes);

        // Initialize MIDI input
        let mut midi_input = midi::MidiInputManager::new();
        midi_input.refresh_ports();
        if !midi_input.list_ports().is_empty() {
            let _ = midi_input.connect(0);
        }
        dispatcher.state_mut().midi.port_names = midi_input
            .list_ports()
            .iter()
            .map(|p| p.name.clone())
            .collect();
        dispatcher.state_mut().midi.connected_port =
            midi_input.connected_port_name().map(|s| s.to_string());

        let recent_projects = state::recent_projects::RecentProjects::load();
        let mut pending_audio_effects: Vec<AudioEffect> = Vec::new();
        let mut needs_full_sync = false;

        // CLI argument: optional project path (skip flags like --verbose)
        let project_arg = std::env::args().skip(1).find(|a| !a.starts_with('-'));
        let explicit_project_requested = project_arg.is_some();
        if let Some(arg) = project_arg {
            let load_path = std::path::PathBuf::from(&arg);
            if load_path.exists() {
                if let Ok((session, instruments)) = state::persistence::load_project(&load_path) {
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

        // Offer crash-recovery load when an autosave snapshot exists and no explicit
        // project path was requested on the CLI.
        if autosave_enabled && !explicit_project_requested && autosave_path.exists() {
            if let Some(confirm) = panes.get_pane_mut::<ConfirmPane>("confirm") {
                confirm.set_confirm(
                    "Recover autosave from previous session?",
                    PendingAction::LoadFrom(autosave_path.clone()),
                );
            }
            panes.push_to(PaneId::Confirm, dispatcher.state());
            layer_stack.set_pane_layer(panes.active().id());
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
            autosave_enabled,
            autosave_interval,
            autosave_path,
            autosave_id: 0,
            autosave_in_progress: false,
            last_autosave_at: Instant::now(),
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
            self.maybe_autosave();

            if self.quit_after_save && !self.dispatcher.state().project.dirty {
                break;
            }

            self.drain_audio_feedback();
            self.drain_midi_events();
            self.maybe_render(backend)?;
        }
        Ok(())
    }

    /// Persist a periodic crash-recovery snapshot when project state is dirty.
    pub(crate) fn maybe_autosave(&mut self) {
        if !self.autosave_enabled || self.autosave_in_progress {
            return;
        }
        if !self.dispatcher.state().project.dirty {
            return;
        }
        if self.last_autosave_at.elapsed() < self.autosave_interval {
            return;
        }

        self.last_autosave_at = Instant::now();
        self.autosave_in_progress = true;
        self.autosave_id = self.autosave_id.wrapping_add(1);

        let id = self.autosave_id;
        let path = self.autosave_path.clone();
        let session = self.dispatcher.state().session.clone();
        let instruments = self.dispatcher.state().instruments.clone();
        let tx = self.dispatcher.io_tx().clone();

        std::thread::spawn(move || {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            let result = crate::state::persistence::save_project(&path, &session, &instruments)
                .map_err(|e| e.to_string());

            let _ = tx.send(IoFeedback::AutosaveComplete { id, path, result });
        });
    }
}

/// Public entry point for standalone mode.
pub fn run(backend: &mut RatatuiBackend) -> std::io::Result<()> {
    let mut runtime = AppRuntime::new();

    // Propagate keyboard enhancement flag to all piano keyboards
    if backend.keyboard_enhancement_enabled() {
        use crate::panes::{InstrumentEditPane, InstrumentPane, PianoRollPane};
        if let Some(p) = runtime.panes.get_pane_mut::<InstrumentPane>("instrument") {
            p.set_enhanced_keyboard(true);
        }
        if let Some(p) = runtime
            .panes
            .get_pane_mut::<InstrumentEditPane>("instrument_edit")
        {
            p.set_enhanced_keyboard(true);
        }
        if let Some(p) = runtime.panes.get_pane_mut::<PianoRollPane>("piano_roll") {
            p.set_enhanced_keyboard(true);
        }
    }

    runtime.run(backend)
}
