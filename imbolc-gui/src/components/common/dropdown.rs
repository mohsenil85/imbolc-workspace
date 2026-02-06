//! Dropdown component for selecting from a list of options.

use dioxus::prelude::*;

/// A dropdown select component.
#[component]
pub fn Dropdown<T: Clone + PartialEq + ToString + 'static>(
    options: Vec<T>,
    selected: T,
    #[props(default = false)] disabled: bool,
    onchange: EventHandler<T>,
) -> Element {
    let selected_str = selected.to_string();

    rsx! {
        select {
            class: "dropdown",
            disabled: disabled,
            value: "{selected_str}",
            onchange: move |evt| {
                let value = evt.value();
                // Find the option that matches the selected value
                if let Some(opt) = options.iter().find(|o| o.to_string() == value) {
                    onchange.call(opt.clone());
                }
            },
            for option in options.iter() {
                option {
                    key: "{option.to_string()}",
                    value: "{option.to_string()}",
                    selected: *option == selected,
                    "{option.to_string()}"
                }
            }
        }
    }
}

/// A labeled dropdown with a title.
#[component]
pub fn LabeledDropdown<T: Clone + PartialEq + ToString + 'static>(
    label: String,
    options: Vec<T>,
    selected: T,
    #[props(default = false)] disabled: bool,
    onchange: EventHandler<T>,
) -> Element {
    let selected_str = selected.to_string();

    rsx! {
        div { class: "labeled-dropdown",
            label { class: "dropdown-label", "{label}" }
            select {
                class: "dropdown",
                disabled: disabled,
                value: "{selected_str}",
                onchange: move |evt| {
                    let value = evt.value();
                    if let Some(opt) = options.iter().find(|o| o.to_string() == value) {
                        onchange.call(opt.clone());
                    }
                },
                for option in options.iter() {
                    option {
                        key: "{option.to_string()}",
                        value: "{option.to_string()}",
                        selected: *option == selected,
                        "{option.to_string()}"
                    }
                }
            }
        }
    }
}
