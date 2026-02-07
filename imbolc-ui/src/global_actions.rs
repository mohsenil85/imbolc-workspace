use crate::action::{
    AudioDirty, ClickAction, MixerAction, PianoRollAction, SequencerAction,
    AutomationAction, Action
};
use crate::audio::AudioHandle;
use crate::state::{AppState, MixerSelection, ClipboardContents};
use crate::dispatch::LocalDispatcher;
use crate::panes::{
    CommandPalettePane, DocsPane, InstrumentEditPane, PaneSwitcherPane, PianoRollPane, SequencerPane,
    AutomationPane, ServerPane, HelpPane, FileBrowserPane, VstParamPane,
    ConfirmPane, SaveAsPane, PendingAction,
};
use crate::ui::{
    self, DispatchResult, Frame, LayerStack, NavIntent, PaneManager,
    SessionAction, StatusEvent, ToggleResult, ViewState
};
use crate::ui::action_id::{ActionId, GlobalActionId, PaneId};

/// Two-digit instrument selection state machine
pub(crate) enum InstrumentSelectMode {
    Normal,
    WaitingFirstDigit,
    WaitingSecondDigit(u8),
}

pub(crate) enum GlobalResult {
    Quit,
    RefreshScreen,
    Handled,
    NotHandled,
}

/// Select instrument by 1-based number (1=first, 10=tenth) and sync piano roll
pub(crate) fn select_instrument(
    number: usize,
    dispatcher: &mut LocalDispatcher,
    panes: &mut PaneManager,
    audio: &mut AudioHandle,
) {
    let idx = number.saturating_sub(1); // Convert 1-based to 0-based
    if idx < dispatcher.state().instruments.instruments.len() {
        dispatcher.dispatch_with_audio(&Action::Instrument(ui::InstrumentAction::Select(idx)), audio);
        sync_piano_roll_to_selection(dispatcher, panes, audio);
        sync_instrument_edit(dispatcher.state(), panes);
    }
}

/// Sync piano roll's current track to match the globally selected instrument,
/// and re-route the active pane if on a F2-family pane (piano_roll/sequencer/waveform).
pub(crate) fn sync_piano_roll_to_selection(
    dispatcher: &mut LocalDispatcher,
    panes: &mut PaneManager,
    audio: &mut AudioHandle,
) {
    let state = dispatcher.state();
    if let Some(selected_idx) = state.instruments.selected {
        // Extract data from instrument before any mutable borrows
        let inst_data = state.instruments.instruments.get(selected_idx).map(|inst| {
            (inst.id, inst.source.is_kit(), inst.source.is_audio_input(), inst.source.is_bus_in())
        });

        if let Some((inst_id, is_kit, is_audio_in, is_bus_in)) = inst_data {
            // Find which track index corresponds to this instrument
            if let Some(track_idx) = state.session.piano_roll.track_order.iter()
                .position(|&id| id == inst_id)
            {
                if let Some(pr_pane) = panes.get_pane_mut::<PianoRollPane>("piano_roll") {
                    pr_pane.set_current_track(track_idx);
                }
            }

            // Sync mixer selection via dispatch
            let active = panes.active().id();
            if active == "mixer" {
                if let MixerSelection::Instrument(_) = dispatcher.state().session.mixer.selection {
                    dispatcher.dispatch_with_audio(
                        &Action::Mixer(MixerAction::SelectAt(MixerSelection::Instrument(selected_idx))),
                        audio,
                    );
                }
            }

            // Re-route if currently on a F2-family pane
            if active == "piano_roll" || active == "sequencer" || active == "waveform" {
                let target = if is_kit {
                    "sequencer"
                } else if is_audio_in || is_bus_in {
                    "waveform"
                } else {
                    "piano_roll"
                };
                if active != target {
                    panes.switch_to(target, dispatcher.state());
                }
            }
        }
    }
}

/// If the instrument edit pane is active, reload it with the currently selected instrument.
pub(crate) fn sync_instrument_edit(state: &AppState, panes: &mut PaneManager) {
    if panes.active().id() == "instrument_edit" {
        if let Some(inst) = state.instruments.selected_instrument() {
            if let Some(edit_pane) = panes.get_pane_mut::<InstrumentEditPane>("instrument_edit") {
                edit_pane.set_instrument(inst);
            }
        }
    }
}

