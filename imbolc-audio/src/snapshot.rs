use imbolc_types::AutomationLane;
use imbolc_types::PianoRollState;
use imbolc_types::{InstrumentState, SessionState};

pub type InstrumentSnapshot = InstrumentState;
pub type SessionSnapshot = SessionState;
pub type PianoRollSnapshot = PianoRollState;
pub type AutomationSnapshot = Vec<AutomationLane>;
