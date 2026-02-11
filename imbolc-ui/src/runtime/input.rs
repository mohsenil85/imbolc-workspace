//! Input processing: event polling, layer resolution, global handler, pane dispatch.
//!
//! All action routing matches must be exhaustive — wildcard arms are denied so
//! that adding a new `UiAction` or `DomainAction` variant forces handling here.
#![deny(clippy::wildcard_enum_match_arm)]

use std::time::Duration;

use imbolc_types::{RoutedAction, UiAction};

use super::AppRuntime;
use crate::action;
use crate::global_actions::*;
use crate::panes::*;
use crate::ui::{self, Action, AppEvent, InputSource, KeyCode, LayerResult};

impl AppRuntime {
    /// Process input events. Returns true if the app should quit.
    pub(crate) fn process_events(
        &mut self,
        backend: &mut crate::ui::RatatuiBackend,
    ) -> std::io::Result<bool> {
        let mut should_quit = false;
        let mut events_processed = 0u8;

        'events: loop {
            let timeout = if events_processed == 0 {
                Duration::from_millis(2)
            } else {
                Duration::ZERO
            };
            let app_event = match backend.poll_event(timeout) {
                Some(e) => e,
                None => break,
            };
            events_processed += 1;

            let pane_action = match app_event {
                AppEvent::Resize(_, _) => {
                    self.render_needed = true;
                    if events_processed >= 16 {
                        break;
                    }
                    continue 'events;
                }
                AppEvent::Mouse(mouse_event) => self.panes.active_mut().handle_mouse(
                    &mouse_event,
                    self.last_area,
                    self.dispatcher.state(),
                ),
                AppEvent::Key(event) => {
                    // Two-digit instrument selection state machine (pre-layer)
                    match &self.select_mode {
                        InstrumentSelectMode::WaitingFirstDigit => {
                            if let KeyCode::Char(c) = event.key {
                                if let Some(d) = c.to_digit(10) {
                                    self.select_mode =
                                        InstrumentSelectMode::WaitingSecondDigit(d as u8);
                                    if events_processed >= 16 {
                                        break;
                                    }
                                    continue 'events;
                                }
                            }
                            self.select_mode = InstrumentSelectMode::Normal;
                        }
                        InstrumentSelectMode::WaitingSecondDigit(first) => {
                            let first = *first;
                            if let KeyCode::Char(c) = event.key {
                                if let Some(d) = c.to_digit(10) {
                                    let combined = first * 10 + d as u8;
                                    let target = if combined == 0 { 10 } else { combined };
                                    select_instrument(
                                        target as usize,
                                        &mut self.dispatcher,
                                        &mut self.panes,
                                        &mut self.audio,
                                    );
                                    self.select_mode = InstrumentSelectMode::Normal;
                                    if events_processed >= 16 {
                                        break;
                                    }
                                    continue 'events;
                                }
                            }
                            self.select_mode = InstrumentSelectMode::Normal;
                        }
                        InstrumentSelectMode::Normal => {}
                    }

                    // Layer resolution
                    match self.layer_stack.resolve(&event) {
                        LayerResult::Action(action) => {
                            match handle_global_action(
                                action,
                                &mut self.dispatcher,
                                &mut self.panes,
                                &mut self.audio,
                                &mut self.app_frame,
                                &mut self.select_mode,
                                &mut self.pending_audio_effects,
                                &mut self.needs_full_sync,
                                &mut self.layer_stack,
                            ) {
                                GlobalResult::Quit => {
                                    should_quit = true;
                                    break 'events;
                                }
                                GlobalResult::RefreshScreen => {
                                    backend.clear()?;
                                    if events_processed >= 16 {
                                        break;
                                    }
                                    continue 'events;
                                }
                                GlobalResult::Handled => {
                                    if events_processed >= 16 {
                                        break;
                                    }
                                    continue 'events;
                                }
                                GlobalResult::NotHandled => self.panes.active_mut().handle_action(
                                    action,
                                    &event,
                                    self.dispatcher.state(),
                                ),
                            }
                        }
                        LayerResult::Blocked | LayerResult::Unresolved => self
                            .panes
                            .active_mut()
                            .handle_raw_input(&event, self.dispatcher.state()),
                    }
                }
            };
            let routed_action = pane_action.route();

            // Log UI actions
            if let RoutedAction::Ui(ref ui_action) = routed_action {
                if let Some(ui_log) = &mut self.ui_log {
                    ui_log.log_ui(self.panes.active().id(), ui_action);
                }
            }

            // Process layer management actions
            process_layer_actions(&pane_action, &mut self.layer_stack, &mut self.panes);

            // Auto-pop text_edit layer when pane is no longer editing
            process_text_edit_auto_pop(&mut self.panes, &mut self.layer_stack);

            // Detect SaveAs cancel during quit flow
            if self.quit_after_save
                && matches!(
                    &routed_action,
                    RoutedAction::Ui(UiAction::Nav(action::NavAction::PopPane))
                )
                && self.panes.active().id() == "save_as"
            {
                self.quit_after_save = false;
            }

            // Bridge mixer detail context to add_effect pane
            if matches!(
                &routed_action,
                RoutedAction::Ui(UiAction::Nav(action::NavAction::PushPane("add_effect")))
            ) && self.panes.active().id() == "mixer"
            {
                if let Some(mixer) = self.panes.get_pane_mut::<MixerPane>("mixer") {
                    let target = mixer.effect_target();
                    if let Some(add_pane) = self.panes.get_pane_mut::<AddEffectPane>("add_effect") {
                        add_pane.set_effect_target(target);
                    }
                }
            }

            // Process navigation and sync pane layer
            process_nav_and_sync(
                &pane_action,
                &mut self.panes,
                &mut self.layer_stack,
                self.dispatcher.state(),
            );

            // Auto-exit clip edit when navigating away from piano roll (standalone-only)
            if matches!(&routed_action, RoutedAction::Ui(UiAction::Nav(_)))
                && self
                    .dispatcher
                    .state()
                    .session
                    .arrangement
                    .editing_clip
                    .is_some()
                && self.panes.active().id() != "piano_roll"
            {
                let mut exit_result = self.dispatcher.dispatch_domain(
                    &action::DomainAction::Arrangement(action::ArrangementAction::ExitClipEdit),
                    &mut self.audio,
                );
                if exit_result.needs_full_sync {
                    self.needs_full_sync = true;
                }
                self.pending_audio_effects
                    .extend(std::mem::take(&mut exit_result.audio_effects));
                apply_dispatch_result(
                    exit_result,
                    &mut self.dispatcher,
                    &mut self.panes,
                    &mut self.app_frame,
                    &mut self.audio,
                );
            }

            // Auto-pop command_palette layer and re-dispatch confirmed command
            if self.layer_stack.has_layer("command_palette")
                && self.panes.active().id() != "command_palette"
            {
                self.layer_stack.pop("command_palette");
                if let Some(palette) = self
                    .panes
                    .get_pane_mut::<CommandPalettePane>("command_palette")
                {
                    if let Some(cmd) = palette.take_command() {
                        let global_result = handle_global_action(
                            cmd,
                            &mut self.dispatcher,
                            &mut self.panes,
                            &mut self.audio,
                            &mut self.app_frame,
                            &mut self.select_mode,
                            &mut self.pending_audio_effects,
                            &mut self.needs_full_sync,
                            &mut self.layer_stack,
                        );
                        if matches!(global_result, GlobalResult::Quit) {
                            should_quit = true;
                            break 'events;
                        }
                        if matches!(global_result, GlobalResult::NotHandled) {
                            let dummy_event =
                                ui::InputEvent::new(KeyCode::Enter, ui::Modifiers::none());
                            let re_action = self.panes.active_mut().handle_action(
                                cmd,
                                &dummy_event,
                                self.dispatcher.state(),
                            );
                            self.panes.process_nav(&re_action, self.dispatcher.state());
                            match re_action.route() {
                                RoutedAction::Ui(UiAction::Nav(_)) => {
                                    sync_pane_layer(&mut self.panes, &mut self.layer_stack);
                                }
                                RoutedAction::Domain(ref domain) => {
                                    let mut r = self
                                        .dispatcher
                                        .dispatch_domain(domain, &mut self.audio);
                                    if r.quit {
                                        should_quit = true;
                                        break 'events;
                                    }
                                    if r.needs_full_sync {
                                        self.needs_full_sync = true;
                                    }
                                    self.pending_audio_effects
                                        .extend(std::mem::take(&mut r.audio_effects));
                                    apply_dispatch_result(
                                        r,
                                        &mut self.dispatcher,
                                        &mut self.panes,
                                        &mut self.app_frame,
                                        &mut self.audio,
                                    );
                                }
                                RoutedAction::Ui(
                                    UiAction::None
                                    | UiAction::Quit
                                    | UiAction::QuitIntent
                                    | UiAction::ExitPerformanceMode
                                    | UiAction::PushLayer(_)
                                    | UiAction::PopLayer(_)
                                    | UiAction::SaveAndQuit,
                                ) => {}
                            }
                        }
                    }
                }
                sync_pane_layer(&mut self.panes, &mut self.layer_stack);
            }

            // Auto-pop pane_switcher layer and switch to selected pane
            process_pane_switcher_auto_pop(
                &mut self.panes,
                &mut self.layer_stack,
                self.dispatcher.state(),
            );

            // Intercept MIDI port actions that need MidiInputManager
            if let Action::Midi(action::MidiAction::ConnectPort(port_idx)) = &pane_action {
                let port_idx = *port_idx;
                self.midi_input.refresh_ports();
                match self.midi_input.connect(port_idx) {
                    Ok(()) => {
                        self.dispatcher.state_mut().midi.connected_port =
                            self.midi_input.connected_port_name().map(|s| s.to_string());
                    }
                    Err(_) => {
                        self.dispatcher.state_mut().midi.connected_port = None;
                    }
                }
                self.dispatcher.state_mut().midi.port_names = self
                    .midi_input
                    .list_ports()
                    .iter()
                    .map(|p| p.name.clone())
                    .collect();
            } else if let Action::Midi(action::MidiAction::DisconnectPort) = &pane_action {
                self.midi_input.disconnect();
                self.dispatcher.state_mut().midi.connected_port = None;
            }

            match routed_action {
                RoutedAction::Ui(UiAction::Quit) => {
                    should_quit = true;
                    break 'events;
                }
                RoutedAction::Ui(UiAction::QuitIntent) => {
                    match handle_quit_intent(
                        &mut self.dispatcher,
                        &mut self.panes,
                        &mut self.layer_stack,
                    ) {
                        GlobalResult::Quit => {
                            should_quit = true;
                            break 'events;
                        }
                        _ => {}
                    }
                }
                RoutedAction::Ui(UiAction::SaveAndQuit) => {
                    handle_save_and_quit(
                        &mut self.dispatcher,
                        &mut self.panes,
                        &mut self.audio,
                        &mut self.app_frame,
                        &mut self.pending_audio_effects,
                        &mut self.needs_full_sync,
                        &mut self.layer_stack,
                        &mut self.quit_after_save,
                    );
                }
                // Already handled above: layer management, navigation, text_edit auto-pop
                RoutedAction::Ui(
                    UiAction::None
                    | UiAction::Nav(_)
                    | UiAction::ExitPerformanceMode
                    | UiAction::PushLayer(_)
                    | UiAction::PopLayer(_),
                ) => {}
                RoutedAction::Domain(ref domain_action) => {
                    let mut dispatch_result = self
                        .dispatcher
                        .dispatch_domain(domain_action, &mut self.audio);
                    if dispatch_result.quit {
                        should_quit = true;
                        break 'events;
                    }
                    if dispatch_result.needs_full_sync {
                        self.needs_full_sync = true;
                    }
                    self.pending_audio_effects
                        .extend(std::mem::take(&mut dispatch_result.audio_effects));
                    apply_dispatch_result(
                        dispatch_result,
                        &mut self.dispatcher,
                        &mut self.panes,
                        &mut self.app_frame,
                        &mut self.audio,
                    );
                }
            }

            if events_processed >= 16 {
                break;
            }
        }
        if events_processed > 0 {
            self.render_needed = true;
        }

        Ok(should_quit)
    }

    /// Process time-based pane updates (key releases, etc.)
    pub(crate) fn process_tick(&mut self) {
        let tick_actions = self.panes.active_mut().tick(self.dispatcher.state());
        if !tick_actions.is_empty() {
            self.render_needed = true;
        }
        for action in &tick_actions {
            match action.route() {
                RoutedAction::Domain(ref domain) => {
                    let r = self.dispatcher.dispatch_domain(domain, &mut self.audio);
                    if r.needs_full_sync {
                        self.needs_full_sync = true;
                    }
                    self.pending_audio_effects.extend(r.audio_effects);
                }
                // Tick actions should only produce domain actions
                RoutedAction::Ui(
                    UiAction::None
                    | UiAction::Quit
                    | UiAction::QuitIntent
                    | UiAction::Nav(_)
                    | UiAction::ExitPerformanceMode
                    | UiAction::PushLayer(_)
                    | UiAction::PopLayer(_)
                    | UiAction::SaveAndQuit,
                ) => {}
            }
        }
    }
}
