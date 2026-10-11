use crate::{
    MemoryStats, Search, Snapshot,
    history::{Change, History},
    snapshot::View,
    storage::{
        Chunks,
        tree::{self, Tree},
    },
};
use std::{fmt, ops::Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryLimit {
    Disabled,
    Bytes(usize),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    InvalidRange,
    NotCharacterBoundary,
    OverlappingEdits,
    InvalidPosition,
    StaleRevision { expected: u64, actual: u64 },
    WrongDocument { expected: u64, actual: u64 },
}
impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for EditError {}
#[derive(Debug, Clone)]
pub struct Edit {
    pub range: Range<usize>,
    pub replacement: String,
}

/// Exact invalidation envelope for the latest content change; not an event queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeSummary {
    pub old_revision: u64,
    pub new_revision: u64,
    pub old_range: Range<usize>,
    pub inserted_bytes: usize,
    pub old_line_count: usize,
    pub new_line_count: usize,
}
/// Original-coordinate batch from a known immutable read revision.
#[derive(Debug, Clone)]
pub struct EditBatch {
    pub base_version: crate::Version,
    pub edits: Vec<Edit>,
}
impl EditBatch {
    pub fn new(snapshot: &Snapshot, edits: Vec<Edit>) -> Self {
        Self {
            base_version: snapshot.version(),
            edits,
        }
    }
}

/// Persistent AVL piece tree. Snapshots share immutable text/index subtrees.
#[derive(Debug)]
pub struct TextBuffer {
    root: Tree,
    document_id: u64,
    revision: u64,
    history: History,
    last_change: Option<ChangeSummary>,
}
impl Clone for TextBuffer {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            document_id: crate::version::new_document(),
            revision: self.revision,
            history: self.history.clone(),
            last_change: None,
        }
    }
}
impl TextBuffer {
    pub fn version(&self) -> crate::Version {
        crate::Version {
            document_id: self.document_id,
            revision: self.revision,
        }
    }

