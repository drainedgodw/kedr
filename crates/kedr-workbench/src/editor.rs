use kedr_core::{Edit, EditBatch, document::Document};
use std::{io, ops::Range, path::Path};
#[derive(Debug, Clone, Copy)]
pub enum Movement {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    DocumentStart,
    DocumentEnd,
    WordLeft,
    WordRight,
}
#[derive(Debug)]
pub struct EditorTab {
    pub document: Document,
    pub cursor: usize,
    pub anchor: usize,
    pub follow_cursor: bool,
    pub desired_column: Option<usize>,
    pub cached_dirty: bool,
    pub newline: &'static str,
}
impl EditorTab {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self::from_document(Document::open(path)?))
    }
    pub fn from_document(document: Document) -> Self {
        let newline = if document.buffer.chunks().any(|c| c.contains("\r\n")) {
            "\r\n"
        } else {
            "\n"
        };
        Self {
            document,
            cursor: 0,
            anchor: 0,
            follow_cursor: true,
            desired_column: None,
            cached_dirty: false,
            newline,
        }
    }
    pub fn name(&self) -> String {
        self.document
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }
    pub fn selection(&self) -> Range<usize> {
        self.cursor.min(self.anchor)..self.cursor.max(self.anchor)
    }
    pub fn selected_text(&self) -> String {
        self.document
            .buffer
            .slice(self.selection())
            .unwrap_or_default()
    }
    pub fn position(&self) -> (usize, usize) {
        self.document
            .buffer
            .byte_to_position(self.cursor)
            .unwrap_or_default()
    }
    pub fn line_bounds(&self, line: usize) -> Range<usize> {
        let b = &self.document.buffer;
        let start = b.position_to_byte(line, 0).unwrap_or(b.len_bytes());
        let end = if line + 1 < b.line_count() {
            b.position_to_byte(line + 1, 0).unwrap() - 1
        } else {
            b.len_bytes()
        };
        start..end
    }
    pub fn set_cursor(&mut self, at: usize, extend: bool) {
        self.cursor = at.min(self.document.buffer.len_bytes());
        while !self.document.buffer.is_char_boundary(self.cursor) {
            self.cursor -= 1;
        }
        if !extend {
            self.anchor = self.cursor;
        }
        self.desired_column = None;
        self.follow_cursor = true;
    }
    fn previous(&self, at: usize) -> usize {
        let mut p = at.saturating_sub(1);
        while !self.document.buffer.is_char_boundary(p) {
            p -= 1;
        }
        p
    }
    fn next(&self, at: usize) -> usize {
        let mut p = (at + 1).min(self.document.buffer.len_bytes());
        while !self.document.buffer.is_char_boundary(p) {
            p += 1;
        }
        p
    }
    pub fn move_cursor(&mut self, movement: Movement, extend: bool) {
        let (row, col) = self.position();
        let mut vertical = None;
        let target = match movement {
            Movement::Left if !extend && self.cursor != self.anchor => self.selection().start,
            Movement::Right if !extend && self.cursor != self.anchor => self.selection().end,
            Movement::Left => self.previous(self.cursor),
            Movement::Right => self.next(self.cursor),
            Movement::Home => self.line_bounds(row).start,
            Movement::End => self.line_bounds(row).end,
            Movement::DocumentStart => 0,
            Movement::DocumentEnd => self.document.buffer.len_bytes(),
            Movement::Up | Movement::Down => {
                let row = if matches!(movement, Movement::Up) {
                    row.saturating_sub(1)
                } else {
                    (row + 1).min(self.document.buffer.line_count() - 1)
                };
                let desired = self.desired_column.unwrap_or(col);
                let max = self
                    .document
                    .buffer
                    .byte_to_position(self.line_bounds(row).end)
                    .unwrap()
                    .1;
                vertical = Some(desired);
                self.document
                    .buffer
                    .position_to_byte(row, desired.min(max))
                    .unwrap()
            }
            Movement::WordLeft => {
                let mut p = self.cursor;
                while p > 0 {
                    let q = self.previous(p);
                    let c = self
                        .document
                        .buffer
                        .slice(q..p)
                        .unwrap()
                        .chars()
                        .next()
                        .unwrap();
                    if c.is_alphanumeric() || c == '_' {
                        break;
                    }
                    p = q;
                }
                while p > 0 {
                    let q = self.previous(p);
                    let c = self
                        .document
                        .buffer
                        .slice(q..p)
                        .unwrap()
                        .chars()
                        .next()
                        .unwrap();
                    if !c.is_alphanumeric() && c != '_' {
                        break;
                    }
                    p = q;
                }
                p
            }
            Movement::WordRight => {
                let mut p = self.cursor;
                let mut word = false;
                while p < self.document.buffer.len_bytes() {
                    let q = self.next(p);
                    let c = self
                        .document
                        .buffer
                        .slice(p..q)
                        .unwrap()
                        .chars()
                        .next()
                        .unwrap();
                    let inside = c.is_alphanumeric() || c == '_';
                    if word && !inside {
                        break;
                    }
                    word |= inside;
                    p = q;
                }
                p
            }
        };
        self.set_cursor(target, extend);
        self.desired_column = vertical;
    }
    pub fn insert(&mut self, text: &str) -> Result<(), String> {
        if text.len() > 1024 * 1024 {
            return Err("A single paste is limited to 1 MiB".into());
        }
        let revision = self.document.buffer.revision();
        let range = self.selection();
        self.document
            .buffer
            .replace(range.clone(), text)
            .map_err(|e| e.to_string())?;
        self.set_cursor(range.start + text.len(), false);
        self.cached_dirty |= self.document.buffer.revision() != revision;
        Ok(())
    }
    pub fn enter(&mut self) -> Result<(), String> {
        let range = self.line_bounds(self.position().0);
        let mut end = (range.start + 4096).min(self.cursor);
        while !self.document.buffer.is_char_boundary(end) {
            end -= 1;
        }
        let prefix = self.document.buffer.slice(range.start..end).unwrap();
        let indent: String = prefix
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        let extra = if prefix.trim_end().ends_with('{') {
            "    "
        } else {
            ""
        };
        self.insert(&format!("{}{indent}{extra}", self.newline))
    }
    pub fn backspace(&mut self) -> Result<(), String> {
        if self.cursor == self.anchor {
            self.anchor = self.previous(self.cursor);
        }
        self.insert("")
    }
    pub fn delete(&mut self) -> Result<(), String> {
        if self.cursor == self.anchor {
            self.anchor = self.next(self.cursor);
        }
        self.insert("")
    }
    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.cursor = self.document.buffer.len_bytes();
        self.follow_cursor = true;
    }
    pub fn undo(&mut self, redo: bool) -> Result<(), String> {
        let changed = if redo {
            self.document.buffer.try_redo()
        } else {
            self.document.buffer.try_undo()
        }
        .map_err(|e| e.to_string())?;
        if changed {
            let event = self.document.buffer.last_change().unwrap();
            self.set_cursor(event.old_range.start + event.inserted_bytes, false);
            self.reconcile_dirty();
        }
        Ok(())
    }
    pub fn indent(&mut self, remove: bool) -> Result<(), String> {
        let selection = self.selection();
        let first = self
            .document
            .buffer
            .byte_to_position(selection.start)
            .unwrap()
            .0;
        let (mut last, col) = self
            .document
            .buffer
            .byte_to_position(selection.end)
            .unwrap();
        if selection.start != selection.end && col == 0 {
            last = last.saturating_sub(1);
        }
        if last - first > 4096 {
            return Err("Indent at most 4096 lines at a time".into());
        }
        let mut edits = Vec::new();
        for line in first..=last {
            let start = self.line_bounds(line).start;
            if remove {
                let mut end = (start + 4).min(self.line_bounds(line).end);
                while !self.document.buffer.is_char_boundary(end) {
                    end -= 1;
                }
                let text = self.document.buffer.slice(start..end).unwrap();
                let n = if text.starts_with('\t') {
                    1
                } else {
                    text.bytes().take_while(|b| *b == b' ').count()
                };
                if n > 0 {
                    edits.push(Edit {
                        range: start..start + n,
                        replacement: String::new(),
                    });
                }
            } else {
                edits.push(Edit {
                    range: start..start,
                    replacement: "    ".into(),
                });
            }
        }
        let (mut cursor, mut anchor) = (self.cursor, self.anchor);
        for edit in &edits {
            let delta = edit.replacement.len() as isize - edit.range.len() as isize;
            if edit.range.start <= self.cursor {
                cursor = cursor.saturating_add_signed(delta);
            }
            if edit.range.start <= self.anchor {
                anchor = anchor.saturating_add_signed(delta);
            }
        }
        self.document
            .buffer
            .apply_batch(&EditBatch::new(&self.document.buffer.snapshot(), edits))
            .map_err(|e| e.to_string())?;
        self.set_cursor(cursor, false);
        self.anchor = anchor.min(self.document.buffer.len_bytes());
        while !self.document.buffer.is_char_boundary(self.anchor) {
            self.anchor -= 1;
        }
        self.cached_dirty = true;
        Ok(())
    }
    pub fn goto(&mut self, line: usize, column: usize) -> Result<(), String> {
        if line == 0 || column == 0 {
            return Err("Line and column start at 1".into());
        }
        let at = self
            .document
            .buffer
            .position_to_byte(line - 1, column - 1)
            .map_err(|_| "Line or Unicode-scalar column is outside this document".to_string())?;
        self.set_cursor(at, false);
        Ok(())
    }
    fn selected_lines(&self) -> (usize, usize) {
        let range = self.selection();
        let first = self
            .document
            .buffer
            .byte_to_position(range.start)
            .unwrap()
            .0;
        let (mut last, column) = self.document.buffer.byte_to_position(range.end).unwrap();
        if !range.is_empty() && column == 0 {
            last = last.saturating_sub(1);
        }
        (first, last)
    }
    fn apply_mapped_edits(&mut self, edits: Vec<Edit>) -> Result<(), String> {
        if edits.is_empty() {
            return Ok(());
        }
        let map = |point: usize| {
            let mut shift = 0isize;
            for edit in &edits {
                if point < edit.range.start {
                    break;
                }
                if point < edit.range.end {
                    return edit.range.start.saturating_add_signed(shift) + edit.replacement.len();
                }
                shift += edit.replacement.len() as isize - edit.range.len() as isize;
            }
            point.saturating_add_signed(shift)
        };
        let (cursor, anchor) = (map(self.cursor), map(self.anchor));
        self.document
            .buffer
            .apply_batch(&EditBatch::new(&self.document.buffer.snapshot(), edits))
            .map_err(|e| e.to_string())?;
        self.set_cursor(cursor, false);
        self.anchor = anchor.min(self.document.buffer.len_bytes());
        while !self.document.buffer.is_char_boundary(self.anchor) {
            self.anchor -= 1;
        }
        self.cached_dirty = true;
        Ok(())
    }
    /// Toggle textual line comments; this is not semantic language analysis.
    pub fn toggle_comment(&mut self) -> Result<(), String> {
        let ext = self
            .document
            .path
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("");
        let prefix = match ext {
            "rs" | "c" | "cpp" | "cc" | "h" | "hpp" | "js" | "jsx" | "ts" | "tsx" | "java"
            | "kt" | "go" | "cs" | "php" | "swift" => "//",
            "py" | "rb" | "sh" | "bash" | "toml" | "yaml" | "yml" | "conf" => "#",
            "sql" | "lua" => "--",
            _ => return Err("No line-comment syntax for this file type".into()),
        };
        let (first, last) = self.selected_lines();
        if last - first >= 4096 {
            return Err("Comment at most 4096 lines at a time".into());
        }
        if self.line_bounds(last).end - self.line_bounds(first).start > 1024 * 1024 {
            return Err("Comment at most 1 MiB of text at a time".into());
        }
        let mut lines = Vec::new();
        for row in first..=last {
            let bounds = self.line_bounds(row);
            if bounds.len() > 1024 * 1024 {
                return Err("Commenting a line over 1 MiB is not supported".into());
            }
            let text = self
                .document
                .buffer
                .slice(bounds.clone())
                .map_err(|e| e.to_string())?;
            let indent = text
                .bytes()
                .take_while(|b| *b == b' ' || *b == b'\t')
                .count();
            if !text[indent..].trim().is_empty() {
                lines.push((bounds.start + indent, text[indent..].to_owned()));
            }
        }
        let remove = !lines.is_empty() && lines.iter().all(|(_, t)| t.starts_with(prefix));
        let edits = lines
            .into_iter()
            .map(|(at, text)| {
                if remove {
                    let count = prefix.len() + usize::from(text[prefix.len()..].starts_with(' '));
                    Edit {
                        range: at..at + count,
                        replacement: String::new(),
                    }
                } else {
                    Edit {
                        range: at..at,
                        replacement: format!("{prefix} "),
                    }
                }
            })
            .collect();
        self.apply_mapped_edits(edits)
    }
    /// Move the current line or selected complete line block by one row.
    pub fn move_lines(&mut self, down: bool) -> Result<(), String> {
        let (first, last) = self.selected_lines();
        let count = self.document.buffer.line_count();
        if (!down && first == 0) || (down && last + 1 >= count) {
            return Ok(());
        }
        let start = self.line_bounds(first).start;
        let block_end = if last + 1 < count {
            self.line_bounds(last + 1).start
        } else {
            self.document.buffer.len_bytes()
        };
        let (left_start, split, right_end) = if down {
            (
                start,
                block_end,
                if last + 2 < count {
                    self.line_bounds(last + 2).start
                } else {
                    self.document.buffer.len_bytes()
                },
            )
        } else {
            (self.line_bounds(first - 1).start, start, block_end)
        };
        if right_end - left_start > 1024 * 1024 {
            return Err("Move at most 1 MiB of text at a time".into());
        }
        let left = self
            .document
            .buffer
            .slice(left_start..split)
            .map_err(|e| e.to_string())?;
        let right = self
            .document
            .buffer
            .slice(split..right_end)
            .map_err(|e| e.to_string())?;
        fn content_and_ending(s: &str) -> (&str, &str) {
            if let Some(t) = s.strip_suffix("\r\n") {
                (t, "\r\n")
            } else if let Some(t) = s.strip_suffix('\n') {
                (t, "\n")
            } else {
                (s, "")
            }
        }
        let (left_content, left_ending) = content_and_ending(&left);
        let (right_content, right_ending) = content_and_ending(&right);
        let replacement = format!("{right_content}{left_ending}{left_content}{right_ending}");
        let new_start = if down {
            left_start + right_content.len() + left_ending.len()
        } else {
            left_start
        };
        let cursor = new_start + self.cursor.saturating_sub(start);
        let anchor = new_start + self.anchor.saturating_sub(start);
        self.document
            .buffer
            .replace(left_start..right_end, &replacement)
            .map_err(|e| e.to_string())?;
        self.set_cursor(cursor.min(right_end), false);
        self.anchor = anchor.min(self.document.buffer.len_bytes());
        while !self.document.buffer.is_char_boundary(self.anchor) {
            self.anchor -= 1;
        }
        self.cached_dirty = true;
        Ok(())
    }
    pub fn join_lines(&mut self) -> Result<(), String> {
        let row = self.position().0;
        if row + 1 >= self.document.buffer.line_count() {
            return Ok(());
        }
        let current = self.line_bounds(row);
        let next = self.line_bounds(row + 1);
        // Bound work on pathological long lines rather than flattening the document.
        if current.len() + next.len() > 1024 * 1024 {
            return Err("Join at most 1 MiB of text at a time".into());
        }
        let left = self
            .document
            .buffer
            .slice(current.clone())
            .map_err(|e| e.to_string())?;
        let right = self
            .document
            .buffer
            .slice(next.clone())
            .map_err(|e| e.to_string())?;
        let left_trimmed = left.trim_end_matches([' ', '\t', '\r']);
        let indent = right
            .bytes()
            .take_while(|b| *b == b' ' || *b == b'\t')
            .count();
        let space = if left_trimmed.is_empty() || right[indent..].trim().is_empty() {
            ""
        } else {
            " "
        };
        self.apply_mapped_edits(vec![Edit {
            range: current.start + left_trimmed.len()..next.start + indent,
            replacement: space.into(),
        }])
    }

    pub fn duplicate(&mut self) -> Result<(), String> {
        if self.cursor != self.anchor {
            let range = self.selection();
            let text = self.selected_text();
            self.set_cursor(range.end, false);
            self.insert(&text)
        } else {
            let old = self.cursor;
            let range = self.line_bounds(self.position().0);
            if range.end < self.document.buffer.len_bytes() {
                let text = self
                    .document
                    .buffer
                    .slice(range.start..range.end + 1)
                    .map_err(|e| e.to_string())?;
                self.set_cursor(range.start, false);
                self.insert(&text)?;
                self.set_cursor(old + text.len(), false);
            } else {
                let text = self
                    .document
                    .buffer
                    .slice(range.clone())
                    .map_err(|e| e.to_string())?;
                let value = format!("{}{}", self.newline, text);
                self.set_cursor(range.end, false);
                self.insert(&value)?;
                self.set_cursor(range.end + self.newline.len() + old - range.start, false);
            }
            Ok(())
        }
    }
    pub fn delete_line(&mut self) -> Result<(), String> {
        let range = self.line_bounds(self.position().0);
        self.anchor = range.start;
        self.cursor = (range.end + usize::from(range.end < self.document.buffer.len_bytes()))
            .min(self.document.buffer.len_bytes());
        self.insert("")
    }
    pub fn find_previous(&mut self, needle: &str) -> bool {
        if needle.is_empty() {
            return false;
        }
        let before = self.selection().start;
        let mut previous = None;
        let mut last = None;
        for r in self.document.buffer.find_iter(needle) {
            if r.start < before {
                previous = Some(r.clone());
            }
            last = Some(r);
        }
        if let Some(r) = previous.or(last) {
            self.anchor = r.start;
            self.cursor = r.end;
            self.follow_cursor = true;
            true
        } else {
            false
        }
    }
    pub fn find_next(&mut self, needle: &str) -> bool {
        if needle.is_empty() {
            return false;
        }
        let next = self
            .document
            .buffer
            .find_iter(needle)
            .find(|r| r.start >= self.cursor)
            .or_else(|| self.document.buffer.find_iter(needle).next());
        if let Some(r) = next {
            self.anchor = r.start;
            self.cursor = r.end;
            self.follow_cursor = true;
            true
        } else {
            false
        }
    }
    pub fn replace_all(&mut self, needle: &str, replacement: &str) -> Result<usize, String> {
        if needle.is_empty() {
            return Ok(0);
        }
        let matches: Vec<_> = self.document.buffer.find_iter(needle).take(10001).collect();
        if matches.len() > 10000 {
            return Err("Replace at most 10,000 matches at a time".into());
        }
        let mut end = 0;
        let edits: Vec<_> = matches
            .into_iter()
            .filter(|r| {
                if r.start < end {
                    false
                } else {
                    end = r.end;
                    true
                }
            })
            .map(|range| Edit {
                range,
                replacement: replacement.into(),
            })
            .collect();
        let n = edits.len();
        self.document
            .buffer
            .apply_batch(&EditBatch::new(&self.document.buffer.snapshot(), edits))
            .map_err(|e| e.to_string())?;
        self.set_cursor(self.cursor, false);
        self.cached_dirty |= n > 0;
        Ok(n)
    }
    pub fn save(&mut self) -> io::Result<()> {
        self.document.save()?;
        self.cached_dirty = false;
        Ok(())
    }
    pub fn reconcile_dirty(&mut self) {
        self.cached_dirty = self.document.is_dirty();
    }
}
