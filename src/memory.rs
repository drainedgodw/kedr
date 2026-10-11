use crate::{
    history::History,
    storage::{
        chunk::Chunk,
        tree::{Kind, Node, Tree},
    },
};
use std::{collections::HashSet, mem::size_of};

/// Reachable allocation estimate, not exclusive ownership, allocator overhead or RSS.
/// Includes roots referenced by this buffer's history; external snapshots are not enumerated.
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryStats {
    /// Heap text; short inline payloads are included in chunk_header_bytes.
    pub text_bytes: usize,
    pub index_bytes: usize,
    pub node_bytes: usize,
    pub chunk_header_bytes: usize,
    pub history_container_bytes: usize,
}
impl MemoryStats {
    pub fn reachable_bytes(&self) -> usize {
        self.text_bytes
            + self.index_bytes
            + self.node_bytes
            + self.chunk_header_bytes
            + self.history_container_bytes
    }
}
pub(crate) fn estimate(root: &Tree, history: &History) -> MemoryStats {
    let mut roots = Vec::new();
    if let Some(root) = root {
        roots.push(root.as_ref());
    }
    for change in history.undo.iter().chain(history.redo.iter()) {
        if let Some(root) = &change.before {
            roots.push(root.as_ref());
        }
        if let Some(root) = &change.after {
            roots.push(root.as_ref());
        }
    }
    let mut nodes = HashSet::new();
    let mut chunks = HashSet::new();
    let mut stats = MemoryStats {
        history_container_bytes: history.container_bytes(),
        ..Default::default()
    };
    while let Some(node) = roots.pop() {
        if !nodes.insert(node as *const Node as usize) {
            continue;
        }
        stats.node_bytes += size_of::<Node>() + 2 * size_of::<usize>();
        match &node.kind {
            Kind::Leaf(piece) => {
                let source = piece.source.as_ref();
                if chunks.insert(source as *const Chunk as usize) {
                    stats.text_bytes += source.text.heap_bytes();
                    stats.index_bytes += source.index_bytes();
                    stats.chunk_header_bytes += size_of::<Chunk>() + 2 * size_of::<usize>();
                }
            }
            Kind::Branch(l, r) => {
                roots.push(l);
                roots.push(r);
            }
        }
    }
    stats
}
