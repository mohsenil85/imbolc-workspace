use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    #[allow(dead_code)]
    Warning,
    Error,
}

impl StatusLevel {
    fn ttl(self) -> Duration {
        match self {
            StatusLevel::Info => Duration::from_secs(3),
            StatusLevel::Warning => Duration::from_secs(5),
            StatusLevel::Error => Duration::from_secs(8),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub level: StatusLevel,
    pub timestamp: Instant,
}

impl StatusMessage {
    fn is_expired(&self) -> bool {
        self.timestamp.elapsed() > self.level.ttl()
    }
}

pub struct StatusBar {
    messages: Vec<StatusMessage>,
    max: usize,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            max: 32,
        }
    }

    pub fn push(&mut self, text: impl Into<String>, level: StatusLevel) {
        self.messages.push(StatusMessage {
            text: text.into(),
            level,
            timestamp: Instant::now(),
        });
        if self.messages.len() > self.max {
            self.messages.remove(0);
        }
    }

    /// Returns the most recent non-expired message, if any.
    pub fn current(&self) -> Option<&StatusMessage> {
        self.messages.iter().rev().find(|m| !m.is_expired())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_current() {
        let mut bar = StatusBar::new();
        assert!(bar.current().is_none());
        bar.push("hello", StatusLevel::Info);
        assert_eq!(bar.current().unwrap().text, "hello");
    }

    #[test]
    fn most_recent_wins() {
        let mut bar = StatusBar::new();
        bar.push("first", StatusLevel::Info);
        bar.push("second", StatusLevel::Warning);
        assert_eq!(bar.current().unwrap().text, "second");
    }

    #[test]
    fn max_cap_evicts_oldest() {
        let mut bar = StatusBar {
            messages: Vec::new(),
            max: 2,
        };
        bar.push("a", StatusLevel::Info);
        bar.push("b", StatusLevel::Info);
        bar.push("c", StatusLevel::Info);
        assert_eq!(bar.messages.len(), 2);
        assert_eq!(bar.messages[0].text, "b");
    }
}
