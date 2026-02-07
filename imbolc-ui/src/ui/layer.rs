use std::collections::HashMap;

use super::keymap::Keymap;
use super::InputEvent;
use super::action_id::ActionId;

/// A named layer with a keymap and transparency setting.
pub struct Layer {
    pub name: &'static str,
    pub keymap: Keymap,
    pub transparent: bool,
}

/// Result of resolving an input event through the layer stack.
pub enum LayerResult {
    /// A layer matched the event with this action ID.
    Action(ActionId),
    /// An opaque layer blocked the event without matching it.
    Blocked,
    /// No layer matched the event and all layers were transparent.
    Unresolved,
}

/// A stack of named layers that resolves input events top-to-bottom.
pub struct LayerStack {
    layers: HashMap<&'static str, Layer>,
    active: Vec<&'static str>, // bottom-to-top
}

impl LayerStack {
    pub fn new(layers: Vec<Layer>) -> Self {
        let map: HashMap<&'static str, Layer> = layers
            .into_iter()
            .map(|l| (l.name, l))
            .collect();
        Self {
            layers: map,
            active: Vec::new(),
        }
    }

    /// Resolve an input event through the active layer stack (top-to-bottom).
    pub fn resolve(&self, event: &InputEvent) -> LayerResult {
        for name in self.active.iter().rev() {
            if let Some(layer) = self.layers.get(name) {
                if let Some(action) = layer.keymap.lookup(event) {
                    return LayerResult::Action(action);
                }
                if !layer.transparent {
                    return LayerResult::Blocked;
                }
            }
        }
        LayerResult::Unresolved
    }

    /// Push a named layer onto the top of the stack.
    pub fn push(&mut self, name: &'static str) {
        if !self.active.contains(&name) {
            self.active.push(name);
        }
    }

    /// Remove a named layer from the stack (wherever it is).
    pub fn pop(&mut self, name: &'static str) {
        self.active.retain(|n| *n != name);
    }

    /// Set the pane layer at position 1 (between global at 0 and mode layers at 2+).
    /// If the layer doesn't exist in the loaded layers, position 1 is left empty.
    pub fn set_pane_layer(&mut self, name: &'static str) {
        // Collect mode layers (everything above position 1)
        let mode_layers: Vec<&'static str> = if self.active.len() > 2 {
            self.active[2..].to_vec()
        } else {
            Vec::new()
        };

        // Keep only global (position 0)
        self.active.truncate(1);

        // Insert pane layer at position 1 if it exists
        if self.layers.contains_key(name) {
            self.active.push(name);
        }

        // Re-add mode layers on top
        self.active.extend(mode_layers);
    }

    /// Check if a layer is currently active.
    pub fn has_layer(&self, name: &str) -> bool {
        self.active.iter().any(|n| *n == name)
    }

    /// Collect all commands from active layers for the command palette.
    /// Walks top-to-bottom (matching resolution priority), deduplicates by action ID.
    pub fn collect_commands(&self) -> Vec<(ActionId, &'static str, String)> {
        let mut seen = std::collections::HashSet::new();
        let mut commands = Vec::new();
        for name in self.active.iter().rev() {
            if let Some(layer) = self.layers.get(name) {
                for binding in layer.keymap.bindings() {
                    if seen.insert(binding.action) {
                        commands.push((binding.action, binding.description, binding.pattern.display()));
                    }
                }
            }
        }
        commands.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::action_id::GlobalActionId;
    use crate::ui::input::KeyCode;

    fn make_layer(name: &'static str, key: char, transparent: bool) -> Layer {
        let action = ActionId::Global(GlobalActionId::Quit);
        Layer {
            name,
            keymap: Keymap::new().bind(key, action, "test"),
            transparent,
        }
    }

    fn make_event(ch: char) -> InputEvent {
        InputEvent::key(KeyCode::Char(ch))
    }

    #[test]
    fn new_empty_stack() {
        let stack = LayerStack::new(vec![]);
        assert!(!stack.has_layer("anything"));
    }

    #[test]
    fn push_adds_layer() {
        let mut stack = LayerStack::new(vec![make_layer("test", 'q', true)]);
        stack.push("test");
        assert!(stack.has_layer("test"));
    }

    #[test]
    fn push_deduplicates() {
        let mut stack = LayerStack::new(vec![make_layer("test", 'q', true)]);
        stack.push("test");
        stack.push("test");
        assert_eq!(stack.active.len(), 1);
    }

    #[test]
    fn pop_removes() {
        let mut stack = LayerStack::new(vec![make_layer("test", 'q', true)]);
        stack.push("test");
        stack.pop("test");
        assert!(!stack.has_layer("test"));
    }

    #[test]
    fn resolve_empty_unresolved() {
        let stack = LayerStack::new(vec![]);
        match stack.resolve(&make_event('q')) {
            LayerResult::Unresolved => {}
            _ => panic!("expected Unresolved"),
        }
    }

    #[test]
    fn resolve_opaque_blocks() {
        // Opaque layer that doesn't match 'x' should block
        let mut stack = LayerStack::new(vec![make_layer("opaque", 'q', false)]);
        stack.push("opaque");
        match stack.resolve(&make_event('x')) {
            LayerResult::Blocked => {}
            _ => panic!("expected Blocked"),
        }
    }

    #[test]
    fn resolve_transparent_falls_through() {
        // Transparent layer that doesn't match should fall through to Unresolved
        let mut stack = LayerStack::new(vec![make_layer("trans", 'q', true)]);
        stack.push("trans");
        match stack.resolve(&make_event('x')) {
            LayerResult::Unresolved => {}
            _ => panic!("expected Unresolved"),
        }
    }

    #[test]
    fn set_pane_layer_replaces_position_1() {
        let mut stack = LayerStack::new(vec![
            make_layer("global", 'g', true),
            make_layer("pane_a", 'a', true),
            make_layer("pane_b", 'b', true),
        ]);
        stack.push("global");  // position 0
        stack.push("pane_a");  // position 1

        stack.set_pane_layer("pane_b");
        assert!(stack.has_layer("global"));
        assert!(stack.has_layer("pane_b"));
        assert!(!stack.has_layer("pane_a"));
    }
}
