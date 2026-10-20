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

#[test]
fn oversized_edited_document_cannot_be_saved_into_an_unopenable_file() {
    let dir = TempDir::new();
    let p = dir.0.join("a");
    fs::write(&p, "original").unwrap();
    let mut d = Document::open(&p).unwrap();
    d.buffer
        .replace(0..8, &"x".repeat(MAX_DOCUMENT_BYTES + 1))
        .unwrap();
    assert!(
        d.save().is_err(),
        "save must enforce the same size limit as open"
    );
    assert_eq!(fs::read_to_string(&p).unwrap(), "original");
    assert!(d.is_dirty());
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn save_rejects_rebound_symlink_target_without_replacing_it() {
    let dir = TempDir::new();
    let p = dir.0.join("a");
    let external = dir.0.join("external");
    fs::write(&p, "original").unwrap();
    fs::write(&external, "original").unwrap();
    let mut d = Document::open(&p).unwrap();
    d.buffer.replace(0..8, "ours").unwrap();
    fs::remove_file(&p).unwrap();
    std::os::unix::fs::symlink(&external, &p).unwrap();
    assert!(
        d.save().is_err(),
        "a rebound path is not the originally opened file"
    );
    assert!(fs::symlink_metadata(&p).unwrap().file_type().is_symlink());
    assert_eq!(fs::read_to_string(&external).unwrap(), "original");
    assert!(d.is_dirty());
}

#[cfg(unix)]
#[test]
fn save_preserves_private_and_executable_permission_bits() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new();
    let p = dir.0.join("a");
    fs::write(&p, "original").unwrap();
    for mode in [0o600, 0o755] {
        fs::set_permissions(&p, fs::Permissions::from_mode(mode)).unwrap();
        let mut d = Document::open(&p).unwrap();
        d.buffer.replace(0..d.buffer.len_bytes(), "next").unwrap();
        d.save().unwrap();
        assert_eq!(fs::metadata(&p).unwrap().permissions().mode() & 0o777, mode);
    }
}

#[cfg(unix)]
#[test]
fn save_rejects_rebound_parent_directory() {
    let dir = TempDir::new();
    let parent = dir.0.join("parent");
    let other = dir.0.join("other");
    fs::create_dir_all(&parent).unwrap();
    fs::create_dir_all(&other).unwrap();
    fs::write(parent.join("a"), "original").unwrap();
    fs::write(other.join("a"), "original").unwrap();
    let mut d = Document::open(parent.join("a")).unwrap();
    d.buffer.replace(0..8, "ours").unwrap();
    fs::rename(&parent, dir.0.join("moved")).unwrap();
    std::os::unix::fs::symlink(&other, &parent).unwrap();
    assert!(d.save().is_err());
    assert_eq!(fs::read_to_string(other.join("a")).unwrap(), "original");
    assert_eq!(
        fs::read_to_string(dir.0.join("moved/a")).unwrap(),
        "original"
    );
    assert!(d.is_dirty());
}
