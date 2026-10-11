use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub(crate) struct Change {
    pub at: usize,
    pub before: String,
    pub after: String,
}
impl Change {
    pub fn bytes(&self) -> usize {
        self.before.len() + self.after.len()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct History {
    pub undo: VecDeque<Change>,
    pub redo: Vec<Change>,
    pub limit: usize,
    bytes: usize,
}
impl History {
    pub fn new(limit: usize) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: Vec::new(),
            limit,
            bytes: 0,
        }
    }
    pub fn record(&mut self, change: Change) {
        self.bytes -= self.redo.iter().map(Change::bytes).sum::<usize>();
        self.redo.clear();
        if change.bytes() > self.limit {
            self.undo.clear();
            self.bytes = 0;
            return;
        }
        self.bytes += change.bytes();
        self.undo.push_back(change);
        while self.bytes > self.limit {
            if let Some(change) = self.undo.pop_front() {
                self.bytes -= change.bytes();
            }
        }
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
}
