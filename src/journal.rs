use crate::{ChangeSummary, EditError};
use std::collections::VecDeque;

/// Optional bounded invalidation history. Disabled by default; not an edit replay log.
#[derive(Debug)]
pub(crate) struct ChangeJournal {
    entries: VecDeque<ChangeSummary>,
    limit: usize,
}
impl ChangeJournal {
    pub fn new(limit: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            limit,
        }
    }
    pub fn limit(&self) -> usize {
        self.limit
    }
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
        while self.entries.len() > limit {
            self.entries.pop_front();
        }
        if limit == 0 {
            self.entries = VecDeque::new();
        }
    }
    pub fn record(&mut self, change: &ChangeSummary) {
        if self.limit == 0 {
            return;
        }
        if self.entries.len() == self.limit {
            self.entries.pop_front();
        }
        self.entries.push_back(change.clone());
    }
    pub fn since(
        &self,
        requested: u64,
        current: u64,
    ) -> Result<impl Iterator<Item = &ChangeSummary>, EditError> {
        if requested > current {
            return Err(EditError::FutureRevision { requested, current });
        }
        let oldest = self.entries.front().map_or(current, |e| e.old_revision);
        if requested < oldest {
            return Err(EditError::ChangeLogGap {
                requested,
                oldest_available: oldest,
            });
        }
        Ok(self
            .entries
            .iter()
            .filter(move |e| e.old_revision >= requested))
    }
    pub fn allocated_bytes(&self) -> usize {
        self.entries.capacity() * std::mem::size_of::<ChangeSummary>()
    }
}
