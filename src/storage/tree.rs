use super::chunk::Chunk;
use std::{ops::Range, sync::Arc};

pub(crate) type Tree = Option<Arc<Node>>;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Metrics {
    pub bytes: usize,
    pub newlines: usize,
    pub scalars: usize,
    pub utf16: usize,
    pub leaves: usize,
}
impl std::ops::Add for Metrics {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            bytes: self.bytes + other.bytes,
            newlines: self.newlines + other.newlines,
            scalars: self.scalars + other.scalars,
            utf16: self.utf16 + other.utf16,
            leaves: self.leaves + other.leaves,
        }
    }
}
#[derive(Debug, Clone)]
pub(crate) struct Piece {
    pub source: Arc<Chunk>,
    pub range: Range<usize>,
}
#[derive(Debug, Clone)]
pub(crate) enum Kind {
    Leaf(Piece),
    Branch(Arc<Node>, Arc<Node>),
}
#[derive(Debug, Clone)]
pub(crate) struct Node {
    pub kind: Kind,
    pub metrics: Metrics,
    pub height: u32,
}
impl Node {
    pub fn leaf(piece: Piece) -> Arc<Self> {
        let start = piece.source.ranks(piece.range.start);
        let end = piece.source.ranks(piece.range.end);
        let m = Metrics {
            bytes: piece.range.len(),
            newlines: piece.source.newline_rank(piece.range.end)
                - piece.source.newline_rank(piece.range.start),
            scalars: end.0 - start.0,
            utf16: end.1 - start.1,
            leaves: 1,
        };
        Arc::new(Self {
            kind: Kind::Leaf(piece),
            metrics: m,
            height: 1,
        })
    }
    fn branch(left: Arc<Self>, right: Arc<Self>) -> Arc<Self> {
        Arc::new(Self {
            metrics: left.metrics + right.metrics,
            height: left.height.max(right.height) + 1,
            kind: Kind::Branch(left, right),
        })
    }
}
pub(crate) fn metrics(tree: &Tree) -> Metrics {
    tree.as_ref().map_or(Metrics::default(), |n| n.metrics)
}
pub(crate) fn from_text(text: String) -> Tree {
    if text.is_empty() {
        None
    } else {
        let len = text.len();
        Some(Node::leaf(Piece {
            source: Chunk::new(text),
            range: 0..len,
        }))
    }
}
pub(crate) fn from_str(text: &str) -> Tree {
    if text.is_empty() {
        None
    } else {
        Some(Node::leaf(Piece {
            source: Chunk::borrowed(text),
            range: 0..text.len(),
        }))
    }
}
fn balanced(left: Arc<Node>, right: Arc<Node>) -> Arc<Node> {
    if left.height > right.height + 1 {
        let Kind::Branch(ll, lr) = &left.kind else {
            unreachable!()
        };
        if ll.height >= lr.height {
            Node::branch(ll.clone(), Node::branch(lr.clone(), right))
        } else {
            let Kind::Branch(lrl, lrr) = &lr.kind else {
                unreachable!()
            };
            Node::branch(
                Node::branch(ll.clone(), lrl.clone()),
                Node::branch(lrr.clone(), right),
            )
        }
    } else if right.height > left.height + 1 {
        let Kind::Branch(rl, rr) = &right.kind else {
            unreachable!()
        };
        if rr.height >= rl.height {
            Node::branch(Node::branch(left, rl.clone()), rr.clone())
        } else {
            let Kind::Branch(rll, rlr) = &rl.kind else {
                unreachable!()
            };
            Node::branch(
                Node::branch(left, rll.clone()),
                Node::branch(rlr.clone(), rr.clone()),
            )
        }
    } else {
        Node::branch(left, right)
    }
}
pub(crate) fn join(left: Tree, right: Tree) -> Tree {
    let (left, right) = match (left, right) {
        (None, t) | (t, None) => return t,
        (Some(l), Some(r)) => (l, r),
    };
    if let (Kind::Leaf(l), Kind::Leaf(r)) = (&left.kind, &right.kind)
        && Arc::ptr_eq(&l.source, &r.source)
        && l.range.end == r.range.start
    {
        return Some(Node::leaf(Piece {
            source: l.source.clone(),
            range: l.range.start..r.range.end,
        }));
    }
    if left.height > right.height + 1 {
        let Kind::Branch(ll, lr) = &left.kind else {
            unreachable!()
        };
        let combined = join(Some(lr.clone()), Some(right)).unwrap();
        Some(balanced(ll.clone(), combined))
    } else if right.height > left.height + 1 {
        let Kind::Branch(rl, rr) = &right.kind else {
            unreachable!()
        };
        let combined = join(Some(left), Some(rl.clone())).unwrap();
        Some(balanced(combined, rr.clone()))
    } else {
        Some(Node::branch(left, right))
    }
}
/// Split at an already validated UTF-8 boundary. Retained subtrees are shared.
pub(crate) fn split(tree: Tree, byte: usize) -> (Tree, Tree) {
    let Some(node) = tree else {
        return (None, None);
    };
    if byte == 0 {
        return (None, Some(node));
    }
    if byte == node.metrics.bytes {
        return (Some(node), None);
    }
    match &node.kind {
        Kind::Leaf(piece) => {
            let at = piece.range.start + byte;
            (
                Some(Node::leaf(Piece {
                    source: piece.source.clone(),
                    range: piece.range.start..at,
                })),
                Some(Node::leaf(Piece {
                    source: piece.source.clone(),
                    range: at..piece.range.end,
                })),
            )
        }
        Kind::Branch(left, right) => {
            if byte < left.metrics.bytes {
                let (a, b) = split(Some(left.clone()), byte);
                (a, join(b, Some(right.clone())))
            } else {
                let (a, b) = split(Some(right.clone()), byte - left.metrics.bytes);
                (join(Some(left.clone()), a), b)
            }
        }
    }
}
pub(crate) fn boundary(tree: &Tree, at: usize) -> bool {
    let len = metrics(tree).bytes;
    if at > len {
        return false;
    }
    if at == len {
        return true;
    }
    let mut node = tree.as_deref().unwrap();
    let mut at = at;
    loop {
        match &node.kind {
            Kind::Leaf(piece) => return piece.source.text.is_char_boundary(piece.range.start + at),
            Kind::Branch(left, right) => {
                if at < left.metrics.bytes {
                    node = left;
                } else {
                    at -= left.metrics.bytes;
                    node = right;
                }
            }
        }
    }
}
#[derive(Default)]
pub(crate) struct Prefix {
    pub newlines: usize,
    pub units: usize,
}
fn units(m: Metrics, encoding: crate::PositionEncoding) -> usize {
    match encoding {
        crate::PositionEncoding::UnicodeScalar => m.scalars,
        crate::PositionEncoding::Utf16 => m.utf16,
    }
}
/// Prefix ranks exclude the character/newline at the supplied byte offset.
/// Scan only the requested encoding; scalar cursors do not decode UTF-16 counts.
pub(crate) fn prefix(tree: &Tree, at: usize, encoding: crate::PositionEncoding) -> Prefix {
    let Some(mut node) = tree.as_deref() else {
        return Prefix::default();
    };
    let mut at = at;
    let mut result = Prefix::default();
    loop {
        if at == node.metrics.bytes {
            result.newlines += node.metrics.newlines;
            result.units += units(node.metrics, encoding);
            return result;
        }
        match &node.kind {
            Kind::Leaf(p) => {
                let end = p.range.start + at;
                result.newlines +=
                    p.source.newline_rank(end) - p.source.newline_rank(p.range.start);
                result.units +=
                    p.source.rank(end, encoding) - p.source.rank(p.range.start, encoding);
                return result;
            }
            Kind::Branch(l, r) => {
                if at < l.metrics.bytes {
                    node = l;
                } else {
                    at -= l.metrics.bytes;
                    result.newlines += l.metrics.newlines;
                    result.units += units(l.metrics, encoding);
                    node = r;
                }
            }
        }
    }
}
pub(crate) fn newline_byte(tree: &Tree, rank: usize) -> usize {
    let mut node = tree.as_deref().unwrap();
    let mut rank = rank;
    let mut offset = 0;
    loop {
        match &node.kind {
            Kind::Leaf(p) => {
                return offset
                    + p.source
                        .newline_byte(p.source.newline_rank(p.range.start) + rank)
                    - p.range.start;
            }
            Kind::Branch(l, r) => {
                if rank < l.metrics.newlines {
                    node = l;
                } else {
                    rank -= l.metrics.newlines;
                    offset += l.metrics.bytes;
                    node = r;
                }
            }
        }
    }
}
pub(crate) fn scalar_byte(tree: &Tree, rank: usize) -> usize {
    if rank == metrics(tree).scalars {
        return metrics(tree).bytes;
    }
    let mut node = tree.as_deref().unwrap();
    let mut rank = rank;
    let mut offset = 0;
    loop {
        match &node.kind {
            Kind::Leaf(p) => {
                return offset
                    + p.source
                        .scalar_byte(p.source.scalar_rank(p.range.start) + rank)
                    - p.range.start;
            }
            Kind::Branch(l, r) => {
                if rank < l.metrics.scalars {
                    node = l;
                } else {
                    rank -= l.metrics.scalars;
                    offset += l.metrics.bytes;
                    node = r;
                }
            }
        }
    }
}

