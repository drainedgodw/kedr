//! Bounded semantic usages; source previews are prepared off the UI thread.
use super::*;
use std::{
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};
#[derive(Debug)]
pub struct ReferenceHit {
    pub target: DefinitionTarget,
    pub preview: String,
}
#[derive(Debug)]
pub struct ReferenceOutcome {
    pub hits: Vec<ReferenceHit>,
    pub documents: Vec<Document>,
}
pub struct ReferenceRunner {
    pub query: DefinitionQuery,
    cancel: Arc<AtomicBool>,
    result: mpsc::Receiver<Result<ReferenceOutcome, String>>,
}
impl ReferenceRunner {
    /// Requires the same explicit external-process authorization as definition lookup.
    pub fn start(query: DefinitionQuery, wake: impl Fn() + Send + 'static) -> Self {
        Self::start_command(query, Command::new("rust-analyzer"), wake)
    }
    fn start_command(
        query: DefinitionQuery,
        mut command: Command,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let stopped = cancel.clone();
        let (send, result) = mpsc::sync_channel(1);
        let request = query.clone();
        thread::spawn(move || {
            let outcome = process::request(
                &request,
                &mut command,
                Duration::from_secs(30),
                &stopped,
                true,
            )
            .and_then(|targets| prepare(&request, targets, &stopped));
            let _ = send.send(outcome);
            wake();
        });
        Self {
            query,
            cancel,
            result,
        }
    }
    pub fn poll(&self) -> Option<Result<ReferenceOutcome, String>> {
        match self.result.try_recv() {
            Ok(v) => Some(v),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(_) => Some(Err("Language worker disconnected".into())),
        }
    }
}
impl Drop for ReferenceRunner {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
fn prepare(
    query: &DefinitionQuery,
    mut targets: Vec<DefinitionTarget>,
    cancel: &AtomicBool,
) -> Result<ReferenceOutcome, String> {
    targets.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.line.cmp(&b.line))
            .then(a.character.cmp(&b.character))
    });
    targets.dedup();
    if targets.len() > 128 {
        return Err("Too many usages; narrow the workspace".into());
    }
    let mut documents = Vec::<Document>::new();
    let mut hits = Vec::new();
    let mut total = 0;
    for target in targets {
        if cancel.load(Ordering::Relaxed) {
            return Err("Language request cancelled".into());
        }
        if !query.files.contains(&target.path) || !target.path.starts_with(&query.root) {
            return Err("Usage is outside the scanned workspace".into());
        }
        let text = if let Some(d) = query.documents.iter().find(|d| d.path == target.path) {
            d.text.clone()
        } else {
            if !documents.iter().any(|d| d.path == target.path) {
                let len = std::fs::metadata(&target.path)
                    .map_err(|e| e.to_string())?
                    .len();
                if len > MAX_DOCUMENT as u64
                    || total + len > MAX_MESSAGE as u64
                    || documents.len() >= 32
                {
                    return Err(
                        "Usage previews exceed the 1 MiB/file, 4 MiB total or 32 loaded file limit"
                            .into(),
                    );
                }
                let d = Document::open(&target.path).map_err(|e| e.to_string())?;
                // Recheck actual contents: a file may grow after metadata was read.
                if d.path != target.path {
                    return Err("Usage source changed canonical path; request again".into());
                }
                let size = d.buffer.len_bytes() as u64;
                if size > MAX_DOCUMENT as u64 || total + size > MAX_MESSAGE as u64 {
                    return Err("Usage source grew beyond the preview limit".into());
                }
                total += size;
                documents.push(d);
            }
            documents
                .iter()
                .find(|d| d.path == target.path)
                .unwrap()
                .buffer
                .to_text()
        };
        let buffer = kedr_core::TextBuffer::new(text.clone());
        buffer
            .position_to_byte_in(target.line, target.character, PositionEncoding::Utf16)
            .map_err(|_| "Usage position is invalid; request again")?;
        let preview: String = text
            .lines()
            .nth(target.line)
            .unwrap_or("")
            .trim()
            .chars()
            .take(180)
            .collect();
        hits.push(ReferenceHit { target, preview });
    }
    Ok(ReferenceOutcome { hits, documents })
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::time::Instant;
    fn query() -> DefinitionQuery {
        let mut w = Workspace::scan(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/studio"),
        )
        .unwrap();
        w.open(w.project.root.join("src/main.rs")).unwrap();
        w.active_mut()
            .unwrap()
            .document
            .buffer
            .replace(0..0, "// unsaved LSP overlay\n")
            .unwrap();
        DefinitionQuery::capture(&w).unwrap()
    }
    #[test]
    fn usage_previews_deduplicate_sort_and_validate_unsaved_utf16() {
        let mut q = query();
        q.documents[0].text = "// 🌲\nfn main() {}\n".into();
        let a = DefinitionTarget {
            path: q.source.clone(),
            line: 1,
            character: 3,
        };
        let b = DefinitionTarget {
            path: q.source.clone(),
            line: 0,
            character: 5,
        };
        let result = prepare(&q, vec![a.clone(), b, a], &AtomicBool::new(false)).unwrap();
        assert_eq!(result.hits.len(), 2);
        assert_eq!(result.hits[0].preview, "// 🌲");
        assert!(result.documents.is_empty());
        assert!(
            prepare(
                &q,
                vec![DefinitionTarget {
                    path: q.source.clone(),
                    line: 0,
                    character: 4
                }],
                &AtomicBool::new(false)
            )
            .is_err()
        );
        assert!(
            prepare(
                &q,
                vec![DefinitionTarget {
                    path: q.source.clone(),
                    line: 99,
                    character: 0
                }],
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
    #[test]
    fn usage_previews_load_each_closed_source_once_and_reject_external_or_cancelled() {
        let q = query();
        let path = q.root.join("src/garden.rs");
        let targets = vec![
            DefinitionTarget {
                path: path.clone(),
                line: 0,
                character: 0,
            },
            DefinitionTarget {
                path,
                line: 1,
                character: 0,
            },
        ];
        let r = prepare(&q, targets.clone(), &AtomicBool::new(false)).unwrap();
        assert_eq!(r.documents.len(), 1);
        assert_eq!(r.hits.len(), 2);
        assert!(prepare(&q, targets, &AtomicBool::new(true)).is_err());
        assert!(
            prepare(
                &q,
                vec![DefinitionTarget {
                    path: q.root.join("../outside.rs"),
                    line: 0,
                    character: 0
                }],
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
    fn wait(r: &ReferenceRunner) -> Result<ReferenceOutcome, String> {
        let deadline = Instant::now() + Duration::from_secs(35);
        loop {
            if let Some(v) = r.poll() {
                return v;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(20));
        }
    }
    #[test]
    fn protocol_requests_references_without_declaration_and_handles_empty() {
        for mode in ["ok", "null"] {
            let mut cmd = Command::new(if cfg!(windows) { "python" } else { "python3" });
            cmd.arg("-u")
                .arg("-c")
                .arg(include_str!("fake_server.py"))
                .arg(mode);
            let r = ReferenceRunner::start_command(query(), cmd, || {});
            let result = wait(&r).unwrap();
            assert_eq!(result.hits.len(), usize::from(mode == "ok"));
        }
    }
    #[test]
    fn references_reject_unsupported_capability_shape_and_cancel() {
        for (mode, expected) in [
            ("no-references", "advertise"),
            ("references-object", "array"),
            ("hang", "cancelled"),
        ] {
            let mut cmd = Command::new(if cfg!(windows) { "python" } else { "python3" });
            cmd.arg("-u")
                .arg("-c")
                .arg(include_str!("fake_server.py"))
                .arg(mode);
            let runner = ReferenceRunner::start_command(query(), cmd, || {});
            if mode == "hang" {
                thread::sleep(Duration::from_millis(80));
                runner.cancel.store(true, Ordering::Relaxed);
            }
            assert!(wait(&runner).unwrap_err().contains(expected));
        }
    }
    #[test]
    fn usage_preview_limits_reject_oversized_file_and_result_set() {
        let mut q = query();
        let targets = (0..129)
            .map(|line| DefinitionTarget {
                path: q.source.clone(),
                line,
                character: 0,
            })
            .collect();
        assert!(
            prepare(&q, targets, &AtomicBool::new(false))
                .unwrap_err()
                .contains("Too many")
        );
        let temp = std::env::temp_dir().join(format!("kedr-usage-limit-{}", std::process::id()));
        std::fs::create_dir_all(&temp).unwrap();
        let path = temp.join("large.rs");
        std::fs::write(&path, vec![b'x'; MAX_DOCUMENT + 1]).unwrap();
        q.root = temp.canonicalize().unwrap();
        let path = path.canonicalize().unwrap();
        q.files = vec![path.clone()];
        q.documents.clear();
        let result = prepare(
            &q,
            vec![DefinitionTarget {
                path,
                line: 0,
                character: 0,
            }],
            &AtomicBool::new(false),
        );
        std::fs::remove_dir_all(temp).unwrap();
        assert!(result.unwrap_err().contains("limit"));
    }
    #[test]
    #[ignore = "requires explicitly installed rust-analyzer; run separately for live acceptance"]
    fn real_rust_analyzer_references() {
        let mut q = query();
        let text = &q.documents[0].text;
        let at = text.find("Garden::new").unwrap() + 1;
        q.cursor = at;
        (q.line, q.character) = kedr_core::TextBuffer::new(text.clone())
            .byte_to_position_in(at, PositionEncoding::Utf16)
            .unwrap();
        let r = ReferenceRunner::start(q.clone(), || {});
        let result = wait(&r).unwrap();
        let lines: BTreeSet<_> = result
            .hits
            .iter()
            .filter(|h| h.target.path == q.source)
            .map(|h| h.target.line)
            .collect();
        assert!(
            lines.contains(&q.line),
            "missing constructor usage: {result:?}"
        );
        let second = q.documents[0].text.rfind("Garden::new").unwrap();
        let second_line = kedr_core::TextBuffer::new(q.documents[0].text.clone())
            .byte_to_position_in(second, PositionEncoding::Utf16)
            .unwrap()
            .0;
        assert!(
            lines.contains(&second_line),
            "missing second constructor usage: {result:?}"
        );
        assert!(
            result
                .hits
                .iter()
                .all(|h| !h.preview.contains("pub struct Garden"))
        );
    }
}
