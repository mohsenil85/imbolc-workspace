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
pub mod side_effects;
mod vst_param;

pub use local::LocalDispatcher;
pub use side_effects::AudioSideEffect;

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::audio::AudioHandle;
use crate::state::AppState;
use crate::action::{Action, AudioDirty, ClickAction, DispatchResult, IoFeedback, TunerAction};
use crate::state::undo::{is_undoable, undo_scope};

pub use helpers::{
    adjust_groove_param, adjust_instrument_param, compute_waveform_peaks,
    maybe_record_automation,
};
pub use helpers::{apply_bus_update, apply_layer_group_update};

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
/// Audio write operations are collected into `effects` rather than executed inline.
/// The caller (`dispatch_with_audio`) applies them after dispatch returns.
/// `audio` is passed as shared ref for read-only queries (`is_running()`, `status()`).
///
/// Automatically pushes undo snapshots for undoable actions before mutating state.
pub fn dispatch_action(
    action: &Action,
    state: &mut AppState,
    audio: &AudioHandle,
    effects: &mut Vec<AudioSideEffect>,
    io_tx: &Sender<IoFeedback>,
) -> DispatchResult {
    // Auto-push undo snapshot for undoable actions
    if is_undoable(action) {
        let scope = undo_scope(action, &state.session, &state.instruments);
        state.undo_history.push_scoped(scope, &state.session, &state.instruments);
        state.project.dirty = true;
    }

    let result = match action {
        Action::Quit => DispatchResult::with_quit(),
        Action::Nav(_) => DispatchResult::none(), // Handled by PaneManager
        Action::Instrument(a) => instrument::dispatch_instrument(a, state, audio, effects),
        Action::Mixer(a) => mixer::dispatch_mixer(a, state, audio, effects),
        Action::PianoRoll(a) => piano_roll::dispatch_piano_roll(a, state, audio, effects),
        Action::Arrangement(a) => arrangement::dispatch_arrangement(a, state, audio, effects),
        Action::Server(a) => server::dispatch_server(a, state, audio, effects),
        Action::Session(a) => session::dispatch_session(a, state, audio, effects, io_tx),
        Action::Sequencer(a) => sequencer::dispatch_sequencer(a, state, audio, effects),
        Action::Chopper(a) => sequencer::dispatch_chopper(a, state, audio, effects),
        Action::Automation(a) => automation::dispatch_automation(a, state, audio, effects),
        Action::Midi(a) => midi::dispatch_midi(a, state),
        Action::Bus(a) => bus::dispatch_bus(a, state),
        Action::LayerGroup(a) => bus::dispatch_layer_group(a, state, audio, effects),
        Action::VstParam(a) => vst_param::dispatch_vst_param(a, state, audio, effects),
        Action::Click(a) => dispatch_click(a, state, effects),
        Action::Tuner(a) => dispatch_tuner(a, effects),
        Action::AudioFeedback(f) => audio_feedback::dispatch_audio_feedback(f, state, audio, effects),
        Action::None => DispatchResult::none(),
        // Layer management actions — handled in main.rs before dispatch
        Action::ExitPerformanceMode | Action::PushLayer(_) | Action::PopLayer(_) => DispatchResult::none(),
        // SaveAndQuit is intercepted in main.rs before reaching dispatch
        Action::SaveAndQuit => DispatchResult::none(),
        Action::Undo => {
            if state.undo_history.undo(&mut state.session, &mut state.instruments) {
                state.project.dirty = true;
                let mut r = DispatchResult::none();
                r.audio_dirty = AudioDirty::all();
                r
            } else {
                DispatchResult::none()
            }
        }
        Action::Redo => {
            if state.undo_history.redo(&mut state.session, &mut state.instruments) {
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

/// Dispatch tuner actions.
fn dispatch_tuner(action: &TunerAction, effects: &mut Vec<AudioSideEffect>) -> DispatchResult {
    match action {
        TunerAction::PlayTone(freq) => {
            effects.push(AudioSideEffect::StartTunerTone { freq: *freq });
        }
        TunerAction::StopTone => {
            effects.push(AudioSideEffect::StopTunerTone);
        }
    }
    DispatchResult::none()
}

/// Dispatch click track actions.
fn dispatch_click(action: &ClickAction, state: &mut AppState, effects: &mut Vec<AudioSideEffect>) -> DispatchResult {
    match action {
        ClickAction::Toggle => {
            state.session.click_track.enabled = !state.session.click_track.enabled;
            effects.push(AudioSideEffect::SetClickEnabled { enabled: state.session.click_track.enabled });
        }
        ClickAction::ToggleMute => {
            state.session.click_track.muted = !state.session.click_track.muted;
            effects.push(AudioSideEffect::SetClickMuted { muted: state.session.click_track.muted });
        }
        ClickAction::AdjustVolume(delta) => {
            state.session.click_track.volume = (state.session.click_track.volume + delta).clamp(0.0, 1.0);
            effects.push(AudioSideEffect::SetClickVolume { volume: state.session.click_track.volume });
        }
        ClickAction::SetVolume(volume) => {
            state.session.click_track.volume = volume.clamp(0.0, 1.0);
            effects.push(AudioSideEffect::SetClickVolume { volume: state.session.click_track.volume });
        }
    }
    DispatchResult::none()
}
