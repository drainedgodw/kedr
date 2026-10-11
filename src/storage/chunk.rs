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
    pub checkpoints: Vec<(usize, usize)>,
    pub ascii: bool,
    pub scalars: usize,
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
        let scalars = if ascii {
            text.len()
        } else {
            let mut count = 0;
            for (at, _) in text.char_indices() {
                if text.len() > INLINE_BYTES && count % CHECKPOINT_STRIDE == 0 {
                    checkpoints.push((at, count));
                }
                count += 1;
            }
            count
        };
        Arc::new(Self {
            text,
            newlines,
            checkpoints,
            ascii,
            scalars,
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
    pub fn scalar_rank(&self, at: usize) -> usize {
        if self.ascii {
            return at;
        }
        if at == self.text.len() {
            return self.scalars;
        }
        if self.checkpoints.is_empty() {
            return self.text[..at].chars().count();
        }
        let i = self
            .checkpoints
            .partition_point(|(p, _)| *p <= at)
            .saturating_sub(1);
        let (byte, scalar) = self.checkpoints[i];
        scalar + self.text[byte..at].chars().count()
    }
    pub fn scalar_byte(&self, scalar: usize) -> usize {
        if self.ascii {
            return scalar;
        }
        if scalar == self.scalars {
            return self.text.len();
        }
        if self.checkpoints.is_empty() {
            return self.text.char_indices().nth(scalar).unwrap().0;
        }
        let i = self
            .checkpoints
            .partition_point(|(_, s)| *s <= scalar)
            .saturating_sub(1);
        let (byte, count) = self.checkpoints[i];
        byte + self.text[byte..]
            .char_indices()
            .nth(scalar - count)
            .unwrap()
            .0
    }
    pub fn index_bytes(&self) -> usize {
        let nl = match &self.newlines {
            Newlines::None | Newlines::Inline => 0,
            Newlines::Narrow(v) => v.capacity() * std::mem::size_of::<u32>(),
            Newlines::Wide(v) => v.capacity() * std::mem::size_of::<usize>(),
        };
        nl + self.checkpoints.capacity() * std::mem::size_of::<(usize, usize)>()
    }
}
