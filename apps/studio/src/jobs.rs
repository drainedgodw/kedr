use kedr_core::document::Document;
use kedr_workbench::{DiskState, FileQuery, SearchReport, Workspace, search_project};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
};
pub enum Request {
    Scan(PathBuf, u64),
    Open(PathBuf, u64),
    Probe(FileQuery, u64),
    Reload(FileQuery, u64),
    Search(PathBuf, Vec<PathBuf>, String, u64, u64),
}
pub enum Response {
    Scanned(u64, Result<Workspace, String>),
    Opened(u64, Result<Document, String>),
    Probed(u64, FileQuery, DiskState),
    Reloaded(u64, FileQuery, Result<Document, String>),
    Searched(u64, u64, SearchReport),
}
pub struct Jobs {
    pub send: SyncSender<Request>,
    pub receive: Receiver<Response>,
    pub generation: Arc<AtomicU64>,
    pub search_generation: Arc<AtomicU64>,
}
impl Jobs {
    pub fn new(ctx: eframe::egui::Context) -> Self {
        let (send, requests) = mpsc::sync_channel(8);
        let (results, receive) = mpsc::sync_channel(8);
        let generation = Arc::new(AtomicU64::new(0));
        let current = generation.clone();
        let search_generation = Arc::new(AtomicU64::new(0));
        let search_current = search_generation.clone();
        std::thread::spawn(move || {
            let mut root = None;
            while let Ok(request) = requests.recv() {
                let id = match &request {
                    Request::Scan(_, id)
                    | Request::Open(_, id)
                    | Request::Probe(_, id)
                    | Request::Reload(_, id)
                    | Request::Search(_, _, _, id, _) => *id,
                };
                if current.load(Ordering::Relaxed) != id {
                    continue;
                }
                let result = match request {
                    Request::Scan(path, id) => {
                        let workspace = Workspace::scan(path).map_err(|e| e.to_string());
                        if let Ok(w) = &workspace {
                            root = Some(w.project.root.clone());
                        }
                        Response::Scanned(id, workspace)
                    }
                    Request::Open(path, id) => {
                        let opened = (|| {
                            let canonical =
                                std::fs::canonicalize(path).map_err(|e| e.to_string())?;
                            if !root.as_ref().is_some_and(|r| canonical.starts_with(r)) {
                                return Err("File is outside the workspace".into());
                            }
                            Document::open(canonical).map_err(|e| e.to_string())
                        })();
                        Response::Opened(id, opened)
                    }
                    Request::Probe(query, id) => {
                        let state = query.probe();
                        Response::Probed(id, query, state)
                    }
                    Request::Reload(query, id) => {
                        let result = query.read().map_err(|e| e.to_string());
                        Response::Reloaded(id, query, result)
                    }
                    Request::Search(root, files, query, id, search_id) => Response::Searched(
                        id,
                        search_id,
                        search_project(&root, &files, &query, search_id, &search_current),
                    ),
                };
                if results.send(result).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self {
            send,
            receive,
            generation,
            search_generation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn background_probe_and_reload_read_real_external_contents_without_writing() {
        let root = std::env::temp_dir().join(format!("kedr-disk-jobs-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let path = root.join("a.rs");
        std::fs::write(&path, "original").unwrap();
        let jobs = Jobs::new(eframe::egui::Context::default());
        jobs.send.send(Request::Scan(root.clone(), 0)).unwrap();
        let Response::Scanned(0, Ok(mut w)) = jobs
            .receive
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
        else {
            panic!()
        };
        w.open(&path).unwrap();
        w.active_mut().unwrap().insert("ours").unwrap();
        let query = FileQuery::capture(&w).unwrap();
        std::fs::write(&path, "external 🌲\r\n").unwrap();
        jobs.send.send(Request::Probe(query.clone(), 0)).unwrap();
        let Response::Probed(0, _, state) = jobs
            .receive
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(state, DiskState::Changed);
        jobs.send.send(Request::Reload(query, 0)).unwrap();
        let Response::Reloaded(0, _, Ok(d)) = jobs
            .receive
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(d.buffer.to_text(), "external 🌲\r\n");
        assert!(
            w.active()
                .unwrap()
                .document
                .buffer
                .to_text()
                .starts_with("ours")
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "external 🌲\r\n");
        drop(jobs);
        std::fs::remove_dir_all(root).unwrap();
    }
}
