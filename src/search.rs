use crate::storage::Chunks;
use std::{
    ops::Range,
    str::{Bytes, MatchIndices},
};

/// Lazy literal matches. Native search for a contiguous chunk; KMP across pieces.
/// No flattened text; non-overlapping semantics. Empty needles yield nothing.
pub struct Search<'a> {
    inner: Mode<'a>,
    needle_len: usize,
}
enum Mode<'a> {
    Empty,
    Native(MatchIndices<'a, &'a str>),
    Streaming(Streaming<'a>),
}
struct Streaming<'a> {
    chunks: Chunks<'a>,
    current: Option<Bytes<'a>>,
    pending: Option<&'a str>,
    needle: &'a [u8],
    prefix: Vec<usize>,
    matched: usize,
    offset: usize,
}
impl<'a> Search<'a> {
    pub(crate) fn new(mut chunks: Chunks<'a>, needle: &'a str) -> Self {
        let needle_len = needle.len();
        if needle.is_empty() {
            return Self {
                inner: Mode::Empty,
                needle_len,
            };
        }
        let Some(first) = chunks.next() else {
            return Self {
                inner: Mode::Empty,
                needle_len,
            };
        };
        let Some(second) = chunks.next() else {
            return Self {
                inner: Mode::Native(first.match_indices(needle)),
                needle_len,
            };
        };
        let bytes = needle.as_bytes();
        let mut prefix = vec![0; bytes.len()];
        for i in 1..bytes.len() {
            let mut j = prefix[i - 1];
            while j > 0 && bytes[i] != bytes[j] {
                j = prefix[j - 1];
            }
            if bytes[i] == bytes[j] {
                j += 1;
            }
            prefix[i] = j;
        }
        Self {
            inner: Mode::Streaming(Streaming {
                chunks,
                current: Some(first.bytes()),
                pending: Some(second),
                needle: bytes,
                prefix,
                matched: 0,
                offset: 0,
            }),
            needle_len,
        }
    }
}
impl Iterator for Search<'_> {
    type Item = Range<usize>;
    fn next(&mut self) -> Option<Self::Item> {
        let Mode::Streaming(s) = &mut self.inner else {
            return match &mut self.inner {
                Mode::Native(iter) => iter.next().map(|(at, _)| at..at + self.needle_len),
                _ => None,
            };
        };
        loop {
            let byte = match s.current.as_mut().and_then(Iterator::next) {
                Some(byte) => byte,
                None => {
                    s.current = Some(s.pending.take().or_else(|| s.chunks.next())?.bytes());
                    continue;
                }
            };
            while s.matched > 0 && byte != s.needle[s.matched] {
                s.matched = s.prefix[s.matched - 1];
            }
            if byte == s.needle[s.matched] {
                s.matched += 1;
            }
            s.offset += 1;
            if s.matched == s.needle.len() {
                s.matched = 0;
                return Some(s.offset - s.needle.len()..s.offset);
            }
        }
    }
}
