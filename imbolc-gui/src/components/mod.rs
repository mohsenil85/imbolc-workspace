//! UI components for the Imbolc GUI.

mod arrangement;
mod detail_view;
mod instrument_editor;
mod mixer;
mod track_list;
mod transport;

pub mod common;

pub use arrangement::Arrangement;
pub use mixer::Mixer;
pub use track_list::TrackList;
pub use transport::Transport;

// These are available but not used in the main app layout yet
#[allow(unused_imports)]
pub use detail_view::DetailView;
#[allow(unused_imports)]
pub use instrument_editor::InstrumentEditor;
