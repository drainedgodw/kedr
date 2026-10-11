use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_DOCUMENT: AtomicU64 = AtomicU64::new(1);

/// Process-local document identity plus monotonic content revision.
/// Not a persisted or globally unique identifier across process restarts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
    pub document_id: u64,
    pub revision: u64,
}
pub(crate) fn new_document() -> u64 {
    NEXT_DOCUMENT.fetch_add(1, Ordering::Relaxed)
}
