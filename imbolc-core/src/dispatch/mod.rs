mod automation;
mod arrangement;
mod audio_feedback;
mod bus;
mod helpers;
mod instrument;
mod local;
mod midi;
mod mixer;
mod piano_roll;
mod sequencer;
mod server;
mod session;
mod vst_param;

pub use local::LocalDispatcher;

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::audio::AudioHandle;
use crate::state::AppState;
use crate::action::{Action, AudioDirty, ClickAction, DispatchResult, IoFeedback};
use crate::state::undo::is_undoable;

pub use helpers::{
    adjust_groove_param, adjust_instrument_param, apply_bus_update, compute_waveform_peaks,
    maybe_record_automation,
};

/// Default path for save file
pub fn default_rack_path() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home)
            .join(".config")
            .join("imbolc")
            .join("default.sqlite")
    } else {
        PathBuf::from("default.sqlite")
    }
}

/// Generate a timestamped path for a recording file in the current directory
fn recording_path(prefix: &str) -> PathBuf {
    let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    dir.join(format!("{}_{}.wav", prefix, secs))
}

/// Dispatch an action. Returns a DispatchResult describing side effects for the UI layer.
/// Dispatch no longer takes panes or app_frame — it operates purely on state and audio engine.
///
/// Automatically pushes undo snapshots for undoable actions before mutating state.
pub fn dispatch_action(
    action: &Action,
    state: &mut AppState,
    audio: &mut AudioHandle,
    io_tx: &Sender<IoFeedback>,
) -> DispatchResult {
    // Auto-push undo snapshot for undoable actions
    if is_undoable(action) {
        let s = state.session.clone();
        let i = state.instruments.clone();
        state.undo_history.push_from(s, i);
        state.project.dirty = true;
    }

    let result = match action {
        Action::Quit => DispatchResult::with_quit(),
        Action::Nav(_) => DispatchResult::none(), // Handled by PaneManager
        Action::Instrument(a) => instrument::dispatch_instrument(a, state, audio),
        Action::Mixer(a) => mixer::dispatch_mixer(a, state, audio),
        Action::PianoRoll(a) => piano_roll::dispatch_piano_roll(a, state, audio),
        Action::Arrangement(a) => arrangement::dispatch_arrangement(a, state, audio),
        Action::Server(a) => server::dispatch_server(a, state, audio),
        Action::Session(a) => session::dispatch_session(a, state, audio, io_tx),
        Action::Sequencer(a) => sequencer::dispatch_sequencer(a, state, audio),
        Action::Chopper(a) => sequencer::dispatch_chopper(a, state, audio),
        Action::Automation(a) => automation::dispatch_automation(a, state, audio),
        Action::Midi(a) => midi::dispatch_midi(a, state),
        Action::Bus(a) => bus::dispatch_bus(a, state, audio),
        Action::VstParam(a) => vst_param::dispatch_vst_param(a, state, audio),
        Action::Click(a) => dispatch_click(a, state, audio),
        Action::AudioFeedback(f) => audio_feedback::dispatch_audio_feedback(f, state, audio),
        Action::None => DispatchResult::none(),
        // Layer management actions — handled in main.rs before dispatch
        Action::ExitPerformanceMode | Action::PushLayer(_) | Action::PopLayer(_) => DispatchResult::none(),
        // SaveAndQuit is intercepted in main.rs before reaching dispatch
        Action::SaveAndQuit => DispatchResult::none(),
        Action::Undo => {
            if let Some(snapshot) = state.undo_history.undo(&state.session, &state.instruments) {
                state.session = snapshot.session;
                state.instruments = snapshot.instruments;
                state.project.dirty = true;
                let mut r = DispatchResult::none();
                r.audio_dirty = AudioDirty::all();
                r
            } else {
                DispatchResult::none()
            }
        }
        Action::Redo => {
            if let Some(snapshot) = state.undo_history.redo(&state.session, &state.instruments) {
                state.session = snapshot.session;
                state.instruments = snapshot.instruments;
                state.project.dirty = true;
                let mut r = DispatchResult::none();
                r.audio_dirty = AudioDirty::all();
                r
            } else {
                DispatchResult::none()
            }
        }
    };

    result
}

/// Dispatch click track actions.
fn dispatch_click(action: &ClickAction, state: &mut AppState, audio: &mut AudioHandle) -> DispatchResult {
    match action {
        ClickAction::Toggle => {
            state.session.click_track.enabled = !state.session.click_track.enabled;
            let _ = audio.set_click_enabled(state.session.click_track.enabled);
        }
        ClickAction::ToggleMute => {
            state.session.click_track.muted = !state.session.click_track.muted;
            let _ = audio.set_click_muted(state.session.click_track.muted);
        }
        ClickAction::AdjustVolume(delta) => {
            state.session.click_track.volume = (state.session.click_track.volume + delta).clamp(0.0, 1.0);
            let _ = audio.set_click_volume(state.session.click_track.volume);
        }
        ClickAction::SetVolume(volume) => {
            state.session.click_track.volume = volume.clamp(0.0, 1.0);
            let _ = audio.set_click_volume(state.session.click_track.volume);
        }
    }
    DispatchResult::none()
}
