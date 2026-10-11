//! GUI-independent editor core. All public text offsets are UTF-8 byte offsets.
pub mod document;
pub mod history;
pub mod project;
pub mod text;
pub use text::{Edit, EditError, HistoryLimit, TextBuffer};
