//! Level meter component.

use dioxus::prelude::*;

/// A level meter display (CSS-based).
#[component]
pub fn Meter(level: f32) -> Element {
    let height = (level * 100.0).clamp(0.0, 100.0);

    rsx! {
        div { class: "meter",
            div {
                class: "meter-fill",
                style: "height: {height}%;"
            }
        }
    }
}
