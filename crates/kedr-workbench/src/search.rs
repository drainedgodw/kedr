use std::{
    fs,
    io::{self, Read},
    ops::Range,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub range: Range<usize>,
    pub preview: String,
    pub needle: String,
}
#[derive(Debug, Default)]
pub struct SearchReport {
    pub hits: Vec<SearchHit>,
    pub files_scanned: usize,
    pub skipped: usize,
    pub limited: bool,
}
/// Saved-file search: 32 MiB total, 1 MiB/file, 1,000 hits. Cancel on generation change.
pub fn search_project(
    root: &Path,
    files: &[PathBuf],
    needle: &str,
    generation: u64,
    current: &Arc<AtomicU64>,
) -> SearchReport {
    let mut r = SearchReport::default();
    let mut total = 0;
    if needle.is_empty() {
        return r;
    }
    for path in files {
        if current.load(Ordering::Relaxed) != generation {
            r.limited = true;
            break;
        }
        let result = (|| -> io::Result<String> {
            let canonical = fs::canonicalize(path)?;
            if !canonical.starts_with(root) {
                return Err(io::Error::other("Outside workspace"));
            }
            let mut bytes = Vec::new();
            fs::File::open(canonical)?
                .take(1024 * 1024 + 1)
                .read_to_end(&mut bytes)?;
            total += bytes.len();
            if bytes.len() > 1024 * 1024 {
                return Err(io::Error::other("Search file limit"));
            }
            String::from_utf8(bytes).map_err(io::Error::other)
        })();
        if total > 32 * 1024 * 1024 {
            r.limited = true;
            break;
        }
        let Ok(text) = result else {
            r.skipped += 1;
            continue;
        };
        r.files_scanned += 1;
        let mut start = 0;
        for (line, content) in text.split_inclusive('\n').enumerate() {
            for (at, _) in content.match_indices(needle) {
                r.hits.push(SearchHit {
                    path: path.clone(),
                    line,
                    column: content[..at].chars().count(),
                    range: start + at..start + at + needle.len(),
                    preview: content
                        .trim_end_matches(['\r', '\n'])
                        .chars()
                        .take(180)
                        .collect(),
                    needle: needle.into(),
                });
                if r.hits.len() == 1000 {
                    r.limited = true;
                    return r;
                }
            }
            start += content.len();
        }
    }
    r
}