/// Sync layer stack pane layer and performance mode state after pane switch.
pub(crate) fn sync_pane_layer(panes: &mut PaneManager, layer_stack: &mut LayerStack) {
    let had_piano = layer_stack.has_layer("piano_mode");
    let had_pad = layer_stack.has_layer("pad_mode");
    layer_stack.set_pane_layer(panes.active().id());

    if had_piano || had_pad {
        if panes.active_mut().supports_performance_mode() {
            if had_piano { panes.active_mut().activate_piano(); }
            if had_pad { panes.active_mut().activate_pad(); }
        } else {
            layer_stack.pop("piano_mode");
            layer_stack.pop("pad_mode");
            panes.active_mut().deactivate_performance();
        }
    }
}

pub(crate) fn handle_global_action(
    action: ActionId,
    dispatcher: &mut LocalDispatcher,
    panes: &mut PaneManager,
    audio: &mut AudioHandle,
    app_frame: &mut Frame,
    select_mode: &mut InstrumentSelectMode,
    pending_audio_dirty: &mut AudioDirty,
    layer_stack: &mut LayerStack,
) -> GlobalResult {
    // Helper to capture current view state
    let capture_view = |panes: &mut PaneManager, state: &AppState| -> ViewState {
        let pane_id = panes.active().id().to_string();
        let inst_selection = state.instruments.selected;
        let edit_tab = panes.get_pane_mut::<InstrumentEditPane>("instrument_edit")
            .map(|ep| ep.tab_index())
            .unwrap_or(0);
        ViewState { pane_id, inst_selection, edit_tab }
    };

    // Helper to restore view state
    let restore_view = |panes: &mut PaneManager, dispatcher: &mut LocalDispatcher, view: &ViewState| {
        dispatcher.state_mut().instruments.selected = view.inst_selection;
        if let Some(edit_pane) = panes.get_pane_mut::<InstrumentEditPane>("instrument_edit") {
            edit_pane.set_tab_index(view.edit_tab);
        }
        panes.switch_to(&view.pane_id, dispatcher.state());
    };

    // Helper for pane switching with view history
    let switch_to_pane = |target: &str, panes: &mut PaneManager, dispatcher: &mut LocalDispatcher, audio: &mut AudioHandle, app_frame: &mut Frame, layer_stack: &mut LayerStack| {
        let current = capture_view(panes, dispatcher.state());
        if app_frame.view_history.is_empty() {
            app_frame.view_history.push(current);
        } else {
            app_frame.view_history[app_frame.history_cursor] = current;
        }
        // Truncate forward history
        app_frame.view_history.truncate(app_frame.history_cursor + 1);
        // Switch and record new view
        panes.switch_to(target, dispatcher.state());
        sync_pane_layer(panes, layer_stack);
        // Sync mixer highlight to global instrument selection on entry
        if target == "mixer" {
            if let Some(selected_idx) = dispatcher.state().instruments.selected {
                dispatcher.dispatch_with_audio(
                    &Action::Mixer(MixerAction::SelectAt(MixerSelection::Instrument(selected_idx))),
                    audio,
                );
            }
        }
        let new_view = capture_view(panes, dispatcher.state());
        app_frame.view_history.push(new_view);
        app_frame.history_cursor = app_frame.view_history.len() - 1;
    };

    match action {
        ActionId::Global(g) => match g {
            GlobalActionId::Quit => {
                if dispatcher.state().project.dirty {
                    panes.push_to("quit_prompt", dispatcher.state());
                    sync_pane_layer(panes, layer_stack);
                    return GlobalResult::Handled;
                }
                return GlobalResult::Quit;
            }
            GlobalActionId::Undo => {
                let r = dispatcher.dispatch_with_audio(&Action::Undo, audio);
                pending_audio_dirty.merge(r.audio_dirty);
                apply_dispatch_result(r, dispatcher, panes, app_frame, audio);
                sync_piano_roll_to_selection(dispatcher, panes, audio);
                sync_instrument_edit(dispatcher.state(), panes);
            }
            GlobalActionId::Redo => {
                let r = dispatcher.dispatch_with_audio(&Action::Redo, audio);
                pending_audio_dirty.merge(r.audio_dirty);
                apply_dispatch_result(r, dispatcher, panes, app_frame, audio);
                sync_piano_roll_to_selection(dispatcher, panes, audio);
                sync_instrument_edit(dispatcher.state(), panes);
            }
            GlobalActionId::Save => {
                if dispatcher.state().project.path.is_none() {
                    // Unnamed project — open SaveAs
                    let default_name = "untitled".to_string();
                    if let Some(sa) = panes.get_pane_mut::<SaveAsPane>("save_as") {
                        sa.reset(&default_name);
                    }
                    panes.push_to("save_as", dispatcher.state());
                    sync_pane_layer(panes, layer_stack);
                } else {
                    let r = dispatcher.dispatch_with_audio(&Action::Session(SessionAction::Save), audio);
                    pending_audio_dirty.merge(r.audio_dirty);
                    apply_dispatch_result(r, dispatcher, panes, app_frame, audio);
                }
            }
            GlobalActionId::Load => {
                if dispatcher.state().project.dirty {
                    if let Some(confirm) = panes.get_pane_mut::<ConfirmPane>("confirm") {
                        confirm.set_confirm("Discard unsaved changes and reload?", PendingAction::LoadDefault);
                    }
                    panes.push_to("confirm", dispatcher.state());
                    sync_pane_layer(panes, layer_stack);
                } else {
                    let r = dispatcher.dispatch_with_audio(&Action::Session(SessionAction::Load), audio);
                    pending_audio_dirty.merge(r.audio_dirty);
                    apply_dispatch_result(r, dispatcher, panes, app_frame, audio);
                }
            }
            GlobalActionId::SaveAs => {
                let default_name = dispatcher.state().project.path.as_ref()
                    .and_then(|p| p.file_stem())
                    .and_then(|s| s.to_str())
                    .unwrap_or("untitled")
                    .to_string();
                if let Some(sa) = panes.get_pane_mut::<SaveAsPane>("save_as") {
                    sa.reset(&default_name);
                }
                panes.push_to("save_as", dispatcher.state());
                sync_pane_layer(panes, layer_stack);
            }
            GlobalActionId::OpenProjectBrowser => {
                panes.push_to("project_browser", dispatcher.state());
                sync_pane_layer(panes, layer_stack);
            }
            GlobalActionId::MasterMute => {
                let r = dispatcher.dispatch_with_audio(&Action::Session(SessionAction::ToggleMasterMute), audio);
                pending_audio_dirty.merge(r.audio_dirty);
                apply_dispatch_result(r, dispatcher, panes, app_frame, audio);
            }
            GlobalActionId::ClickTrackToggle => {
                let r = dispatcher.dispatch_with_audio(&Action::Click(ClickAction::Toggle), audio);
                pending_audio_dirty.merge(r.audio_dirty);
                apply_dispatch_result(r, dispatcher, panes, app_frame, audio);
            }
            GlobalActionId::RecordMaster => {
                let r = dispatcher.dispatch_with_audio(&Action::Server(ui::ServerAction::RecordMaster), audio);
                pending_audio_dirty.merge(r.audio_dirty);
                apply_dispatch_result(r, dispatcher, panes, app_frame, audio);
            }
            GlobalActionId::Copy => {
                copy_from_active_pane(dispatcher, panes, audio);
            }
            GlobalActionId::Cut => {
                let action = cut_from_active_pane(dispatcher, panes, audio);
                if let Some(action) = action {
                    let r = dispatcher.dispatch_with_audio(&action, audio);
                    pending_audio_dirty.merge(r.audio_dirty);
                    apply_dispatch_result(r, dispatcher, panes, app_frame, audio);
                }
            }
            GlobalActionId::Paste => {
                let action = paste_to_active_pane(dispatcher.state_mut(), panes);
                if let Some(action) = action {
                    let r = dispatcher.dispatch_with_audio(&action, audio);
                    pending_audio_dirty.merge(r.audio_dirty);
                    apply_dispatch_result(r, dispatcher, panes, app_frame, audio);
                }
            }
            GlobalActionId::SelectAll => {
                select_all_in_active_pane(dispatcher.state_mut(), panes);
            }
            GlobalActionId::SwitchPane(PaneId::InstrumentEdit) => {
                let target = if dispatcher.state().instruments.instruments.is_empty() {
                    "add"
                } else {
                    "instrument_edit"
                };
                switch_to_pane(target, panes, dispatcher, audio, app_frame, layer_stack);
            }
            GlobalActionId::SwitchPane(PaneId::InstrumentList) => {
                switch_to_pane("instrument", panes, dispatcher, audio, app_frame, layer_stack);
            }
            GlobalActionId::SwitchPane(PaneId::PianoRollOrSequencer) => {
                let target = if let Some(inst) = dispatcher.state().instruments.selected_instrument() {
                    if inst.source.is_kit() {
                        "sequencer"
                    } else if inst.source.is_audio_input() || inst.source.is_bus_in() {
                        "waveform"
                    } else {
                        "piano_roll"
                    }
                } else {
                    "piano_roll"
                };
                switch_to_pane(target, panes, dispatcher, audio, app_frame, layer_stack);
            }
            GlobalActionId::SwitchPane(PaneId::Track) => {
                switch_to_pane("track", panes, dispatcher, audio, app_frame, layer_stack);
            }
            GlobalActionId::SwitchPane(PaneId::Mixer) => {
                switch_to_pane("mixer", panes, dispatcher, audio, app_frame, layer_stack);
            }
            GlobalActionId::SwitchPane(PaneId::Server) => {
                switch_to_pane("server", panes, dispatcher, audio, app_frame, layer_stack);
            }
            GlobalActionId::SwitchPane(PaneId::Automation) => {
                switch_to_pane("automation", panes, dispatcher, audio, app_frame, layer_stack);
            }
            GlobalActionId::SwitchPane(PaneId::Eq) => {
                switch_to_pane("eq", panes, dispatcher, audio, app_frame, layer_stack);
            }
            GlobalActionId::SwitchPane(PaneId::MidiSettings) => {
                switch_to_pane("midi_settings", panes, dispatcher, audio, app_frame, layer_stack);
            }
            GlobalActionId::SwitchPane(PaneId::Groove) => {
                switch_to_pane("groove", panes, dispatcher, audio, app_frame, layer_stack);
            }
            GlobalActionId::SwitchPane(PaneId::FrameEdit) => {
                if panes.active().id() == "frame_edit" {
                    panes.pop(dispatcher.state());
                } else {
                    panes.push_to("frame_edit", dispatcher.state());
                }
            }
            GlobalActionId::NavBack => {
                let history = &mut app_frame.view_history;
                if !history.is_empty() {
                    let current = capture_view(panes, dispatcher.state());
                    history[app_frame.history_cursor] = current;

                    let at_front = app_frame.history_cursor == history.len() - 1;
                    if at_front {
                        if app_frame.history_cursor > 0 {
                            app_frame.history_cursor -= 1;
                            let view = history[app_frame.history_cursor].clone();
                            restore_view(panes, dispatcher, &view);
                            sync_pane_layer(panes, layer_stack);
                        }
                    } else {
                        if app_frame.history_cursor < history.len() - 1 {
                            app_frame.history_cursor += 1;
                            let view = history[app_frame.history_cursor].clone();
                            restore_view(panes, dispatcher, &view);
                            sync_pane_layer(panes, layer_stack);
                        }
                    }
                }
            }
            GlobalActionId::NavForward => {
                let history = &mut app_frame.view_history;
                if !history.is_empty() {
                    let current = capture_view(panes, dispatcher.state());
                    history[app_frame.history_cursor] = current;

                    let at_front = app_frame.history_cursor == history.len() - 1;
                    if at_front {
                        let target = app_frame.history_cursor.saturating_sub(2);
                        if target != app_frame.history_cursor {
                            app_frame.history_cursor = target;
                            let view = history[app_frame.history_cursor].clone();
                            restore_view(panes, dispatcher, &view);
                            sync_pane_layer(panes, layer_stack);
                        }
                    } else {
                        let target = (app_frame.history_cursor + 2).min(history.len() - 1);
                        if target != app_frame.history_cursor {
                            app_frame.history_cursor = target;
                            let view = history[app_frame.history_cursor].clone();
                            restore_view(panes, dispatcher, &view);
                            sync_pane_layer(panes, layer_stack);
                        }
                    }
                }
            }
            GlobalActionId::Help => {
                if panes.active().id() != "help" {
                    let current_id = panes.active().id();
                    let current_keymap = panes.active().keymap().clone();
                    let title = match current_id {
                        "instrument" => "Instruments",
                        "mixer" => "Mixer",
                        "server" => "Server",
                        "piano_roll" => "Piano Roll",
                        "sequencer" => "Step Sequencer",
                        "add" => "Add Instrument",
                        "instrument_edit" => "Edit Instrument",
                        "track" => "Track",
                        "waveform" => "Waveform",
                        "automation" => "Automation",
                        "eq" => "Parametric EQ",
                        _ => current_id,
                    };
                    if let Some(help) = panes.get_pane_mut::<HelpPane>("help") {
                        help.set_context(current_id, title, &current_keymap);
                    }
                    panes.push_to("help", dispatcher.state());
                }
            }
            GlobalActionId::OpenDocs => {
                // Open docs for the currently selected instrument's source type
                if let Some(docs) = panes.get_pane_mut::<DocsPane>("docs") {
                    if let Some(inst) = dispatcher.state().instruments.selected_instrument() {
                        let short_name = inst.source.short_name().to_lowercase();
                        docs.open_for_source(&short_name);
                    } else {
                        docs.open_browser();
                    }
                }
                panes.push_to("docs", dispatcher.state());
                sync_pane_layer(panes, layer_stack);
            }
            GlobalActionId::OpenLearn => {
                // Open the topic browser
                if let Some(docs) = panes.get_pane_mut::<DocsPane>("docs") {
                    docs.open_browser();
                }
                panes.push_to("docs", dispatcher.state());
                sync_pane_layer(panes, layer_stack);
            }
            GlobalActionId::SelectInstrument(n) => {
                select_instrument(n as usize, dispatcher, panes, audio);
            }
            GlobalActionId::SelectPrevInstrument => {
                dispatcher.dispatch_with_audio(&Action::Instrument(ui::InstrumentAction::SelectPrev), audio);
                sync_piano_roll_to_selection(dispatcher, panes, audio);
                sync_instrument_edit(dispatcher.state(), panes);
            }
            GlobalActionId::SelectNextInstrument => {
                dispatcher.dispatch_with_audio(&Action::Instrument(ui::InstrumentAction::SelectNext), audio);
                sync_piano_roll_to_selection(dispatcher, panes, audio);
                sync_instrument_edit(dispatcher.state(), panes);
            }
            GlobalActionId::SelectTwoDigit => {
                *select_mode = InstrumentSelectMode::WaitingFirstDigit;
            }
            GlobalActionId::TogglePianoMode => {
                let result = panes.active_mut().toggle_performance_mode(dispatcher.state());
                match result {
                    ToggleResult::ActivatedPiano => {
                        layer_stack.push("piano_mode");
                    }
                    ToggleResult::ActivatedPad => {
                        layer_stack.push("pad_mode");
                    }
                    ToggleResult::Deactivated => {
                        layer_stack.pop("piano_mode");
                        layer_stack.pop("pad_mode");
                    }
                    ToggleResult::CycledLayout | ToggleResult::NotSupported => {}
                }
            }
            GlobalActionId::AddInstrument => {
                switch_to_pane("add", panes, dispatcher, audio, app_frame, layer_stack);
            }
            GlobalActionId::DeleteInstrument => {
                if let Some(instrument) = dispatcher.state().instruments.selected_instrument() {
                    let id = instrument.id;
                    let r = dispatcher.dispatch_with_audio(&Action::Instrument(ui::InstrumentAction::Delete(id)), audio);
                    pending_audio_dirty.merge(r.audio_dirty);
                    apply_dispatch_result(r, dispatcher, panes, app_frame, audio);
                    // Re-sync edit pane after deletion
                    sync_instrument_edit(dispatcher.state(), panes);
                }
            }
            GlobalActionId::CommandPalette => {
                let commands = layer_stack.collect_commands();
                if let Some(palette) = panes.get_pane_mut::<CommandPalettePane>("command_palette") {
                    palette.open(commands);
                }
                panes.push_to("command_palette", dispatcher.state());
                layer_stack.push("command_palette");
            }
            GlobalActionId::PaneSwitcher => {
                if let Some(switcher) = panes.get_pane_mut::<PaneSwitcherPane>("pane_switcher") {
                    switcher.open();
                }
                panes.push_to("pane_switcher", dispatcher.state());
                layer_stack.push("pane_switcher");
            }
            GlobalActionId::PlayStop => {
                // Skip during export/render
                if dispatcher.state().io.pending_export.is_some() || dispatcher.state().io.pending_render.is_some() {
                    return GlobalResult::Handled;
                }
                let state = dispatcher.state_mut();
                let pr = &mut state.session.piano_roll;
                pr.playing = !pr.playing;
                let playing = pr.playing;
                audio.set_playing(playing);
                if !playing {
                    dispatcher.state_mut().audio.playhead = 0;
                    audio.reset_playhead();
                    if audio.is_running() {
                        audio.release_all_voices();
                    }
                    audio.clear_active_notes();
                }
                dispatcher.state_mut().session.piano_roll.recording = false;

                // Unify: toggle all drum sequencers
                for inst in &mut dispatcher.state_mut().instruments.instruments {
                    if let Some(seq) = &mut inst.drum_sequencer {
                        seq.playing = playing;
                        if !playing {
                            seq.current_step = 0;
                            seq.step_accumulator = 0.0;
                        }
                    }
                }
                pending_audio_dirty.instruments = true;
            }
            GlobalActionId::Escape => {
                // Global escape — falls through to pane when no mode layer handles it
                return GlobalResult::NotHandled;
            }
            GlobalActionId::RefreshScreen => {
                return GlobalResult::RefreshScreen;
            }
            GlobalActionId::CycleTheme => {
                let r = dispatcher.dispatch_with_audio(&Action::Session(SessionAction::CycleTheme), audio);
                pending_audio_dirty.merge(r.audio_dirty);
                apply_dispatch_result(r, dispatcher, panes, app_frame, audio);
            }
            GlobalActionId::RequestPrivilege => {
                // No-op in standalone mode (handled in network client loop)
            }
        },
        _ => return GlobalResult::NotHandled,
    }
    GlobalResult::Handled
}

