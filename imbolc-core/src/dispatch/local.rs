//! LocalDispatcher: owns state and dispatches domain actions.

use std::sync::mpsc::Sender;

use imbolc_types::{Action, DispatchResult, DomainAction};

use crate::action::IoFeedback;
use imbolc_audio::AudioHandle;
use crate::state::AppState;

use super::dispatch_action;

/// Local dispatcher that executes actions directly on in-process state.
///
/// This is the standard dispatcher for standalone operation. It owns the
/// application state and I/O feedback channel sender. The audio handle is
/// passed separately to avoid borrow conflicts (state and audio often need
/// to be accessed together).
///
/// Unlike a remote dispatcher, LocalDispatcher provides direct access to
/// AppState for rendering.
pub struct LocalDispatcher {
    state: AppState,
    io_tx: Sender<IoFeedback>,
}

impl LocalDispatcher {
    /// Create a new LocalDispatcher that owns the given state and I/O channel.
    pub fn new(state: AppState, io_tx: Sender<IoFeedback>) -> Self {
        Self { state, io_tx }
    }

    /// Access the application state for rendering.
    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Mutable access to application state for IoFeedback handling and other updates.
    pub fn state_mut(&mut self) -> &mut AppState {
        &mut self.state
    }

    /// Access the I/O feedback sender.
    pub fn io_tx(&self) -> &Sender<IoFeedback> {
        &self.io_tx
    }

    /// Dispatch an action using the provided audio handle.
    ///
    /// Extracts the `DomainAction` from the `Action` and forwards it to
    /// `dispatch_action()`. UI-only actions (Nav, Quit, etc.) return a
    /// no-op result.
    ///
    /// After dispatch, forwards the action to the audio thread for incremental
    /// state projection. The audio thread applies the action's state mutations
    /// to its local copies, avoiding full-state clones.
    pub fn dispatch_with_audio(&mut self, action: &Action, audio: &mut AudioHandle) -> DispatchResult {
        let domain = match action.to_domain() {
            Some(d) => d,
            None => return DispatchResult::none(),
        };
        self.dispatch_domain(&domain, audio)
    }

    /// Dispatch a domain action directly, bypassing the Action wrapper.
    pub fn dispatch_domain(&mut self, action: &DomainAction, audio: &mut AudioHandle) -> DispatchResult {
        let mut result = dispatch_action(action, &mut self.state, audio, &self.io_tx);
        let reducible = imbolc_types::reduce::is_reducible(action);
        audio.forward_action(action, &result.audio_effects);
        result.needs_full_sync = !reducible && !result.audio_effects.is_empty();
        result
    }
}
