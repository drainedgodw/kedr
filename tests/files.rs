mod common;
use common::TempDir;
use kedr_core::{
    document::{Document, MAX_DOCUMENT_BYTES},
    project::{Project, ScanOptions},
};
use std::fs;

#[test]
fn document_roundtrip_preserves_bom_crlf_and_unicode() {
    let dir = TempDir::new();
    let p = dir.0.join("test.py");
    fs::write(&p, "\u{feff}🌲\r\nhello\r\n").unwrap();
    let mut d = Document::open(&p).unwrap();
    assert!(!d.is_dirty());
    d.buffer.replace(9..14, "world").unwrap();
    assert!(d.is_dirty());
    d.save().unwrap();
    assert!(!d.is_dirty());
    assert_eq!(fs::read_to_string(p).unwrap(), "\u{feff}🌲\r\nworld\r\n");
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
}
#[test]
fn external_change_is_not_overwritten() {
    let dir = TempDir::new();
    let p = dir.0.join("a");
    fs::write(&p, "original").unwrap();
    let mut d = Document::open(&p).unwrap();
    d.buffer.replace(0..8, "ours").unwrap();
    fs::write(&p, "theirs").unwrap();
    assert!(d.save().is_err());
    assert_eq!(fs::read_to_string(&p).unwrap(), "theirs");
    assert!(d.is_dirty());
}
#[test]
fn undo_back_to_saved_is_clean() {
    let dir = TempDir::new();
    let p = dir.0.join("a");
    fs::write(&p, "a").unwrap();
    let mut d = Document::open(&p).unwrap();
    d.buffer.replace(0..1, "b").unwrap();
    assert!(d.is_dirty());
    d.buffer.undo();
    assert!(!d.is_dirty());
}
#[test]
fn binary_and_oversized_files_are_rejected() {
    let dir = TempDir::new();
    let p = dir.0.join("a");
    fs::write(&p, [0xff, 0xfe]).unwrap();
    assert!(Document::open(&p).is_err());
    let f = fs::File::create(&p).unwrap();
    f.set_len((MAX_DOCUMENT_BYTES + 1) as u64).unwrap();
    assert!(Document::open(&p).is_err());
}
#[test]
fn missing_file_does_not_reappear_on_save() {
    let dir = TempDir::new();
    let p = dir.0.join("a");
    fs::write(&p, "a").unwrap();
    let mut d = Document::open(&p).unwrap();
    d.buffer.replace(0..1, "b").unwrap();
    fs::remove_file(&p).unwrap();
    assert!(d.save().is_err());
    assert!(!p.exists());
}
#[test]
fn fixtures_are_unique_in_parallel() {
    let threads: Vec<_> = (0..32).map(|_| std::thread::spawn(TempDir::new)).collect();
    let dirs: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    let unique: std::collections::HashSet<_> = dirs.iter().map(|d| &d.0).collect();
    assert_eq!(unique.len(), dirs.len());
}
#[test]
fn scanning_ignores_generated_directories_and_sorts_paths() {
    let dir = TempDir::new();
    fs::create_dir(dir.0.join(".git")).unwrap();
    fs::write(dir.0.join(".git/hidden"), "").unwrap();
    fs::write(dir.0.join("z.py"), "").unwrap();
    fs::write(dir.0.join("a.py"), "").unwrap();
    let p = Project::scan(&dir.0, &ScanOptions::default()).unwrap();
    assert_eq!(p.files.len(), 2);
    assert!(p.files[0].ends_with("a.py"));
    assert!(!p.truncated);
}
#[test]
fn file_directory_and_depth_limits_are_reported() {
    let dir = TempDir::new();
    fs::create_dir_all(dir.0.join("a/b/c")).unwrap();
    for i in 0..8 {
        fs::write(dir.0.join(format!("{i}.py")), "").unwrap();
    }
    let p = Project::scan(
        &dir.0,
        &ScanOptions {
            max_files: 3,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(p.files.len(), 3);
    assert!(p.truncated);
    let p = Project::scan(
        &dir.0,
        &ScanOptions {
            max_depth: 0,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(p.truncated);
    let p = Project::scan(
        &dir.0,
        &ScanOptions {
            max_directories: 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(p.directories_visited, 1);
    assert!(p.truncated);
}
#[cfg(unix)]
#[test]
fn scanning_never_follows_symlink_cycles() {
    let dir = TempDir::new();
    std::os::unix::fs::symlink(&dir.0, dir.0.join("loop")).unwrap();
    fs::write(dir.0.join("a"), "").unwrap();
    let p = Project::scan(&dir.0, &ScanOptions::default()).unwrap();
    assert_eq!(p.files.len(), 1);
    assert!(!p.truncated);
}

#[test]
fn saved_snapshot_is_immutable_and_never_exposes_unsaved_edits() {
    let dir = TempDir::new();
    let p = dir.0.join("a");
    fs::write(&p, "original").unwrap();
    let mut d = Document::open(&p).unwrap();
    let saved = d.saved_snapshot();
    assert!(std::sync::Arc::ptr_eq(&saved, &d.saved_snapshot()));
    d.buffer.replace(0..8, "ours").unwrap();
    assert_eq!(saved.as_ref(), "original");
    assert_eq!(d.saved_snapshot().as_ref(), "original");
    d.save().unwrap();
    assert_eq!(saved.as_ref(), "original");
    assert_eq!(d.saved_snapshot().as_ref(), "ours");
    assert!(!std::sync::Arc::ptr_eq(&saved, &d.saved_snapshot()));
}
