//! Button components.

use dioxus::prelude::*;

/// A standard button.
#[component]
pub fn Button(
    label: String,
    #[props(default = false)] disabled: bool,
    onclick: EventHandler<()>,
) -> Element {
    rsx! {
        button {
            class: "btn",
            disabled: disabled,
            onclick: move |_| onclick.call(()),
            "{label}"
        }
    }
}

/// A toggle button that shows active state.
#[component]
pub fn ToggleButton(label: String, active: bool, onclick: EventHandler<()>) -> Element {
    rsx! {
        button {
            class: if active { "btn toggle-btn active" } else { "btn toggle-btn" },
            onclick: move |_| onclick.call(()),
            "{label}"
        }
    }
}
