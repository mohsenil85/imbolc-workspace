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

use imbolc_audio::AudioHandle;
use crate::state::AppState;
use crate::action::{AudioEffect, ClickAction, DispatchResult, DomainAction, IoFeedback, TunerAction};
use crate::state::undo::{coalesce_key, is_undoable, undo_scope, UndoScope};

pub use helpers::{compute_waveform_peaks, maybe_record_automation};
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

/// Dispatch a domain action. Returns a DispatchResult describing side effects for the UI layer.
///
/// Only handles domain actions (state mutations). UI-layer actions (Nav, PushLayer,
/// PopLayer, ExitPerformanceMode, Quit, SaveAndQuit) are handled by the main loop.
///
/// Audio operations are executed directly via `AudioHandle` methods.
///
/// Automatically pushes undo snapshots for undoable actions before mutating state.
pub fn dispatch_action(
    action: &DomainAction,
    state: &mut AppState,
    audio: &mut AudioHandle,
    io_tx: &Sender<IoFeedback>,
) -> DispatchResult {
    // Auto-push undo snapshot for undoable actions (with coalescing for param sweeps)
    if is_undoable(action) {
        let automation_recording = state.recording.automation_recording && state.audio.playing;
        let scope = undo_scope(action, &state.session, &state.instruments, automation_recording);
        let key = coalesce_key(action, &state.session, &state.instruments);
        state.undo_history.push_coalesced(scope, &state.session, &state.instruments, key);
        state.project.dirty = true;
    }

    match action {
        DomainAction::Instrument(a) => instrument::dispatch_instrument(a, state, audio),
        DomainAction::Mixer(a) => mixer::dispatch_mixer(a, state, audio),
        DomainAction::PianoRoll(a) => piano_roll::dispatch_piano_roll(a, state, audio),
        DomainAction::Arrangement(a) => arrangement::dispatch_arrangement(a, state, audio),
        DomainAction::Server(a) => server::dispatch_server(a, state, audio),
        DomainAction::Session(a) => session::dispatch_session(a, state, audio, io_tx),
        DomainAction::Sequencer(a) => sequencer::dispatch_sequencer(a, state, audio),
        DomainAction::Chopper(a) => sequencer::dispatch_chopper(a, state, audio),
        DomainAction::Automation(a) => automation::dispatch_automation(a, state, audio),
        DomainAction::Midi(a) => midi::dispatch_midi(a, state),
        DomainAction::Bus(a) => bus::dispatch_bus(a, state),
        DomainAction::LayerGroup(a) => bus::dispatch_layer_group(a, state, audio),
        DomainAction::VstParam(a) => vst_param::dispatch_vst_param(a, state, audio),
        DomainAction::Click(a) => dispatch_click(a, state, audio),
        DomainAction::Tuner(a) => dispatch_tuner(a, audio),
        DomainAction::AudioFeedback(f) => audio_feedback::dispatch_audio_feedback(f, state, audio),
        DomainAction::Undo => {
            if let Some(scope) = state.undo_history.undo(&mut state.session, &mut state.instruments) {
                state.project.dirty = true;
                let mut r = DispatchResult::none();
                r.audio_effects = audio_effects_for_undo_scope(scope);
                r
            } else {
                DispatchResult::none()
            }
        }
        DomainAction::Redo => {
            if let Some(scope) = state.undo_history.redo(&mut state.session, &mut state.instruments) {
                state.project.dirty = true;
                let mut r = DispatchResult::none();
                r.audio_effects = audio_effects_for_undo_scope(scope);
                r
            } else {
                DispatchResult::none()
            }
        }
    }
}

/// Map an undo scope to the minimal audio effects needed.
fn audio_effects_for_undo_scope(scope: UndoScope) -> Vec<AudioEffect> {
    match scope {
        UndoScope::SingleInstrument(id) => AudioEffect::for_instrument(id),
        _ => AudioEffect::all(),
    }
}

/// Dispatch tuner actions.
fn dispatch_tuner(action: &TunerAction, audio: &mut AudioHandle) -> DispatchResult {
    match action {
        TunerAction::PlayTone(freq) => {
            audio.start_tuner_tone(*freq);
        }
        TunerAction::StopTone => {
            audio.stop_tuner_tone();
        }
    }
    DispatchResult::none()
}

/// Dispatch click track actions.
fn dispatch_click(action: &ClickAction, state: &mut AppState, audio: &mut AudioHandle) -> DispatchResult {
    // Delegate pure state mutation to the shared reducer
    imbolc_types::reduce::reduce_action(
        &DomainAction::Click(action.clone()),
        &mut state.instruments,
        &mut state.session,
    );

    // Side effects: forward updated state to audio engine
    match action {
        ClickAction::Toggle => {
            let _ = audio.set_click_enabled(state.session.click_track.enabled);
        }
        ClickAction::ToggleMute => {
            let _ = audio.set_click_muted(state.session.click_track.muted);
        }
        ClickAction::AdjustVolume(_) | ClickAction::SetVolume(_) => {
            let _ = audio.set_click_volume(state.session.click_track.volume);
        }
    }
    DispatchResult::none()
}
