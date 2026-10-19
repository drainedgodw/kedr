//! Bounded plaintext draft snapshots. Never writes to workspace files.
use crate::{EditorTab, Workspace};
use kedr_core::{
    Version,
    document::{Document, MAX_DOCUMENT_BYTES},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    sync::Arc,
};
pub const MAX_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_FILE: usize = 100 * 1024 * 1024;
#[derive(Clone, Debug)]
pub struct Draft {
    pub path: PathBuf,
    pub baseline: String,
    pub text: String,
    pub cursor: usize,
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub root: PathBuf,
    pub drafts: Vec<Draft>,
}
#[derive(Clone, Debug)]
pub struct Offer {
    pub id: usize,
    pub root: PathBuf,
    pub count: usize,
}
pub struct Guard {
    root: PathBuf,
    tabs: Vec<(PathBuf, Version, Arc<str>)>,
}
impl Guard {
    pub fn capture(w: &Workspace) -> Self {
        Self {
            root: w.project.root.clone(),
            tabs: w
                .tabs
                .iter()
                .map(|t| {
                    (
                        t.document.path.clone(),
                        t.document.buffer.version(),
                        t.document.saved_snapshot(),
                    )
                })
                .collect(),
        }
    }
    pub fn matches(&self, w: &Workspace) -> bool {
        self.root == w.project.root
            && self.tabs.len() == w.tabs.len()
            && self.tabs.iter().zip(&w.tabs).all(|((p, v, s), t)| {
                *p == t.document.path
                    && *v == t.document.buffer.version()
                    && Arc::ptr_eq(s, &t.document.saved_snapshot())
            })
    }
}
impl Snapshot {
    pub fn capture(w: &Workspace) -> io::Result<Self> {
        let mut drafts = Vec::new();
        let mut bytes = 0usize;
        for t in &w.tabs {
            if !t.document.is_dirty() {
                continue;
            }
            bytes = bytes
                .saturating_add(t.document.buffer.len_bytes())
                .saturating_add(t.document.saved_snapshot().len());
            if bytes > MAX_BYTES {
                return Err(io::Error::other(
                    "Draft recovery quota exceeded (16 MiB text plus baselines); last snapshot retained",
                ));
            }
            drafts.push(Draft {
                path: t.document.path.clone(),
                baseline: t.document.saved_snapshot().to_string(),
                text: t.document.buffer.to_text(),
                cursor: t.cursor,
            });
        }
        let s = Self {
            root: w.project.root.clone(),
            drafts,
        };
        s.validate()?;
        Ok(s)
    }
    pub fn validate(&self) -> io::Result<()> {
        if !self.root.is_absolute() || self.drafts.len() > 32 {
            return Err(io::Error::other("Invalid recovery workspace or tab quota"));
        }
        let mut bytes = 0usize;
        let mut paths = std::collections::BTreeSet::new();
        for d in &self.drafts {
            let rel = d.path.strip_prefix(&self.root).map_err(io::Error::other)?;
            if rel.as_os_str().is_empty()
                || rel.components().any(|c| !matches!(c, Component::Normal(_)))
                || !paths.insert(&d.path)
                || d.text.len() > MAX_DOCUMENT_BYTES
                || d.baseline.len() > MAX_DOCUMENT_BYTES
                || d.text == d.baseline
            {
                return Err(io::Error::other(
                    "Invalid recovery draft path, text or duplicate",
                ));
            }
            bytes = bytes
                .saturating_add(d.text.len())
                .saturating_add(d.baseline.len());
        }
        if bytes > MAX_BYTES {
            return Err(io::Error::other("Recovery text quota exceeded"));
        }
        Ok(())
    }
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(&json!({"format":1,"root":self.root,"drafts":self.drafts.iter().map(|d|json!({"path":d.path,"baseline":d.baseline,"text":d.text,"cursor":d.cursor})).collect::<Vec<_>>()})).map_err(io::Error::other)
    }
    pub fn read(path: &Path) -> io::Result<Self> {
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > MAX_FILE as u64
        {
            return Err(io::Error::other(
                "Recovery requires a bounded regular file, not a symlink",
            ));
        }
        let mut bytes = Vec::new();
        fs::File::open(path)?
            .take((MAX_FILE + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_FILE {
            return Err(io::Error::other("Recovery file quota exceeded"));
        }
        let v: Value = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        if v["format"].as_u64() != Some(1) {
            return Err(io::Error::other("Unsupported recovery format"));
        }
        let root = serde_json::from_value(v["root"].clone()).map_err(io::Error::other)?;
        let a = v["drafts"]
            .as_array()
            .ok_or_else(|| io::Error::other("Missing recovery drafts"))?;
        if a.len() > 32 {
            return Err(io::Error::other("Recovery tab quota exceeded"));
        }
        let mut drafts = Vec::new();
        for d in a {
            drafts.push(Draft {
                path: serde_json::from_value(d["path"].clone()).map_err(io::Error::other)?,
                baseline: d["baseline"]
                    .as_str()
                    .ok_or_else(|| io::Error::other("Missing baseline"))?
                    .into(),
                text: d["text"]
                    .as_str()
                    .ok_or_else(|| io::Error::other("Missing draft text"))?
                    .into(),
                cursor: d["cursor"]
                    .as_u64()
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or_else(|| io::Error::other("Invalid draft cursor"))?,
            })
        }
        let s = Self { root, drafts };
        s.validate()?;
        Ok(s)
    }
    /// Build buffers only. Missing targets are recoverable but cannot be saved implicitly.
    pub fn prepare(&self, root: &Path) -> io::Result<Vec<EditorTab>> {
        self.validate()?;
        if root != self.root || root.canonicalize()? != self.root {
            return Err(io::Error::other(
                "Open the original recovery workspace first",
            ));
        }
        let mut tabs = Vec::new();
        for d in &self.drafts {
            if fs::symlink_metadata(&d.path)
                .is_ok_and(|m| !m.is_file() || m.file_type().is_symlink())
            {
                return Err(io::Error::other("Recovery target is not a regular file"));
            }
            match d.path.canonicalize() {
                Ok(p) if p == d.path && p.starts_with(root) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    let p = d
                        .path
                        .parent()
                        .ok_or_else(|| io::Error::other("Missing parent"))?
                        .canonicalize()?;
                    if !p.starts_with(root) || Some(p.as_path()) != d.path.parent() {
                        return Err(io::Error::other("Recovery parent escaped workspace"));
                    }
                }
                _ => return Err(io::Error::other("Recovery target identity changed")),
            }
            let doc = Document::from_recovery(d.path.clone(), d.baseline.clone(), d.text.clone())?;
            let mut t = EditorTab::from_document(doc);
            t.set_cursor(d.cursor, false);
            t.reconcile_dirty();
            tabs.push(t);
        }
        Ok(tabs)
    }
}
/// Commit immutable revisions in a private session directory. Last good revision survives failure.
pub fn write_revision(dir: &Path, revision: u64, snapshot: &Snapshot) -> io::Result<()> {
    let data = snapshot.encode()?;
    fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    let temp = dir.join(format!("{revision:020}.tmp"));
    let dest = dir.join(format!("{revision:020}.json"));
    let result = (|| {
        let mut o = fs::OpenOptions::new();
        o.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            o.mode(0o600);
        }
        let mut f = o.open(&temp)?;
        f.write_all(&data)?;
        f.sync_all()?;
        drop(f);
        if dest.exists() {
            return Err(io::Error::other("Recovery revision already exists"));
        }
        fs::rename(&temp, &dest)?;
        #[cfg(unix)]
        fs::File::open(dir)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result?;
    for e in fs::read_dir(dir)? {
        let p = e?.path();
        if p != dest && p.extension().is_some_and(|e| e == "json") {
            let _ = fs::remove_file(p);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "kedr-recovery-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(&p).unwrap();
            Self(p.canonicalize().unwrap())
        }
        fn workspace(&self) -> Workspace {
            fs::write(self.0.join("main.rs"), "old 🌲\r\n").unwrap();
            let mut w = Workspace::scan(&self.0).unwrap();
            w.open(self.0.join("main.rs")).unwrap();
            w
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn snapshot_round_trip_and_restore_keep_disk_and_baseline() {
        let f = Fixture::new();
        let mut w = f.workspace();
        w.active_mut().unwrap().insert("draft ").unwrap();
        let s = Snapshot::capture(&w).unwrap();
        let j = f.0.join("journal");
        write_revision(&j, 1, &s).unwrap();
        let loaded = Snapshot::read(&j.join("00000000000000000001.json")).unwrap();
        fs::write(f.0.join("main.rs"), "external").unwrap();
        let mut tabs = loaded.prepare(&f.0).unwrap();
        assert_eq!(tabs[0].document.buffer.to_text(), "draft old 🌲\r\n");
        assert!(tabs[0].document.is_dirty());
        assert!(tabs[0].save().is_err());
        assert_eq!(fs::read_to_string(f.0.join("main.rs")).unwrap(), "external");
    }
    #[test]
    fn missing_target_recovered_in_memory_not_recreated() {
        let f = Fixture::new();
        let mut w = f.workspace();
        w.active_mut().unwrap().insert("draft").unwrap();
        let s = Snapshot::capture(&w).unwrap();
        fs::remove_file(f.0.join("main.rs")).unwrap();
        let mut t = s.prepare(&f.0).unwrap();
        assert!(t[0].save().is_err());
        assert!(!f.0.join("main.rs").exists());
    }
    #[test]
    fn failed_stage_preserves_last_snapshot_and_clean_tombstone_wins() {
        let f = Fixture::new();
        let mut w = f.workspace();
        w.active_mut().unwrap().insert("draft").unwrap();
        let s = Snapshot::capture(&w).unwrap();
        let j = f.0.join("journal");
        write_revision(&j, 1, &s).unwrap();
        fs::write(j.join("00000000000000000002.tmp"), "collision").unwrap();
        assert!(write_revision(&j, 2, &s).is_err());
        assert_eq!(
            Snapshot::read(&j.join("00000000000000000001.json"))
                .unwrap()
                .drafts
                .len(),
            1
        );
        write_revision(
            &j,
            3,
            &Snapshot {
                root: f.0.clone(),
                drafts: Vec::new(),
            },
        )
        .unwrap();
        assert!(
            Snapshot::read(&j.join("00000000000000000003.json"))
                .unwrap()
                .drafts
                .is_empty()
        );
        assert!(!j.join("00000000000000000001.json").exists());
    }
    #[test]
    fn malformed_version_traversal_duplicates_and_quotas_rejected() {
        let f = Fixture::new();
        let mut w = f.workspace();
        w.active_mut().unwrap().insert("draft").unwrap();
        let mut s = Snapshot::capture(&w).unwrap();
        s.drafts[0].path = f.0.join("../outside");
        assert!(s.validate().is_err());
        s.drafts[0].path = f.0.join("main.rs");
        s.drafts.push(s.drafts[0].clone());
        assert!(s.validate().is_err());
        s.drafts.pop();
        s.drafts[0].text = "x".repeat(MAX_BYTES);
        assert!(s.validate().is_err());
        let p = f.0.join("bad.json");
        fs::write(&p, r#"{"format":99,"drafts":[]}"#).unwrap();
        assert!(Snapshot::read(&p).is_err());
    }
    #[test]
    fn guard_rejects_edit_undo_save_close_and_workspace_change() {
        let f = Fixture::new();
        let mut w = f.workspace();
        let g = Guard::capture(&w);
        assert!(g.matches(&w));
        w.active_mut().unwrap().insert("x").unwrap();
        assert!(!g.matches(&w));
        w.active_mut().unwrap().undo(false).unwrap();
        assert!(!g.matches(&w));
        let g = Guard::capture(&w);
        w.active_mut().unwrap().save().unwrap();
        assert!(!g.matches(&w));
        let g = Guard::capture(&w);
        w.discard(0);
        w.open(f.0.join("main.rs")).unwrap();
        assert!(!g.matches(&w));
        let other = Fixture::new();
        assert!(!g.matches(&other.workspace()));
    }
    #[cfg(unix)]
    #[test]
    fn escaped_symlink_and_snapshot_symlink_rejected() {
        use std::os::unix::fs::symlink;
        let f = Fixture::new();
        let other = Fixture::new();
        let mut w = f.workspace();
        w.active_mut().unwrap().insert("draft").unwrap();
        let s = Snapshot::capture(&w).unwrap();
        fs::remove_file(f.0.join("main.rs")).unwrap();
        fs::write(other.0.join("external"), "outside").unwrap();
        symlink(other.0.join("external"), f.0.join("main.rs")).unwrap();
        assert!(s.prepare(&f.0).is_err());
        symlink(other.0.join("external"), f.0.join("snapshot.json")).unwrap();
        assert!(Snapshot::read(&f.0.join("snapshot.json")).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn private_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let f = Fixture::new();
        let mut w = f.workspace();
        w.active_mut().unwrap().insert("draft").unwrap();
        let dir = f.0.join("journal");
        write_revision(&dir, 1, &Snapshot::capture(&w).unwrap()).unwrap();
        assert_eq!(
            fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(dir.join("00000000000000000001.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    #[test]
    fn recovery_crash_child() {
        let Some(root) = std::env::var_os("KEDR_RECOVERY_TEST_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        let s = Snapshot {
            root: root.clone(),
            drafts: vec![Draft {
                path: root.join("main.rs"),
                baseline: "disk".into(),
                text: "recovered 🌲\r\n".into(),
                cursor: 0,
            }],
        };
        write_revision(&root.join("journal"), 1, &s).unwrap();
        loop {
            std::thread::sleep(Duration::from_secs(1))
        }
    }
    #[test]
    fn committed_drafts_survive_process_termination() {
        let f = Fixture::new();
        fs::write(f.0.join("main.rs"), "disk").unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "recovery::tests::recovery_crash_child",
                "--nocapture",
            ])
            .env("KEDR_RECOVERY_TEST_ROOT", &f.0)
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let p = f.0.join("journal/00000000000000000001.json");
        let at = Instant::now();
        while !p.exists() && at.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(10))
        }
        let _ = child.kill();
        let _ = child.wait();
        assert!(p.exists(), "child did not checkpoint");
        let s = Snapshot::read(&p).unwrap();
        assert_eq!(
            s.prepare(&f.0).unwrap()[0].document.buffer.to_text(),
            "recovered 🌲\r\n"
        );
        assert_eq!(fs::read_to_string(f.0.join("main.rs")).unwrap(), "disk");
    }
}
