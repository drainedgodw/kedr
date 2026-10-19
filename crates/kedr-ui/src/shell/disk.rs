use super::*;
use kedr_workbench::{DiskState, EditorTab, FileQuery};
impl Shell {
    /// Invalidate pending completions when the host begins a workspace switch,
    /// including a switch whose scan eventually fails. Does not cancel OS I/O.
    pub fn cancel_file_reload(&mut self) {
        if self.reload_pending.take().is_some() {
            self.status = "Reload cancelled".into();
        }
        if matches!(self.dialog, Some(Dialog::Reload(_))) {
            self.dialog = None;
        }
    }
    pub fn file_query(&self) -> Option<FileQuery> {
        FileQuery::capture(self.workspace.as_ref()?)
    }
    pub fn apply_disk_probe(&mut self, query: &FileQuery, state: DiskState) {
        if !self.workspace.as_ref().is_some_and(|w| query.is_current(w)) {
            return;
        }
        self.disk_notice = if state == DiskState::Current {
            None
        } else {
            Some((query.path.clone(), state))
        };
    }
    pub(super) fn request_reload(&mut self) {
        if self.loading {
            self.error("Wait for the workspace to finish opening");
            return;
        }
        if self.reload_pending.is_some() {
            self.error("A reload is already running");
            return;
        }
        if let Some(query) = self.file_query() {
            self.dialog = Some(Dialog::Reload(query));
        }
    }
    pub(super) fn begin_reload(&mut self, query: FileQuery) {
        if self.reload_pending.is_some()
            || !self.workspace.as_ref().is_some_and(|w| query.is_current(w))
        {
            self.error("Editor changed; request Reload file again");
            return;
        }
        self.reload_pending = Some(query.clone());
        self.effects.push(Effect::ReloadFile(query));
        self.status = "Reloading file…".into();
    }
    pub fn complete_reload(&mut self, query: &FileQuery, result: Result<Document, String>) {
        if !self
            .reload_pending
            .as_ref()
            .is_some_and(|q| q.same_request(query))
        {
            return;
        }
        self.reload_pending = None;
        if !self.workspace.as_ref().is_some_and(|w| query.is_current(w)) {
            self.status = "Reload discarded: editor changed".into();
            return;
        }
        let document = match result {
            Ok(document) if document.path == query.path => document,
            Ok(_) => {
                self.error("Reload returned a different file; editor unchanged");
                return;
            }
            Err(error) => {
                self.error(format!(
                    "Reload failed; your editor contents are unchanged: {error}"
                ));
                return;
            }
        };
        self.cancel_definition();
        self.pending_hit = None;
        self.pending_diagnostic = None;
        self.restoring = None;
        let tab = self.workspace.as_mut().unwrap().active_mut().unwrap();
        let (line, column) = tab.position();
        let mut replacement = EditorTab::from_document(document);
        // Restore a clamped Unicode-scalar position, never a raw offset into a changed UTF-8 sequence.
        let row = line.min(replacement.document.buffer.line_count().saturating_sub(1));
        let max_column = replacement
            .document
            .buffer
            .byte_to_position(replacement.line_bounds(row).end)
            .unwrap()
            .1;
        let at = replacement
            .document
            .buffer
            .position_to_byte(row, column.min(max_column))
            .unwrap();
        replacement.set_cursor(at, false);
        *tab = replacement;
        self.disk_notice = None;
        self.editor_cache.clear();
        self.editor_cache.focus_next = true;
        self.observe_location();
        self.error = None;
        self.status = "File reloaded from disk".into();
    }
    pub(super) fn disk_banner(&mut self, ui: &mut egui::Ui, p: Palette) {
        let path = self
            .workspace
            .as_ref()
            .and_then(Workspace::active)
            .map(|t| &t.document.path);
        let notice = self
            .disk_notice
            .as_ref()
            .filter(|(name, _)| Some(name) == path)
            .map(|(_, state)| state.clone());
        if let Some(state) = notice {
            let message = match &state {
                DiskState::Current => return,
                DiskState::Changed => {
                    "File changed on disk. Editor contents have not been replaced."
                }
                DiskState::Missing => {
                    "File is missing on disk. Editor contents are kept; saving will not recreate it."
                }
                DiskState::Unreadable(_) => {
                    "File cannot be checked on disk. Editor contents are kept."
                }
            };
            egui::Frame::new()
                .fill(p.surface)
                .inner_margin(8)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(RichText::new(message).color(p.amber));
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(
                                self.reload_pending.is_none(),
                                egui::Button::new("Reload from disk…"),
                            )
                            .clicked()
                        {
                            self.request_reload();
                        }
                        ui.label(
                            RichText::new("No automatic overwrite.")
                                .small()
                                .color(p.muted),
                        );
                    });
                    if let DiskState::Unreadable(error) = state {
                        ui.add(
                            egui::Label::new(RichText::new(&error).small().color(p.muted))
                                .truncate(),
                        )
                        .on_hover_text(error);
                    }
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn shell() -> (Self, Shell) {
            let root = std::env::temp_dir().join(format!(
                "kedr-reload-ui-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).unwrap();
            let f = Self(root.canonicalize().unwrap());
            fs::write(f.0.join("a.rs"), "one 🌲\r\nsecond\r\n").unwrap();
            fs::write(f.0.join("b.rs"), "other").unwrap();
            let mut s = Shell::new("CodeCedar", Settings::default());
            s.set_workspace(Workspace::scan(&f.0).unwrap());
            s.accept_document(Document::open(f.0.join("a.rs")).unwrap());
            (f, s)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn tab(s: &mut Shell) -> &mut EditorTab {
        s.workspace.as_mut().unwrap().active_mut().unwrap()
    }
    fn start(s: &mut Shell) -> FileQuery {
        s.action(Action::Reload);
        let Some(Dialog::Reload(q)) = s.dialog.clone() else {
            panic!("confirmation required")
        };
        assert!(s.effects.is_empty());
        s.begin_reload(q.clone());
        s.dialog = None;
        assert!(matches!(s.effects.pop(), Some(Effect::ReloadFile(_))));
        q
    }
    #[test]
    fn poll_never_replaces_dirty_text_and_rejects_obsolete_results() {
        let (_f, mut s) = Fixture::shell();
        let q = s.file_query().unwrap();
        tab(&mut s).insert("ours").unwrap();
        s.apply_disk_probe(&q, DiskState::Changed);
        assert!(s.disk_notice.is_none());
        let q = s.file_query().unwrap();
        s.apply_disk_probe(&q, DiskState::Changed);
        assert!(s.disk_notice.is_some());
        assert!(tab(&mut s).document.buffer.to_text().starts_with("ours"));
        s.apply_disk_probe(&q, DiskState::Current);
        assert!(s.disk_notice.is_none());
    }
    #[test]
    fn explicit_reload_discards_only_confirmed_file_and_clamps_unicode_cursor() {
        let (f, mut s) = Fixture::shell();
        s.accept_document(Document::open(f.0.join("b.rs")).unwrap());
        tab(&mut s).insert("keep").unwrap();
        s.activate_path(&f.0.join("a.rs"));
        tab(&mut s).goto(2, 6).unwrap();
        tab(&mut s).insert("ours").unwrap();
        let q = start(&mut s);
        assert!(!s.action_available(Action::Reload));
        fs::write(&q.path, "🌲").unwrap();
        let d = q.read().unwrap();
        s.complete_reload(&q, Ok(d));
        assert_eq!(tab(&mut s).document.buffer.to_text(), "🌲");
        assert_eq!(tab(&mut s).cursor, "🌲".len());
        assert!(!tab(&mut s).document.is_dirty());
        assert_eq!(tab(&mut s).newline, "\n");
        assert!(s.action_available(Action::Reload));
        assert!(s.editor_cache.focus_next);
        tab(&mut s).undo(false).unwrap();
        assert_eq!(tab(&mut s).document.buffer.to_text(), "🌲");
        s.activate_path(&f.0.join("b.rs"));
        assert!(tab(&mut s).document.buffer.to_text().starts_with("keep"));
        assert!(tab(&mut s).document.is_dirty());
        assert_eq!(fs::read_to_string(f.0.join("b.rs")).unwrap(), "other");
    }
    #[test]
    fn edits_while_loading_discard_reload_and_keep_undo() {
        let (_f, mut s) = Fixture::shell();
        let q = start(&mut s);
        let doc = q.read().unwrap();
        tab(&mut s).insert("later").unwrap();
        s.complete_reload(&q, Ok(doc));
        assert!(tab(&mut s).document.buffer.to_text().starts_with("later"));
        assert_eq!(s.status, "Reload discarded: editor changed");
        assert!(s.reload_pending.is_none());
        tab(&mut s).undo(false).unwrap();
        assert_eq!(
            tab(&mut s).document.buffer.to_text(),
            "one 🌲\r\nsecond\r\n"
        );
    }
    #[test]
    fn changed_confirmation_cannot_start_a_reload() {
        let (_f, mut s) = Fixture::shell();
        s.action(Action::Reload);
        let Some(Dialog::Reload(q)) = s.dialog.clone() else {
            panic!()
        };
        tab(&mut s).insert("later").unwrap();
        s.begin_reload(q);
        assert!(s.reload_pending.is_none());
        assert!(s.effects.is_empty());
        assert!(s.error.as_deref().unwrap().contains("Editor changed"));
    }
    #[test]
    fn missing_or_wrong_target_never_drops_edits_or_recreates_file() {
        let (f, mut s) = Fixture::shell();
        tab(&mut s).insert("ours").unwrap();
        let q = start(&mut s);
        fs::remove_file(&q.path).unwrap();
        s.complete_reload(&q, q.read().map_err(|e| e.to_string()));
        assert!(tab(&mut s).document.buffer.to_text().starts_with("ours"));
        assert!(!q.path.exists());
        assert!(s.error.as_deref().unwrap().contains("unchanged"));
        let q = start(&mut s);
        s.complete_reload(&q, Ok(Document::open(f.0.join("b.rs")).unwrap()));
        assert!(tab(&mut s).document.buffer.to_text().starts_with("ours"));
    }
    #[test]
    fn switched_or_reopened_tab_and_workspace_ignore_pending_read() {
        let (f, mut s) = Fixture::shell();
        let q = start(&mut s);
        let doc = q.read().unwrap();
        s.accept_document(Document::open(f.0.join("b.rs")).unwrap());
        s.complete_reload(&q, Ok(doc));
        assert_eq!(tab(&mut s).name(), "b.rs");
        s.activate_path(&q.path);
        let q = start(&mut s);
        let doc = q.read().unwrap();
        let w = s.workspace.as_mut().unwrap();
        w.discard(w.active);
        w.open(&q.path).unwrap();
        s.complete_reload(&q, Ok(doc));
        assert_eq!(s.status, "Reload discarded: editor changed");
        let q = start(&mut s);
        let doc = q.read().unwrap();
        s.set_workspace(Workspace::scan(&f.0).unwrap());
        s.complete_reload(&q, Ok(doc));
        assert!(s.workspace.as_ref().unwrap().tabs.is_empty());
        assert!(s.reload_pending.is_none());
    }
    #[test]
    fn compact_confirmation_enter_is_not_implicit_discard_and_escape_cancels() {
        let (_f, mut s) = Fixture::shell();
        tab(&mut s).insert("ours").unwrap();
        s.action(Action::Reload);
        let ctx = Context::default();
        let frame = |s: &mut Shell, key| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(760., 520.),
                    )),
                    focused: true,
                    events: vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |ctx| {
                    s.show(ctx);
                },
            );
        };
        frame(&mut s, Key::Enter);
        assert!(matches!(s.dialog, Some(Dialog::Reload(_))));
        assert!(s.reload_pending.is_none());
        assert!(s.effects.is_empty());
        frame(&mut s, Key::Escape);
        assert!(s.dialog.is_none());
        assert!(tab(&mut s).document.buffer.to_text().starts_with("ours"));
    }
    #[test]
    fn abandoned_reload_does_not_block_retry_or_complete_a_new_request() {
        let (_f, mut s) = Fixture::shell();
        let old = start(&mut s);
        let old_doc = old.read().unwrap();
        s.cancel_file_reload();
        assert!(s.action_available(Action::Reload));
        // Simulate a failed workspace scan: the old workspace/tab remains active.
        let new = start(&mut s);
        assert!(!old.same_request(&new));
        s.complete_reload(&old, Ok(old_doc));
        assert!(
            s.reload_pending
                .as_ref()
                .is_some_and(|q| q.same_request(&new))
        );
        s.complete_reload(&new, new.read().map_err(|e| e.to_string()));
        assert!(s.reload_pending.is_none());
        assert_eq!(s.status, "File reloaded from disk");
    }
}
