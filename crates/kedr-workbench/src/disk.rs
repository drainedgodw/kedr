//! Explicit reloads and content-based active-file polling; no filesystem watcher.
use crate::Workspace;
use kedr_core::{
    Version,
    document::{Document, MAX_DOCUMENT_BYTES},
};
use std::{
    io,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
static NEXT_QUERY: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct FileQuery {
    pub path: PathBuf,
    root: PathBuf,
    version: Version,
    saved: Arc<str>,
    request_id: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiskState {
    Current,
    Changed,
    Missing,
    Unreadable(String),
}
impl FileQuery {
    pub fn capture(w: &Workspace) -> Option<Self> {
        let t = w.active()?;
        Some(Self {
            request_id: NEXT_QUERY.fetch_add(1, Ordering::Relaxed),
            path: t.document.path.clone(),
            root: w.project.root.clone(),
            version: t.document.buffer.version(),
            saved: t.document.saved_snapshot(),
        })
    }
    pub fn same_request(&self, other: &Self) -> bool {
        self.request_id == other.request_id
            && self.path == other.path
            && self.root == other.root
            && self.version == other.version
            && Arc::ptr_eq(&self.saved, &other.saved)
    }
    /// Identity and baseline checks reject saves, edits/undo, reopen and workspace changes.
    pub fn is_current(&self, w: &Workspace) -> bool {
        w.project.root == self.root
            && w.active().is_some_and(|t| {
                t.document.path == self.path
                    && t.document.buffer.version() == self.version
                    && Arc::ptr_eq(&self.saved, &t.document.saved_snapshot())
            })
    }
    /// Run only off the UI thread. Missing/invalid targets never modify the editor or disk.
    pub fn read(&self) -> io::Result<Document> {
        let path = self.path.canonicalize()?;
        if path != self.path || !path.starts_with(&self.root) {
            return Err(io::Error::other(
                "File identity changed or escaped the workspace",
            ));
        }
        let metadata = std::fs::metadata(&path)?;
        if !metadata.is_file() || metadata.len() > MAX_DOCUMENT_BYTES as u64 {
            return Err(io::Error::other(
                "Reload requires a regular UTF-8 file of at most 16 MiB",
            ));
        }
        let doc = Document::open(&path)?;
        if doc.path != self.path || self.path.canonicalize()? != path {
            return Err(io::Error::other("File identity changed during read"));
        }
        Ok(doc)
    }
    pub fn probe(&self) -> DiskState {
        match self.read() {
            Ok(doc) if doc.saved_snapshot().as_ref() == self.saved.as_ref() => DiskState::Current,
            Ok(_) => DiskState::Changed,
            Err(e) if e.kind() == io::ErrorKind::NotFound => DiskState::Missing,
            Err(e) => DiskState::Unreadable(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "kedr-disk-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&p).unwrap();
            Self(p.canonicalize().unwrap())
        }
        fn workspace(&self) -> Workspace {
            fs::write(self.0.join("a.rs"), "🌲\r\noriginal\r\n").unwrap();
            let mut w = Workspace::scan(&self.0).unwrap();
            w.open(self.0.join("a.rs")).unwrap();
            w
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn content_poll_detects_same_length_changes_and_deletion_without_mutation() {
        let f = Fixture::new();
        let mut w = f.workspace();
        let q = FileQuery::capture(&w).unwrap();
        assert_eq!(q.probe(), DiskState::Current);
        fs::write(&q.path, "🌲\r\nexternal\r\n").unwrap();
        assert_eq!(q.probe(), DiskState::Changed);
        assert!(!w.active().unwrap().document.is_dirty());
        w.active_mut().unwrap().insert("unsaved ").unwrap();
        assert_eq!(q.probe(), DiskState::Changed);
        assert!(
            w.active()
                .unwrap()
                .document
                .buffer
                .to_text()
                .starts_with("unsaved ")
        );
        fs::remove_file(&q.path).unwrap();
        assert_eq!(q.probe(), DiskState::Missing);
        assert!(q.read().is_err());
        assert!(!q.path.exists());
    }
    #[test]
    fn reload_snapshot_preserves_unicode_newlines_and_guarded_save() {
        let f = Fixture::new();
        let w = f.workspace();
        let q = FileQuery::capture(&w).unwrap();
        fs::write(&q.path, "\u{feff}new 🌲\r\n").unwrap();
        let mut d = q.read().unwrap();
        assert_eq!(d.buffer.to_text(), "\u{feff}new 🌲\r\n");
        assert!(!d.is_dirty());
        d.buffer.replace(0..0, "ours").unwrap();
        fs::write(&q.path, "next writer").unwrap();
        assert!(d.save().is_err());
        assert_eq!(fs::read_to_string(&q.path).unwrap(), "next writer");
    }
    #[test]
    fn query_invalidates_on_edits_undo_save_reopen_switch_and_workspace() {
        let f = Fixture::new();
        let mut w = f.workspace();
        let q = FileQuery::capture(&w).unwrap();
        assert!(q.is_current(&w));
        assert!(q.same_request(&q.clone()));
        w.active_mut().unwrap().insert("edit").unwrap();
        assert!(!q.is_current(&w));
        w.active_mut().unwrap().undo(false).unwrap();
        assert!(!q.is_current(&w));
        let q = FileQuery::capture(&w).unwrap();
        w.active_mut().unwrap().save().unwrap();
        assert!(!q.is_current(&w));
        let q = FileQuery::capture(&w).unwrap();
        w.discard(0);
        w.open(&q.path).unwrap();
        assert!(!q.is_current(&w));
        let q = FileQuery::capture(&w).unwrap();
        fs::write(f.0.join("b.rs"), "b").unwrap();
        w.open(f.0.join("b.rs")).unwrap();
        assert!(!q.is_current(&w));
        let other = Fixture::new();
        assert!(!q.is_current(&other.workspace()));
    }
    #[test]
    fn binary_large_and_directory_replacements_are_not_loaded() {
        let f = Fixture::new();
        let w = f.workspace();
        let q = FileQuery::capture(&w).unwrap();
        fs::write(&q.path, [0xff, 0xfe]).unwrap();
        assert!(matches!(q.probe(), DiskState::Unreadable(_)));
        fs::File::create(&q.path)
            .unwrap()
            .set_len((MAX_DOCUMENT_BYTES + 1) as u64)
            .unwrap();
        assert!(q.read().is_err());
        fs::remove_file(&q.path).unwrap();
        fs::create_dir(&q.path).unwrap();
        assert!(q.read().is_err());
    }
    #[cfg(unix)]
    #[test]
    fn escaping_symlink_is_rejected_before_read() {
        let f = Fixture::new();
        let other = Fixture::new();
        let w = f.workspace();
        let q = FileQuery::capture(&w).unwrap();
        let outside = other.0.join("secret");
        fs::write(&outside, "outside").unwrap();
        fs::remove_file(&q.path).unwrap();
        std::os::unix::fs::symlink(&outside, &q.path).unwrap();
        assert!(q.read().is_err());
        assert!(matches!(q.probe(), DiskState::Unreadable(_)));
    }
}
