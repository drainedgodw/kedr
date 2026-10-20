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
        if self.buffer.len_bytes() > MAX_DOCUMENT_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "File exceeds 16 MiB limit",
            ));
        }
        validate_target(&self.path)?;
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
            validate_target(&self.path)?;
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
/// Refuse observed path rebinding and non-regular objects before opening them.
/// This is not a filesystem lock or protection against adversarial TOCTOU races.
fn validate_target(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || path.canonicalize()? != path {
        return Err(io::Error::other(
            "File identity changed or target is not a regular file",
        ));
    }
    Ok(())
}
fn read_text(path: &Path) -> io::Result<String> {
    validate_target(path)?;
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
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
}

#[cfg(all(test, unix))]
mod audit_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn staging_is_private_before_any_document_bytes_are_written() {
        let dir = std::env::temp_dir().join(format!("kedr-private-stage-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let (path, file) = stage(&dir).unwrap();
        let mode = file.metadata().unwrap().permissions().mode() & 0o777;
        drop(file);
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
        assert_eq!(
            mode & 0o077,
            0,
            "temporary source must not be group/world-readable"
        );
    }
}
