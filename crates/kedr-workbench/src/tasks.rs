use crate::{DiagnosticReport, diagnostics::Records};
use std::{
    collections::VecDeque,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    Check,
    Test,
}
impl TaskKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Check => "Cargo check",
            Self::Test => "Cargo test",
        }
    }
    pub fn args(self) -> &'static [&'static str] {
        match self {
            Self::Check => &["check", "--message-format=json"],
            Self::Test => &["test"],
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskPhase {
    Running,
    Success,
    Failed(Option<i32>),
    Stopped,
    Error(String),
}
struct TaskState {
    phase: TaskPhase,
    output: VecDeque<String>,
    output_bytes: usize,
    diagnostics: Arc<DiagnosticReport>,
}
impl TaskState {
    fn append(&mut self, text: &str) {
        const MAX_OUTPUT: usize = 256 * 1024;
        let mut start = text.len().saturating_sub(MAX_OUTPUT);
        while !text.is_char_boundary(start) {
            start += 1;
        }
        let text = text[start..].replace('\u{1b}', "␛");
        self.output_bytes += text.len();
        self.output.push_back(text);
        while self.output_bytes > MAX_OUTPUT || self.output.len() > 4096 {
            if let Some(old) = self.output.pop_front() {
                self.output_bytes -= old.len();
            }
        }
    }
    fn record(&mut self, line: &[u8], structured: bool) {
        match structured
            .then(|| Arc::make_mut(&mut self.diagnostics).ingest(line))
            .flatten()
        {
            Some(rendered) => self.append(&rendered),
            None => {
                self.append(&String::from_utf8_lossy(line));
                self.append("\n");
            }
        }
    }
}
type State = Arc<Mutex<TaskState>>;
pub struct TaskRunner {
    state: State,
    cancel: Arc<AtomicBool>,
    pub kind: TaskKind,
}
impl TaskRunner {
    /// Caller obtains explicit trust/confirmation. No shell interpolation or automatic startup.
    pub fn start(
        root: &Path,
        kind: TaskKind,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self, String> {
        let root = std::fs::canonicalize(root).map_err(|e| e.to_string())?;
        if !root.join("Cargo.toml").is_file() {
            return Err("This workspace has no Cargo.toml".into());
        }
        let state = Arc::new(Mutex::new(TaskState {
            phase: TaskPhase::Running,
            output: VecDeque::new(),
            output_bytes: 0,
            diagnostics: Arc::default(),
        }));
        let cancel = Arc::new(AtomicBool::new(false));
        let result = Self {
            state: state.clone(),
            cancel: cancel.clone(),
            kind,
        };
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(wake);
        thread::spawn(move || {
            let mut child = match Command::new("cargo")
                .args(kind.args())
                .current_dir(root)
                .env("CARGO_TERM_COLOR", "never")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    state.lock().unwrap().phase = TaskPhase::Error(e.to_string());
                    wake();
                    return;
                }
            };
            fn pipe(
                mut input: impl Read + Send + 'static,
                state: State,
                wake: Arc<dyn Fn() + Send + Sync>,
                structured: bool,
            ) {
                thread::spawn(move || {
                    let mut buf = [0u8; 4096];
                    let mut records = Records::default();
                    while let Ok(n) = input.read(&mut buf) {
                        if n == 0 {
                            break;
                        }
                        let mut lock = state.lock().unwrap();
                        if records.feed(&buf[..n], |line| lock.record(line, structured)) {
                            Arc::make_mut(&mut lock.diagnostics).limited = true;
                            lock.append("[Oversized output record omitted]\n");
                        }
                        drop(lock);
                        wake();
                    }
                    records.finish(|line| state.lock().unwrap().record(line, structured));
                    wake();
                });
            }
            pipe(
                child.stdout.take().unwrap(),
                state.clone(),
                wake.clone(),
                kind == TaskKind::Check,
            );
            pipe(
                child.stderr.take().unwrap(),
                state.clone(),
                wake.clone(),
                false,
            );
            loop {
                if cancel.load(Ordering::Relaxed) {
                    let _ = child.kill();
                    let _ = child.wait();
                    state.lock().unwrap().phase = TaskPhase::Stopped;
                    break;
                }
                match child.try_wait() {
                    Ok(Some(s)) => {
                        state.lock().unwrap().phase = if s.success() {
                            TaskPhase::Success
                        } else {
                            TaskPhase::Failed(s.code())
                        };
                        break;
                    }
                    Ok(None) => thread::sleep(Duration::from_millis(40)),
                    Err(e) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        state.lock().unwrap().phase = TaskPhase::Error(e.to_string());
                        break;
                    }
                }
            }
            wake();
        });
        Ok(result)
    }
    /// Deterministic screenshot fixture. Never spawns Cargo or represents a live result.
    #[doc(hidden)]
    pub fn capture_fixture() -> Self {
        let mut diagnostics = DiagnosticReport::default();
        diagnostics.ingest(br#"{"reason":"compiler-message","message":{"level":"error","message":"[Capture fixture] cannot find function `missing_name` in this scope","code":{"code":"E0425"},"rendered":"Capture fixture only: error[E0425]: missing_name\n","spans":[{"is_primary":true,"file_name":"src/main.rs","line_start":6,"column_start":1}]}}"#);
        Self {
            state: Arc::new(Mutex::new(TaskState {
                phase: TaskPhase::Failed(Some(101)),
                output: VecDeque::from(
                    ["Capture fixture only — not a live compiler run.\n".into()],
                ),
                output_bytes: "Capture fixture only — not a live compiler run.\n".len(),
                diagnostics: Arc::new(diagnostics),
            })),
            cancel: Arc::new(AtomicBool::new(false)),
            kind: TaskKind::Check,
        }
    }
    pub fn phase(&self) -> TaskPhase {
        self.state.lock().unwrap().phase.clone()
    }
    pub fn output(&self) -> String {
        self.state.lock().unwrap().output.iter().cloned().collect()
    }
    pub fn diagnostics(&self) -> Arc<DiagnosticReport> {
        self.state.lock().unwrap().diagnostics.clone()
    }
    pub fn stop(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
impl Drop for TaskRunner {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_tail_stays_bounded_with_unicode_and_escape() {
        let mut state = TaskState {
            phase: TaskPhase::Running,
            output: VecDeque::new(),
            output_bytes: 0,
            diagnostics: Arc::default(),
        };
        state.append(&"🌲".repeat(100_000));
        state.append("\u{1b}[31mend");
        assert!(state.output_bytes <= 256 * 1024);
        assert!(state.output.iter().any(|s| s.contains("␛")));
        for _ in 0..5000 {
            state.append("x");
        }
        assert!(state.output.len() <= 4096);
    }
}
