//! Dispatch helpers for components.

use dioxus::prelude::*;
use imbolc_types::Action;

use crate::state::SharedState;

/// Hook to get the shared state signal for dispatching actions.
/// Components should use this to get access to the state and dispatch actions.
pub fn use_dispatch() -> Signal<SharedState> {
    use_context::<Signal<SharedState>>()
}

/// Extension trait for dispatching actions on the SharedState signal.
pub trait DispatchExt {
    fn dispatch_action(&mut self, action: Action);
}

impl DispatchExt for Signal<SharedState> {
    fn dispatch_action(&mut self, action: Action) {
        self.write().dispatch(action);
    }
}
