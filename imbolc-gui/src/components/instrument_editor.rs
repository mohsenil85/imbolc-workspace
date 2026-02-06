//! Instrument editor component.

use dioxus::prelude::*;

use crate::components::common::Slider;
use crate::dispatch::{use_dispatch, DispatchExt};
use crate::state::SharedState;

/// Instrument editor showing source, filter, effects, and parameters.
#[component]
pub fn InstrumentEditor() -> Element {
    let state = use_context::<Signal<SharedState>>();
    let mut dispatch = use_dispatch();

    let instrument_data = {
        let s = state.read();
        s.app.instruments.selected_instrument().map(|i| {
            let (filter_type, cutoff, resonance) = match &i.filter {
                Some(f) => (Some(f.filter_type), f.cutoff.value, f.resonance.value),
                None => (None, 1000.0, 0.5),
            };
            (
                i.id,
                i.name.clone(),
                format!("{:?}", i.source),
                i.level,
                i.pan,
                filter_type,
                cutoff,
                resonance,
            )
        })
    };

    match instrument_data {
        Some((_id, name, source_type, level, pan, filter_type, cutoff, resonance)) => {
            rsx! {
                div { class: "instrument-editor",
                    h3 { "{name}" }
                    div { class: "editor-section",
                        h4 { "Source" }
                        div { class: "source-type", "{source_type}" }
                    }
                    div { class: "editor-section",
                        h4 { "Mixer" }
                        div { class: "param-row",
                            label { "Level" }
                            Slider {
                                value: level,
                                min: 0.0,
                                max: 1.0,
                                vertical: false,
                                onchange: move |new_level: f32| {
                                    let delta = new_level - level;
                                    dispatch.dispatch_action(imbolc_types::Action::Mixer(imbolc_types::MixerAction::AdjustLevel(delta)));
                                }
                            }
                            span { "{level:.2}" }
                        }
                        div { class: "param-row",
                            label { "Pan" }
                            Slider {
                                value: (pan + 1.0) / 2.0,
                                min: 0.0,
                                max: 1.0,
                                vertical: false,
                                onchange: move |new_pan_norm: f32| {
                                    let new_pan = new_pan_norm * 2.0 - 1.0;
                                    let delta = new_pan - pan;
                                    dispatch.dispatch_action(imbolc_types::Action::Mixer(imbolc_types::MixerAction::AdjustPan(delta)));
                                }
                            }
                            span { "{pan:.2}" }
                        }
                    }
                    div { class: "editor-section",
                        h4 { "Filter" }
                        div { class: "filter-type", "{filter_type:?}" }
                        div { class: "param-row",
                            label { "Cutoff" }
                            Slider {
                                value: cutoff,
                                min: 20.0,
                                max: 20000.0,
                                vertical: false,
                                onchange: move |_new_cutoff: f32| {
                                    // TODO: Dispatch filter cutoff change
                                }
                            }
                            span { "{cutoff:.0} Hz" }
                        }
                        div { class: "param-row",
                            label { "Resonance" }
                            Slider {
                                value: resonance,
                                min: 0.0,
                                max: 1.0,
                                vertical: false,
                                onchange: move |_new_res: f32| {
                                    // TODO: Dispatch filter resonance change
                                }
                            }
                            span { "{resonance:.2}" }
                        }
                    }
                }
            }
        }
        None => {
            rsx! {
                div { class: "instrument-editor empty",
                    "No instrument selected"
                }
            }
        }
    }
}