/// Apply status events from dispatch or setup to the server pane
pub(crate) fn apply_status_events(events: &[StatusEvent], panes: &mut PaneManager) {
    for event in events {
        if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
            server.set_status(event.status, &event.message);
            if let Some(running) = event.server_running {
                server.set_server_running(running);
            }
        }
    }
}

/// Apply a DispatchResult to the UI layer: process nav intents, status events, project name,
/// and audio control signals (stop_playback, reset_playhead).
pub(crate) fn apply_dispatch_result(
    result: DispatchResult,
    dispatcher: &mut LocalDispatcher,
    panes: &mut PaneManager,
    app_frame: &mut Frame,
    audio: &mut AudioHandle,
) {
    // Process nav intents
    for intent in &result.nav {
        match intent {
            NavIntent::OpenFileBrowser(file_action) => {
                if let Some(fb) = panes.get_pane_mut::<FileBrowserPane>("file_browser") {
                    fb.open_for(file_action.clone(), None);
                }
                panes.push_to("file_browser", dispatcher.state());
            }
            NavIntent::OpenVstParams(instrument_id, target) => {
                if let Some(vp) = panes.get_pane_mut::<VstParamPane>("vst_params") {
                    vp.set_target(*instrument_id, *target);
                }
                panes.push_to("vst_params", dispatcher.state());
            }
            _ => {}
        }
    }
    panes.process_nav_intents(&result.nav, dispatcher.state());

    // Process status events
    apply_status_events(&result.status, panes);

    // Process project name
    if let Some(ref name) = result.project_name {
        app_frame.set_project_name(name.to_string());
    }

    // Process audio control signals (avoids circular dispatch → audio → dispatch pattern)
    if result.stop_playback {
        audio.set_playing(false);
    }
    if result.reset_playhead {
        audio.reset_playhead();
    }
}

