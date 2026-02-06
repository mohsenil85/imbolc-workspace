// Re-export core crate modules so crate::state, crate::audio, etc. resolve throughout the binary
pub use imbolc_core::action;
pub use imbolc_core::audio;
pub use imbolc_core::config;
pub use imbolc_core::dispatch;
pub use imbolc_core::midi;
pub use imbolc_core::scd_parser;
pub use imbolc_core::state;

mod panes;
mod setup;
mod ui;
mod global_actions;
mod midi_dispatch;

use std::fs::File;
use std::time::{Duration, Instant};

use audio::commands::AudioCmd;
use audio::AudioHandle;
use action::{AudioDirty, IoFeedback};
use dispatch::LocalDispatcher;
use panes::{AddEffectPane, AddPane, AutomationPane, CommandPalettePane, ConfirmPane, DocsPane, EqPane, FileBrowserPane, FrameEditPane, HelpPane, HomePane, InstrumentEditPane, InstrumentPane, InstrumentPickerPane, MidiSettingsPane, MixerPane, PaneSwitcherPane, PianoRollPane, ProjectBrowserPane, QuitPromptPane, SaveAsPane, SampleChopperPane, SequencerPane, ServerPane, TrackPane, VstParamPane, WaveformPane};
use state::AppState;
use ui::{
    Action, AppEvent, Frame, InputSource, KeyCode, Keymap, LayerResult,
    LayerStack, PaneManager, RatatuiBackend, keybindings,
};
use global_actions::*;

