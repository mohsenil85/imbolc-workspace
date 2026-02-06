use crate::state::automation::AutomationLane;
use crate::state::piano_roll::PianoRollState;
use crate::state::{InstrumentState, SessionState};

pub type InstrumentSnapshot = InstrumentState;
pub type SessionSnapshot = SessionState;
pub type PianoRollSnapshot = PianoRollState;
pub type AutomationSnapshot = Vec<AutomationLane>;
