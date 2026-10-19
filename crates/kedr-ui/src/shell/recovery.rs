use super::*;
impl Shell {
    pub fn recovery_guard(&self) -> Option<kedr_workbench::recovery::Guard> {
        self.workspace
            .as_ref()
            .map(kedr_workbench::recovery::Guard::capture)
    }
    pub fn accept_recovery(
        &mut self,
        guard: &kedr_workbench::recovery::Guard,
        tabs: Vec<kedr_workbench::EditorTab>,
    ) -> Result<(), String> {
        if self.loading {
            return Err("Workspace opening is in progress; nothing restored".into());
        }
        let w = self
            .workspace
            .as_mut()
            .ok_or("Open the original workspace first")?;
        if !guard.matches(w) {
            return Err(
                "Workspace changed while reading drafts; nothing restored. Try again.".into(),
            );
        }
        let new = tabs
            .iter()
            .filter(|t| {
                !w.tabs
                    .iter()
                    .any(|old| old.document.path == t.document.path)
            })
            .count();
        if w.tabs.len() + new > 32
            || tabs.iter().any(|t| {
                !t.document.path.starts_with(&w.project.root)
                    || w.tabs
                        .iter()
                        .any(|old| old.document.path == t.document.path && old.document.is_dirty())
            })
        {
            return Err(
                "Recovery would replace current edits or exceed 32 tabs; nothing restored".into(),
            );
        }
        self.cancel_definition();
        self.cancel_file_reload();
        let w = self.workspace.as_mut().unwrap();
        for t in tabs {
            if let Some(i) = w
                .tabs
                .iter()
                .position(|old| old.document.path == t.document.path)
            {
                w.tabs[i] = t;
                w.active = i
            } else {
                w.tabs.push(t);
                w.active = w.tabs.len() - 1
            }
        }
        self.editor_cache.clear();
        self.editor_cache.focus_next = true;
        self.search_report = None;
        self.restoring = None;
        self.pending_hit = None;
        self.pending_diagnostic = None;
        self.status =
            "Drafts restored in editor only. Disk may differ; saves remain guarded.".into();
        self.dialog = None;
        Ok(())
    }
    pub(super) fn recovery_ui(&mut self, ui: &mut egui::Ui, p: Palette) {
        ui.heading("Recover unsaved drafts");
        ui.label("Local plaintext snapshots may contain sensitive code. No project code is run.");
        ui.label("Open the original workspace, then restore into editor buffers. Disk is never overwritten here. Other sessions may still be running.");
        let root = self.workspace.as_ref().map(|w| w.project.root.clone());
        let mut chosen = None;
        let mut open = None;
        egui::ScrollArea::vertical()
            .id_salt("recovery-offers")
            .max_height(240.)
            .show(ui, |ui| {
                if self.recovery_offers.is_empty() {
                    ui.label("No previous draft snapshots found.");
                }
                for o in &self.recovery_offers {
                    ui.separator();
                    ui.add(egui::Label::new(o.root.to_string_lossy()).truncate())
                        .on_hover_text(o.root.display().to_string());
                    ui.label(
                        RichText::new(format!(
                            "{} unsaved file(s) · snapshot {}",
                            o.count,
                            o.id + 1
                        ))
                        .color(p.muted),
                    );
                    ui.horizontal_wrapped(|ui| {
                        if root.as_ref() != Some(&o.root)
                            && ui
                                .add_enabled(
                                    !self.recovery_busy && !self.loading,
                                    egui::Button::new("Open workspace"),
                                )
                                .clicked()
                        {
                            open = Some(o.root.clone());
                        }
                        if ui
                            .add_enabled(
                                !self.loading
                                    && !self.recovery_busy
                                    && root.as_ref() == Some(&o.root),
                                egui::Button::new("Restore drafts"),
                            )
                            .clicked()
                        {
                            chosen = Some((o.id, true))
                        }
                        if ui
                            .add_enabled(!self.recovery_busy, egui::Button::new("Delete snapshot…"))
                            .clicked()
                        {
                            chosen = Some((o.id, false))
                        }
                    });
                }
            });
        ui.add_space(8.);
        ui.label(RichText::new("Restoring keeps the source snapshot. Missing or conflicting disk files are not recreated or force-saved.").small().color(p.muted));
        if let Some(path) = open {
            self.request_project(path);
        }
        if let Some((id, restore)) = chosen {
            if restore {
                self.recovery_busy = true;
                self.effects.push(Effect::RestoreDrafts(id))
            } else {
                self.dialog = Some(Dialog::ForgetRecovery(id))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kedr_workbench::recovery::Snapshot;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "kedr-recovery-ui-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(&p).unwrap();
            Self(p.canonicalize().unwrap())
        }
        fn shell(&self) -> Shell {
            fs::write(self.0.join("a.rs"), "a 🌲\r\n").unwrap();
            fs::write(self.0.join("b.rs"), "b").unwrap();
            let mut w = Workspace::scan(&self.0).unwrap();
            w.open(self.0.join("a.rs")).unwrap();
            let mut s = Shell::new("Test", Settings::default());
            s.set_workspace(w);
            s
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn snapshot(s: &mut Shell) -> Snapshot {
        let w = s.workspace.as_mut().unwrap();
        w.active_mut().unwrap().insert("recovered ").unwrap();
        let snap = Snapshot::capture(w).unwrap();
        w.active_mut().unwrap().undo(false).unwrap();
        snap
    }
    #[test]
    fn restore_keeps_other_dirty_tabs_and_never_saves() {
        let f = Fixture::new();
        let mut s = f.shell();
        let snap = snapshot(&mut s);
        let w = s.workspace.as_mut().unwrap();
        w.open(f.0.join("b.rs")).unwrap();
        w.active_mut().unwrap().insert("other").unwrap();
        let g = s.recovery_guard().unwrap();
        s.accept_recovery(&g, snap.prepare(&f.0).unwrap()).unwrap();
        let w = s.workspace.as_ref().unwrap();
        assert_eq!(w.tabs.len(), 2);
        assert!(w.tabs.iter().all(|t| t.document.is_dirty()));
        assert_eq!(fs::read_to_string(f.0.join("a.rs")).unwrap(), "a 🌲\r\n");
    }
    #[test]
    fn restore_is_all_or_nothing_for_current_dirty_and_edited_tabs() {
        let f = Fixture::new();
        let mut s = f.shell();
        let snap = snapshot(&mut s);
        s.workspace
            .as_mut()
            .unwrap()
            .active_mut()
            .unwrap()
            .insert("current")
            .unwrap();
        let g = s.recovery_guard().unwrap();
        assert!(s.accept_recovery(&g, snap.prepare(&f.0).unwrap()).is_err());
        assert!(
            s.workspace
                .as_ref()
                .unwrap()
                .active()
                .unwrap()
                .document
                .buffer
                .to_text()
                .starts_with("current")
        );
        let g = s.recovery_guard().unwrap();
        s.workspace
            .as_mut()
            .unwrap()
            .active_mut()
            .unwrap()
            .insert("next")
            .unwrap();
        assert!(s.accept_recovery(&g, snap.prepare(&f.0).unwrap()).is_err());
    }
    #[test]
    fn restore_rejected_while_workspace_opening() {
        let f = Fixture::new();
        let mut s = f.shell();
        let snap = snapshot(&mut s);
        let g = s.recovery_guard().unwrap();
        s.loading = true;
        assert!(s.accept_recovery(&g, snap.prepare(&f.0).unwrap()).is_err());
        assert!(
            !s.workspace
                .as_ref()
                .unwrap()
                .active()
                .unwrap()
                .document
                .is_dirty()
        );
    }
    #[test]
    fn one_colliding_draft_prevents_any_partial_restore() {
        let f = Fixture::new();
        let mut s = f.shell();
        let mut snap = snapshot(&mut s);
        snap.drafts.push(kedr_workbench::recovery::Draft {
            path: f.0.join("b.rs"),
            baseline: "b".into(),
            text: "recovered b".into(),
            cursor: 0,
        });
        s.workspace
            .as_mut()
            .unwrap()
            .open(f.0.join("b.rs"))
            .unwrap();
        s.workspace
            .as_mut()
            .unwrap()
            .active_mut()
            .unwrap()
            .insert("current ")
            .unwrap();
        let g = s.recovery_guard().unwrap();
        assert!(s.accept_recovery(&g, snap.prepare(&f.0).unwrap()).is_err());
        let w = s.workspace.as_ref().unwrap();
        assert!(!w.tabs[0].document.is_dirty());
        assert_eq!(w.tabs[0].document.buffer.to_text(), "a 🌲\r\n");
        assert_eq!(w.tabs[1].document.buffer.to_text(), "current b");
    }
    #[test]
    fn incidental_enter_and_escape_do_not_restore_or_delete() {
        let f = Fixture::new();
        let mut s = f.shell();
        s.capture_state("recovery");
        let ctx = Context::default();
        let frame = |s: &mut Shell, key: Option<Key>| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(760., 520.),
                )),
                events: key
                    .map(|key| {
                        vec![egui::Event::Key {
                            key,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: Modifiers::NONE,
                        }]
                    })
                    .unwrap_or_default(),
                ..Default::default()
            };
            let mut effects = Vec::new();
            let _ = ctx.run(input, |ctx| effects = s.show(ctx));
            effects
        };
        assert!(frame(&mut s, None).is_empty());
        assert!(frame(&mut s, Some(Key::Enter)).is_empty());
        assert!(frame(&mut s, Some(Key::Escape)).is_empty());
        assert!(s.dialog.is_none());
        s.dialog = Some(Dialog::ForgetRecovery(0));
        assert!(frame(&mut s, None).is_empty());
        assert!(frame(&mut s, Some(Key::Enter)).is_empty());
        assert_eq!(s.recovery_offers.len(), 1);
    }
}
