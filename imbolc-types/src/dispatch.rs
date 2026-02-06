//! Dispatch abstraction for remote and local execution.

use crate::{Action, DispatchResult};

/// Trait for dispatching actions to the state engine.
///
/// Implementations can be local (direct state mutation) or remote (network-based).
/// The binary uses this trait to abstract over the dispatch mechanism.
pub trait Dispatcher {
    /// Dispatch an action and return the result.
    fn dispatch(&mut self, action: &Action) -> DispatchResult;
}