fn copy_from_active_pane(
    dispatcher: &mut LocalDispatcher,
    panes: &mut PaneManager,
    audio: &mut AudioHandle,
) {
    let pane_id = panes.active().id();
    match pane_id {
        "piano_roll" => {
            if let Some(pane) = panes.get_pane_mut::<PianoRollPane>("piano_roll") {
                let (track, start_tick, end_tick, start_pitch, end_pitch) = pane.selection_region();
                dispatcher.dispatch_with_audio(
                    &Action::PianoRoll(PianoRollAction::CopyNotes {
                        track, start_tick, end_tick, start_pitch, end_pitch,
                    }),
                    audio,
                );
            }
        }
        "sequencer" => {
            if let Some(pane) = panes.get_pane_mut::<SequencerPane>("sequencer") {
                let (start_pad, end_pad, start_step, end_step) = pane.selection_region();
                dispatcher.dispatch_with_audio(
                    &Action::Sequencer(SequencerAction::CopySteps {
                        start_pad, end_pad, start_step, end_step,
                    }),
                    audio,
                );
            }
        }
        "automation" => {
            if let Some(pane) = panes.get_pane_mut::<AutomationPane>("automation") {
                if let Some((lane_id, start_tick, end_tick)) = pane.selection_region(dispatcher.state()) {
                    dispatcher.dispatch_with_audio(
                        &Action::Automation(AutomationAction::CopyPoints(lane_id, start_tick, end_tick)),
                        audio,
                    );
                }
            }
        }
        _ => {}
    }
}

