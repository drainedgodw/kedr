use crate::history::{Change, History};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Original,
    Added,
}
#[derive(Debug, Clone, Copy)]
struct Piece {
    source: Source,
    start: usize,
    len: usize,
}

/// Piece table with delta-based undo. Editing never copies the whole document.
/// The piece index is currently a Vec, so fragmented edits still cost O(pieces).
#[derive(Debug, Clone)]
pub struct TextBuffer {
    original: String,
    added: String,
    pieces: Vec<Piece>,
    len: usize,
    lines: usize,
    revision: u64,
    history: History,
}
impl TextBuffer {
    pub fn new(text: impl Into<String>) -> Self {
        Self::with_history(text, HistoryLimit::Bytes(8 * 1024 * 1024))
    }
    pub fn with_history(text: impl Into<String>, limit: HistoryLimit) -> Self {
        let original = text.into();
        let len = original.len();
        let lines = original.bytes().filter(|b| *b == b'\n').count() + 1;
        let pieces = if len == 0 {
            Vec::new()
        } else {
            vec![Piece {
                source: Source::Original,
                start: 0,
                len,
            }]
        };
        let limit = match limit {
            HistoryLimit::Disabled => 0,
            HistoryLimit::Bytes(n) => n,
        };
        Self {
            original,
            added: String::new(),
            pieces,
            len,
            lines,
            revision: 0,
            history: History::new(limit),
        }
    }
    pub fn len_bytes(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn line_count(&self) -> usize {
        self.lines
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn piece_count(&self) -> usize {
        self.pieces.len()
    }
    pub fn history_bytes(&self) -> usize {
        self.history.bytes()
    }
    /// Allocated text capacity, excluding pieces, history and allocator metadata.
    pub fn text_capacity_bytes(&self) -> usize {
        self.original.capacity() + self.added.capacity()
    }
    pub fn chunks(&self) -> impl Iterator<Item = &str> {
        self.pieces
            .iter()
            .map(|p| &self.source(p.source)[p.start..p.start + p.len])
    }
    pub fn to_text(&self) -> String {
        let mut text = String::with_capacity(self.len);
        for chunk in self.chunks() {
            text.push_str(chunk);
        }
        text
    }
    pub fn equals(&self, text: &str) -> bool {
        if text.len() != self.len {
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
    fn source(&self, source: Source) -> &str {
        match source {
            Source::Original => &self.original,
            Source::Added => &self.added,
        }
    }
    pub fn is_char_boundary(&self, offset: usize) -> bool {
        if offset > self.len {
            return false;
        }
        if offset == self.len {
            return true;
        }
        let mut at = 0;
        for p in &self.pieces {
            if offset < at + p.len {
                return self
                    .source(p.source)
                    .is_char_boundary(p.start + offset - at);
            }
            at += p.len;
        }
        offset == 0
    }
    fn validate(&self, range: &Range<usize>) -> Result<(), EditError> {
        if range.start > range.end || range.end > self.len {
            return Err(EditError::InvalidRange);
        }
        if !self.is_char_boundary(range.start) || !self.is_char_boundary(range.end) {
            return Err(EditError::NotCharacterBoundary);
        }
        Ok(())
    }
    pub fn slice(&self, range: Range<usize>) -> Result<String, EditError> {
        self.validate(&range)?;
        let mut result = String::with_capacity(range.len());
        let mut at = 0;
        for chunk in self.chunks() {
            let end = at + chunk.len();
            let left = range.start.max(at);
            let right = range.end.min(end);
            if left < right {
                result.push_str(&chunk[left - at..right - at]);
            }
            if end >= range.end {
                break;
            }
            at = end;
        }
        Ok(result)
    }
    pub fn replace(&mut self, range: Range<usize>, replacement: &str) -> Result<(), EditError> {
        let before = self.slice(range.clone())?;
        if before == replacement {
            return Ok(());
        }
        let change = Change {
            at: range.start,
            before,
            after: replacement.to_owned(),
        };
        self.replace_raw(range, replacement);
        if self.history.limit > 0 {
            self.history.record(change);
        }
        Ok(())
    }
    fn replace_raw(&mut self, range: Range<usize>, replacement: &str) {
        let removed_lines = self
            .slice(range.clone())
            .expect("validated range")
            .bytes()
            .filter(|b| *b == b'\n')
            .count();
        let inserted_lines = replacement.bytes().filter(|b| *b == b'\n').count();
        let added_at = self.added.len();
        self.added.push_str(replacement);
        let inserted = Piece {
            source: Source::Added,
            start: added_at,
            len: replacement.len(),
        };
        let mut pieces = Vec::with_capacity(self.pieces.len() + 2);
        let mut at = 0;
        let mut did_insert = false;
        for p in self.pieces.iter().copied() {
            let end = at + p.len;
            if end <= range.start {
                pieces.push(p);
            } else if at >= range.end {
                if !did_insert {
                    if inserted.len > 0 {
                        pieces.push(inserted);
                    }
                    did_insert = true;
                }
                pieces.push(p);
            } else {
                if at < range.start {
                    pieces.push(Piece {
                        len: range.start - at,
                        ..p
                    });
                }
                if !did_insert {
                    if inserted.len > 0 {
                        pieces.push(inserted);
                    }
                    did_insert = true;
                }
                if end > range.end {
                    pieces.push(Piece {
                        start: p.start + range.end - at,
                        len: end - range.end,
                        ..p
                    });
                }
            }
            at = end;
        }
        if !did_insert && inserted.len > 0 {
            pieces.push(inserted);
        }
        self.pieces.clear();
        for piece in pieces {
            if let Some(last) = self.pieces.last_mut()
                && last.source == piece.source
                && last.start + last.len == piece.start
            {
                last.len += piece.len;
                continue;
            }
            self.pieces.push(piece);
        }
        self.len = self.len - range.len() + replacement.len();
        self.lines = self.lines - removed_lines + inserted_lines;
        self.revision = self.revision.saturating_add(1);
    }
    /// Apply non-overlapping original-coordinate edits as one undoable change.
    /// Validation completes before mutation; malformed batches do not partly apply.
    pub fn apply_edits(&mut self, edits: &[Edit]) -> Result<(), EditError> {
        if edits.is_empty() {
            return Ok(());
        }
        let mut edits: Vec<_> = edits.iter().collect();
        edits.sort_by_key(|e| e.range.start);
        for (i, edit) in edits.iter().enumerate() {
            self.validate(&edit.range)?;
            if i > 0
                && (edit.range.start < edits[i - 1].range.end
                    || edit.range.start == edits[i - 1].range.start)
            {
                return Err(EditError::OverlappingEdits);
            }
        }
        let span = edits[0].range.start..edits.last().unwrap().range.end;
        let mut text = self.slice(span.clone())?;
        for edit in edits.into_iter().rev() {
            text.replace_range(
                edit.range.start - span.start..edit.range.end - span.start,
                &edit.replacement,
            );
        }
        self.replace(span, &text)
    }
    pub fn undo(&mut self) -> bool {
        let Some(change) = self.history.undo.pop_back() else {
            return false;
        };
        self.replace_raw(change.at..change.at + change.after.len(), &change.before);
        self.history.redo.push(change);
        true
    }
    pub fn redo(&mut self) -> bool {
        let Some(change) = self.history.redo.pop() else {
            return false;
        };
        self.replace_raw(change.at..change.at + change.before.len(), &change.after);
        self.history.undo.push_back(change);
        true
    }
    /// Reclaim dead added-text storage. History remains valid (it stores deltas).
    pub fn compact(&mut self) {
        self.original = self.to_text();
        self.added = String::new();
        self.pieces.clear();
        if self.len > 0 {
            self.pieces.push(Piece {
                source: Source::Original,
                start: 0,
                len: self.len,
            });
        }
    }
    /// Zero-based line and Unicode scalar column. O(document bytes), not graphemes.
    pub fn byte_to_position(&self, offset: usize) -> Result<(usize, usize), EditError> {
        if !self.is_char_boundary(offset) {
            return Err(EditError::NotCharacterBoundary);
        }
        let mut line = 0;
        let mut col = 0;
        let mut at = 0;
        for chunk in self.chunks() {
            for c in chunk.chars() {
                if at == offset {
                    return Ok((line, col));
                }
                at += c.len_utf8();
                if c == '\n' {
                    line += 1;
                    col = 0;
                } else {
                    col += 1;
                }
            }
        }
        Ok((line, col))
    }
    pub fn position_to_byte(&self, line: usize, column: usize) -> Result<usize, EditError> {
        let mut row = 0;
        let mut col = 0;
        let mut at = 0;
        for chunk in self.chunks() {
            for c in chunk.chars() {
                if (row, col) == (line, column) {
                    return Ok(at);
                }
                if c == '\n' {
                    row += 1;
                    col = 0;
                } else {
                    col += 1;
                }
                at += c.len_utf8();
            }
        }
        if (row, col) == (line, column) {
            Ok(at)
        } else {
            Err(EditError::InvalidPosition)
        }
    }
    /// Literal non-overlapping search; currently materializes the document once.
    pub fn find_all(&self, needle: &str) -> Vec<Range<usize>> {
        if needle.is_empty() {
            return Vec::new();
        }
        self.to_text()
            .match_indices(needle)
            .map(|(at, _)| at..at + needle.len())
            .collect()
    }
}
