//! LocalDispatcher: Dispatcher implementation for local execution.

use std::sync::mpsc::Sender;

use imbolc_types::{Action, Dispatcher, DispatchResult};

use crate::action::IoFeedback;
use crate::audio::AudioHandle;
use crate::state::AppState;

use super::dispatch_action;

/// Local dispatcher that executes actions directly on in-process state.
///
/// This is the standard dispatcher for standalone operation. It holds references
/// to the application state, audio handle, and I/O feedback channel.
pub struct LocalDispatcher<'a> {
    pub state: &'a mut AppState,
    pub audio: &'a mut AudioHandle,
    pub io_tx: &'a Sender<IoFeedback>,
}

impl<'a> LocalDispatcher<'a> {
    pub fn new(
        state: &'a mut AppState,
        audio: &'a mut AudioHandle,
        io_tx: &'a Sender<IoFeedback>,
    ) -> Self {
        Self { state, audio, io_tx }
    }
}

impl<'a> Dispatcher for LocalDispatcher<'a> {
    fn dispatch(&mut self, action: &Action) -> DispatchResult {
        dispatch_action(action, self.state, self.audio, self.io_tx)
    }
}
