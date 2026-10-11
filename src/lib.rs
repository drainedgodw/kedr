//! GUI-independent editor core. All public text offsets are UTF-8 byte offsets.
pub mod document;
mod history;
mod memory;
pub mod project;
mod search;
mod snapshot;
mod storage;
pub mod text;
mod version;
pub use memory::MemoryStats;
pub use search::Search;
pub use snapshot::Snapshot;
pub use storage::Chunks;
pub use text::{ChangeSummary, Edit, EditBatch, EditError, HistoryLimit, TextBuffer};
pub use version::Version;