fn cut_from_active_pane(
    dispatcher: &mut LocalDispatcher,
    panes: &mut PaneManager,
    audio: &mut AudioHandle,
) -> Option<Action> {
    // Copy first
    copy_from_active_pane(dispatcher, panes, audio);

    // Then return delete action
    let pane_id = panes.active().id();
    match pane_id {
        "piano_roll" => {
            if let Some(pane) = panes.get_pane_mut::<PianoRollPane>("piano_roll") {
                 if let Some((anchor_tick, anchor_pitch)) = pane.selection_anchor {
                     let (tick_start, tick_end) = if anchor_tick <= pane.cursor_tick {
                         (anchor_tick, pane.cursor_tick + pane.ticks_per_cell())
                     } else {
                         (pane.cursor_tick, anchor_tick + pane.ticks_per_cell())
                     };
                     let (pitch_start, pitch_end) = if anchor_pitch <= pane.cursor_pitch {
                         (anchor_pitch, pane.cursor_pitch)
                     } else {
                         (pane.cursor_pitch, anchor_pitch)
                     };

                     // Clear selection after cut
                     pane.selection_anchor = None;

                     return Some(Action::PianoRoll(PianoRollAction::DeleteNotesInRegion {
                         track: pane.current_track,
                         start_tick: tick_start,
                         end_tick: tick_end,
                         start_pitch: pitch_start,
                         end_pitch: pitch_end,
                     }));
                 }
            }
        }
        "sequencer" => {
            if let Some(pane) = panes.get_pane_mut::<SequencerPane>("sequencer") {
                 if let Some((anchor_pad, anchor_step)) = pane.selection_anchor {
                     let (pad_start, pad_end) = if anchor_pad <= pane.cursor_pad {
                         (anchor_pad, pane.cursor_pad)
                     } else {
                         (pane.cursor_pad, anchor_pad)
                     };
                     let (step_start, step_end) = if anchor_step <= pane.cursor_step {
                         (anchor_step, pane.cursor_step)
                     } else {
                         (pane.cursor_step, anchor_step)
                     };
                     pane.selection_anchor = None;

                     return Some(Action::Sequencer(SequencerAction::DeleteStepsInRegion {
                         start_pad: pad_start,
                         end_pad: pad_end,
                         start_step: step_start,
                         end_step: step_end,
                     }));
                 }
            }
        }
        "automation" => {
            if let Some(pane) = panes.get_pane_mut::<AutomationPane>("automation") {
                if let Some(anchor_tick) = pane.selection_anchor_tick {
                     let (tick_start, tick_end) = if anchor_tick <= pane.cursor_tick {
                         (anchor_tick, pane.cursor_tick)
                     } else {
                         (pane.cursor_tick, anchor_tick)
                     };
                     if let Some(lane_id) = pane.selected_lane_id(dispatcher.state()) {
                         pane.selection_anchor_tick = None;
                         return Some(Action::Automation(AutomationAction::DeletePointsInRange(lane_id, tick_start, tick_end)));
                     }
                }
            }
        }
        _ => {}
    }
    None
}