fn init_logging(verbose: bool) {
    use simplelog::*;

    let log_level = if verbose { LevelFilter::Debug } else { LevelFilter::Warn };

    let log_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("imbolc")
        .join("imbolc.log");

    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let log_file = File::create(&log_path).unwrap_or_else(|_| {
        File::create("/tmp/imbolc.log").expect("Cannot create log file")
    });

    WriteLogger::init(log_level, Config::default(), log_file)
        .expect("Failed to initialize logger");

    log::info!("imbolc starting (log level: {:?})", log_level);
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let verbose = args.iter().any(|a| a == "--verbose" || a == "-v");
    init_logging(verbose);

    // Check for network modes
    let server_mode = args.iter().any(|a| a == "--server");
    let discover_mode = args.iter().any(|a| a == "--discover");
    let connect_addr = args.iter()
        .position(|a| a == "--connect")
        .and_then(|i| args.get(i + 1).cloned());

    // Parse --own flag for ownership requests (comma-separated instrument IDs)
    let own_instruments: Vec<u32> = args.iter()
        .position(|a| a == "--own")
        .and_then(|i| args.get(i + 1))
        .map(|s| {
            s.split(',')
                .filter_map(|n| n.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default();

    #[cfg(feature = "net")]
    {
        if server_mode {
            return run_server();
        }
        if discover_mode {
            #[cfg(feature = "mdns")]
            return run_discovery(own_instruments);
            #[cfg(not(feature = "mdns"))]
            {
                eprintln!("Discovery mode requires the 'mdns' feature. Build with: cargo build --features mdns");
                std::process::exit(1);
            }
        }
        if let Some(addr) = connect_addr {
            return run_client(&addr, own_instruments);
        }
    }

    #[cfg(not(feature = "net"))]
    {
        let _ = own_instruments; // Silence unused warning when net feature disabled
        if server_mode || connect_addr.is_some() {
            eprintln!("Network mode requires the 'net' feature. Build with: cargo build --features net");
            std::process::exit(1);
        }
    }

    let mut backend = RatatuiBackend::new()?;
    backend.start()?;

    let result = run(&mut backend);

    backend.stop()?;
    result
}

fn pane_keymap(keymaps: &mut std::collections::HashMap<String, Keymap>, id: &str) -> Keymap {
    keymaps.remove(id).unwrap_or_else(Keymap::new)
}

fn run(backend: &mut RatatuiBackend) -> std::io::Result<()> {
    let (io_tx, io_rx) = std::sync::mpsc::channel::<IoFeedback>();
    let config = config::Config::load();
    let mut state = AppState::new_with_defaults(config.defaults());
    state.keyboard_layout = config.keyboard_layout();

    // Load keybindings from embedded TOML (with optional user override)
    let (layers, mut keymaps) = keybindings::load_keybindings();

    // file_browser keymap is used by both FileBrowserPane and SampleChopperPane's internal browser
    let file_browser_km = keymaps.get("file_browser").cloned().unwrap_or_else(Keymap::new);

    let mut panes = PaneManager::new(Box::new(InstrumentEditPane::new(pane_keymap(&mut keymaps, "instrument_edit"))));
    panes.add_pane(Box::new(HomePane::new(pane_keymap(&mut keymaps, "home"))));
    panes.add_pane(Box::new(AddPane::new(pane_keymap(&mut keymaps, "add"))));
    panes.add_pane(Box::new(InstrumentPane::new(pane_keymap(&mut keymaps, "instrument"))));
    panes.add_pane(Box::new(ServerPane::new(pane_keymap(&mut keymaps, "server"))));
    panes.add_pane(Box::new(MixerPane::new(pane_keymap(&mut keymaps, "mixer"))));
    panes.add_pane(Box::new(HelpPane::new(pane_keymap(&mut keymaps, "help"))));
    panes.add_pane(Box::new(PianoRollPane::new(pane_keymap(&mut keymaps, "piano_roll"))));
    panes.add_pane(Box::new(SequencerPane::new(pane_keymap(&mut keymaps, "sequencer"))));
    panes.add_pane(Box::new(FrameEditPane::new(pane_keymap(&mut keymaps, "frame_edit"))));
    panes.add_pane(Box::new(SampleChopperPane::new(pane_keymap(&mut keymaps, "sample_chopper"), file_browser_km)));
    panes.add_pane(Box::new(AddEffectPane::new(pane_keymap(&mut keymaps, "add_effect"))));
    panes.add_pane(Box::new(InstrumentPickerPane::new(pane_keymap(&mut keymaps, "add"))));
    panes.add_pane(Box::new(FileBrowserPane::new(pane_keymap(&mut keymaps, "file_browser"))));
    panes.add_pane(Box::new(TrackPane::new(pane_keymap(&mut keymaps, "track"))));
    panes.add_pane(Box::new(WaveformPane::new(pane_keymap(&mut keymaps, "waveform"))));
    panes.add_pane(Box::new(AutomationPane::new(pane_keymap(&mut keymaps, "automation"))));
    panes.add_pane(Box::new(EqPane::new(pane_keymap(&mut keymaps, "eq"))));
    panes.add_pane(Box::new(VstParamPane::new(pane_keymap(&mut keymaps, "vst_params"))));
    panes.add_pane(Box::new(ConfirmPane::new(pane_keymap(&mut keymaps, "confirm"))));
    panes.add_pane(Box::new(QuitPromptPane::new(pane_keymap(&mut keymaps, "quit_prompt"))));
    panes.add_pane(Box::new(ProjectBrowserPane::new(pane_keymap(&mut keymaps, "project_browser"))));
    panes.add_pane(Box::new(SaveAsPane::new(pane_keymap(&mut keymaps, "save_as"))));
    panes.add_pane(Box::new(CommandPalettePane::new(pane_keymap(&mut keymaps, "command_palette"))));
    panes.add_pane(Box::new(PaneSwitcherPane::new(pane_keymap(&mut keymaps, "pane_switcher"))));
    panes.add_pane(Box::new(MidiSettingsPane::new(pane_keymap(&mut keymaps, "midi_settings"))));
    panes.add_pane(Box::new(DocsPane::new(pane_keymap(&mut keymaps, "docs"))));

    // Create layer stack
    let mut layer_stack = LayerStack::new(layers);
    layer_stack.push("global");
    if state.instruments.instruments.is_empty() {
        panes.switch_to("add", &state);
    }
    layer_stack.set_pane_layer(panes.active().id());

    // Create audio handle and sync initial state
    let mut audio = AudioHandle::new();
    audio.sync_state(&state);

    // Create the dispatcher that owns state and io_tx (audio stays separate to avoid borrow conflicts)
    let mut dispatcher = LocalDispatcher::new(state, io_tx.clone());

    let mut app_frame = Frame::new();

    // Initialize MIDI input
    let mut midi_input = midi::MidiInputManager::new();
    midi_input.refresh_ports();
    // Auto-connect first available port
    if !midi_input.list_ports().is_empty() {
        let _ = midi_input.connect(0);
    }
    dispatcher.state_mut().midi.port_names = midi_input.list_ports().iter().map(|p| p.name.clone()).collect();
    dispatcher.state_mut().midi.connected_port = midi_input.connected_port_name().map(|s| s.to_string());
    let mut recent_projects = state::recent_projects::RecentProjects::load();
    let mut last_render_time = Instant::now();
    let mut select_mode = InstrumentSelectMode::Normal;
    let mut pending_audio_dirty = AudioDirty::default();
    let mut quit_after_save = false;

    // CLI argument: optional project path (skip flags like --verbose)
    let project_arg = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with('-'));
    if let Some(arg) = project_arg {
        let load_path = std::path::PathBuf::from(&arg);
        if load_path.exists() {
            // Load existing project
            if let Ok((session, instruments)) = state::persistence::load_project(&load_path) {
                let name = load_path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("untitled")
                    .to_string();
                let state = dispatcher.state_mut();
                state.session = session;
                state.instruments = instruments;
                state.project.path = Some(load_path);
                state.project.dirty = false;
                app_frame.set_project_name(name);
                pending_audio_dirty.merge(AudioDirty::all());

                if dispatcher.state().instruments.instruments.is_empty() {
                    panes.switch_to("add", dispatcher.state());
                } else {
                    panes.switch_to("instrument_edit", dispatcher.state());
                }
                layer_stack.set_pane_layer(panes.active().id());
            }
        } else {
            // New project at specified path
            let name = load_path.file_stem()
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
        apply_status_events(&startup_events, &mut panes);
    }

    // Track last render area for mouse hit-testing
    let mut last_area = ratatui::layout::Rect::new(0, 0, 80, 24);

    loop {
        // Sync layer stack in case dispatch switched panes last iteration
        layer_stack.set_pane_layer(panes.active().id());

        if let Some(app_event) = backend.poll_event(Duration::from_millis(2)) {
            let pane_action = match app_event {
                AppEvent::Mouse(mouse_event) => {
                    panes.active_mut().handle_mouse(&mouse_event, last_area, dispatcher.state())
                }
                AppEvent::Key(event) => {
                    // Two-digit instrument selection state machine (pre-layer)
                    match &select_mode {
                        InstrumentSelectMode::WaitingFirstDigit => {
                            if let KeyCode::Char(c) = event.key {
                                if let Some(d) = c.to_digit(10) {
                                    select_mode = InstrumentSelectMode::WaitingSecondDigit(d as u8);
                                    continue;
                                }
                            }
                            // Non-digit cancels
                            select_mode = InstrumentSelectMode::Normal;
                            // Fall through to normal handling
                        }
                        InstrumentSelectMode::WaitingSecondDigit(first) => {
                            let first = *first;
                            if let KeyCode::Char(c) = event.key {
                                if let Some(d) = c.to_digit(10) {
                                    let combined = first * 10 + d as u8;
                                    let target = if combined == 0 { 10 } else { combined };
                                    select_instrument(target as usize, &mut dispatcher, &mut panes, &mut audio);
                                    select_mode = InstrumentSelectMode::Normal;
                                    continue;
                                }
                            }
                            // Non-digit cancels
                            select_mode = InstrumentSelectMode::Normal;
                            // Fall through to normal handling
                        }
                        InstrumentSelectMode::Normal => {}
                    }

                    // Layer resolution
                    match layer_stack.resolve(&event) {
                        LayerResult::Action(action) => {
                            match handle_global_action(
                                action,
                                &mut dispatcher,
                                &mut panes,
                                &mut audio,
                                &mut app_frame,
                                &mut select_mode,
                                &mut pending_audio_dirty,
                                &mut layer_stack,
                            ) {
                                GlobalResult::Quit => break,
                                GlobalResult::RefreshScreen => {
                                    backend.clear()?;
                                    continue;
                                }
                                GlobalResult::Handled => continue,
                                GlobalResult::NotHandled => {
                                    panes.active_mut().handle_action(action, &event, dispatcher.state())
                                }
                            }
                        }
                        LayerResult::Blocked | LayerResult::Unresolved => {
                            panes.active_mut().handle_raw_input(&event, dispatcher.state())
                        }
                    }
                }
            };

            // Process layer management actions
            match &pane_action {
                Action::PushLayer(name) => {
                    layer_stack.push(name);
                }
                Action::PopLayer(name) => {
                    layer_stack.pop(name);
                }
                Action::ExitPerformanceMode => {
                    layer_stack.pop("piano_mode");
                    layer_stack.pop("pad_mode");
                    panes.active_mut().deactivate_performance();
                }
                _ => {}
            }

            // Auto-pop text_edit layer when pane is no longer editing
            if layer_stack.has_layer("text_edit") {
                let still_editing = match panes.active().id() {
                    "instrument_edit" => {
                        panes.get_pane_mut::<InstrumentEditPane>("instrument_edit")
                            .map_or(false, |p| p.is_editing())
                    }
                    "frame_edit" => {
                        panes.get_pane_mut::<FrameEditPane>("frame_edit")
                            .map_or(false, |p| p.is_editing())
                    }
                    _ => false,
                };
                if !still_editing {
                    layer_stack.pop("text_edit");
                }
            }

            // Detect SaveAs cancel during quit flow: if we're quitting and
            // the user pops the save_as pane, cancel the quit
            if quit_after_save
                && matches!(&pane_action, Action::Nav(action::NavAction::PopPane))
                && panes.active().id() == "save_as"
            {
                quit_after_save = false;
            }

            // Process navigation
            panes.process_nav(&pane_action, dispatcher.state());

            // Sync pane layer after navigation
            if matches!(&pane_action, Action::Nav(_)) {
                sync_pane_layer(&mut panes, &mut layer_stack);

                // Auto-exit clip edit when navigating away from piano roll
                if dispatcher.state().session.arrangement.editing_clip.is_some()
                    && panes.active().id() != "piano_roll"
                {
                    let exit_result = dispatcher.dispatch_with_audio(&Action::Arrangement(action::ArrangementAction::ExitClipEdit), &mut audio);
                    pending_audio_dirty.merge(exit_result.audio_dirty);
                    apply_dispatch_result(exit_result, &mut dispatcher, &mut panes, &mut app_frame, &mut audio);
                }
            }

            // Auto-pop command_palette layer and re-dispatch confirmed command
            if layer_stack.has_layer("command_palette") && panes.active().id() != "command_palette" {
                layer_stack.pop("command_palette");
                if let Some(palette) = panes.get_pane_mut::<CommandPalettePane>("command_palette") {
                    if let Some(cmd) = palette.take_command() {
                        let global_result = handle_global_action(
                            cmd, &mut dispatcher, &mut panes, &mut audio, &mut app_frame,
                            &mut select_mode, &mut pending_audio_dirty, &mut layer_stack,
                        );
                        if matches!(global_result, GlobalResult::Quit) { break; }
                        if matches!(global_result, GlobalResult::NotHandled) {
                            let dummy_event = ui::InputEvent::new(KeyCode::Enter, ui::Modifiers::none());
                            let re_action = panes.active_mut().handle_action(cmd, &dummy_event, dispatcher.state());
                            panes.process_nav(&re_action, dispatcher.state());
                            if matches!(&re_action, Action::Nav(_)) {
                                sync_pane_layer(&mut panes, &mut layer_stack);
                            }
                            let r = dispatcher.dispatch_with_audio(&re_action, &mut audio);
                            if r.quit { break; }
                            pending_audio_dirty.merge(r.audio_dirty);
                            apply_dispatch_result(r, &mut dispatcher, &mut panes, &mut app_frame, &mut audio);
                        }
                    }
                }
                sync_pane_layer(&mut panes, &mut layer_stack);
            }

            // Auto-pop pane_switcher layer and switch to selected pane
            if layer_stack.has_layer("pane_switcher") && panes.active().id() != "pane_switcher" {
                layer_stack.pop("pane_switcher");
                if let Some(switcher) = panes.get_pane_mut::<PaneSwitcherPane>("pane_switcher") {
                    if let Some(pane_id) = switcher.take_pane() {
                        panes.switch_to(pane_id, dispatcher.state());
                        sync_pane_layer(&mut panes, &mut layer_stack);
                    }
                }
            }

            // Intercept MIDI port actions that need MidiInputManager
            if let Action::Midi(action::MidiAction::ConnectPort(port_idx)) = &pane_action {
                let port_idx = *port_idx;
                midi_input.refresh_ports();
                match midi_input.connect(port_idx) {
                    Ok(()) => {
                        dispatcher.state_mut().midi.connected_port = midi_input.connected_port_name().map(|s| s.to_string());
                    }
                    Err(_) => {
                        dispatcher.state_mut().midi.connected_port = None;
                    }
                }
                dispatcher.state_mut().midi.port_names = midi_input.list_ports().iter().map(|p| p.name.clone()).collect();
            } else if let Action::Midi(action::MidiAction::DisconnectPort) = &pane_action {
                midi_input.disconnect();
                dispatcher.state_mut().midi.connected_port = None;
            }

            // Intercept SaveAndQuit — handle in main.rs, not dispatch
            if matches!(&pane_action, Action::SaveAndQuit) {
                if dispatcher.state().project.path.is_some() {
                    let r = dispatcher.dispatch_with_audio(&Action::Session(action::SessionAction::Save), &mut audio);
                    pending_audio_dirty.merge(r.audio_dirty);
                    apply_dispatch_result(r, &mut dispatcher, &mut panes, &mut app_frame, &mut audio);
                    quit_after_save = true;
                } else {
                    // No project path — open SaveAs, then quit after save completes
                    let default_name = "untitled".to_string();
                    if let Some(sa) = panes.get_pane_mut::<SaveAsPane>("save_as") {
                        sa.reset(&default_name);
                    }
                    // Pop the quit prompt first, then push save_as
                    panes.pop(dispatcher.state());
                    panes.push_to("save_as", dispatcher.state());
                    sync_pane_layer(&mut panes, &mut layer_stack);
                    quit_after_save = true;
                }
            } else {
                let dispatch_result = dispatcher.dispatch_with_audio(&pane_action, &mut audio);
                if dispatch_result.quit {
                    break;
                }
                pending_audio_dirty.merge(dispatch_result.audio_dirty);
                apply_dispatch_result(dispatch_result, &mut dispatcher, &mut panes, &mut app_frame, &mut audio);
            }
        }

        // Process time-based pane updates (key releases, etc.)
        let tick_actions = panes.active_mut().tick(dispatcher.state());
        for action in tick_actions {
            let r = dispatcher.dispatch_with_audio(&action, &mut audio);
            pending_audio_dirty.merge(r.audio_dirty);
        }

        if pending_audio_dirty.any() {
            audio.flush_dirty(dispatcher.state(), pending_audio_dirty);
            pending_audio_dirty.clear();
        }

        // Drain I/O feedback
        while let Ok(feedback) = io_rx.try_recv() {
            match feedback {
                IoFeedback::SaveComplete { id, path, result } => {
                    if id != dispatcher.state().io.generation.save {
                        continue;
                    }
                    let status = match result {
                        Ok(name) => {
                            let state = dispatcher.state_mut();
                            state.project.path = Some(path.clone());
                            state.project.dirty = false;
                            recent_projects.add(&path, &name);
                            recent_projects.save();
                            app_frame.set_project_name(name);
                            "Saved project".to_string()
                        }
                        Err(e) => format!("Save failed: {}", e),
                    };
                    if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
                        server.set_status(audio.status(), &status);
                    }
                }
                IoFeedback::LoadComplete { id, path, result } => {
                    if id != dispatcher.state().io.generation.load {
                        continue;
                    }
                     match result {
                         Ok((new_session, new_instruments, name)) => {
                             {
                                 let state = dispatcher.state_mut();
                                 state.undo_history.clear();
                                 state.session = new_session;
                                 state.instruments = new_instruments;
                                 state.project.path = Some(path.clone());
                                 state.project.dirty = false;
                             }
                             recent_projects.add(&path, &name);
                             recent_projects.save();
                             app_frame.set_project_name(name);

                             if dispatcher.state().instruments.instruments.is_empty() {
                                 panes.switch_to("add", dispatcher.state());
                             }

                             let dirty = AudioDirty::all();
                             pending_audio_dirty.merge(dirty);

                             // Queue VST state restores - collect data first to avoid borrow conflicts
                             let vst_restores: Vec<_> = dispatcher.state().instruments.instruments.iter()
                                 .flat_map(|inst| {
                                     let mut restores = Vec::new();
                                     if let (state::SourceType::Vst(_), Some(ref path)) = (&inst.source, &inst.vst_state_path) {
                                         restores.push((inst.id, action::VstTarget::Source, path.clone()));
                                     }
                                     for effect in &inst.effects {
                                         if let (state::EffectType::Vst(_), Some(ref path)) = (&effect.effect_type, &effect.vst_state_path) {
                                             restores.push((inst.id, action::VstTarget::Effect(effect.id), path.clone()));
                                         }
                                     }
                                     restores
                                 })
                                 .collect();

                             for (instrument_id, target, path) in vst_restores {
                                 let _ = audio.send_cmd(audio::commands::AudioCmd::LoadVstState {
                                     instrument_id,
                                     target,
                                     path,
                                 });
                             }

                             if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
                                 server.set_status(audio.status(), "Project loaded");
                             }
                         }
                         Err(e) => {
                             if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
                                 server.set_status(audio.status(), &format!("Load failed: {}", e));
                             }
                         }
                     }
                }
                IoFeedback::ImportSynthDefComplete { id, result } => {
                    if id != dispatcher.state().io.generation.import_synthdef {
                        continue;
                    }
                     match result {
                         Ok((custom, synthdef_name, scsyndef_path)) => {
                             // Register it
                             let _id = dispatcher.state_mut().session.custom_synthdefs.add(custom);
                             pending_audio_dirty.session = true;

                             if audio.is_running() {
                                 if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
                                     server.set_status(audio.status(), &format!("Loading custom synthdef: {}", synthdef_name));
                                 }

                                 let (reply_tx, reply_rx) = std::sync::mpsc::channel();
                                 let load_path = scsyndef_path.clone();
                                 let io_tx_clone = io_tx.clone();
                                 let load_id = id;
                                 let name = synthdef_name.clone();

                                 match audio.send_cmd(AudioCmd::LoadSynthDefFile { path: load_path, reply: reply_tx }) {
                                     Ok(()) => {
                                         std::thread::spawn(move || {
                                             let result = match reply_rx.recv() {
                                                 Ok(Ok(())) => Ok(name),
                                                 Ok(Err(e)) => Err(e),
                                                 Err(_) => Err("Audio thread disconnected".to_string()),
                                             };
                                             let _ = io_tx_clone.send(IoFeedback::ImportSynthDefLoaded { id: load_id, result });
                                         });
                                     }
                                     Err(e) => {
                                         if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
                                             server.set_status(audio.status(), &format!("Failed to load synthdef: {}", e));
                                         }
                                     }
                                 }
                             } else {
                                 if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
                                     server.set_status(audio.status(), &format!("Imported custom synthdef: {}", synthdef_name));
                                 }
                             }
                         }
                         Err(e) => {
                             if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
                                 server.set_status(audio.status(), &format!("Import error: {}", e));
                             }
                         }
                     }
                }
                IoFeedback::ImportSynthDefLoaded { id, result } => {
                    if id != dispatcher.state().io.generation.import_synthdef {
                        continue;
                    }
                    let status = match result {
                        Ok(name) => format!("Loaded custom synthdef: {}", name),
                        Err(e) => format!("Failed to load synthdef: {}", e),
                    };
                    if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
                        server.set_status(audio.status(), &status);
                    }
                }
            }
        }

        // Quit after save completes
        if quit_after_save && !dispatcher.state().project.dirty {
            break;
        }

        // Drain audio feedback
        for feedback in audio.drain_feedback() {
            let action = Action::AudioFeedback(feedback);
            let r = dispatcher.dispatch_with_audio(&action, &mut audio);
            pending_audio_dirty.merge(r.audio_dirty);
            apply_dispatch_result(r, &mut dispatcher, &mut panes, &mut app_frame, &mut audio);
        }

        // Poll MIDI events
        for event in midi_input.poll_events() {
            if let Some(action) = midi_dispatch::process_midi_event(&event, dispatcher.state()) {
                let r = dispatcher.dispatch_with_audio(&action, &mut audio);
                pending_audio_dirty.merge(r.audio_dirty);
            }
        }

        // Visual updates and rendering at ~60fps
        let now_render = Instant::now();
        if now_render.duration_since(last_render_time).as_millis() >= 16 {
            last_render_time = now_render;

            // Update master meter from real audio peak
            {
                let peak = if audio.is_running() {
                    audio.master_peak()
                } else {
                    0.0
                };
                let mute = dispatcher.state().session.mixer.master_mute;
                app_frame.set_master_peak(peak, mute);
            }

            // Update SC CPU and latency indicators
            {
                let cpu = if audio.is_running() { audio.sc_cpu() } else { 0.0 };
                let osc_latency = if audio.is_running() { audio.osc_latency_ms() } else { 0.0 };
                let audio_latency = audio.audio_latency_ms();
                app_frame.set_sc_metrics(cpu, osc_latency, audio_latency);
            }

            // Update recording state
            {
                let state = dispatcher.state_mut();
                state.recording.recording = audio.is_recording();
                state.recording.recording_secs = audio.recording_elapsed()
                    .map(|d| d.as_secs()).unwrap_or(0);
                app_frame.recording = state.recording.recording;
                app_frame.recording_secs = state.recording.recording_secs;
            }

            // Update visualization data from audio analysis synths
            {
                let state = dispatcher.state_mut();
                state.audio.visualization.spectrum_bands = audio.spectrum_bands();
                let (peak_l, peak_r, rms_l, rms_r) = audio.lufs_data();
                state.audio.visualization.peak_l = peak_l;
                state.audio.visualization.peak_r = peak_r;
                state.audio.visualization.rms_l = rms_l;
                state.audio.visualization.rms_r = rms_r;
                let scope = audio.scope_buffer();
                state.audio.visualization.scope_buffer.clear();
                state.audio.visualization.scope_buffer.extend(scope);
            }

            // Update waveform cache for waveform pane
            if panes.active().id() == "waveform" {
                if let Some(wf) = panes.get_pane_mut::<WaveformPane>("waveform") {
                    if dispatcher.state().recorded_waveform_peaks.is_none() {
                        let inst_data = dispatcher.state().instruments.selected_instrument()
                            .filter(|s| s.source.is_audio_input() || s.source.is_bus_in())
                            .map(|s| s.id);
                        wf.audio_in_waveform = inst_data.map(|id| audio.audio_in_waveform(id));
                    }
                }
            } else {
                if let Some(wf) = panes.get_pane_mut::<WaveformPane>("waveform") {
                    wf.audio_in_waveform = None;
                }
                dispatcher.state_mut().recorded_waveform_peaks = None;
            }

            // Copy audio-owned state into AppState for pane rendering.
            {
                let ars = audio.read_state();
                let state = dispatcher.state_mut();
                state.audio.playhead = ars.playhead;
                state.audio.bpm = ars.bpm;
                state.audio.server_status = ars.server_status;
            }

            // Render
            let mut frame = backend.begin_frame()?;
            let area = frame.area();
            last_area = area;
            let mut rbuf = ui::RenderBuf::new(frame.buffer_mut());
            app_frame.render_buf(area, &mut rbuf, dispatcher.state());
            panes.render(area, &mut rbuf, dispatcher.state());
            backend.end_frame(frame)?;
        }
    }

    Ok(())
}

