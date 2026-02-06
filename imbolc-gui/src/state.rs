//! State management for the GUI.
//!
//! Wraps AppState and AudioHandle for use with Dioxus signals.

use std::sync::mpsc::{self, Receiver, Sender};

use imbolc_core::action::IoFeedback;
use imbolc_core::audio::AudioHandle;
use imbolc_core::config::Config;
use imbolc_core::state::AppState;
use imbolc_types::Action;

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
        let result = imbolc_core::dispatch::dispatch_action(
            &action,
            &mut self.app,
            &mut self.audio,
            &self.io_tx,
        );

        // Handle audio dirty flags
        if result.audio_dirty.any() {
            self.audio.flush_dirty(&self.app, result.audio_dirty);
        }

        // Handle quit
        if result.quit {
            std::process::exit(0);
        }
    }

    /// Poll audio feedback and I/O feedback.
    pub fn poll_audio_feedback(&mut self) {
        // Drain audio feedback
        let feedback = self.audio.drain_feedback();
        for fb in feedback {
            // Convert AudioFeedback to Action and dispatch
            let action = Action::AudioFeedback(fb);
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