fn paste_to_active_pane(state: &mut AppState, panes: &mut PaneManager) -> Option<Action> {
    if let Some(contents) = &state.clipboard.contents {
        match contents {
            ClipboardContents::PianoRollNotes(notes) => {
                if panes.active().id() == "piano_roll" {
                    if let Some(pane) = panes.get_pane_mut::<PianoRollPane>("piano_roll") {
                        // anchor is cursor position
                        let action = Action::PianoRoll(PianoRollAction::PasteNotes {
                            track: pane.current_track,
                            anchor_tick: pane.cursor_tick,
                            anchor_pitch: pane.cursor_pitch,
                            notes: notes.clone(),
                        });
                        // Clear selection if any (optional, but good UX)
                        pane.selection_anchor = None;
                        return Some(action);
                    }
                }
            }
            ClipboardContents::DrumSteps { steps } => {
                if panes.active().id() == "sequencer" {
                    if let Some(pane) = panes.get_pane_mut::<SequencerPane>("sequencer") {
                        let action = Action::Sequencer(SequencerAction::PasteSteps {
                            anchor_pad: pane.cursor_pad,
                            anchor_step: pane.cursor_step,
                            steps: steps.clone(),
                        });
                        pane.selection_anchor = None;
                        return Some(action);
                    }
                }
            }
            ClipboardContents::AutomationPoints { points } => {
                if panes.active().id() == "automation" {
                    if let Some(pane) = panes.get_pane_mut::<AutomationPane>("automation") {
                        if let Some(lane_id) = pane.selected_lane_id(state) {
                            let action = Action::Automation(AutomationAction::PastePoints(
                                lane_id,
                                pane.cursor_tick,
                                points.clone(),
                            ));
                            pane.selection_anchor_tick = None;
                            return Some(action);
                        }
                    }
                }
            }
        }
    }
    None
}

