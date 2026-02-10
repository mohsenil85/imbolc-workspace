//! Effect slot component for displaying and editing a single effect in the chain.

use dioxus::prelude::*;

use crate::components::common::Slider;
use crate::dispatch::{use_dispatch, DispatchExt};
use imbolc_types::{Action, EffectId, EffectType, InstrumentAction, InstrumentId};

/// A single effect slot in the effects chain.
#[component]
pub fn EffectSlotComponent(
    instrument_id: InstrumentId,
    effect_id: EffectId,
    chain_index: usize,
    effect_name: String,
    enabled: bool,
    params: Vec<(String, f32, f32, f32)>, // (name, value, min, max)
    #[props(default = false)] can_move_up: bool,
    #[props(default = false)] can_move_down: bool,
) -> Element {
    let mut dispatch = use_dispatch();

    rsx! {
        div { class: if enabled { "effect-slot" } else { "effect-slot bypassed" },
            // Header row
            div { class: "effect-slot-header",
                span { class: "effect-name", "{effect_name}" }

                div { class: "effect-controls",
                    // Move up button
                    button {
                        class: "effect-btn",
                        disabled: !can_move_up,
                        onclick: move |_| {
                            dispatch.dispatch_action(Action::Instrument(
                                InstrumentAction::MoveStage(instrument_id, chain_index, -1)
                            ));
                        },
                        "^"
                    }
                    // Move down button
                    button {
                        class: "effect-btn",
                        disabled: !can_move_down,
                        onclick: move |_| {
                            dispatch.dispatch_action(Action::Instrument(
                                InstrumentAction::MoveStage(instrument_id, chain_index, 1)
                            ));
                        },
                        "v"
                    }
                    // Bypass toggle
                    button {
                        class: if enabled { "effect-btn bypass" } else { "effect-btn bypass active" },
                        onclick: move |_| {
                            dispatch.dispatch_action(Action::Instrument(
                                InstrumentAction::ToggleEffectBypass(instrument_id, effect_id)
                            ));
                        },
                        "B"
                    }
                    // Remove button
                    button {
                        class: "effect-btn remove",
                        onclick: move |_| {
                            dispatch.dispatch_action(Action::Instrument(
                                InstrumentAction::RemoveEffect(instrument_id, effect_id)
                            ));
                        },
                        "X"
                    }
                }
            }

            // Parameters
            if enabled {
                div { class: "effect-params",
                    for (idx, (param_name, param_value, param_min, param_max)) in params.iter().enumerate() {
                        {
                            let pname = param_name.clone();
                            let pvalue = *param_value;
                            let pmin = *param_min;
                            let pmax = *param_max;
                            let param_idx = imbolc_types::ParamIndex::new(idx);

                            rsx! {
                                div {
                                    key: "{pname}",
                                    class: "effect-param-row",
                                    label { class: "param-name", "{pname}" }
                                    Slider {
                                        value: pvalue,
                                        min: pmin,
                                        max: pmax,
                                        vertical: false,
                                        onchange: move |new_val: f32| {
                                            let delta = new_val - pvalue;
                                            dispatch.dispatch_action(Action::Instrument(
                                                InstrumentAction::AdjustEffectParam(
                                                    instrument_id,
                                                    effect_id,
                                                    param_idx,
                                                    delta
                                                )
                                            ));
                                        }
                                    }
                                    span { class: "param-value", "{pvalue:.2}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Add effect dropdown and button.
#[component]
pub fn AddEffectButton(
    instrument_id: InstrumentId,
) -> Element {
    let mut dispatch = use_dispatch();
    let mut selected_type = use_signal(|| EffectType::Delay);

    // Get available effect types
    let effect_types = EffectType::all();

    rsx! {
        div { class: "add-effect-row",
            select {
                class: "effect-type-select",
                onchange: move |evt| {
                    let value = evt.value();
                    // Find matching effect type
                    for et in EffectType::all() {
                        if et.name() == value {
                            selected_type.set(et);
                            break;
                        }
                    }
                },
                for effect_type in effect_types.iter() {
                    option {
                        key: "{effect_type.name()}",
                        value: "{effect_type.name()}",
                        "{effect_type.name()}"
                    }
                }
            }
            button {
                class: "add-effect-btn",
                onclick: move |_| {
                    let et = *selected_type.read();
                    dispatch.dispatch_action(Action::Instrument(
                        InstrumentAction::AddEffect(instrument_id, et)
                    ));
                },
                "+ Add Effect"
            }
        }
    }
}
