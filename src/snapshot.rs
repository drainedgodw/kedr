use crate::{
    EditError, Search,
    storage::{
        Chunks,
        tree::{self, Tree},
    },
};
use std::ops::Range;

/// Immutable, thread-safe read version. Cloning shares text/index nodes in O(1).
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub(crate) root: Tree,
    pub(crate) version: crate::Version,
}
impl Snapshot {
    pub fn version(&self) -> crate::Version {
        self.version
    }
    pub fn revision(&self) -> u64 {
        self.version.revision
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
    pub fn chunks(&self) -> Chunks<'_> {
        Chunks::new(&self.root, 0..self.len_bytes())
    }
    pub fn to_text(&self) -> String {
        View(&self.root).to_text()
    }
    pub fn slice(&self, range: Range<usize>) -> Result<String, EditError> {
        View(&self.root).slice(range)
    }
    pub fn byte_to_position(&self, at: usize) -> Result<(usize, usize), EditError> {
        View(&self.root).byte_to_position(at)
    }
    pub fn position_to_byte(&self, line: usize, column: usize) -> Result<usize, EditError> {
        View(&self.root).position_to_byte(line, column)
    }
    pub fn len_utf16(&self) -> usize {
        tree::metrics(&self.root).utf16
    }
    pub fn byte_to_position_in(
        &self,
        at: usize,
        encoding: crate::PositionEncoding,
    ) -> Result<(usize, usize), EditError> {
        View(&self.root).byte_to_position_in(at, encoding)
    }
    pub fn position_to_byte_in(
        &self,
        line: usize,
        column: usize,
        encoding: crate::PositionEncoding,
    ) -> Result<usize, EditError> {
        View(&self.root).position_to_byte_in(line, column, encoding)
    }
    /// Rebuild storage on a background thread without changing content/version.
    pub fn compacted(&self) -> Self {
        Self {
            root: tree::from_text(self.to_text()),
            version: self.version,
        }
    }
    pub fn find_iter<'a>(&'a self, needle: &'a str) -> Search<'a> {
        Search::new(self.chunks(), needle)
    }
    pub fn find_all(&self, needle: &str) -> Vec<Range<usize>> {
        self.find_iter(needle).collect()
    }
}
pub(crate) struct View<'a>(pub &'a Tree);
impl View<'_> {
    pub fn validate(&self, range: &Range<usize>) -> Result<(), EditError> {
        if range.start > range.end || range.end > tree::metrics(self.0).bytes {
            return Err(EditError::InvalidRange);
        }
        if !tree::boundary(self.0, range.start) || !tree::boundary(self.0, range.end) {
            return Err(EditError::NotCharacterBoundary);
        }
        Ok(())
    }
    pub fn to_text(&self) -> String {
        let mut text = String::with_capacity(tree::metrics(self.0).bytes);
        for chunk in Chunks::new(self.0, 0..tree::metrics(self.0).bytes) {
            text.push_str(chunk);
        }
        text
    }
    pub fn slice(&self, range: Range<usize>) -> Result<String, EditError> {
        self.validate(&range)?;
        let mut text = String::with_capacity(range.len());
        for chunk in Chunks::new(self.0, range) {
            text.push_str(chunk);
        }
        Ok(text)
    }
    pub fn range_equals(&self, range: Range<usize>, text: &str) -> bool {
        range.len() == text.len()
            && Chunks::new(self.0, range)
                .flat_map(str::bytes)
                .eq(text.bytes())
    }
    pub fn byte_to_position(&self, at: usize) -> Result<(usize, usize), EditError> {
        self.byte_to_position_in(at, crate::PositionEncoding::UnicodeScalar)
    }
    pub fn position_to_byte(&self, line: usize, column: usize) -> Result<usize, EditError> {
        self.position_to_byte_in(line, column, crate::PositionEncoding::UnicodeScalar)
    }
    pub fn byte_to_position_in(
        &self,
        at: usize,
        encoding: crate::PositionEncoding,
    ) -> Result<(usize, usize), EditError> {
        if !tree::boundary(self.0, at) {
            return Err(EditError::NotCharacterBoundary);
        }
        let prefix = tree::prefix(self.0, at, encoding);
        let start = if prefix.newlines == 0 {
            0
        } else {
            tree::newline_byte(self.0, prefix.newlines - 1) + 1
        };
        let first = tree::prefix(self.0, start, encoding);
        let col = prefix.units - first.units;
        Ok((prefix.newlines, col))
    }
    pub fn position_to_byte_in(
        &self,
        line: usize,
        column: usize,
        encoding: crate::PositionEncoding,
    ) -> Result<usize, EditError> {
        let m = tree::metrics(self.0);
        if line > m.newlines {
            return Err(EditError::InvalidPosition);
        }
        let start = if line == 0 {
            0
        } else {
            tree::newline_byte(self.0, line - 1) + 1
        };
        let end = if line == m.newlines {
            m.bytes
        } else {
            tree::newline_byte(self.0, line)
        };
        let first = tree::prefix(self.0, start, encoding);
        let last = tree::prefix(self.0, end, encoding);
        let (rank, limit) = (first.units, last.units);
        let target = rank.checked_add(column).ok_or(EditError::InvalidPosition)?;
        if target > limit {
            return Err(EditError::InvalidPosition);
        }
        match encoding {
            crate::PositionEncoding::UnicodeScalar => Ok(tree::scalar_byte(self.0, target)),
            crate::PositionEncoding::Utf16 => {
                tree::utf16_byte(self.0, target).ok_or(EditError::InvalidPosition)
            }
        }
    }
}
