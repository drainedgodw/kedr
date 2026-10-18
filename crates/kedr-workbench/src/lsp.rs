//! On-demand, explicitly authorized LSP navigation requests. No automatic startup.
mod process;
mod references;
use crate::Workspace;
use kedr_core::{PositionEncoding, Version, document::Document};
pub use process::DefinitionRunner;
pub use references::{ReferenceHit, ReferenceOutcome, ReferenceRunner};
use serde_json::Value;
use std::{
    io::{self, BufRead, Read, Write},
    path::PathBuf,
};
use url::Url;
pub const MAX_MESSAGE: usize = 4 * 1024 * 1024;
const MAX_DOCUMENT: usize = 1024 * 1024;
#[derive(Debug, Clone)]
pub struct OpenDocument {
    pub path: PathBuf,
    pub version: Version,
    pub text: String,
}
#[derive(Debug, Clone)]
pub struct DefinitionQuery {
    pub root: PathBuf,
    pub files: Vec<PathBuf>,
    pub documents: Vec<OpenDocument>,
    pub stamps: Vec<(PathBuf, Version)>,
    pub source: PathBuf,
    pub cursor: usize,
    pub line: usize,
    pub character: usize,
}
impl DefinitionQuery {
    pub fn capture(w: &Workspace) -> Result<Self, String> {
        let tab = w.active().ok_or("Open a Rust source file first")?;
        if tab.document.path.extension().and_then(|s| s.to_str()) != Some("rs") {
            return Err("Language navigation currently supports Rust sources only".into());
        }
        let (line, character) = tab
            .document
            .buffer
            .byte_to_position_in(tab.cursor, PositionEncoding::Utf16)
            .map_err(|e| e.to_string())?;
        let mut total = 0;
        let mut documents = Vec::new();
        for t in &w.tabs {
            // Non-Rust dirty files (e.g. Cargo.toml) cannot be overlaid by this adapter.
            if t.document.path.extension().and_then(|s| s.to_str()) != Some("rs") {
                if t.document.is_dirty() {
                    return Err("Save non-Rust files before using the language server".into());
                }
                continue;
            }
            let len = t.document.buffer.len_bytes();
            total += len;
            if len > MAX_DOCUMENT || total > MAX_MESSAGE || documents.len() >= 32 {
                return Err(
                    "Language request exceeds the 1 MiB/file, 4 MiB total or 32 file limit".into(),
                );
            }
            documents.push(OpenDocument {
                path: t.document.path.clone(),
                version: t.document.buffer.version(),
                text: t.document.buffer.to_text(),
            });
        }
        Ok(Self {
            root: w.project.root.clone(),
            files: w.project.files.clone(),
            documents,
            stamps: w
                .tabs
                .iter()
                .map(|t| (t.document.path.clone(), t.document.buffer.version()))
                .collect(),
            source: tab.document.path.clone(),
            cursor: tab.cursor,
            line,
            character,
        })
    }
    /// Reject edits, undo, reopened documents, tab switches/caret movement and workspace switches.
    pub fn is_current(&self, w: &Workspace) -> bool {
        w.project.root == self.root
            && w.active()
                .is_some_and(|t| t.document.path == self.source && t.cursor == self.cursor)
            && w.tabs.len() == self.stamps.len()
            && self.stamps.iter().all(|(path, version)| {
                w.tabs
                    .iter()
                    .any(|t| t.document.path == *path && t.document.buffer.version() == *version)
            })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionTarget {
    pub path: PathBuf,
    pub line: usize,
    pub character: usize,
}
#[derive(Debug)]
pub struct DefinitionOutcome {
    pub target: DefinitionTarget,
    /// Loaded off the UI thread only when the target was not already open.
    pub document: Option<Document>,
    pub alternatives: usize,
}
pub fn file_uri(path: &std::path::Path) -> Result<String, String> {
    Url::from_file_path(path)
        .map(|u| u.to_string())
        .map_err(|_| "Cannot encode local file URI".into())
}
fn position(v: &Value) -> Result<(usize, usize), String> {
    let n = |name| {
        v.get(name)
            .and_then(Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| format!("Invalid definition {name}"))
    };
    Ok((n("line")?, n("character")?))
}
pub fn definition_targets(
    value: &Value,
    query: &DefinitionQuery,
) -> Result<Vec<DefinitionTarget>, String> {
    if value.is_null() {
        return Ok(Vec::new());
    }
    let values = match value.as_array() {
        Some(v) => v.clone(),
        None => vec![value.clone()],
    };
    if values.len() > 128 {
        return Err("Language server returned too many definitions".into());
    }
    let mut targets = Vec::new();
    for v in values {
        let link = v.get("targetUri").is_some();
        let uri = v
            .get(if link { "targetUri" } else { "uri" })
            .and_then(Value::as_str)
            .ok_or("Definition has no file URI")?;
        let uri = Url::parse(uri).map_err(|e| e.to_string())?;
        if uri.scheme() != "file"
            || uri.query().is_some()
            || uri.fragment().is_some()
            || uri
                .host_str()
                .is_some_and(|h| !h.is_empty() && h != "localhost")
        {
            return Err("Definition URI is not a local file".into());
        }
        let path = uri
            .to_file_path()
            .map_err(|_| "Invalid definition file URI")?;
        let path = path.canonicalize().map_err(|e| e.to_string())?;
        if !path.starts_with(&query.root) || !query.files.contains(&path) {
            return Err("Definition is outside the scanned workspace".into());
        }
        let range = v
            .get(if link {
                "targetSelectionRange"
            } else {
                "range"
            })
            .ok_or("Definition has no range")?;
        let (line, character) = position(&range["start"])?;
        let end = position(&range["end"])?;
        if end < (line, character) {
            return Err("Invalid reversed definition range".into());
        }
        let target = DefinitionTarget {
            path,
            line,
            character,
        };
        if !targets.contains(&target) {
            targets.push(target);
        }
    }
    Ok(targets)
}
/// Strict bounded Content-Length framing, including fragmented headers/bodies.
pub fn read_message(input: &mut impl BufRead) -> io::Result<Value> {
    let mut length = None;
    let mut used = 0;
    loop {
        let mut line = Vec::new();
        input.take(8193).read_until(b'\n', &mut line)?;
        used += line.len();
        if line.is_empty() || used > 8192 || !line.ends_with(b"\r\n") {
            return Err(io::Error::other("Invalid or oversized LSP header"));
        }
        if line == b"\r\n" {
            break;
        }
        let text = std::str::from_utf8(&line).map_err(io::Error::other)?;
        let (name, value) = text
            .trim_end()
            .split_once(':')
            .ok_or_else(|| io::Error::other("Invalid LSP header"))?;
        if name.eq_ignore_ascii_case("Content-Length") {
            if length.is_some() {
                return Err(io::Error::other("Duplicate Content-Length"));
            }
            length = Some(value.trim().parse::<usize>().map_err(io::Error::other)?);
        }
    }
    let length = length
        .filter(|n| *n > 0 && *n <= MAX_MESSAGE)
        .ok_or_else(|| io::Error::other("Missing or oversized Content-Length"))?;
    let mut bytes = vec![0; length];
    input.read_exact(&mut bytes)?;
    let value: Value = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
    if !value.is_object() || value["jsonrpc"] != "2.0" {
        return Err(io::Error::other("Invalid JSON-RPC envelope"));
    }
    Ok(value)
}
pub fn write_message(out: &mut impl Write, value: &Value) -> io::Result<()> {
    let bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
    if bytes.len() > MAX_MESSAGE {
        return Err(io::Error::other("LSP message too large"));
    }
    write!(out, "Content-Length: {}\r\n\r\n", bytes.len())?;
    out.write_all(&bytes)?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{BufReader, Cursor};
    fn fixture() -> Workspace {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/studio");
        let mut w = Workspace::scan(root).unwrap();
        w.open(w.project.root.join("src/main.rs")).unwrap();
        w
    }
    #[test]
    fn framing_uses_utf8_bytes_and_handles_tiny_reads_and_concatenation() {
        let value = json!({"jsonrpc":"2.0","id":1,"result":"🌲é"});
        let mut bytes = Vec::new();
        write_message(&mut bytes, &value).unwrap();
        write_message(&mut bytes, &value).unwrap();
        let mut reader = BufReader::with_capacity(1, Cursor::new(bytes));
        assert_eq!(read_message(&mut reader).unwrap(), value);
        assert_eq!(read_message(&mut reader).unwrap(), value);
    }
    #[test]
    fn malformed_and_oversized_frames_are_rejected() {
        for raw in [
            b"Content-Length: 99999999\r\n\r\n".as_slice(),
            b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}",
            b"Content-Length: 2\n\n{}",
            b"Content-Length: -1\r\n\r\n",
            b"\r\n",
            b"Content-Length: 2\r\n\r\n{",
            b"Content-Length: 2\r\n\r\n{}",
        ] {
            assert!(read_message(&mut Cursor::new(raw)).is_err());
        }
        assert!(read_message(&mut Cursor::new(vec![b'x'; 9000])).is_err());
    }
    #[test]
    fn query_tracks_unicode_and_rejects_edit_undo_caret_and_reopen() {
        let mut w = fixture();
        let t = w.active_mut().unwrap();
        t.document.buffer.replace(0..0, "🌲é").unwrap();
        t.set_cursor("🌲é".len(), false);
        let q = DefinitionQuery::capture(&w).unwrap();
        assert_eq!((q.line, q.character), (0, 3));
        assert!(q.is_current(&w));
        w.active_mut()
            .unwrap()
            .document
            .buffer
            .replace(0..0, "x")
            .unwrap();
        assert!(!q.is_current(&w));
        w.active_mut().unwrap().document.buffer.undo();
        assert!(!q.is_current(&w));
        let q = DefinitionQuery::capture(&w).unwrap();
        w.active_mut().unwrap().set_cursor(0, false);
        assert!(!q.is_current(&w));
        let q = DefinitionQuery::capture(&w).unwrap();
        w.tabs.clear();
        w.open(q.source.clone()).unwrap();
        assert!(!q.is_current(&w));
    }
    #[test]
    fn other_document_edits_and_workspace_changes_invalidate_query() {
        let mut w = fixture();
        w.open(w.project.root.join("src/garden.rs")).unwrap();
        let q = DefinitionQuery::capture(&w).unwrap();
        w.tabs[0]
            .document
            .buffer
            .replace(0..0, "// edited\n")
            .unwrap();
        assert!(!q.is_current(&w));
        let q = DefinitionQuery::capture(&w).unwrap();
        w.project.root.push("other");
        assert!(!q.is_current(&w));
    }
    #[test]
    fn non_rust_overlays_and_large_documents_are_rejected() {
        let mut w = fixture();
        w.open(w.project.root.join("Cargo.toml")).unwrap();
        w.active_mut()
            .unwrap()
            .document
            .buffer
            .replace(0..0, "# unsaved\n")
            .unwrap();
        w.active = 0;
        assert!(DefinitionQuery::capture(&w).is_err());
        w.tabs.pop();
        w.active_mut()
            .unwrap()
            .document
            .buffer
            .replace(0..0, &"x".repeat(MAX_DOCUMENT))
            .unwrap();
        assert!(DefinitionQuery::capture(&w).is_err());
    }
    #[test]
    fn accepts_locations_links_null_and_deduplicates() {
        let w = fixture();
        let q = DefinitionQuery::capture(&w).unwrap();
        let uri = file_uri(&q.source).unwrap();
        let range = json!({"start":{"line":1,"character":2},"end":{"line":1,"character":4}});
        let a = json!({"uri":uri,"range":range});
        let b = json!({"targetUri":uri,"targetSelectionRange":range,"targetRange":range});
        assert_eq!(definition_targets(&a, &q).unwrap().len(), 1);
        assert_eq!(definition_targets(&json!([a, b]), &q).unwrap().len(), 1);
        assert!(definition_targets(&Value::Null, &q).unwrap().is_empty());
    }
    #[test]
    fn uri_escaping_roundtrip_and_untrusted_targets_rejected() {
        let w = fixture();
        let q = DefinitionQuery::capture(&w).unwrap();
        let encoded = file_uri(&q.root.join("a #🌲.rs")).unwrap();
        assert!(encoded.contains("%23"));
        let decoded = Url::parse(&encoded).unwrap().to_file_path().unwrap();
        // Windows file URLs omit the verbatim \? prefix returned by canonicalize.
        // Compare the existing parent canonically, without creating a fixture file.
        assert_eq!(decoded.parent().unwrap().canonicalize().unwrap(), q.root);
        assert_eq!(decoded.file_name().unwrap(), "a #🌲.rs");
        assert_eq!(file_uri(&decoded).unwrap(), encoded);
        for uri in [
            "https://example.org/x.rs".to_string(),
            "file://remote/x.rs".into(),
            format!("{}#x", file_uri(&q.source).unwrap()),
            file_uri(&q.root.join("Cargo.toml")).unwrap(),
        ] {
            let v = json!({"uri":uri,"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}}});
            if uri.ends_with("Cargo.toml") {
                // Scanned local non-Rust definitions are allowed only as response targets.
                assert!(definition_targets(&v, &q).is_ok());
            } else {
                assert!(definition_targets(&v, &q).is_err());
            }
        }
        let outside = file_uri(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("Cargo.toml")
                .canonicalize()
                .unwrap(),
        )
        .unwrap();
        assert!(definition_targets(&json!({"uri":outside,"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}}}),&q).is_err());
        assert!(definition_targets(&json!({"uri":file_uri(&q.source).unwrap(),"range":{"start":{"line":2,"character":0},"end":{"line":1,"character":0}}}),&q).is_err());
    }
}
