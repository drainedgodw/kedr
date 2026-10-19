use crate::TextBuffer;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

pub const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct Document {
    pub path: PathBuf,
    pub buffer: TextBuffer,
    saved: Arc<str>,
}
impl Document {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = fs::canonicalize(path)?;
        let text = read_text(&path)?;
        Ok(Self {
            path,
            buffer: TextBuffer::new(text.clone()),
            saved: text.into(),
        })
    }
    /// Construct an unsaved recovered buffer without reading or writing its target.
    pub fn from_recovery(path: PathBuf, baseline: String, text: String) -> io::Result<Self> {
        if !path.is_absolute()
            || baseline.len() > MAX_DOCUMENT_BYTES
            || text.len() > MAX_DOCUMENT_BYTES
        {
            return Err(io::Error::other("Invalid recovered document"));
        }
        Ok(Self {
            path,
            buffer: TextBuffer::new(text),
            saved: baseline.into(),
        })
    }
    /// Cheap immutable saved baseline for off-thread disk checks. Never includes unsaved edits.
    pub fn saved_snapshot(&self) -> Arc<str> {
        Arc::clone(&self.saved)
    }
    pub fn is_dirty(&self) -> bool {
        !self.buffer.equals(&self.saved)
    }
    /// Same-directory staged write + sync + atomic rename on supported local filesystems.
    /// The external-content check is not a lock against concurrent writers.
    pub fn save(&mut self) -> io::Result<()> {
        let disk = read_text(&self.path)?;
        if disk.as_str() != self.saved.as_ref() {
            return Err(io::Error::other(
                "File changed externally; refusing overwrite",
            ));
        }
        let parent = self
            .path
            .parent()
            .ok_or_else(|| io::Error::other("File has no parent"))?;
        let (temp, mut file) = stage(parent)?;
        let result = (|| {
            for chunk in self.buffer.chunks() {
                file.write_all(chunk.as_bytes())?;
            }
            file.set_permissions(fs::metadata(&self.path)?.permissions())?;
            file.sync_all()?;
            drop(file);
            // Recheck immediately before committing the staged file.
            if read_text(&self.path)?.as_str() != self.saved.as_ref() {
                return Err(io::Error::other("File changed during save"));
            }
            fs::rename(&temp, &self.path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result?;
        self.saved = self.buffer.to_text().into();
        Ok(())
    }
}
fn read_text(path: &Path) -> io::Result<String> {
    let file = File::open(path)?;
    let mut bytes = Vec::new();
    file.take((MAX_DOCUMENT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "File exceeds 16 MiB limit",
        ));
    }
    String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
fn stage(parent: &Path) -> io::Result<(PathBuf, File)> {
    loop {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(".kedr-save-{}-{id}", std::process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
}