pub(crate) fn utf16_byte(tree: &Tree, rank: usize) -> Option<usize> {
    let m = metrics(tree);
    if rank > m.utf16 {
        return None;
    }
    if rank == m.utf16 {
        return Some(m.bytes);
    }
    let mut node = tree.as_deref()?;
    let mut rank = rank;
    let mut offset = 0;
    loop {
        match &node.kind {
            Kind::Leaf(p) => {
                return p
                    .source
                    .utf16_byte(p.source.ranks(p.range.start).1 + rank)
                    .map(|b| offset + b - p.range.start);
            }
            Kind::Branch(l, r) => {
                if rank < l.metrics.utf16 {
                    node = l;
                } else {
                    rank -= l.metrics.utf16;
                    offset += l.metrics.bytes;
                    node = r;
                }
            }
        }
    }
}

/// Borrowed in-order chunks; a ranged walk skips entire unrelated subtrees.
pub struct Chunks<'a> {
    stack: Vec<(&'a Node, usize)>,
    range: Range<usize>,
}
impl<'a> Chunks<'a> {
    pub(crate) fn new(tree: &'a Tree, range: Range<usize>) -> Self {
        let mut stack = Vec::with_capacity(tree.as_ref().map_or(0, |n| n.height as usize));
        if let Some(root) = tree.as_deref() {
            stack.push((root, 0));
        }
        Self { stack, range }
    }
}
impl<'a> Iterator for Chunks<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((node, offset)) = self.stack.pop() {
            if offset >= self.range.end || offset + node.metrics.bytes <= self.range.start {
                continue;
            }
            match &node.kind {
                Kind::Leaf(p) => {
                    let start = self.range.start.saturating_sub(offset);
                    let end = (self.range.end - offset).min(p.range.len());
                    return Some(&p.source.text[p.range.start + start..p.range.start + end]);
                }
                Kind::Branch(l, r) => {
                    self.stack.push((r, offset + l.metrics.bytes));
                    self.stack.push((l, offset));
                }
            }
        }
        None
    }
}
#[cfg(test)]
pub(crate) fn verify(tree: &Tree) {
    fn walk(n: &Node) -> Metrics {
        match &n.kind {
            Kind::Leaf(p) => {
                assert!(!p.range.is_empty());
                assert!(p.source.text.is_char_boundary(p.range.start));
                assert!(p.source.text.is_char_boundary(p.range.end));
                assert_eq!(n.height, 1);
                let expected = Node::leaf(p.clone()).metrics;
                assert_eq!(expected, n.metrics);
                expected
            }
            Kind::Branch(l, r) => {
                assert!(
                    l.height.abs_diff(r.height) <= 1,
                    "Unbalanced: {} / {}",
                    l.height,
                    r.height
                );
                assert_eq!(n.height, l.height.max(r.height) + 1);
                let expected = walk(l) + walk(r);
                assert_eq!(n.metrics, expected);
                expected
            }
        }
    }
    if let Some(root) = tree {
        walk(root);
    }
}