    pub fn new(text: impl Into<String>) -> Self {
        Self::with_history(text, HistoryLimit::Bytes(8 * 1024 * 1024))
    }
    pub fn with_history(text: impl Into<String>, limit: HistoryLimit) -> Self {
        let limit = match limit {
            HistoryLimit::Disabled => 0,
            HistoryLimit::Bytes(n) => n,
        };
        Self {
            root: tree::from_text(text.into()),
            document_id: crate::version::new_document(),
            revision: 0,
            history: History::new(limit),
            last_change: None,
        }
    }
    pub fn len_bytes(&self) -> usize {
        tree::metrics(&self.root).bytes
    }
    pub fn is_empty(&self) -> bool {
        self.len_bytes() == 0
    }
    pub fn line_count(&self) -> usize {
        tree::metrics(&self.root).newlines + 1
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn piece_count(&self) -> usize {
        tree::metrics(&self.root).leaves
    }
    pub fn tree_height(&self) -> u32 {
        self.root.as_ref().map_or(0, |n| n.height)
    }
    pub fn history_bytes(&self) -> usize {
        self.history.bytes()
    }
    pub fn set_history_step_limit(&mut self, limit: usize) {
        self.history.set_step_limit(limit);
    }
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
    pub fn memory_stats(&self) -> MemoryStats {
        crate::memory::estimate(&self.root, &self.history)
    }
    /// Backing text bytes reachable from the buffer/history. O(reachable nodes).
    pub fn text_capacity_bytes(&self) -> usize {
        self.memory_stats().text_bytes
    }
    pub fn chunks(&self) -> Chunks<'_> {
        Chunks::new(&self.root, 0..self.len_bytes())
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            root: self.root.clone(),
            version: self.version(),
        }
    }
    pub fn last_change(&self) -> Option<&ChangeSummary> {
        self.last_change.as_ref()
    }
    pub fn take_change(&mut self) -> Option<ChangeSummary> {
        self.last_change.take()
    }
    pub fn to_text(&self) -> String {
        View(&self.root).to_text()
    }
    pub fn equals(&self, text: &str) -> bool {
        if text.len() != self.len_bytes() {
            return false;
        }
        let mut at = 0;
        for chunk in self.chunks() {
            if text.as_bytes()[at..at + chunk.len()] != *chunk.as_bytes() {
                return false;
            }
            at += chunk.len();
        }
        true
    }
    pub fn is_char_boundary(&self, offset: usize) -> bool {
        tree::boundary(&self.root, offset)
    }
    pub fn slice(&self, range: Range<usize>) -> Result<String, EditError> {
        View(&self.root).slice(range)
    }
    pub fn replace(&mut self, range: Range<usize>, replacement: &str) -> Result<(), EditError> {
        View(&self.root).validate(&range)?;
        if View(&self.root).range_equals(range.clone(), replacement) {
            return Ok(());
        }
        let before = if self.history.limit > 0 {
            self.root.clone()
        } else {
            None
        };
        self.replace_tree(range.clone(), tree::from_str(replacement));
        if self.history.limit > 0 {
            self.history.record(Change {
                at: range.start,
                before,
                after: self.root.clone(),
                removed_bytes: range.len(),
                inserted_bytes: replacement.len(),
            });
        }
        Ok(())
    }
    fn changed(&mut self, old_range: Range<usize>, inserted_bytes: usize, old_line_count: usize) {
        let old_revision = self.revision;
        self.revision = self.revision.saturating_add(1);
        self.last_change = Some(ChangeSummary {
            old_revision,
            new_revision: self.revision,
            old_range,
            inserted_bytes,
            old_line_count,
            new_line_count: self.line_count(),
        });
    }
    /// Internal tree splice; validated coordinates, no string materialization.
    fn replace_tree(&mut self, range: Range<usize>, replacement: Tree) -> Tree {
        let old_lines = self.line_count();
        let inserted = tree::metrics(&replacement).bytes;
        let (left, tail) = tree::split(self.root.take(), range.start);
        let (removed, right) = tree::split(tail, range.len());
        self.root = tree::join(tree::join(left, replacement), right);
        self.changed(range, inserted, old_lines);
        removed
    }
    pub fn apply_batch(&mut self, batch: &EditBatch) -> Result<Option<ChangeSummary>, EditError> {
        if batch.base_version.document_id != self.document_id {
            return Err(EditError::WrongDocument {
                expected: batch.base_version.document_id,
                actual: self.document_id,
            });
        }
        if batch.base_version.revision != self.revision {
            return Err(EditError::StaleRevision {
                expected: batch.base_version.revision,
                actual: self.revision,
            });
        }
        let previous = self.revision;
        self.apply_edits(&batch.edits)?;
        Ok((self.revision != previous).then(|| self.last_change.clone().unwrap()))
    }
    /// Non-overlapping original-coordinate edits form one history transaction.
    pub fn apply_edits(&mut self, edits: &[Edit]) -> Result<(), EditError> {
        if edits.is_empty() {
            return Ok(());
        }
        let mut edits: Vec<_> = edits.iter().collect();
        edits.sort_by_key(|e| e.range.start);
        for (i, edit) in edits.iter().enumerate() {
            View(&self.root).validate(&edit.range)?;
            if i > 0
                && (edit.range.start < edits[i - 1].range.end
                    || edit.range.start == edits[i - 1].range.start)
            {
                return Err(EditError::OverlappingEdits);
            }
        }
        if edits
            .iter()
            .all(|e| View(&self.root).range_equals(e.range.clone(), &e.replacement))
        {
            return Ok(());
        }
        let span = edits[0].range.start..edits.last().unwrap().range.end;
        let root_before = self.root.clone();
        let (_, tail) = tree::split(self.root.clone(), span.start);
        let (before, _) = tree::split(tail, span.len());
        let mut after = before.clone();
        for edit in edits.into_iter().rev() {
            let (left, tail) = tree::split(after, edit.range.start - span.start);
            let (_, right) = tree::split(tail, edit.range.len());
            after = tree::join(tree::join(left, tree::from_str(&edit.replacement)), right);
        }
        self.replace_tree(span.clone(), after.clone());
        if self.history.limit > 0 {
            self.history.record(Change {
                at: span.start,
                before: root_before,
                after: self.root.clone(),
                removed_bytes: span.len(),
                inserted_bytes: tree::metrics(&after).bytes,
            });
        }
        Ok(())
    }
    pub fn undo(&mut self) -> bool {
        let Some(change) = self.history.undo.pop_back() else {
            return false;
        };
        let old_lines = self.line_count();
        self.root = change.before.clone();
        self.changed(
            change.at..change.at + change.inserted_bytes,
            change.removed_bytes,
            old_lines,
        );
        self.history.redo.push(change);
        true
    }
    pub fn redo(&mut self) -> bool {
        let Some(change) = self.history.redo.pop() else {
            return false;
        };
        let old_lines = self.line_count();
        self.root = change.after.clone();
        self.changed(
            change.at..change.at + change.removed_bytes,
            change.inserted_bytes,
            old_lines,
        );
        self.history.undo.push_back(change);
        true
    }
    /// Consolidate the live root. History/external snapshots may retain old chunks.
    pub fn compact(&mut self) {
        self.root = tree::from_text(self.to_text());
    }
    pub fn byte_to_position(&self, offset: usize) -> Result<(usize, usize), EditError> {
        View(&self.root).byte_to_position(offset)
    }
    pub fn position_to_byte(&self, line: usize, column: usize) -> Result<usize, EditError> {
        View(&self.root).position_to_byte(line, column)
    }
    pub fn find_iter<'a>(&'a self, needle: &'a str) -> Search<'a> {
        Search::new(self.chunks(), needle)
    }
    pub fn find_all(&self, needle: &str) -> Vec<Range<usize>> {
        self.find_iter(needle).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn avl_invariants_survive_fragmentation_and_undo() {
        let mut b = TextBuffer::new("x".repeat(4096));
        for i in 0..4096 {
            let mut at = (i * 2654435761usize) % b.len_bytes();
            while !b.is_char_boundary(at) {
                at -= 1;
            }
            b.replace(at..at, "🌲\n").unwrap();
            tree::verify(&b.root);
        }
        for _ in 0..4096 {
            assert!(b.undo());
            tree::verify(&b.root);
        }
        assert_eq!(b.to_text(), "x".repeat(4096));
    }
}
