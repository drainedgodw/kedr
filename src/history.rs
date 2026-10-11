use crate::storage::tree::Tree;
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub(crate) struct Change {
    pub at: usize,
    pub before: Tree,
    pub after: Tree,
    pub removed_bytes: usize,
    pub inserted_bytes: usize,
}
impl Change {
    pub fn bytes(&self) -> usize {
        self.removed_bytes + self.inserted_bytes
    }
}
#[derive(Clone, Debug)]
pub(crate) struct History {
    pub undo: VecDeque<Change>,
    pub redo: Vec<Change>,
    pub limit: usize,
    bytes: usize,
    step_limit: usize,
}
impl History {
    pub fn new(limit: usize) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: Vec::new(),
            limit,
            bytes: 0,
            step_limit: 4096,
        }
    }
    pub fn record(&mut self, change: Change) {
        self.bytes -= self.redo.iter().map(Change::bytes).sum::<usize>();
        self.redo.clear();
        if change.bytes() > self.limit {
            self.clear();
            return;
        }
        self.bytes += change.bytes();
        self.undo.push_back(change);
        self.prune();
    }
    fn prune(&mut self) {
        while self.bytes > self.limit || self.undo.len() + self.redo.len() > self.step_limit {
            if let Some(change) = self.undo.pop_front() {
                self.bytes -= change.bytes();
            } else if !self.redo.is_empty() {
                self.bytes -= self.redo.remove(0).bytes();
            } else {
                break;
            }
        }
    }
    pub fn set_step_limit(&mut self, limit: usize) {
        self.step_limit = limit;
        self.prune();
    }
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.bytes = 0;
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
    pub fn container_bytes(&self) -> usize {
        (self.undo.capacity() + self.redo.capacity()) * std::mem::size_of::<Change>()
    }
}
