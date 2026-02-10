use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// Best-effort registry tracking which SuperCollider synth nodes are believed
/// to be alive.  When scsynth crashes mid-session, `invalidate_all()` clears
/// the set so subsequent `check_node()` calls surface stale-node warnings
/// instead of silently sending OSC to dead nodes.
pub struct NodeRegistry {
    live_nodes: HashSet<i32>,
    created_at: HashMap<i32, Instant>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            live_nodes: HashSet::new(),
            created_at: HashMap::new(),
        }
    }

    /// Record that a node has been created on the server.
    pub fn register(&mut self, node_id: i32) {
        self.live_nodes.insert(node_id);
        self.created_at.insert(node_id, Instant::now());
    }

    /// Record that a node has been freed (or is about to be freed).
    pub fn unregister(&mut self, node_id: i32) {
        self.live_nodes.remove(&node_id);
        self.created_at.remove(&node_id);
    }

    /// Mark all nodes as dead (e.g. after a server crash).
    pub fn invalidate_all(&mut self) {
        self.live_nodes.clear();
        self.created_at.clear();
    }

    /// Number of nodes currently believed to be alive.
    pub fn live_count(&self) -> usize {
        self.live_nodes.len()
    }
}

#[cfg(test)]
impl NodeRegistry {
    /// Returns `true` if the node is believed to be alive.
    pub fn is_live(&self, node_id: i32) -> bool {
        self.live_nodes.contains(&node_id)
    }

    /// Check whether a node is tracked as live.  If it is not, log a warning
    /// and return `false`.
    pub fn check_node(&self, node_id: i32) -> bool {
        if self.live_nodes.contains(&node_id) {
            true
        } else {
            log::warn!(
                target: "audio::nodes",
                "node {} is not tracked as live",
                node_id
            );
            false
        }
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_unregister() {
        let mut reg = NodeRegistry::new();
        reg.register(100);
        assert!(reg.is_live(100));
        assert_eq!(reg.live_count(), 1);

        reg.unregister(100);
        assert!(!reg.is_live(100));
        assert_eq!(reg.live_count(), 0);
    }

    #[test]
    fn invalidate_all_clears_everything() {
        let mut reg = NodeRegistry::new();
        reg.register(1);
        reg.register(2);
        reg.register(3);
        assert_eq!(reg.live_count(), 3);

        reg.invalidate_all();
        assert_eq!(reg.live_count(), 0);
        assert!(!reg.is_live(1));
    }

    #[test]
    fn check_node_returns_false_for_unknown() {
        let reg = NodeRegistry::new();
        assert!(!reg.check_node(999));
    }

    #[test]
    fn check_node_returns_true_for_live() {
        let mut reg = NodeRegistry::new();
        reg.register(42);
        assert!(reg.check_node(42));
    }
}
