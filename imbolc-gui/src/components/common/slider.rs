//! Slider component for parameter adjustment.

use dioxus::prelude::*;

/// A slider for adjusting float values.
#[component]
pub fn Slider(
    value: f32,
    min: f32,
    max: f32,
    #[props(default = false)] vertical: bool,
    onchange: EventHandler<f32>,
) -> Element {
    let orientation = if vertical { "vertical" } else { "horizontal" };

    rsx! {
        input {
            r#type: "range",
            class: "slider {orientation}",
            min: "{min}",
            max: "{max}",
            step: "0.001",
            value: "{value}",
            oninput: move |evt| {
                if let Ok(v) = evt.value().parse::<f32>() {
                    onchange.call(v);
                }
            }
        }
    }
}
