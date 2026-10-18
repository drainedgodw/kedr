//! Bounded, session-local navigation. Positions are 1-based Unicode-scalar coordinates.
use std::path::PathBuf;
const LIMIT: usize = 128;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
}
#[derive(Debug, Default)]
pub struct NavigationHistory {
    entries: Vec<Location>,
    index: usize,
}
impl NavigationHistory {
    pub fn current(&self) -> Option<&Location> {
        self.entries.get(self.index)
    }
    pub fn can_back(&self) -> bool {
        !self.entries.is_empty() && self.index > 0
    }
    pub fn can_forward(&self) -> bool {
        self.index + 1 < self.entries.len()
    }
    /// Explicit navigation creates a checkpoint, discarding a forward branch.
    pub fn visit(&mut self, location: Location) {
        if self.current() == Some(&location) {
            return;
        }
        if !self.entries.is_empty() {
            self.entries.truncate(self.index + 1);
        }
        self.entries.push(location);
        if self.entries.len() > LIMIT {
            self.entries.remove(0);
        }
        self.index = self.entries.len() - 1;
    }
    /// Cursor movement is not a new navigation event. A file switch is.
    pub fn observe(&mut self, location: Location) {
        if self
            .current()
            .is_some_and(|current| current.path == location.path)
        {
            self.entries[self.index] = location;
        } else {
            self.visit(location);
        }
    }
    pub fn back(&mut self) -> Option<Location> {
        if !self.can_back() {
            return None;
        }
        self.index -= 1;
        self.current().cloned()
    }
    pub fn forward(&mut self) -> Option<Location> {
        if !self.can_forward() {
            return None;
        }
        self.index += 1;
        self.current().cloned()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn loc(file: &str, line: usize) -> Location {
        Location {
            path: file.into(),
            line,
            column: 1,
        }
    }
    #[test]
    fn empty_and_single_have_no_targets() {
        let mut h = NavigationHistory::default();
        assert!(h.back().is_none() && h.forward().is_none());
        h.visit(loc("a", 1));
        h.visit(loc("a", 1));
        assert!(!h.can_back() && !h.can_forward());
    }
    #[test]
    fn cursor_observation_does_not_flood_history() {
        let mut h = NavigationHistory::default();
        for line in 1..500 {
            h.observe(loc("a", line));
        }
        h.observe(loc("b", 3));
        assert_eq!(h.back(), Some(loc("a", 499)));
        assert_eq!(h.forward(), Some(loc("b", 3)));
    }
    #[test]
    fn explicit_same_file_jump_restores_position() {
        let mut h = NavigationHistory::default();
        h.visit(loc("a", 3));
        h.visit(loc("a", 90));
        assert_eq!(h.back(), Some(loc("a", 3)));
        assert_eq!(h.forward(), Some(loc("a", 90)));
    }
    #[test]
    fn new_navigation_discards_forward_branch() {
        let mut h = NavigationHistory::default();
        for name in ["a", "b", "c"] {
            h.visit(loc(name, 1));
        }
        h.back();
        h.visit(loc("d", 4));
        assert!(!h.can_forward());
        assert_eq!(h.back(), Some(loc("b", 1)));
    }
    #[test]
    fn bounded_history_retains_latest_entries() {
        let mut h = NavigationHistory::default();
        for line in 1..=200 {
            h.visit(loc("a", line));
        }
        let mut n = 0;
        while h.back().is_some() {
            n += 1;
        }
        assert_eq!(n, 127);
        assert_eq!(h.current(), Some(&loc("a", 73)));
    }
}
