use kedr_workbench::{
    EditorTab, Movement, TaskKind, TaskPhase, TaskRunner, Workspace, fuzzy_score, search_project,
};
use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new(text: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "kedr-workbench-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::write(p.join("main.rs"), text).unwrap();
        Self(p)
    }
    fn editor(&self) -> EditorTab {
        EditorTab::open(self.0.join("main.rs")).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn unicode_typing_selection_undo_roundtrip() {
    let f = Fixture::new("a🌲b");
    let mut e = f.editor();
    e.set_cursor(5, false);
    e.backspace().unwrap();
    assert_eq!(e.document.buffer.to_text(), "ab");
    e.undo(false).unwrap();
    assert_eq!(e.document.buffer.to_text(), "a🌲b");
    e.move_cursor(Movement::Left, false);
    assert_eq!(e.cursor, 1);
    e.insert("🙂").unwrap();
    e.select_all();
    assert_eq!(e.selected_text(), "a🙂🌲b");
}
#[test]
fn vertical_navigation_preserves_desired_column() {
    let f = Fixture::new("abcd\nx\nabcde");
    let mut e = f.editor();
    e.set_cursor(3, false);
    e.move_cursor(Movement::Down, false);
    assert_eq!(e.position(), (1, 1));
    e.move_cursor(Movement::Down, false);
    assert_eq!(e.position(), (2, 3));
}
#[test]
fn indentation_is_one_undo_transaction() {
    let f = Fixture::new("one\n🌲two\n");
    let mut e = f.editor();
    e.select_all();
    e.indent(false).unwrap();
    assert_eq!(e.document.buffer.to_text(), "    one\n    🌲two\n");
    e.undo(false).unwrap();
    assert_eq!(e.document.buffer.to_text(), "one\n🌲two\n");
}
#[test]
fn replacement_handles_overlapping_matches() {
    let f = Fixture::new("aaaaa");
    let mut e = f.editor();
    assert_eq!(e.replace_all("aa", "x").unwrap(), 2);
    assert_eq!(e.document.buffer.to_text(), "xxa");
    e.undo(false).unwrap();
    assert_eq!(e.document.buffer.to_text(), "aaaaa");
}
#[test]
fn crlf_enter_and_autoindent_are_preserved() {
    let f = Fixture::new("fn main() {\r\n}\r\n");
    let mut e = f.editor();
    e.set_cursor(11, false);
    e.enter().unwrap();
    assert!(
        e.document
            .buffer
            .to_text()
            .starts_with("fn main() {\r\n    \r\n")
    );
}
#[test]
fn save_refuses_external_overwrite() {
    let f = Fixture::new("a");
    let mut e = f.editor();
    e.insert("b").unwrap();
    fs::write(&e.document.path, "external").unwrap();
    assert!(e.save().is_err());
    assert_eq!(fs::read_to_string(&e.document.path).unwrap(), "external");
}
#[test]
fn dirty_close_is_not_automatic() {
    let f = Fixture::new("a");
    let mut w = Workspace::scan(&f.0).unwrap();
    w.open(f.0.join("main.rs")).unwrap();
    w.active_mut().unwrap().insert("b").unwrap();
    assert!(w.close_clean(0).is_err());
    assert_eq!(w.tabs.len(), 1);
    w.active_mut().unwrap().undo(false).unwrap();
    assert!(w.close_clean(0).is_ok());
}
#[test]
fn workspace_rejects_other_directories() {
    let f = Fixture::new("a");
    let other = Fixture::new("b");
    let mut w = Workspace::scan(&f.0).unwrap();
    assert!(w.open(other.0.join("main.rs")).is_err());
}
#[test]
fn quick_open_scores_contiguous_and_fuzzy_paths() {
    assert!(fuzzy_score("src/editor.rs", "edrs").is_some());
    assert!(fuzzy_score("src/editor.rs", "missing").is_none());
    assert!(
        fuzzy_score("src/editor.rs", "editor").unwrap()
            > fuzzy_score("some/e/d/i/t/o/r", "editor").unwrap()
    );
}
#[test]
fn search_reports_saved_files_and_cancellation() {
    let f = Fixture::new("🌲 found\nfound again");
    let w = Workspace::scan(&f.0).unwrap();
    let g = Arc::new(AtomicU64::new(1));
    let r = search_project(&w.project.root, &w.project.files, "found", 1, &g);
    assert_eq!(r.hits.len(), 2);
    assert_eq!((r.hits[0].line, r.hits[0].column), (0, 2));
    g.store(2, Ordering::Relaxed);
    assert!(search_project(&w.project.root, &w.project.files, "found", 1, &g).limited);
}
#[test]
fn no_op_does_not_set_dirty_marker() {
    let f = Fixture::new("a");
    let mut e = f.editor();
    e.backspace().unwrap();
    assert!(!e.cached_dirty);
}
#[test]
fn task_requires_a_cargo_manifest() {
    let f = Fixture::new("a");
    assert!(TaskRunner::start(&f.0, TaskKind::Check, || {}).is_err());
}
#[test]
fn trusted_authored_fixture_runs_a_real_task() {
    let f = Fixture::new("fn main() {}\n");
    fs::write(f.0.join("Cargo.toml"),"[package]\nname='kedr-task-fixture'\nversion='0.1.0'\nedition='2024'\n[[bin]]\nname='fixture'\npath='main.rs'\n[workspace]\n").unwrap();
    let task = TaskRunner::start(&f.0, TaskKind::Check, || {}).unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while task.phase() == TaskPhase::Running && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(task.phase(), TaskPhase::Success);
}

#[test]
fn tab_limit_preserves_existing_tabs_and_allows_reactivation() {
    let f = Fixture::new("a");
    for i in 0..33 {
        fs::write(f.0.join(format!("file-{i}.rs")), "a").unwrap();
    }
    let mut w = Workspace::scan(&f.0).unwrap();
    for i in 0..32 {
        w.open(f.0.join(format!("file-{i}.rs"))).unwrap();
    }
    assert!(w.open(f.0.join("file-32.rs")).is_err());
    assert_eq!(w.tabs.len(), 32);
    w.open(f.0.join("file-0.rs")).unwrap();
    assert_eq!(w.active, 0);
    assert_eq!(w.tabs.len(), 32);
}
