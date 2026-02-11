mod lane;
mod state;
mod target;
#[cfg(test)]
mod tests;
mod types;

pub use lane::AutomationLane;
pub use state::AutomationState;
pub use target::{AutomationTarget, AutomationTargetExt};
pub use types::{AutomationLaneId, AutomationPoint, CurveType};
