use super::*;
use serde_json::json;
use std::{
    io::BufReader,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread,
    time::{Duration, Instant},
};
/// Each query gets an isolated short-lived server. Full didOpen overlays carry current unsaved
/// Rust buffers; no stale cross-request server state, no background project execution on startup.
pub struct DefinitionRunner {
    pub query: DefinitionQuery,
    cancel: Arc<AtomicBool>,
    result: Receiver<Result<Option<DefinitionOutcome>, String>>,
}
impl DefinitionRunner {
    /// The caller must first obtain explicit authorization to execute rust-analyzer/project code.
    pub fn start(query: DefinitionQuery, wake: impl Fn() + Send + 'static) -> Self {
        Self::start_command(
            query,
            Command::new("rust-analyzer"),
            Duration::from_secs(30),
            wake,
        )
    }
    fn start_command(
        query: DefinitionQuery,
        mut command: Command,
        timeout: Duration,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let stopped = cancel.clone();
        let (send, result) = mpsc::sync_channel(1);
        let request = query.clone();
        thread::spawn(move || {
            let outcome = run(&request, &mut command, timeout, &stopped);
            let _ = send.send(outcome);
            wake();
        });
        Self {
            query,
            cancel,
            result,
        }
    }
    pub fn poll(&self) -> Option<Result<Option<DefinitionOutcome>, String>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("Language worker disconnected".into()))
            }
        }
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
impl Drop for DefinitionRunner {
    fn drop(&mut self) {
        self.cancel();
    }
}
struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn run(
    query: &DefinitionQuery,
    command: &mut Command,
    timeout: Duration,
    cancel: &AtomicBool,
) -> Result<Option<DefinitionOutcome>, String> {
    let targets = request(query, command, timeout, cancel, false)?;
    let outcome = if let Some(target) = targets.first() {
        let document = if query.documents.iter().any(|d| d.path == target.path) {
            None
        } else {
            Some(Document::open(&target.path).map_err(|e| e.to_string())?)
        };
        Some(DefinitionOutcome {
            target: target.clone(),
            document,
            alternatives: targets.len() - 1,
        })
    } else {
        None
    };
    Ok(outcome)
}
pub(super) fn request(
    query: &DefinitionQuery,
    command: &mut Command,
    timeout: Duration,
    cancel: &AtomicBool,
    references: bool,
) -> Result<Vec<DefinitionTarget>, String> {
    let deadline = Instant::now() + timeout;
    if cancel.load(Ordering::Relaxed) {
        return Err("Language request cancelled".into());
    }
    // Executable resolution and PATH are user-controlled; this is explicit execution, not a sandbox.
    command
        .current_dir(&query.root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut process = Process(command.spawn().map_err(|e| {
        format!("Cannot start rust-analyzer: {e}. Install it and make it available on PATH.")
    })?);
    let mut stdin = process.0.stdin.take().unwrap();
    let stdout = process.0.stdout.take().unwrap();
    let (out, outgoing) = mpsc::sync_channel::<Value>(64);
    let (incoming, input) = mpsc::sync_channel(8);
    let errors = incoming.clone();
    thread::spawn(move || {
        while let Ok(v) = outgoing.recv() {
            if let Err(e) = write_message(&mut stdin, &v) {
                let _ = errors.send(Err(e.to_string()));
                break;
            }
        }
    });
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let message = read_message(&mut reader).map_err(|e| e.to_string());
            let failed = message.is_err();
            if incoming.send(message).is_err() || failed {
                break;
            }
        }
    });
    send(
        &out,
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":std::process::id(),"rootUri":file_uri(&query.root)?,"capabilities":{"experimental":{"serverStatusNotification":true},"general":{"positionEncodings":["utf-16"]},"textDocument":{"definition":{"linkSupport":true},"references":{}}},"initializationOptions":{"checkOnSave":false,"cargo":{"buildScripts":{"enable":false}},"procMacro":{"enable":false}}}}),
    )?;
    let initialized = response(&input, &out, 1, deadline, cancel)?;
    if initialized["capabilities"]
        .get("positionEncoding")
        .is_some_and(|v| v != "utf-16")
    {
        return Err("Language server selected an unsupported position encoding".into());
    }
    if !initialized["capabilities"]
        .get(if references {
            "referencesProvider"
        } else {
            "definitionProvider"
        })
        .is_some_and(|v| v == true || v.is_object())
    {
        return Err(format!(
            "Language server does not advertise {}",
            if references {
                "Find usages"
            } else {
                "Go to definition"
            }
        ));
    }
    send(
        &out,
        json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    )?;
    for doc in &query.documents {
        send(
            &out,
            json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":file_uri(&doc.path)?,"languageId":"rust","version":1,"text":doc.text}}}),
        )?;
    }
    // rust-analyzer reports quiescence after workspace loading; do not race initial indexing.
    response(&input, &out, 0, deadline, cancel)?;
    let uri = file_uri(&query.source)?;
    let mut params = json!({"textDocument":{"uri":uri},"position":{"line":query.line,"character":query.character}});
    if references {
        params["context"] = json!({"includeDeclaration":false});
    }
    send(
        &out,
        json!({"jsonrpc":"2.0","id":2,"method":if references { "textDocument/references" } else { "textDocument/definition" },"params":params}),
    )?;
    let value = response(&input, &out, 2, deadline, cancel)?;
    // Best-effort orderly shutdown. Process guard still kills/reaps a non-cooperating child.
    send(
        &out,
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
    )?;
    if response(
        &input,
        &out,
        3,
        deadline.min(Instant::now() + Duration::from_millis(250)),
        cancel,
    )
    .is_ok()
    {
        let _ = send(&out, json!({"jsonrpc":"2.0","method":"exit"}));
    }
    if references && !value.is_null() && !value.is_array() {
        return Err("References response must be an array or null".into());
    }
    definition_targets(&value, query)
}
fn send(out: &SyncSender<Value>, value: Value) -> Result<(), String> {
    out.try_send(value)
        .map_err(|_| "Language server output queue is unavailable or full".into())
}
fn response(
    input: &Receiver<Result<Value, String>>,
    out: &SyncSender<Value>,
    id: u64,
    deadline: Instant,
    cancel: &AtomicBool,
) -> Result<Value, String> {
    let mut received = 0;
    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = send(
                out,
                json!({"jsonrpc":"2.0","method":"$/cancelRequest","params":{"id":id}}),
            );
            return Err("Language request cancelled".into());
        }
        if Instant::now() >= deadline {
            return Err("Language server timed out".into());
        }
        let v = match input.recv_timeout(Duration::from_millis(20)) {
            Ok(v) => v?,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(_) => return Err("Language server disconnected".into()),
        };
        received += 1;
        if received > 4096 {
            return Err("Language server exceeded the message budget".into());
        }
        if let Some(method) = v.get("method").and_then(Value::as_str) {
            if id == 0 && method == "experimental/serverStatus" && v["params"]["quiescent"] == true
            {
                return Ok(Value::Null);
            }
            if let Some(request_id) = v.get("id") {
                // No server-initiated edits/commands/configuration side effects are allowed.
                let reply = match method {
                    "workspace/configuration" => {
                        json!({"result":vec![Value::Null;v["params"]["items"].as_array().map_or(0,Vec::len).min(128)]})
                    }
                    "window/workDoneProgress/create" => json!({"result":null}),
                    _ => {
                        json!({"error":{"code":-32601,"message":"Server request is not supported by this adapter"}})
                    }
                };
                let mut reply = reply;
                reply["jsonrpc"] = json!("2.0");
                reply["id"] = request_id.clone();
                send(out, reply)?;
            }
            continue;
        }
        if v["id"].as_u64() != Some(id) {
            continue;
        }
        if let Some(e) = v.get("error") {
            return Err(format!(
                "Language server error: {}",
                e["message"]
                    .as_str()
                    .unwrap_or("Unknown error")
                    .chars()
                    .take(512)
                    .collect::<String>()
            ));
        }
        return v
            .get("result")
            .cloned()
            .ok_or_else(|| "Language response has no result".into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn query() -> DefinitionQuery {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/studio");
        let mut w = Workspace::scan(root).unwrap();
        w.open(w.project.root.join("src/main.rs")).unwrap();
        w.active_mut()
            .unwrap()
            .document
            .buffer
            .replace(0..0, "// unsaved LSP overlay\n")
            .unwrap();
        DefinitionQuery::capture(&w).unwrap()
    }
    fn fixture(mode: &str, timeout: Duration) -> DefinitionRunner {
        let mut cmd = Command::new(if cfg!(windows) { "python" } else { "python3" });
        cmd.arg("-u")
            .arg("-c")
            .arg(include_str!("fake_server.py"))
            .arg(mode);
        DefinitionRunner::start_command(query(), cmd, timeout, || {})
    }
    fn wait(r: &DefinitionRunner) -> Result<Option<DefinitionOutcome>, String> {
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            if let Some(result) = r.poll() {
                return result;
            }
            assert!(Instant::now() < deadline, "Runner failed to finish");
            thread::sleep(Duration::from_millis(10));
        }
    }
    #[test]
    fn protocol_fixture_receives_unsaved_overlay_and_rejects_server_edits() {
        let r = fixture("ok", Duration::from_secs(5));
        let outcome = wait(&r).unwrap().unwrap();
        assert_eq!(outcome.target.path, r.query.source);
        assert_eq!(outcome.target.line, 1);
        assert!(outcome.document.is_none());
    }
    #[test]
    fn protocol_fixture_handles_null_errors_encoding_and_malformed_frames() {
        assert!(
            wait(&fixture("null", Duration::from_secs(5)))
                .unwrap()
                .is_none()
        );
        let error = wait(&fixture("large-error", Duration::from_secs(5))).unwrap_err();
        assert!(error.len() < 600);
        for (mode, needle) in [
            ("error", "Fixture failure"),
            ("encoding", "encoding"),
            ("malformed", "Content-Length"),
        ] {
            assert!(
                wait(&fixture(mode, Duration::from_secs(5)))
                    .unwrap_err()
                    .contains(needle)
            );
        }
    }
    #[test]
    fn deadline_and_cancel_stop_a_nonresponding_server() {
        let r = fixture("hang", Duration::from_millis(150));
        assert!(wait(&r).unwrap_err().contains("timed out"));
        assert!(
            wait(&fixture("index-hang", Duration::from_millis(150)))
                .unwrap_err()
                .contains("timed out")
        );
        let r = fixture("hang", Duration::from_secs(5));
        thread::sleep(Duration::from_millis(100));
        r.cancel();
        assert!(wait(&r).unwrap_err().contains("cancelled"));
    }
    #[test]
    #[ignore = "requires explicitly installed rust-analyzer; run separately for live acceptance"]
    fn real_rust_analyzer_definition() {
        let mut q = query();
        // Resolve Garden::new in the real two-file Cargo fixture, not fixture output.
        let text = &q.documents[0].text;
        let at = text.find("Garden::new").unwrap() + 1;
        q.cursor = at;
        let buffer = kedr_core::TextBuffer::new(text.clone());
        (q.line, q.character) = buffer
            .byte_to_position_in(at, kedr_core::PositionEncoding::Utf16)
            .unwrap();
        // Target is unsaved too: its two inserted lines must be reflected in the response.
        let mut target = Document::open(q.root.join("src/garden.rs")).unwrap();
        target
            .buffer
            .replace(0..0, "// unsaved target 🌲\n// second line\n")
            .unwrap();
        let text = target.buffer.to_text();
        let name = text.find("struct Garden").unwrap() + "struct ".len();
        let expected = target
            .buffer
            .byte_to_position_in(name, kedr_core::PositionEncoding::Utf16)
            .unwrap();
        q.documents.push(OpenDocument {
            path: target.path,
            version: target.buffer.version(),
            text,
        });
        let r = DefinitionRunner::start(q, || {});
        let deadline = Instant::now() + Duration::from_secs(35);
        let outcome = loop {
            if let Some(result) = r.poll() {
                break result
                    .unwrap()
                    .expect("No definition returned by real rust-analyzer");
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(30));
        };
        assert!(outcome.target.path.ends_with("src/garden.rs"));
        assert!(outcome.document.is_none());
        assert_eq!((outcome.target.line, outcome.target.character), expected);
    }
}
