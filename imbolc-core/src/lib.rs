//! # imbolc-core
//!
//! Backend library for the Imbolc DAW. Provides state management, action dispatch,
//! audio engine integration, and persistence — independent of any UI framework.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use imbolc_core::state::AppState;
//! use imbolc_core::config::Config;
//! use imbolc_core::audio::AudioHandle;
//! use imbolc_core::dispatch::dispatch_action;
//! use imbolc_core::action::{Action, IoFeedback};
//!
//! // 1. Create state with musical defaults from config
//! let config = Config::load();
//! let mut state = AppState::new_with_defaults(config.defaults());
//!
//! // 2. Create audio engine handle
//! let mut audio = AudioHandle::new();
//! let (io_tx, io_rx) = std::sync::mpsc::channel::<IoFeedback>();
//!
//! // 3. Dispatch actions to mutate state (undo snapshots are pushed automatically)
//! let mut effects = Vec::new();
//! let result = dispatch_action(&action, &mut state, &audio, &mut effects, &io_tx);
//!
//! // 4. Process DispatchResult: audio_dirty flags, nav intents, status events
//! // ForwardAction handles incremental projection; apply_dirty handles fallback sync
//! // audio.forward_action(&action, result.audio_dirty);
//! // audio.apply_dirty(&state, result.audio_dirty, needs_full_sync);
//!
//! // 5. Drain IoFeedback from io_rx for async save/load completions
//! // 6. Drain AudioFeedback via audio.drain_feedback() for render/export progress
//! ```
//!
//! ## Module Overview
//!
//! - [`state`] — All application state: `AppState`, instruments, session, piano roll,
//!   automation, clipboard, undo history, persistence (SQLite save/load)
//! - [`action`] — Action enums (`Action`, `PianoRollAction`, `SequencerAction`, etc.),
//!   `DispatchResult`, `AudioDirty` flags, `NavIntent`, `IoFeedback`
//! - [`dispatch`] — `dispatch_action()` — the single entry point for state mutation.
//!   Automatically manages undo snapshots for undoable actions.
//! - [`audio`] — `AudioHandle` (main-thread interface) and audio thread communication
//!   via `AudioCmd`/`AudioFeedback` over MPSC channels
//! - [`config`] — TOML configuration loading (musical defaults, embedded + user override)
//! - [`midi`] — MIDI utilities
//! - [`scd_parser`] — SuperCollider .scd file parser

pub mod action;
pub mod audio;
pub mod config;
pub mod dispatch;
pub mod midi;
pub mod paths;
pub mod scd_parser;
pub mod state;
pub mod vst3_probe;
