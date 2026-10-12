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
#[derive(Debug, Clone, Copy)]
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
            Self::Check => &["check", "--message-format=short"],
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
type State = Arc<Mutex<(TaskPhase, VecDeque<String>)>>;
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
        let state = Arc::new(Mutex::new((TaskPhase::Running, VecDeque::new())));
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
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    state.lock().unwrap().0 = TaskPhase::Error(e.to_string());
                    wake();
                    return;
                }
            };
            fn pipe(
                mut input: impl Read + Send + 'static,
                state: State,
                wake: Arc<dyn Fn() + Send + Sync>,
            ) {
                thread::spawn(move || {
                    let mut buf = [0u8; 4096];
                    while let Ok(n) = input.read(&mut buf) {
                        if n == 0 {
                            break;
                        }
                        let mut lock = state.lock().unwrap();
                        if lock.1.len() == 64 {
                            lock.1.pop_front();
                        }
                        lock.1
                            .push_back(String::from_utf8_lossy(&buf[..n]).replace('\u{1b}', "␛"));
                        drop(lock);
                        wake();
                    }
                });
            }
            pipe(child.stdout.take().unwrap(), state.clone(), wake.clone());
            pipe(child.stderr.take().unwrap(), state.clone(), wake.clone());
            loop {
                if cancel.load(Ordering::Relaxed) {
                    let _ = child.kill();
                    let _ = child.wait();
                    state.lock().unwrap().0 = TaskPhase::Stopped;
                    break;
                }
                match child.try_wait() {
                    Ok(Some(s)) => {
                        state.lock().unwrap().0 = if s.success() {
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
                        state.lock().unwrap().0 = TaskPhase::Error(e.to_string());
                        break;
                    }
                }
            }
            wake();
        });
        Ok(result)
    }
    pub fn phase(&self) -> TaskPhase {
        self.state.lock().unwrap().0.clone()
    }
    pub fn output(&self) -> String {
        self.state.lock().unwrap().1.iter().cloned().collect()
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
