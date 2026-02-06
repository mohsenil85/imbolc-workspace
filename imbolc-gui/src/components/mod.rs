//! UI components for the Imbolc GUI.

mod arrangement;
mod detail_view;
mod effect_slot;
mod instrument_editor;
mod mixer;
mod piano_roll_view;
mod track_list;
mod transport;
mod waveform_view;

pub mod common;

pub use arrangement::Arrangement;
pub use detail_view::DetailView;
pub use effect_slot::{AddEffectButton, EffectSlotComponent};
pub use instrument_editor::InstrumentEditor;
pub use mixer::Mixer;
pub use piano_roll_view::PianoRollView;
pub use track_list::TrackList;
pub use transport::Transport;
pub use waveform_view::WaveformView;