// =============================================================================
// Network Mode Functions (requires "net" feature)
// =============================================================================

#[cfg(feature = "net")]
fn run_server() -> std::io::Result<()> {
    use std::thread;
    use std::time::Duration;
    use imbolc_net::{NetServer, NetworkState};
    use imbolc_types::Action;

    log::info!("Starting Imbolc server mode");

    let (io_tx, io_rx) = std::sync::mpsc::channel::<action::IoFeedback>();
    let config = config::Config::load();
    let state = state::AppState::new_with_defaults(config.defaults());

    // Create the dispatcher
    let mut dispatcher = LocalDispatcher::new(state, io_tx.clone());

    // Create audio handle and sync initial state
    let mut audio = AudioHandle::new();
    audio.sync_state(dispatcher.state());

    // Auto-start SuperCollider
    let startup_events = setup::auto_start_sc(&mut audio);
    for event in startup_events {
        log::info!("Startup: {:?}", event);
    }

    // Bind server
    let mut server = NetServer::bind("0.0.0.0:9999")?;
    log::info!("Server listening on 0.0.0.0:9999");

    // Register with mDNS for LAN discovery
    #[cfg(feature = "mdns")]
    let _discovery = {
        match imbolc_net::DiscoveryServer::new("Imbolc Session", 9999) {
            Ok(d) => {
                log::info!("mDNS discovery registered");
                Some(d)
            }
            Err(e) => {
                log::warn!("Failed to register mDNS discovery: {}", e);
                None
            }
        }
    };

    let mut pending_audio_dirty = action::AudioDirty::default();
    let mut last_metering = Instant::now();
    #[cfg(feature = "mdns")]
    let mut last_client_count = 0usize;

    loop {
        // Build network state snapshot
        let network_state = NetworkState {
            session: dispatcher.state().session.clone(),
            instruments: dispatcher.state().instruments.clone(),
            ownership: server.build_ownership_map(),
            privileged_client: server.privileged_client_info(),
        };

        // Accept new connections
        server.accept_connections(&network_state);

        // Poll for client actions
        for (client_id, net_action) in server.poll_actions(&network_state) {
            log::debug!("Received action from {:?}: {:?}", client_id, net_action);

            // Convert NetworkAction to Action
            let action = network_action_to_action(net_action);

            // Dispatch
            let result = dispatcher.dispatch_with_audio(&action, &mut audio);
            pending_audio_dirty.merge(result.audio_dirty);

            if result.quit {
                log::info!("Quit requested, shutting down server");
                server.broadcast_shutdown();
                return Ok(());
            }
        }

        // Flush audio dirty flags
        if pending_audio_dirty.any() {
            audio.flush_dirty(dispatcher.state(), pending_audio_dirty);
            pending_audio_dirty.clear();

            // Broadcast updated state
            let network_state = NetworkState {
                session: dispatcher.state().session.clone(),
                instruments: dispatcher.state().instruments.clone(),
                ownership: server.build_ownership_map(),
                privileged_client: server.privileged_client_info(),
            };
            server.broadcast_state(&network_state);
        }

        // Drain I/O feedback (simplified - no UI updates in server mode)
        while let Ok(feedback) = io_rx.try_recv() {
            log::debug!("I/O feedback: {:?}", feedback);
        }

        // Drain audio feedback
        for feedback in audio.drain_feedback() {
            let action = Action::AudioFeedback(feedback);
            let result = dispatcher.dispatch_with_audio(&action, &mut audio);
            pending_audio_dirty.merge(result.audio_dirty);
        }

        // Send metering at ~30Hz
        let now = Instant::now();
        if now.duration_since(last_metering).as_millis() >= 33 {
            last_metering = now;
            let ars = audio.read_state();
            let (peak_l, peak_r) = (audio.master_peak(), audio.master_peak());
            server.broadcast_metering(ars.playhead, ars.bpm, (peak_l, peak_r));

            // Update mDNS client count if changed
            #[cfg(feature = "mdns")]
            {
                let count = server.client_count();
                if count != last_client_count {
                    last_client_count = count;
                    if let Some(ref discovery) = _discovery {
                        discovery.update_client_count(count);
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(2));
    }
}

/// Discover available Imbolc servers on the LAN and connect to one.
#[cfg(all(feature = "net", feature = "mdns"))]
fn run_discovery(own_instruments: Vec<u32>) -> std::io::Result<()> {
    use std::io::{self, Write};
    use std::time::Duration;
    use imbolc_net::DiscoveryClient;

    println!("Searching for Imbolc servers on LAN...\n");

    // Browse for 3 seconds
    let servers = DiscoveryClient::browse_for(Duration::from_secs(3))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    if servers.is_empty() {
        println!("No Imbolc servers found on the local network.");
        println!("\nYou can start a server with: imbolc --server");
        println!("Or connect directly with: imbolc --connect <ip:port>");
        return Ok(());
    }

    println!("Available Imbolc servers on LAN:\n");
    for (i, server) in servers.iter().enumerate() {
        println!(
            "  {}. {}\n     Session: \"{}\" ({} {})\n",
            i + 1,
            server.address,
            server.session_name,
            server.client_count,
            if server.client_count == 1 { "client" } else { "clients" }
        );
    }

    print!("Select server [1-{}] or enter IP address: ", servers.len());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    // Parse selection
    let addr = if let Ok(num) = input.parse::<usize>() {
        if num >= 1 && num <= servers.len() {
            servers[num - 1].address.clone()
        } else {
            println!("Invalid selection");
            return Ok(());
        }
    } else if !input.is_empty() {
        // Use as direct address
        input.to_string()
    } else {
        println!("No selection made");
        return Ok(());
    };

    println!("\nConnecting to {}...", addr);
    run_client(&addr, own_instruments)
}

#[cfg(feature = "net")]
fn run_client(addr: &str, own_instruments: Vec<u32>) -> std::io::Result<()> {
    use std::time::Duration;
    use imbolc_net::RemoteDispatcher;
    use ui::action_id::{ActionId, GlobalActionId};

    log::info!("Connecting to server at {}", addr);

    // Get hostname for client name
    let client_name = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    // Convert CLI instrument IDs to InstrumentId type
    let requested_instruments: Vec<_> = own_instruments.into_iter().collect();

    let mut remote = RemoteDispatcher::connect(addr, &client_name, requested_instruments)?;
    log::info!(
        "Connected to server as {:?}, owning {} instruments",
        remote.client_id(),
        remote.owned_instruments().len()
    );

    let mut backend = RatatuiBackend::new()?;
    backend.start()?;

    // Load keybindings
    let (layers, mut keymaps) = ui::keybindings::load_keybindings();
    let file_browser_km = keymaps.get("file_browser").cloned().unwrap_or_else(Keymap::new);

    let mut panes = PaneManager::new(Box::new(InstrumentEditPane::new(pane_keymap(&mut keymaps, "instrument_edit"))));
    panes.add_pane(Box::new(HomePane::new(pane_keymap(&mut keymaps, "home"))));
    panes.add_pane(Box::new(AddPane::new(pane_keymap(&mut keymaps, "add"))));
    panes.add_pane(Box::new(InstrumentPane::new(pane_keymap(&mut keymaps, "instrument"))));
    panes.add_pane(Box::new(ServerPane::new(pane_keymap(&mut keymaps, "server"))));
    panes.add_pane(Box::new(MixerPane::new(pane_keymap(&mut keymaps, "mixer"))));
    panes.add_pane(Box::new(HelpPane::new(pane_keymap(&mut keymaps, "help"))));
    panes.add_pane(Box::new(PianoRollPane::new(pane_keymap(&mut keymaps, "piano_roll"))));
    panes.add_pane(Box::new(SequencerPane::new(pane_keymap(&mut keymaps, "sequencer"))));
    panes.add_pane(Box::new(FrameEditPane::new(pane_keymap(&mut keymaps, "frame_edit"))));
    panes.add_pane(Box::new(SampleChopperPane::new(pane_keymap(&mut keymaps, "sample_chopper"), file_browser_km)));
    panes.add_pane(Box::new(AddEffectPane::new(pane_keymap(&mut keymaps, "add_effect"))));
    panes.add_pane(Box::new(InstrumentPickerPane::new(pane_keymap(&mut keymaps, "add"))));
    panes.add_pane(Box::new(FileBrowserPane::new(pane_keymap(&mut keymaps, "file_browser"))));
    panes.add_pane(Box::new(TrackPane::new(pane_keymap(&mut keymaps, "track"))));
    panes.add_pane(Box::new(WaveformPane::new(pane_keymap(&mut keymaps, "waveform"))));
    panes.add_pane(Box::new(AutomationPane::new(pane_keymap(&mut keymaps, "automation"))));
    panes.add_pane(Box::new(EqPane::new(pane_keymap(&mut keymaps, "eq"))));
    panes.add_pane(Box::new(VstParamPane::new(pane_keymap(&mut keymaps, "vst_params"))));
    panes.add_pane(Box::new(ConfirmPane::new(pane_keymap(&mut keymaps, "confirm"))));
    panes.add_pane(Box::new(QuitPromptPane::new(pane_keymap(&mut keymaps, "quit_prompt"))));
    panes.add_pane(Box::new(ProjectBrowserPane::new(pane_keymap(&mut keymaps, "project_browser"))));
    panes.add_pane(Box::new(SaveAsPane::new(pane_keymap(&mut keymaps, "save_as"))));
    panes.add_pane(Box::new(CommandPalettePane::new(pane_keymap(&mut keymaps, "command_palette"))));
    panes.add_pane(Box::new(MidiSettingsPane::new(pane_keymap(&mut keymaps, "midi_settings"))));

    // Create layer stack
    let mut layer_stack = ui::LayerStack::new(layers);
    layer_stack.push("global");

    // Build a synthetic AppState from the network state for rendering
    let config = config::Config::load();
    let mut local_state = state::AppState::new_with_defaults(config.defaults());
    local_state.session = remote.state().session.clone();
    local_state.instruments = remote.state().instruments.clone();
    sync_network_context(&mut local_state, &remote);

    if local_state.instruments.instruments.is_empty() {
        panes.switch_to("add", &local_state);
    }
    layer_stack.set_pane_layer(panes.active().id());

    let app_frame = Frame::new();
    let mut last_render_time = Instant::now();
    let mut last_area = ratatui::layout::Rect::new(0, 0, 80, 24);

    loop {
        // Poll for server updates
        if remote.poll_updates() {
            // State was updated from server
            local_state.session = remote.state().session.clone();
            local_state.instruments = remote.state().instruments.clone();
            sync_network_context(&mut local_state, &remote);
        }

        // Update metering
        let metering = remote.metering();
        local_state.audio.playhead = metering.playhead;
        local_state.audio.bpm = metering.bpm;

        // Check for server shutdown
        if remote.server_shutdown() {
            log::info!("Server shut down, exiting");
            break;
        }

        if let Some(app_event) = backend.poll_event(Duration::from_millis(2)) {
            let pane_action = match app_event {
                ui::AppEvent::Mouse(mouse_event) => {
                    panes.active_mut().handle_mouse(&mouse_event, last_area, &local_state)
                }
                ui::AppEvent::Key(event) => {
                    match layer_stack.resolve(&event) {
                        ui::LayerResult::Action(action) => {
                            // Handle quit locally
                            if matches!(action, ActionId::Global(GlobalActionId::Quit)) {
                                break;
                            }
                            panes.active_mut().handle_action(action, &event, &local_state)
                        }
                        ui::LayerResult::Blocked | ui::LayerResult::Unresolved => {
                            panes.active_mut().handle_raw_input(&event, &local_state)
                        }
                    }
                }
            };

            // Handle layer management locally
            match &pane_action {
                Action::PushLayer(name) => layer_stack.push(name),
                Action::PopLayer(name) => layer_stack.pop(name),
                Action::ExitPerformanceMode => {
                    layer_stack.pop("piano_mode");
                    layer_stack.pop("pad_mode");
                    panes.active_mut().deactivate_performance();
                }
                _ => {}
            }

            // Navigation handled locally
            panes.process_nav(&pane_action, &local_state);
            if matches!(&pane_action, Action::Nav(_)) {
                layer_stack.set_pane_layer(panes.active().id());
            }

            // Convert to NetworkAction and send to server
            if let Some(net_action) = action_to_network_action(&pane_action) {
                if let Err(e) = remote.dispatch(net_action) {
                    log::error!("Failed to send action to server: {}", e);
                    break;
                }
            }

            // Local quit
            if matches!(&pane_action, Action::Quit) {
                break;
            }
        }

        // Render at ~60fps
        let now_render = Instant::now();
        if now_render.duration_since(last_render_time).as_millis() >= 16 {
            last_render_time = now_render;

            let mut frame = backend.begin_frame()?;
            let area = frame.area();
            last_area = area;
            let mut rbuf = ui::RenderBuf::new(frame.buffer_mut());
            app_frame.render_buf(area, &mut rbuf, &local_state);
            panes.render(area, &mut rbuf, &local_state);
            backend.end_frame(frame)?;
        }
    }

    let _ = remote.disconnect();
    backend.stop()?;
    Ok(())
}

/// Convert NetworkAction to Action for dispatch.
#[cfg(feature = "net")]
fn network_action_to_action(net_action: imbolc_net::NetworkAction) -> Action {
    use imbolc_net::NetworkAction;
    match net_action {
        NetworkAction::None => Action::None,
        NetworkAction::Quit => Action::Quit,
        NetworkAction::Instrument(a) => Action::Instrument(a),
        NetworkAction::Mixer(a) => Action::Mixer(a),
        NetworkAction::PianoRoll(a) => Action::PianoRoll(a),
        NetworkAction::Arrangement(a) => Action::Arrangement(a),
        NetworkAction::Server(a) => Action::Server(a),
        NetworkAction::Session(a) => Action::Session(a),
        NetworkAction::Sequencer(a) => Action::Sequencer(a),
        NetworkAction::Chopper(a) => Action::Chopper(a),
        NetworkAction::Automation(a) => Action::Automation(a),
        NetworkAction::Midi(a) => Action::Midi(a),
        NetworkAction::Bus(a) => Action::Bus(a),
        NetworkAction::VstParam(a) => Action::VstParam(a),
        NetworkAction::Undo => Action::Undo,
        NetworkAction::Redo => Action::Redo,
    }
}

/// Convert Action to NetworkAction for transmission (returns None for local-only actions).
#[cfg(feature = "net")]
fn action_to_network_action(action: &Action) -> Option<imbolc_net::NetworkAction> {
    use imbolc_net::NetworkAction;
    match action {
        Action::None => Some(NetworkAction::None),
        Action::Quit => Some(NetworkAction::Quit),
        Action::Instrument(a) => Some(NetworkAction::Instrument(a.clone())),
        Action::Mixer(a) => Some(NetworkAction::Mixer(a.clone())),
        Action::PianoRoll(a) => Some(NetworkAction::PianoRoll(a.clone())),
        Action::Arrangement(a) => Some(NetworkAction::Arrangement(a.clone())),
        Action::Server(a) => Some(NetworkAction::Server(a.clone())),
        Action::Session(a) => Some(NetworkAction::Session(a.clone())),
        Action::Sequencer(a) => Some(NetworkAction::Sequencer(a.clone())),
        Action::Chopper(a) => Some(NetworkAction::Chopper(a.clone())),
        Action::Automation(a) => Some(NetworkAction::Automation(a.clone())),
        Action::Midi(a) => Some(NetworkAction::Midi(a.clone())),
        Action::Bus(a) => Some(NetworkAction::Bus(a.clone())),
        Action::VstParam(a) => Some(NetworkAction::VstParam(a.clone())),
        Action::Undo => Some(NetworkAction::Undo),
        Action::Redo => Some(NetworkAction::Redo),
        // Local-only actions
        Action::Nav(_) => None,
        Action::AudioFeedback(_) => None,
        Action::ExitPerformanceMode => None,
        Action::PushLayer(_) => None,
        Action::PopLayer(_) => None,
        Action::SaveAndQuit => None,
    }
}

/// Sync network display context from RemoteDispatcher to AppState.
#[cfg(feature = "net")]
fn sync_network_context(local_state: &mut state::AppState, remote: &imbolc_net::RemoteDispatcher) {
    use std::collections::HashMap;
    use imbolc_net::OwnershipStatus;
    use state::{NetworkDisplayContext, OwnershipDisplayStatus};

    let mut ownership = HashMap::new();

    for inst in &local_state.instruments.instruments {
        let status = match remote.ownership_status(inst.id) {
            OwnershipStatus::OwnedByMe => OwnershipDisplayStatus::OwnedByMe,
            OwnershipStatus::OwnedByOther(name) => OwnershipDisplayStatus::OwnedByOther(name),
            OwnershipStatus::Unowned => OwnershipDisplayStatus::Unowned,
        };
        ownership.insert(inst.id, status);
    }

    let privileged_client_name = remote.privileged_client().map(|(_, name)| name.to_string());

    local_state.network = Some(NetworkDisplayContext {
        ownership,
        is_privileged: remote.is_privileged(),
        privileged_client_name,
    });
}
