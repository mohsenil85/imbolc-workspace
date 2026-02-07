//! Instrument editor component.

use dioxus::prelude::*;

use crate::components::common::Slider;
use crate::components::effect_slot::{AddEffectButton, EffectSlotComponent};
use crate::dispatch::{use_dispatch, DispatchExt};
use crate::state::SharedState;
use imbolc_types::{
    Action, FilterType, InstrumentAction, LfoShape, ParameterTarget, MixerAction, SourceType,
};

/// Instrument editor showing source, filter, effects, LFO, envelope, and parameters.
#[component]
pub fn InstrumentEditor() -> Element {
    let state = use_context::<Signal<SharedState>>();
    let mut dispatch = use_dispatch();

    let instrument_data = {
        let s = state.read();
        s.app.instruments.selected_instrument().map(|i| {
            // Gather filter info
            let (filter_enabled, filter_type, cutoff, resonance) = match &i.filter {
                Some(f) => (true, Some(f.filter_type), f.cutoff.value, f.resonance.value),
                None => (false, None, 1000.0, 0.5),
            };

            // Gather effects info
            let effects_info: Vec<(u32, String, bool, Vec<(String, f32, f32, f32)>)> = i
                .effects
                .iter()
                .map(|e| {
                    let params: Vec<(String, f32, f32, f32)> = e
                        .params
                        .iter()
                        .map(|p| (p.name.clone(), p.value.to_f32(), p.min, p.max))
                        .collect();
                    (e.id, e.effect_type.name().to_string(), e.enabled, params)
                })
                .collect();

            // LFO info
            let lfo = &i.lfo;
            let lfo_info = (lfo.enabled, lfo.rate, lfo.depth, lfo.shape, lfo.target);

            // Envelope info
            let env = &i.amp_envelope;
            let env_info = (env.attack, env.decay, env.sustain, env.release);

            // Source params info
            let source_params: Vec<(String, f32, f32, f32)> = i
                .source_params
                .iter()
                .map(|p| (p.name.clone(), p.value.to_f32(), p.min, p.max))
                .collect();

            (
                i.id,
                i.name.clone(),
                i.source,
                source_params,
                i.level,
                i.pan,
                filter_enabled,
                filter_type,
                cutoff,
                resonance,
                effects_info,
                lfo_info,
                env_info,
                i.polyphonic,
                i.active,
            )
        })
    };

    match instrument_data {
        Some((
            id,
            name,
            source,
            source_params,
            level,
            pan,
            filter_enabled,
            filter_type,
            cutoff,
            resonance,
            effects_info,
            (lfo_enabled, lfo_rate, lfo_depth, lfo_shape, lfo_target),
            (env_attack, env_decay, env_sustain, env_release),
            _polyphonic,
            _active,
        )) => {
            let source_name = source.name();
            let effects_count = effects_info.len();

            rsx! {
                div { class: "instrument-editor",
                    // Header
                    div { class: "editor-header",
                        h3 { "{name}" }
                        div { class: "source-selector",
                            label { "Source: " }
                            select {
                                class: "source-select",
                                value: "{source_name}",
                                onchange: move |evt| {
                                    let value = evt.value();
                                    for st in SourceType::all() {
                                        if st.name() == value {
                                            // For now, just log - full source change would need InstrumentUpdate
                                            log::info!("Source change to {:?} requested", st);
                                            break;
                                        }
                                    }
                                },
                                for st in SourceType::all() {
                                    option {
                                        key: "{st.name()}",
                                        value: "{st.name()}",
                                        selected: st.name() == source_name,
                                        "{st.name()}"
                                    }
                                }
                            }
                        }
                    }

                    // Source Parameters
                    if !source_params.is_empty() {
                        div { class: "editor-section",
                            h4 { "Source Parameters" }
                            for (pname, pvalue, pmin, pmax) in source_params.iter() {
                                div {
                                    key: "{pname}",
                                    class: "param-row",
                                    label { "{pname}" }
                                    Slider {
                                        value: *pvalue,
                                        min: *pmin,
                                        max: *pmax,
                                        vertical: false,
                                        onchange: move |_new_val: f32| {
                                            // Source param changes would need InstrumentUpdate action
                                        }
                                    }
                                    span { "{pvalue:.2}" }
                                }
                            }
                        }
                    }

                    // Mixer Section
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
                                    dispatch.dispatch_action(Action::Mixer(MixerAction::AdjustLevel(delta)));
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
                                    dispatch.dispatch_action(Action::Mixer(MixerAction::AdjustPan(delta)));
                                }
                            }
                            span { "{pan:.2}" }
                        }
                    }

                    // Filter Section
                    div { class: "editor-section",
                        div { class: "section-header",
                            h4 { "Filter" }
                            button {
                                class: if filter_enabled { "toggle-btn active" } else { "toggle-btn" },
                                onclick: move |_| {
                                    dispatch.dispatch_action(Action::Instrument(
                                        InstrumentAction::ToggleFilter(id)
                                    ));
                                },
                                if filter_enabled { "On" } else { "Off" }
                            }
                        }
                        if filter_enabled {
                            div { class: "filter-controls",
                                div { class: "param-row",
                                    label { "Type" }
                                    select {
                                        class: "filter-type-select",
                                        onchange: move |evt| {
                                            let value = evt.value();
                                            for ft in FilterType::all() {
                                                if ft.name() == value {
                                                    dispatch.dispatch_action(Action::Instrument(
                                                        InstrumentAction::SetFilter(id, Some(ft))
                                                    ));
                                                    break;
                                                }
                                            }
                                        },
                                        for ft in FilterType::all() {
                                            option {
                                                key: "{ft.name()}",
                                                value: "{ft.name()}",
                                                selected: filter_type == Some(ft),
                                                "{ft.name()}"
                                            }
                                        }
                                    }
                                }
                                div { class: "param-row",
                                    label { "Cutoff" }
                                    Slider {
                                        value: cutoff,
                                        min: 20.0,
                                        max: 20000.0,
                                        vertical: false,
                                        onchange: move |new_cutoff: f32| {
                                            let delta = new_cutoff - cutoff;
                                            dispatch.dispatch_action(Action::Instrument(
                                                InstrumentAction::AdjustFilterCutoff(id, delta)
                                            ));
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
                                        onchange: move |new_res: f32| {
                                            let delta = new_res - resonance;
                                            dispatch.dispatch_action(Action::Instrument(
                                                InstrumentAction::AdjustFilterResonance(id, delta)
                                            ));
                                        }
                                    }
                                    span { "{resonance:.2}" }
                                }
                            }
                        }
                    }

                    // Effects Section
                    div { class: "editor-section",
                        h4 { "Effects ({effects_count})" }
                        div { class: "effects-chain",
                            for (idx, (effect_id, effect_name, enabled, params)) in effects_info.iter().enumerate() {
                                EffectSlotComponent {
                                    key: "{effect_id}",
                                    instrument_id: id,
                                    effect_id: *effect_id,
                                    effect_name: effect_name.clone(),
                                    enabled: *enabled,
                                    params: params.clone(),
                                    can_move_up: idx > 0,
                                    can_move_down: idx < effects_count - 1,
                                }
                            }
                        }
                        AddEffectButton { instrument_id: id }
                    }

                    // LFO Section
                    div { class: "editor-section",
                        div { class: "section-header",
                            h4 { "LFO" }
                            button {
                                class: if lfo_enabled { "toggle-btn active" } else { "toggle-btn" },
                                onclick: move |_| {
                                    dispatch.dispatch_action(Action::Instrument(
                                        InstrumentAction::ToggleLfo(id)
                                    ));
                                },
                                if lfo_enabled { "On" } else { "Off" }
                            }
                        }
                        if lfo_enabled {
                            div { class: "lfo-controls",
                                div { class: "param-row",
                                    label { "Rate" }
                                    Slider {
                                        value: lfo_rate,
                                        min: 0.1,
                                        max: 20.0,
                                        vertical: false,
                                        onchange: move |new_rate: f32| {
                                            let delta = new_rate - lfo_rate;
                                            dispatch.dispatch_action(Action::Instrument(
                                                InstrumentAction::AdjustLfoRate(id, delta)
                                            ));
                                        }
                                    }
                                    span { "{lfo_rate:.2} Hz" }
                                }
                                div { class: "param-row",
                                    label { "Depth" }
                                    Slider {
                                        value: lfo_depth,
                                        min: 0.0,
                                        max: 1.0,
                                        vertical: false,
                                        onchange: move |new_depth: f32| {
                                            let delta = new_depth - lfo_depth;
                                            dispatch.dispatch_action(Action::Instrument(
                                                InstrumentAction::AdjustLfoDepth(id, delta)
                                            ));
                                        }
                                    }
                                    span { "{lfo_depth:.2}" }
                                }
                                div { class: "param-row",
                                    label { "Shape" }
                                    select {
                                        class: "lfo-shape-select",
                                        onchange: move |evt| {
                                            if let Some(shape) = LfoShape::from_name(&evt.value()) {
                                                dispatch.dispatch_action(Action::Instrument(
                                                    InstrumentAction::SetLfoShape(id, shape)
                                                ));
                                            }
                                        },
                                        for shape in LfoShape::all() {
                                            option {
                                                key: "{shape.name()}",
                                                value: "{shape.name()}",
                                                selected: shape == lfo_shape,
                                                "{shape.name()}"
                                            }
                                        }
                                    }
                                }
                                div { class: "param-row",
                                    label { "Target" }
                                    select {
                                        class: "lfo-target-select",
                                        onchange: move |evt| {
                                            if let Some(target) = ParameterTarget::from_short_name(&evt.value()) {
                                                dispatch.dispatch_action(Action::Instrument(
                                                    InstrumentAction::SetLfoTarget(id, target)
                                                ));
                                            }
                                        },
                                        for target in [ParameterTarget::FilterCutoff, ParameterTarget::FilterResonance, ParameterTarget::Level, ParameterTarget::Pitch, ParameterTarget::Pan] {
                                            option {
                                                key: "{target.short_name()}",
                                                value: "{target.short_name()}",
                                                selected: target == lfo_target,
                                                "{target.name()}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Envelope Section
                    if !source.is_vst() {
                        div { class: "editor-section",
                            h4 { "Envelope" }
                            div { class: "envelope-controls",
                                div { class: "param-row",
                                    label { "Attack" }
                                    Slider {
                                        value: env_attack,
                                        min: 0.001,
                                        max: 2.0,
                                        vertical: false,
                                        onchange: move |new_val: f32| {
                                            let delta = new_val - env_attack;
                                            dispatch.dispatch_action(Action::Instrument(
                                                InstrumentAction::AdjustEnvelopeAttack(id, delta)
                                            ));
                                        }
                                    }
                                    span { "{env_attack:.3}s" }
                                }
                                div { class: "param-row",
                                    label { "Decay" }
                                    Slider {
                                        value: env_decay,
                                        min: 0.001,
                                        max: 2.0,
                                        vertical: false,
                                        onchange: move |new_val: f32| {
                                            let delta = new_val - env_decay;
                                            dispatch.dispatch_action(Action::Instrument(
                                                InstrumentAction::AdjustEnvelopeDecay(id, delta)
                                            ));
                                        }
                                    }
                                    span { "{env_decay:.3}s" }
                                }
                                div { class: "param-row",
                                    label { "Sustain" }
                                    Slider {
                                        value: env_sustain,
                                        min: 0.0,
                                        max: 1.0,
                                        vertical: false,
                                        onchange: move |new_val: f32| {
                                            let delta = new_val - env_sustain;
                                            dispatch.dispatch_action(Action::Instrument(
                                                InstrumentAction::AdjustEnvelopeSustain(id, delta)
                                            ));
                                        }
                                    }
                                    span { "{env_sustain:.2}" }
                                }
                                div { class: "param-row",
                                    label { "Release" }
                                    Slider {
                                        value: env_release,
                                        min: 0.001,
                                        max: 5.0,
                                        vertical: false,
                                        onchange: move |new_val: f32| {
                                            let delta = new_val - env_release;
                                            dispatch.dispatch_action(Action::Instrument(
                                                InstrumentAction::AdjustEnvelopeRelease(id, delta)
                                            ));
                                        }
                                    }
                                    span { "{env_release:.3}s" }
                                }
                            }
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
