pub mod arp_state;
pub mod audio_thread;
pub mod bus_allocator;
pub mod click_tick;
pub mod commands;
pub mod devices;
pub mod engine;
pub mod event_log;
pub mod handle;
pub mod input;
pub mod osc_client;
pub mod osc_sender;
pub mod paths;
pub mod playback;
pub mod drum_tick;
pub mod arpeggiator_tick;
pub mod snapshot;
pub mod telemetry;
pub mod triple_buffer;

pub use engine::{AudioEngine, ServerStatus};
pub use handle::{AudioHandle, AudioReadState};
pub use input::AudioInputManager;
pub use osc_client::AudioMonitor;

use imbolc_types::{InstrumentState, SessionState};

/// Trait for types that provide the session and instrument state needed by the
/// audio subsystem. Implemented by `AppState` in imbolc-core to break the
/// circular dependency.
pub trait AudioStateProvider {
    fn session(&self) -> &SessionState;
    fn instruments(&self) -> &InstrumentState;
}
