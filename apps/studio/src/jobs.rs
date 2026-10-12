use kedr_core::document::Document;
use kedr_workbench::{SearchReport, Workspace, search_project};
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
    Search(PathBuf, Vec<PathBuf>, String, u64, u64),
}
pub enum Response {
    Scanned(u64, Result<Workspace, String>),
    Opened(u64, Result<Document, String>),
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
