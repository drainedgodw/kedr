use egui::{Context, Event, Key, Modifiers, Pos2, RawInput, Rect, Vec2};
use kedr_ui::{Action, Settings, Shell};
use kedr_workbench::Workspace;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new(text: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "kedr-ui-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::write(p.join("main.rs"), text).unwrap();
        Self(p)
    }
    fn shell(&self) -> Shell {
        let mut w = Workspace::scan(&self.0).unwrap();
        w.open(self.0.join("main.rs")).unwrap();
        let mut s = Shell::new("Kedr Studio", Settings::default());
        s.set_workspace(w);
        s
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn frame(ctx: &Context, s: &mut Shell, events: Vec<Event>) {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200., 800.))),
        events,
        focused: true,
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| {
        let _ = s.show(ctx);
    });
}
fn key(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}
#[test]
fn native_editor_actions_use_the_core() {
    let f = Fixture::new("fn main() {}\n");
    let mut s = f.shell();
    let c = Context::default();
    frame(&c, &mut s, Vec::new());
    frame(&c, &mut s, vec![Event::Text("🌲".into())]);
    assert!(
        s.workspace
            .as_ref()
            .unwrap()
            .active()
            .unwrap()
            .document
            .buffer
            .to_text()
            .starts_with('🌲')
    );
    frame(&c, &mut s, vec![key(Key::Z, Modifiers::COMMAND)]);
    assert_eq!(
        s.workspace
            .as_ref()
            .unwrap()
            .active()
            .unwrap()
            .document
            .buffer
            .to_text(),
        "fn main() {}\n"
    );
}
#[test]
fn ime_preedit_does_not_edit_until_commit() {
    let f = Fixture::new("a");
    let mut s = f.shell();
    let c = Context::default();
    frame(&c, &mut s, Vec::new());
    frame(
        &c,
        &mut s,
        vec![
            Event::Ime(egui::ImeEvent::Enabled),
            Event::Ime(egui::ImeEvent::Preedit("木".into())),
        ],
    );
    assert_eq!(
        s.workspace
            .as_ref()
            .unwrap()
            .active()
            .unwrap()
            .document
            .buffer
            .to_text(),
        "a"
    );
    frame(
        &c,
        &mut s,
        vec![Event::Ime(egui::ImeEvent::Commit("木".into()))],
    );
    assert_eq!(
        s.workspace
            .as_ref()
            .unwrap()
            .active()
            .unwrap()
            .document
            .buffer
            .to_text(),
        "木a"
    );
}
#[test]
fn viewport_does_not_paint_all_document_lines() {
    let f = Fixture::new(&"let value = 42;\n".repeat(100000));
    let mut s = f.shell();
    let c = Context::default();
    frame(&c, &mut s, Vec::new());
    assert!(s.editor_cache.painted_rows < 80);
    assert!(s.editor_cache.painted_rows > 5);
}
#[test]
fn palette_captures_typing_without_mutating_document() {
    let f = Fixture::new("a");
    let mut s = f.shell();
    let c = Context::default();
    frame(&c, &mut s, Vec::new());
    s.action(Action::QuickOpen);
    frame(&c, &mut s, Vec::new());
    frame(&c, &mut s, vec![Event::Text("main".into())]);
    assert_eq!(
        s.workspace
            .as_ref()
            .unwrap()
            .active()
            .unwrap()
            .document
            .buffer
            .to_text(),
        "a"
    );
}
#[test]
fn task_request_requires_a_trust_dialog() {
    let f = Fixture::new("a");
    let mut s = f.shell();
    s.action(Action::Check);
    let c = Context::default();
    let mut effects = Vec::new();
    let _ = c.run(RawInput::default(), |ctx| effects = s.show(ctx));
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e, kedr_ui::Effect::RunTask(_)))
    );
}
#[test]
fn closing_a_modified_tab_preserves_it() {
    let f = Fixture::new("a");
    let mut s = f.shell();
    s.workspace
        .as_mut()
        .unwrap()
        .active_mut()
        .unwrap()
        .insert("b")
        .unwrap();
    s.action(Action::CloseTab);
    assert_eq!(s.workspace.as_ref().unwrap().tabs.len(), 1);
}
#[test]
fn modal_blocks_document_keyboard_input() {
    let f = Fixture::new("a");
    let mut s = f.shell();
    let c = Context::default();
    frame(&c, &mut s, Vec::new());
    s.action(Action::Settings);
    frame(&c, &mut s, Vec::new());
    frame(&c, &mut s, vec![Event::Text("bad".into())]);
    assert_eq!(
        s.workspace
            .as_ref()
            .unwrap()
            .active()
            .unwrap()
            .document
            .buffer
            .to_text(),
        "a"
    );
}
#[test]
fn huge_line_preview_keeps_full_content() {
    let f = Fixture::new(&"a".repeat(100000));
    let mut s = f.shell();
    let c = Context::default();
    frame(&c, &mut s, Vec::new());
    let t = s.workspace.as_mut().unwrap().active_mut().unwrap();
    t.set_cursor(90000, false);
    frame(&c, &mut s, Vec::new());
    assert_eq!(
        s.workspace
            .as_ref()
            .unwrap()
            .active()
            .unwrap()
            .document
            .buffer
            .len_bytes(),
        100000
    );
}
