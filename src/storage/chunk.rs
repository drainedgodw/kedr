use std::{ops::Deref, sync::Arc};
const CHECKPOINT_STRIDE: usize = 256;
const INLINE_BYTES: usize = 24;

/// Short edit payloads need no separate text allocation.
#[derive(Debug)]
pub(crate) enum Text {
    Inline { bytes: [u8; INLINE_BYTES], len: u8 },
    Heap(Box<str>),
}
impl Text {
    fn borrowed(text: &str) -> Self {
        if text.len() > INLINE_BYTES {
            return Self::Heap(text.into());
        }
        let mut bytes = [0; INLINE_BYTES];
        bytes[..text.len()].copy_from_slice(text.as_bytes());
        Self::Inline {
            bytes,
            len: text.len() as u8,
        }
    }
    fn owned(text: String) -> Self {
        if text.len() <= INLINE_BYTES {
            Self::borrowed(&text)
        } else {
            Self::Heap(text.into_boxed_str())
        }
    }
    pub fn heap_bytes(&self) -> usize {
        match self {
            Self::Inline { .. } => 0,
            Self::Heap(text) => text.len(),
        }
    }
}
impl Deref for Text {
    type Target = str;
    fn deref(&self) -> &str {
        match self {
            Self::Inline { bytes, len } => {
                std::str::from_utf8(&bytes[..*len as usize]).expect("copied valid UTF-8")
            }
            Self::Heap(text) => text,
        }
    }
}
#[derive(Debug)]
enum Newlines {
    None,
    Inline,
    Narrow(Vec<u32>),
    Wide(Vec<usize>),
}
#[derive(Debug)]
pub(crate) struct Chunk {
    pub text: Text,
    newlines: Newlines,
    pub checkpoints: Vec<(usize, usize, usize)>,
    pub ascii: bool,
    pub scalars: usize,
    pub utf16: usize,
}
impl Chunk {
    pub fn new(text: String) -> Arc<Self> {
        Self::build(Text::owned(text))
    }
    pub fn borrowed(text: &str) -> Arc<Self> {
        Self::build(Text::borrowed(text))
    }
    fn build(text: Text) -> Arc<Self> {
        let ascii = text.is_ascii();
        let newlines = if !text.contains('\n') {
            Newlines::None
        } else if text.len() <= INLINE_BYTES {
            Newlines::Inline
        } else if u32::try_from(text.len()).is_ok() {
            Newlines::Narrow(
                text.bytes()
                    .enumerate()
                    .filter_map(|(i, b)| (b == b'\n').then_some(i as u32))
                    .collect(),
            )
        } else {
            Newlines::Wide(
                text.bytes()
                    .enumerate()
                    .filter_map(|(i, b)| (b == b'\n').then_some(i))
                    .collect(),
            )
        };
        let mut checkpoints = Vec::new();
        let indexed = text.len() > INLINE_BYTES;
        let view: &str = &text;
        let (scalars, utf16) = if ascii {
            (view.len(), view.len())
        } else {
            let mut scalars = 0;
            let mut units = 0;
            for (count, (at, ch)) in view.char_indices().enumerate() {
                if indexed && count.is_multiple_of(CHECKPOINT_STRIDE) {
                    checkpoints.push((at, count, units));
                }
                scalars = count + 1;
                units += ch.len_utf16();
            }
            (scalars, units)
        };
        Arc::new(Self {
            text,
            newlines,
            checkpoints,
            ascii,
            scalars,
            utf16,
        })
    }
    pub fn newline_rank(&self, at: usize) -> usize {
        match &self.newlines {
            Newlines::None => 0,
            Newlines::Inline => self.text[..at].bytes().filter(|b| *b == b'\n').count(),
            Newlines::Narrow(v) => v.partition_point(|p| (*p as usize) < at),
            Newlines::Wide(v) => v.partition_point(|p| *p < at),
        }
    }
    pub fn newline_byte(&self, rank: usize) -> usize {
        match &self.newlines {
            Newlines::None => unreachable!("no newline"),
            Newlines::Inline => self
                .text
                .bytes()
                .enumerate()
                .filter_map(|(i, b)| (b == b'\n').then_some(i))
                .nth(rank)
                .unwrap(),
            Newlines::Narrow(v) => v[rank] as usize,
            Newlines::Wide(v) => v[rank],
        }
    }
    pub fn ranks(&self, at: usize) -> (usize, usize) {
        if self.ascii {
            return (at, at);
        }
        if at == self.text.len() {
            return (self.scalars, self.utf16);
        }
        let (byte, mut scalar, mut units) = if self.checkpoints.is_empty() {
            (0, 0, 0)
        } else {
            let i = self
                .checkpoints
                .partition_point(|(p, _, _)| *p <= at)
                .saturating_sub(1);
            self.checkpoints[i]
        };
        for ch in self.text[byte..at].chars() {
            scalar += 1;
            units += ch.len_utf16();
        }
        (scalar, units)
    }
    pub fn scalar_rank(&self, at: usize) -> usize {
        if self.ascii {
            return at;
        }
        if at == self.text.len() {
            return self.scalars;
        }
        let (byte, scalar) = if self.checkpoints.is_empty() {
            (0, 0)
        } else {
            let i = self
                .checkpoints
                .partition_point(|(p, _, _)| *p <= at)
                .saturating_sub(1);
            let (byte, scalar, _) = self.checkpoints[i];
            (byte, scalar)
        };
        scalar + self.text[byte..at].chars().count()
    }
    pub fn rank(&self, at: usize, encoding: crate::PositionEncoding) -> usize {
        match encoding {
            crate::PositionEncoding::UnicodeScalar => self.scalar_rank(at),
            crate::PositionEncoding::Utf16 => self.ranks(at).1,
        }
    }
    pub fn scalar_byte(&self, scalar: usize) -> usize {
        if self.ascii {
            return scalar;
        }
        if scalar == self.scalars {
            return self.text.len();
        }
        let (byte, count) = if self.checkpoints.is_empty() {
            (0, 0)
        } else {
            let i = self
                .checkpoints
                .partition_point(|(_, s, _)| *s <= scalar)
                .saturating_sub(1);
            let (b, s, _) = self.checkpoints[i];
            (b, s)
        };
        byte + self.text[byte..]
            .char_indices()
            .nth(scalar - count)
            .unwrap()
            .0
    }
    pub fn utf16_byte(&self, target: usize) -> Option<usize> {
        if target > self.utf16 {
            return None;
        }
        if self.ascii {
            return Some(target);
        }
        if target == self.utf16 {
            return Some(self.text.len());
        }
        let (byte, mut units) = if self.checkpoints.is_empty() {
            (0, 0)
        } else {
            let i = self
                .checkpoints
                .partition_point(|(_, _, u)| *u <= target)
                .saturating_sub(1);
            let (b, _, u) = self.checkpoints[i];
            (b, u)
        };
        for (at, ch) in self.text[byte..].char_indices() {
            if units == target {
                return Some(byte + at);
            }
            units += ch.len_utf16();
            if units > target {
                return None;
            }
        }
        None
    }
    pub fn index_bytes(&self) -> usize {
        let nl = match &self.newlines {
            Newlines::None | Newlines::Inline => 0,
            Newlines::Narrow(v) => v.capacity() * std::mem::size_of::<u32>(),
            Newlines::Wide(v) => v.capacity() * std::mem::size_of::<usize>(),
        };
        nl + self.checkpoints.capacity() * std::mem::size_of::<(usize, usize, usize)>()
    }
}
