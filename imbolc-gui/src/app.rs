//! Root application component and state initialization.

use dioxus::prelude::*;

use crate::components::{Arrangement, DetailView, InstrumentEditor, Mixer, TrackList, Transport};
use crate::file_ops;
use crate::keybindings::{GuiAction, Keybindings};
use crate::state::SharedState;
use imbolc_types::{Action, ArrangementAction, MixerAction, SessionAction};

const MAIN_CSS: &str = include_str!("styles/main.css");

/// Root application component.
#[component]
pub fn App() -> Element {
    // Initialize shared state with context provider
    let mut shared_state = use_context_provider(|| Signal::new(SharedState::new()));

    // Create keybindings
    let keybindings = use_signal(|| Keybindings::new());

    // Track which panel is focused
    let mut focused_panel = use_signal(|| "arrangement");

    // Track if we're showing the detail view
    let mut show_detail = use_signal(|| false);

    // Poll audio feedback at ~30fps using Dioxus spawn
    use_future(move || async move {
        loop {
            {
                let mut state = shared_state.write();
                state.poll_audio_feedback();
            }
            // Use async-std for sleeping
            async_std::task::sleep(std::time::Duration::from_millis(33)).await;
        }
    });

    // Keyboard handler
    let handle_keydown = move |evt: KeyboardEvent| {
        let kb = keybindings.read();
        if let Some(action) = kb.lookup(&evt.data()) {
            match action {
                GuiAction::TogglePlay => {
                    shared_state.write().dispatch(Action::Arrangement(ArrangementAction::PlayStop));
                }
                GuiAction::Stop => {
                    // Stop and reset playhead
                    let mut state = shared_state.write();
                    if state.app.session.piano_roll.playing {
                        state.dispatch(Action::Arrangement(ArrangementAction::PlayStop));
                    }
                }
                GuiAction::Record => {
                    log::info!("Record toggled");
                }
                GuiAction::FocusTrackList => {
                    focused_panel.set("track_list");
                }
                GuiAction::FocusMixer => {
                    focused_panel.set("mixer");
                }
                GuiAction::FocusArrangement => {
                    focused_panel.set("arrangement");
                    show_detail.set(false);
                }
                GuiAction::FocusDetail => {
                    show_detail.set(true);
                }
                GuiAction::NewProject => {
                    log::info!("New project requested");
                    shared_state.write().dispatch(Action::Session(SessionAction::NewProject));
                }
                GuiAction::OpenProject => {
                    spawn(async move {
                        if let Some(path) = file_ops::open_project_dialog().await {
                            log::info!("Opening project: {:?}", path);
                            shared_state.write().dispatch(Action::Session(SessionAction::LoadFrom(path)));
                        }
                    });
                }
                GuiAction::SaveProject => {
                    shared_state.write().dispatch(Action::Session(SessionAction::Save));
                }
                GuiAction::SaveProjectAs => {
                    spawn(async move {
                        if let Some(path) = file_ops::save_project_dialog().await {
                            log::info!("Saving project as: {:?}", path);
                            shared_state.write().dispatch(Action::Session(SessionAction::SaveAs(path)));
                        }
                    });
                }
                GuiAction::Undo => {
                    shared_state.write().dispatch(Action::Undo);
                }
                GuiAction::Redo => {
                    shared_state.write().dispatch(Action::Redo);
                }
                GuiAction::Delete => {
                    log::info!("Delete requested");
                }
                GuiAction::ToggleMute => {
                    shared_state.write().dispatch(Action::Mixer(MixerAction::ToggleMute));
                }
                GuiAction::ToggleSolo => {
                    shared_state.write().dispatch(Action::Mixer(MixerAction::ToggleSolo));
                }
            }
        }
    };

    let show_detail_view = *show_detail.read();

    rsx! {
        style { {MAIN_CSS} }
        div {
            class: "app",
            tabindex: "0",
            onkeydown: handle_keydown,

            Transport {}
            div { class: "main-content",
                TrackList {}
                div { class: "center-panel",
                    Arrangement {}
                    if show_detail_view {
                        DetailView {}
                    }
                }
                InstrumentEditor {}
            }
            Mixer {}
        }
    }
}
