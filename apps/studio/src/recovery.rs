//! Separate bounded worker for local recovery; capture mode never starts it.
use kedr_workbench::{
    EditorTab,
    recovery::{Offer, Snapshot, write_revision},
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender},
    time::{SystemTime, UNIX_EPOCH},
};
pub enum Request {
    Checkpoint(Snapshot),
    Restore(usize, PathBuf),
    Forget(usize),
    Finish,
}
pub enum Response {
    Offers(Vec<Offer>),
    Restored(Result<Vec<EditorTab>, String>),
    Forgotten(usize, Result<(), String>),
    Error(String),
    Finished(Result<(), String>),
}
pub struct Recovery {
    pub send: SyncSender<Request>,
    pub receive: Receiver<Response>,
}
fn storage() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
    }
    .filter(|p| p.is_absolute())
    .map(|p| p.join("Kedr/recovery-v1"))
}
/// Ignore incomplete .tmp files. Never silently fall back from a corrupt newest revision.
fn discover(base: &Path) -> (Vec<PathBuf>, Vec<Offer>, Vec<String>) {
    let mut paths = Vec::new();
    let mut offers = Vec::new();
    let mut errors = Vec::new();
    let mut total = 0u64;
    let entries = match fs::read_dir(base) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (paths, offers, errors),
        Err(e) => return (paths, offers, vec![e.to_string()]),
    };
    for (index, entry) in entries.take(1025).enumerate() {
        if index == 1024 {
            errors.push(
                "Recovery session directory quota exceeded; remaining entries retained".into(),
            );
            break;
        }
        let result = (|| -> Result<(), String> {
            let e = entry.map_err(|e| e.to_string())?;
            if !e.file_type().map_err(|e| e.to_string())?.is_dir() {
                return Ok(());
            }
            let mut latest = None;
            let mut count = 0;
            for f in fs::read_dir(e.path()).map_err(|e| e.to_string())? {
                count += 1;
                if count > 128 {
                    return Err("Too many recovery revisions; directory retained".into());
                }
                let f = f.map_err(|e| e.to_string())?;
                let p = f.path();
                let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                if p.extension().is_some_and(|x| x == "json")
                    && stem.len() == 20
                    && stem.bytes().all(|b| b.is_ascii_digit())
                    && latest.as_ref().is_none_or(|old: &PathBuf| p > *old)
                {
                    latest = Some(p)
                }
            }
            let Some(path) = latest else { return Ok(()) };
            let n = fs::symlink_metadata(&path)
                .map_err(|e| e.to_string())?
                .len();
            total = total.saturating_add(n);
            if total > 64 * 1024 * 1024 || paths.len() >= 32 {
                return Err("Recovery scan quota exceeded; snapshots retained".into());
            }
            let s = Snapshot::read(&path)
                .map_err(|e| format!("Recovery snapshot retained but unreadable: {e}"))?;
            if !s.drafts.is_empty() {
                let id = paths.len();
                offers.push(Offer {
                    id,
                    root: s.root,
                    count: s.drafts.len(),
                });
                paths.push(path)
            }
            Ok(())
        })();
        if let Err(e) = result {
            errors.push(e)
        }
    }
    (paths, offers, errors)
}
/// Delete selected and retired older revisions, newest last. Otherwise a failed
/// cleanup during a previous commit could resurrect an explicitly deleted draft.
/// Never remove newer revisions, temporary files or unrelated directory entries.
fn forget_snapshot(path: &Path) -> Result<(), String> {
    fn numbered(p: &Path) -> bool {
        p.extension().is_some_and(|s| s == "json")
            && p.file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.len() == 20 && s.bytes().all(|b| b.is_ascii_digit()))
    }
    let dir = path.parent().ok_or("Missing recovery directory")?;
    if !numbered(path) || dir.canonicalize().map_err(|e| e.to_string())? != dir {
        return Err("Recovery directory identity changed".into());
    }
    let selected = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !selected.is_file() || selected.file_type().is_symlink() {
        return Err("Recovery snapshot is not a regular file".into());
    }
    let mut retired = Vec::new();
    for (index, entry) in fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .take(129)
        .enumerate()
    {
        if index == 128 {
            return Err("Too many recovery revisions; snapshots retained".into());
        }
        let entry = entry.map_err(|e| e.to_string())?;
        let p = entry.path();
        if numbered(&p) && p.as_path() < path {
            let m = fs::symlink_metadata(&p).map_err(|e| e.to_string())?;
            if !m.is_file() || m.file_type().is_symlink() {
                return Err(
                    "Retired recovery revision is not a regular file; selected snapshot retained"
                        .into(),
                );
            }
            retired.push(p);
        }
    }
    retired.sort();
    for p in retired {
        fs::remove_file(p).map_err(|e| {
            format!("Older revision deletion failed; selected snapshot retained: {e}")
        })?;
    }
    // Persist removal of older entries before retiring the selected revision.
    #[cfg(unix)]
    fs::File::open(dir)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    fs::remove_file(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    fs::File::open(dir)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
}
impl Recovery {
    pub fn start(ctx: eframe::egui::Context) -> Result<Self, String> {
        let base =
            storage().ok_or("No absolute local state directory; draft recovery unavailable")?;
        Self::start_at(base, ctx)
    }
    fn start_at(base: PathBuf, ctx: eframe::egui::Context) -> Result<Self, String> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let dir = base.join(format!("session-{}-{nonce}", std::process::id()));
        let (send, rx) = mpsc::sync_channel(1);
        let (tx, receive) = mpsc::channel();
        std::thread::spawn(move || {
            let (paths, offers, errors) = discover(&base);
            let _ = tx.send(Response::Offers(offers));
            for e in errors {
                let _ = tx.send(Response::Error(e));
            }
            ctx.request_repaint();
            let mut revision = 0u64;
            let mut last_root = None;
            while let Ok(request) = rx.recv() {
                let response = match request {
                    Request::Checkpoint(s) => {
                        revision += 1;
                        last_root = Some(s.root.clone());
                        match write_revision(&dir, revision, &s) {
                            Ok(()) => {
                                ctx.request_repaint();
                                continue;
                            }
                            Err(e) => Response::Error(format!(
                                "Draft checkpoint failed; last good revision retained: {e}"
                            )),
                        }
                    }
                    Request::Restore(id, root) => Response::Restored(
                        paths
                            .get(id)
                            .ok_or_else(|| "Unknown recovery snapshot".into())
                            .and_then(|p| Snapshot::read(p).map_err(|e| e.to_string()))
                            .and_then(|s| s.prepare(&root).map_err(|e| e.to_string())),
                    ),
                    Request::Forget(id) => {
                        let r = paths
                            .get(id)
                            .ok_or_else(|| "Unknown recovery snapshot".into())
                            .and_then(|p| forget_snapshot(p));
                        Response::Forgotten(id, r)
                    }
                    Request::Finish => {
                        let result = if let Some(root) = last_root.clone() {
                            revision += 1;
                            write_revision(
                                &dir,
                                revision,
                                &Snapshot {
                                    root,
                                    drafts: Vec::new(),
                                },
                            )
                            .map_err(|e| e.to_string())
                        } else {
                            Ok(())
                        };
                        let done = result.is_ok();
                        let _ = tx.send(Response::Finished(result));
                        ctx.request_repaint();
                        if done {
                            // Empty committed tombstone exists first; remove only our own session.
                            // Keep the tombstone if older revisions could not be cleaned up.
                            if let Ok(mut entries) = fs::read_dir(&dir)
                                && let Some(Ok(e)) = entries.next()
                                && entries.next().is_none()
                            {
                                let _ = fs::remove_file(e.path());
                                let _ = fs::remove_dir(&dir);
                            }
                            break;
                        } else {
                            continue;
                        }
                    }
                };
                if tx.send(response).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Ok(Self { send, receive })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn deletion_fixture() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "kedr-delete-contract-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p.canonicalize().unwrap()
    }
    #[test]
    fn forgetting_does_not_delete_newer_temporary_or_unrelated_files() {
        let p = deletion_fixture();
        for name in [
            "00000000000000000001.json",
            "00000000000000000002.json",
            "00000000000000000003.json",
            "00000000000000000001.tmp",
            "notes.json",
        ] {
            fs::write(p.join(name), "retained").unwrap();
        }
        forget_snapshot(&p.join("00000000000000000002.json")).unwrap();
        assert!(!p.join("00000000000000000001.json").exists());
        assert!(!p.join("00000000000000000002.json").exists());
        for name in [
            "00000000000000000003.json",
            "00000000000000000001.tmp",
            "notes.json",
        ] {
            assert_eq!(fs::read_to_string(p.join(name)).unwrap(), "retained");
        }
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn invalid_retired_entry_keeps_selected_snapshot_and_reports_error() {
        let p = deletion_fixture();
        fs::create_dir(p.join("00000000000000000001.json")).unwrap();
        let selected = p.join("00000000000000000002.json");
        fs::write(&selected, "selected").unwrap();
        assert!(forget_snapshot(&selected).is_err());
        assert_eq!(fs::read_to_string(selected).unwrap(), "selected");
        fs::remove_dir_all(p).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn forgetting_rejects_symlinked_session_directory() {
        let p = deletion_fixture();
        let real = p.join("real");
        fs::create_dir(&real).unwrap();
        let selected = real.join("00000000000000000001.json");
        fs::write(&selected, "selected").unwrap();
        let link = p.join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert!(forget_snapshot(&link.join("00000000000000000001.json")).is_err());
        assert_eq!(fs::read_to_string(selected).unwrap(), "selected");
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn discovery_ignores_partial_but_not_corrupt_newest() {
        let base = std::env::temp_dir().join(format!(
            "kedr-discovery-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let dir = base.join("session");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("00000000000000000001.tmp"), "partial").unwrap();
        assert!(discover(&base).2.is_empty());
        fs::write(dir.join("00000000000000000002.json"), "corrupt").unwrap();
        let (p, o, e) = discover(&base);
        assert!(p.is_empty() && o.is_empty() && !e.is_empty());
        assert!(dir.join("00000000000000000002.json").exists());
        fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn deleting_latest_snapshot_does_not_resurrect_retired_revisions() {
        use kedr_workbench::{Workspace, recovery::write_revision};
        use std::time::Duration;
        let base = std::env::temp_dir().join(format!(
            "kedr-forget-audit-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let project = base.join("project");
        fs::create_dir_all(&project).unwrap();
        let project = project.canonicalize().unwrap();
        fs::write(project.join("a.rs"), "disk").unwrap();
        let mut w = Workspace::scan(&project).unwrap();
        w.open(project.join("a.rs")).unwrap();
        w.active_mut().unwrap().insert("draft").unwrap();
        let state = base.join("state");
        let dir = state.join("old");
        write_revision(&dir, 1, &Snapshot::capture(&w).unwrap()).unwrap();
        let retired = fs::read(dir.join("00000000000000000001.json")).unwrap();
        write_revision(&dir, 2, &Snapshot::capture(&w).unwrap()).unwrap();
        // Model a failed best-effort cleanup after the newer revision committed.
        fs::write(dir.join("00000000000000000001.json"), retired).unwrap();
        let r = Recovery::start_at(state.clone(), Default::default()).unwrap();
        assert!(
            matches!(r.receive.recv_timeout(Duration::from_secs(5)).unwrap(), Response::Offers(o) if o.len()==1)
        );
        r.send.send(Request::Forget(0)).unwrap();
        assert!(matches!(
            r.receive.recv_timeout(Duration::from_secs(5)).unwrap(),
            Response::Forgotten(0, Ok(()))
        ));
        r.send.send(Request::Finish).unwrap();
        let _ = r.receive.recv_timeout(Duration::from_secs(5)).unwrap();
        let offers = discover(&state).1.len();
        fs::remove_dir_all(base).unwrap();
        assert_eq!(
            offers, 0,
            "deletion must not expose an older draft on next startup"
        );
    }
    #[test]
    fn worker_restore_delete_and_orderly_shutdown_do_not_write_project() {
        use kedr_workbench::{Workspace, recovery::write_revision};
        use std::time::Duration;
        let base = std::env::temp_dir().join(format!(
            "kedr-worker-recovery-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let project = base.join("project");
        fs::create_dir_all(&project).unwrap();
        let project = project.canonicalize().unwrap();
        fs::write(project.join("a.rs"), "disk").unwrap();
        let mut w = Workspace::scan(&project).unwrap();
        w.open(project.join("a.rs")).unwrap();
        w.active_mut().unwrap().insert("draft").unwrap();
        let state = base.join("state");
        write_revision(&state.join("old"), 1, &Snapshot::capture(&w).unwrap()).unwrap();
        let r = Recovery::start_at(state.clone(), Default::default()).unwrap();
        assert!(
            matches!(r.receive.recv_timeout(Duration::from_secs(5)).unwrap(),Response::Offers(o) if o.len()==1)
        );
        r.send.send(Request::Restore(0, project.clone())).unwrap();
        assert!(
            matches!(r.receive.recv_timeout(Duration::from_secs(5)).unwrap(),Response::Restored(Ok(t)) if t[0].document.buffer.to_text()=="draftdisk")
        );
        r.send
            .send(Request::Checkpoint(Snapshot::capture(&w).unwrap()))
            .unwrap();
        r.send.send(Request::Finish).unwrap();
        assert!(matches!(
            r.receive.recv_timeout(Duration::from_secs(5)).unwrap(),
            Response::Finished(Ok(()))
        ));
        // Only old source remains recoverable, not this cleanly finished session.
        assert_eq!(discover(&state).1.len(), 1);
        assert_eq!(fs::read_to_string(project.join("a.rs")).unwrap(), "disk");
        let r = Recovery::start_at(state.clone(), Default::default()).unwrap();
        let _ = r.receive.recv_timeout(Duration::from_secs(5)).unwrap();
        r.send.send(Request::Forget(0)).unwrap();
        assert!(matches!(
            r.receive.recv_timeout(Duration::from_secs(5)).unwrap(),
            Response::Forgotten(0, Ok(()))
        ));
        r.send.send(Request::Finish).unwrap();
        let _ = r.receive.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(discover(&state).1.is_empty());
        fs::remove_dir_all(base).unwrap();
    }
}
