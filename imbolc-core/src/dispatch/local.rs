//! LocalDispatcher: Dispatcher implementation for local execution.

use std::sync::mpsc::Sender;

use imbolc_types::{Action, Dispatcher, DispatchResult};

use crate::action::IoFeedback;
use crate::audio::AudioHandle;
use crate::state::AppState;

use super::dispatch_action;
use super::side_effects::{AudioSideEffect, apply_side_effects};

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
    /// This method allows passing audio separately to avoid borrow conflicts.
    ///
    /// After dispatch, forwards the action to the audio thread for incremental
    /// state projection (Phase 2). The audio thread applies the action's state
    /// mutations to its local copies, avoiding full-state clones.
    pub fn dispatch_with_audio(&mut self, action: &Action, audio: &mut AudioHandle) -> DispatchResult {
        let mut effects: Vec<AudioSideEffect> = Vec::new();
        let result = dispatch_action(action, &mut self.state, &*audio, &mut effects, &self.io_tx);
        apply_side_effects(&effects, audio);
        // Forward action to audio thread for incremental state projection
        audio.forward_action(action, &self.state, result.audio_dirty);
        result
    }
}

impl Dispatcher for LocalDispatcher {
    fn dispatch(&mut self, _action: &Action) -> DispatchResult {
        // This method can't work without audio - use dispatch_with_audio instead.
        // This is a temporary limitation until we have a better abstraction.
        panic!("LocalDispatcher::dispatch() requires audio handle - use dispatch_with_audio() instead")
    }
}
