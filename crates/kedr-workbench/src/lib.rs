//! GUI-independent editing workflows and bounded project services.
mod editor;
mod search;
mod tasks;
mod workspace;
pub use editor::{EditorTab, Movement};
pub use search::{SearchHit, SearchReport, search_project};
pub use tasks::{TaskKind, TaskPhase, TaskRunner};
pub use workspace::{FileNode, Workspace, fuzzy_score};
