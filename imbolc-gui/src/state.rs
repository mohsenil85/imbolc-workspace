//! State management for the GUI.
//!
//! Wraps AppState and AudioHandle for use with Dioxus signals.

use std::sync::mpsc::{self, Receiver, Sender};

use imbolc_core::action::IoFeedback;
use imbolc_core::audio::AudioHandle;
use imbolc_core::config::Config;
use imbolc_core::state::AppState;
use imbolc_types::{Action, RoutedAction, UiAction};

/// Shared state wrapper for the GUI.
///
/// Contains the core AppState, AudioHandle, and I/O channels.
/// Wrapped in a Dioxus Signal for reactivity.
pub struct SharedState {
    pub app: AppState,
    pub audio: AudioHandle,
    io_tx: Sender<IoFeedback>,
    io_rx: Receiver<IoFeedback>,
}

impl SharedState {
    pub fn new() -> Self {
        let config = Config::load();
        let app = AppState::new_with_defaults(config.defaults());
        let audio = AudioHandle::new();
        let (io_tx, io_rx) = mpsc::channel();

        Self {
            app,
            audio,
            io_tx,
            io_rx,
        }
    }

    /// Dispatch an action to the core.
    pub fn dispatch(&mut self, action: Action) {
        match action.route() {
            RoutedAction::Domain(domain) => {
                let result = imbolc_core::dispatch::dispatch_action(
                    &domain,
                    &mut self.app,
                    &mut self.audio,
                    &self.io_tx,
                );

                // Forward action to audio thread for incremental state projection
                let reducible = imbolc_types::reduce::is_reducible(&domain);
                self.audio.forward_action(&domain, &result.audio_effects);

                // Handle audio effects
                if !result.audio_effects.is_empty() {
                    let needs_full_sync = !reducible;
                    self.audio
                        .apply_effects(&self.app, &result.audio_effects, needs_full_sync);
                }

                // Handle quit
                if result.quit {
                    std::process::exit(0);
                }
            }
            RoutedAction::Ui(ui) => match ui {
                UiAction::Quit => std::process::exit(0),
                UiAction::None
                | UiAction::QuitIntent
                | UiAction::Nav(_)
                | UiAction::ExitPerformanceMode
                | UiAction::PushLayer(_)
                | UiAction::PopLayer(_)
                | UiAction::SaveAndQuit => {}
            },
        }
    }

    /// Poll audio feedback and I/O feedback.
    pub fn poll_audio_feedback(&mut self) {
        // Drain audio feedback
        let feedback = self.audio.drain_feedback();
        for fb in feedback {
            // Convert AudioFeedback to DomainAction and dispatch
            let action = imbolc_types::DomainAction::AudioFeedback(fb);
            let _ = imbolc_core::dispatch::dispatch_action(
                &action,
                &mut self.app,
                &mut self.audio,
                &self.io_tx,
            );
        }

        // Drain I/O feedback
        while let Ok(io_fb) = self.io_rx.try_recv() {
            match io_fb {
                IoFeedback::SaveComplete { result, .. } => {
                    self.app.io.save_in_progress = false;
                    match result {
                        Ok(_) => {
                            self.app.io.last_io_error = None;
                        }
                        Err(e) => {
                            log::error!("Save failed: {}", e);
                            self.app.io.last_io_error = Some(e);
                        }
                    }
                }
                IoFeedback::LoadComplete { result, .. } => {
                    self.app.io.load_in_progress = false;
                    match result {
                        Ok((session, instruments, _)) => {
                            self.app.session = session;
                            self.app.instruments = instruments;
                            self.app.io.last_io_error = None;
                        }
                        Err(e) => {
                            log::error!("Load failed: {}", e);
                            self.app.io.last_io_error = Some(e);
                        }
                    }
                }
                IoFeedback::AutosaveComplete { .. } => {}
                IoFeedback::ImportSynthDefComplete { .. } => {}
                IoFeedback::ImportSynthDefLoaded { .. } => {}
            }
        }
    }

    /// Get the I/O sender for async operations.
    #[allow(dead_code)]
    pub fn io_tx(&self) -> &Sender<IoFeedback> {
        &self.io_tx
    }
}