fn select_all_in_active_pane(state: &mut AppState, panes: &mut PaneManager) {
    let pane_id = panes.active().id();
    match pane_id {
        "piano_roll" => {
            if let Some(pane) = panes.get_pane_mut::<PianoRollPane>("piano_roll") {
                if let Some(track) = state.session.piano_roll.track_at(pane.current_track) {
                    if let Some(min_tick) = track.notes.iter().map(|n| n.tick).min() {
                        let max_tick = track.notes.iter().map(|n| n.tick + n.duration).max().unwrap_or(min_tick);
                        let min_pitch = track.notes.iter().map(|n| n.pitch).min().unwrap_or(0);
                        let max_pitch = track.notes.iter().map(|n| n.pitch).max().unwrap_or(127);

                        pane.selection_anchor = Some((min_tick, min_pitch));
                        pane.cursor_tick = max_tick;
                        pane.cursor_pitch = max_pitch;
                        pane.scroll_to_cursor();
                    }
                }
            }
        }
        "sequencer" => {
            if let Some(pane) = panes.get_pane_mut::<SequencerPane>("sequencer") {
                if let Some(seq) = state.instruments.selected_drum_sequencer() {
                    let pattern = seq.pattern();
                    pane.selection_anchor = Some((0, 0));
                    pane.cursor_pad = crate::state::drum_sequencer::NUM_PADS - 1;
                    pane.cursor_step = pattern.length - 1;
                }
            }
        }
        _ => {}
    }
}
