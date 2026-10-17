//! Bounded Cargo JSON diagnostics. Positions refer to saved source, not live buffers.
use std::path::{Component, Path, PathBuf};

pub const MAX_RECORD: usize = 1024 * 1024;
const MAX_DIAGNOSTICS: usize = 1000;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLocation {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
}
impl DiagnosticLocation {
    /// Resolve only existing workspace files; generated/external sources stay non-navigable.
    pub fn resolve(&self, root: &Path) -> Option<PathBuf> {
        if self.line == 0 || self.column == 0 {
            return None;
        }
        let path = if self.file.is_absolute() {
            self.file.clone()
        } else {
            if self
                .file
                .components()
                .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
            {
                return None;
            }
            root.join(&self.file)
        };
        let path = path.canonicalize().ok()?;
        (path.starts_with(root.canonicalize().ok()?) && path.is_file()).then_some(path)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: String,
    pub code: Option<String>,
    pub message: String,
    pub location: Option<DiagnosticLocation>,
}
#[derive(Debug, Clone, Default)]
pub struct DiagnosticReport {
    pub items: Vec<Diagnostic>,
    pub limited: bool,
}
impl DiagnosticReport {
    /// Returns rendered compiler text, or None for an ordinary console line.
    pub(crate) fn ingest(&mut self, bytes: &[u8]) -> Option<String> {
        let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
        let reason = value.get("reason")?.as_str()?;
        if reason != "compiler-message" {
            return Some(String::new());
        }
        let m = value.get("message")?;
        let rendered = m
            .get("rendered")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        let level = m.get("level").and_then(|v| v.as_str()).unwrap_or("");
        if !matches!(level, "error" | "warning") {
            return Some(rendered);
        }
        let location = m
            .get("spans")
            .and_then(|v| v.as_array())
            .and_then(|spans| {
                spans
                    .iter()
                    .find(|s| s.get("is_primary").and_then(|v| v.as_bool()) == Some(true))
            })
            .and_then(|s| {
                Some(DiagnosticLocation {
                    file: s
                        .get("file_name")?
                        .as_str()?
                        .chars()
                        .take(4096)
                        .collect::<String>()
                        .into(),
                    line: usize::try_from(s.get("line_start")?.as_u64()?).ok()?,
                    column: usize::try_from(s.get("column_start")?.as_u64()?).ok()?,
                })
            });
        let bounded = |text: &str, max: usize| text.chars().take(max).collect::<String>();
        let d = Diagnostic {
            level: level.into(),
            code: m
                .get("code")
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_str())
                .map(|s| bounded(s, 64)),
            message: m
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Compiler diagnostic")
                .chars()
                .take(4096)
                .collect(),
            location,
        };
        if !self.items.contains(&d) {
            if self.items.len() < MAX_DIAGNOSTICS {
                self.items.push(d);
            } else {
                self.limited = true;
            }
        }
        Some(rendered)
    }
}
/// Frames arbitrary read boundaries before UTF-8/JSON decoding. Oversized records are discarded.
#[derive(Default)]
pub(crate) struct Records {
    bytes: Vec<u8>,
    skipping: bool,
}
impl Records {
    pub fn feed(&mut self, bytes: &[u8], mut emit: impl FnMut(&[u8])) -> bool {
        let mut limited = false;
        for &byte in bytes {
            if byte == b'\n' {
                if !self.skipping {
                    emit(&self.bytes);
                }
                self.bytes.clear();
                self.skipping = false;
            } else if !self.skipping {
                if self.bytes.len() == MAX_RECORD {
                    self.bytes.clear();
                    self.skipping = true;
                    limited = true;
                } else {
                    self.bytes.push(byte);
                }
            }
        }
        limited
    }
    pub fn finish(&mut self, mut emit: impl FnMut(&[u8])) {
        if !self.skipping && !self.bytes.is_empty() {
            emit(&self.bytes);
        }
        self.bytes.clear();
        self.skipping = false;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn message(file: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"reason":"compiler-message", "message":{
            "level":"error","message":"не найдено 🌲","code":{"code":"E0425"},
            "rendered":"error: не найдено 🌲\n", "spans":[{"is_primary":true,"file_name":file,"line_start":3,"column_start":5}]
        }})).unwrap()
    }
    #[test]
    fn split_utf8_and_json_are_preserved() {
        let mut bytes = message("src/main.rs");
        bytes.push(b'\n');
        let mut records = Records::default();
        let mut report = DiagnosticReport::default();
        for b in bytes {
            records.feed(&[b], |line| {
                report.ingest(line);
            });
        }
        assert_eq!(report.items.len(), 1);
        assert_eq!(report.items[0].message, "не найдено 🌲");
        assert_eq!(report.items[0].location.as_ref().unwrap().column, 5);
    }
    #[test]
    fn duplicates_non_diagnostics_and_malformed_records() {
        let mut report = DiagnosticReport::default();
        report.ingest(&message("src/main.rs"));
        report.ingest(&message("src/main.rs"));
        assert_eq!(report.items.len(), 1);
        assert_eq!(report.ingest(b"ordinary stderr"), None);
        assert_eq!(
            report.ingest(br#"{"reason":"build-finished","success":false}"#),
            Some(String::new())
        );
    }
    #[test]
    fn diagnostics_are_bounded() {
        let mut report = DiagnosticReport::default();
        for i in 0..1002 {
            report.ingest(&message(&format!("src/{i}.rs")));
        }
        assert_eq!(report.items.len(), 1000);
        assert!(report.limited);
    }
    #[test]
    fn oversize_record_recovers_and_eof_is_flushed() {
        let mut records = Records::default();
        let mut lines = Vec::new();
        assert!(records.feed(&vec![b'x'; MAX_RECORD + 1], |_| panic!()));
        records.feed(b"\nvalid", |line| lines.push(line.to_vec()));
        records.finish(|line| lines.push(line.to_vec()));
        assert_eq!(lines, vec![b"valid".to_vec()]);
    }
    #[test]
    fn locations_reject_parent_traversal_and_missing_files() {
        let root = std::env::current_dir().unwrap();
        for file in ["../Cargo.toml", "does-not-exist.rs"] {
            assert!(
                DiagnosticLocation {
                    file: file.into(),
                    line: 1,
                    column: 1
                }
                .resolve(&root)
                .is_none()
            );
        }
        assert!(
            DiagnosticLocation {
                file: "Cargo.toml".into(),
                line: 1,
                column: 1
            }
            .resolve(&root)
            .is_some()
        );
    }
    #[cfg(unix)]
    #[test]
    fn locations_reject_external_symlinks() {
        let dir = std::env::temp_dir().join(format!("kedr-diagnostic-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let link = dir.join("outside");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink("/etc/passwd", &link).unwrap();
        assert!(
            DiagnosticLocation {
                file: "outside".into(),
                line: 1,
                column: 1
            }
            .resolve(&dir)
            .is_none()
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
