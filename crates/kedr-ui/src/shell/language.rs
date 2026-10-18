use super::*;
use kedr_core::PositionEncoding;
use kedr_workbench::lsp::{
    DefinitionOutcome, DefinitionQuery, DefinitionRunner, ReferenceOutcome, ReferenceRunner,
};
pub(super) struct Usages {
    pub query: DefinitionQuery,
    pub result: ReferenceOutcome,
    pub selected: usize,
    pub fixture: bool,
}
impl Shell {
    pub fn definition_query(&self) -> Result<DefinitionQuery, String> {
        DefinitionQuery::capture(self.workspace.as_ref().ok_or("Open a workspace first")?)
    }
    pub fn set_definition(&mut self, runner: DefinitionRunner) {
        self.cancel_definition();
        self.definition = Some(runner);
        self.status = "Resolving definition…".into();
    }
    pub fn cancel_definition(&mut self) {
        self.usages = None;
        if matches!(self.dialog, Some(Dialog::Usages)) {
            self.dialog = None;
        }
        let had_references = self.references.take().is_some();
        if self.definition.take().is_some() || had_references {
            self.status = "Language request cancelled".into();
        }
    }
    pub(super) fn poll_definition(&mut self, ctx: &Context) {
        let Some(runner) = &self.definition else {
            return;
        };
        if !self
            .workspace
            .as_ref()
            .is_some_and(|w| runner.query.is_current(w))
        {
            self.cancel_definition();
            self.status = "Language request discarded: editor changed".into();
            return;
        }
        let Some(result) = runner.poll() else {
            ctx.request_repaint_after(std::time::Duration::from_millis(40));
            return;
        };
        self.definition = None;
        match result {
            Ok(Some(outcome)) => self.apply_definition(outcome),
            Ok(None) => self.status = "No definition found".into(),
            Err(error) => {
                self.status = "Language request failed".into();
                self.error(error);
            }
        }
    }
    pub(super) fn language_effect(&self) -> Effect {
        if self.language_action == Action::References {
            Effect::References
        } else {
            Effect::Definition
        }
    }
    pub fn set_references(&mut self, runner: ReferenceRunner) {
        self.cancel_definition();
        self.references = Some(runner);
        self.status = "Finding usages…".into();
    }
    pub(super) fn poll_references(&mut self, ctx: &Context) {
        if self.usages.as_ref().is_some_and(|u| {
            !self
                .workspace
                .as_ref()
                .is_some_and(|w| u.query.is_current(w))
        }) {
            self.usages = None;
            if matches!(self.dialog, Some(Dialog::Usages)) {
                self.dialog = None;
            }
            self.status = "Usages discarded: editor changed".into();
        }
        let Some(runner) = &self.references else {
            return;
        };
        if !self
            .workspace
            .as_ref()
            .is_some_and(|w| runner.query.is_current(w))
        {
            self.cancel_definition();
            self.status = "Language request discarded: editor changed".into();
            return;
        }
        let Some(result) = runner.poll() else {
            ctx.request_repaint_after(std::time::Duration::from_millis(40));
            return;
        };
        let query = runner.query.clone();
        self.references = None;
        match result {
            Ok(result) => {
                self.status = format!("{} usages found", result.hits.len());
                self.usages = Some(Usages {
                    query,
                    result,
                    selected: 0,
                    fixture: false,
                });
                self.dialog = Some(Dialog::Usages);
            }
            Err(error) => {
                self.status = "Language request failed".into();
                self.error(error);
            }
        }
    }
    pub(super) fn open_usage(&mut self, index: usize) {
        let Some(mut usages) = self.usages.take() else {
            return;
        };
        self.dialog = None;
        if !self
            .workspace
            .as_ref()
            .is_some_and(|w| usages.query.is_current(w))
        {
            self.error("Usages are stale; run Find usages again");
            return;
        }
        let Some(hit) = usages.result.hits.get(index) else {
            self.error("Usage is no longer available");
            return;
        };
        let target = hit.target.clone();
        let document = usages
            .result
            .documents
            .iter()
            .position(|d| d.path == target.path)
            .map(|i| usages.result.documents.remove(i));
        self.apply_definition(DefinitionOutcome {
            target,
            document,
            alternatives: 0,
        });
        if self.status == "Definition selected" {
            self.status = "Usage selected".into();
        }
    }
    pub(super) fn apply_definition(&mut self, mut outcome: DefinitionOutcome) {
        let Some(w) = &self.workspace else { return };
        if !w.project.files.contains(&outcome.target.path) {
            self.error("Definition is outside the scanned workspace");
            return;
        }
        let buffer = w
            .tabs
            .iter()
            .find(|t| t.document.path == outcome.target.path)
            .map(|t| &t.document.buffer)
            .or_else(|| {
                outcome
                    .document
                    .as_ref()
                    .filter(|d| d.path == outcome.target.path)
                    .map(|d| &d.buffer)
            });
        let Some(buffer) = buffer else {
            self.error("Definition source is no longer available");
            return;
        };
        let at = match buffer.position_to_byte_in(
            outcome.target.line,
            outcome.target.character,
            PositionEncoding::Utf16,
        ) {
            Ok(at) => at,
            Err(_) => {
                self.error("Definition position is invalid; request it again");
                return;
            }
        };
        self.checkpoint();
        self.pending_hit = None;
        self.pending_diagnostic = None;
        self.restoring = None;
        if let Some(document) = outcome.document.take() {
            if let Err(e) = self.workspace.as_mut().unwrap().accept_document(document) {
                self.error(e.to_string());
                return;
            }
            self.rebuild_nav();
        }
        // Commit the final position before recording history; do not insert an intermediate tab position.
        if !self
            .workspace
            .as_mut()
            .unwrap()
            .activate_path(&outcome.target.path)
        {
            self.error("Could not activate definition source");
            return;
        }
        self.workspace
            .as_mut()
            .unwrap()
            .active_mut()
            .unwrap()
            .set_cursor(at, false);
        self.checkpoint();
        self.observe_location();
        self.editor_cache.focus_next = true;
        self.status = if outcome.alternatives == 0 {
            "Definition selected".into()
        } else {
            format!(
                "First definition selected ({} alternatives)",
                outcome.alternatives
            )
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kedr_workbench::lsp::DefinitionTarget;
    fn fixture() -> (Shell, PathBuf) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/studio")
            .canonicalize()
            .unwrap();
        let mut s = Shell::new("CodeCedar", Settings::default());
        s.set_workspace(Workspace::scan(&root).unwrap());
        s.accept_document(Document::open(root.join("src/main.rs")).unwrap());
        (s, root)
    }
    #[test]
    fn definition_requires_separate_explicit_language_authorization() {
        let (mut s, root) = fixture();
        s.trusted = true; // Cargo authorization is not language authorization.
        s.action(Action::Definition);
        assert!(matches!(s.dialog, Some(Dialog::LanguageTrust)));
        assert!(s.effects.is_empty());
        assert!(s.definition.is_none());
        s.language_trusted = true;
        s.dialog = None;
        s.action(Action::Definition);
        assert!(matches!(s.effects.pop(), Some(Effect::Definition)));
        s.set_workspace(Workspace::scan(root).unwrap());
        assert!(!s.language_trusted);
    }
    #[test]
    fn definition_uses_unsaved_target_utf16_and_history_without_replacing_text() {
        let (mut s, root) = fixture();
        let source = root.join("src/main.rs");
        let target = root.join("src/garden.rs");
        s.accept_document(Document::open(&target).unwrap());
        s.workspace
            .as_mut()
            .unwrap()
            .active_mut()
            .unwrap()
            .document
            .buffer
            .replace(0..0, "// 🌲é\n")
            .unwrap();
        let original = s
            .workspace
            .as_ref()
            .unwrap()
            .active()
            .unwrap()
            .document
            .buffer
            .to_text();
        s.activate_path(&source);
        s.apply_definition(DefinitionOutcome {
            target: DefinitionTarget {
                path: target.clone(),
                line: 0,
                character: 6,
            },
            document: None,
            alternatives: 0,
        });
        let tab = s.workspace.as_ref().unwrap().active().unwrap();
        assert_eq!(tab.document.path, target);
        assert_eq!(tab.cursor, "// 🌲é".len());
        assert_eq!(tab.document.buffer.to_text(), original);
        assert!(tab.document.is_dirty());
        s.action(Action::Back);
        assert_eq!(
            s.workspace
                .as_ref()
                .unwrap()
                .active()
                .unwrap()
                .document
                .path,
            source
        );
    }
    #[test]
    fn definition_opens_background_loaded_document_and_rejects_invalid_utf16() {
        let (mut s, root) = fixture();
        let source = root.join("src/main.rs");
        let target = root.join("src/garden.rs");
        s.apply_definition(DefinitionOutcome {
            target: DefinitionTarget {
                path: target.clone(),
                line: 5,
                character: 0,
            },
            document: Some(Document::open(&target).unwrap()),
            alternatives: 1,
        });
        assert_eq!(
            s.workspace.as_ref().unwrap().active().unwrap().position(),
            (5, 0)
        );
        assert!(s.status.contains("1 alternatives"));
        s.activate_path(&source);
        s.workspace
            .as_mut()
            .unwrap()
            .active_mut()
            .unwrap()
            .document
            .buffer
            .replace(0..0, "🌲")
            .unwrap();
        s.apply_definition(DefinitionOutcome {
            target: DefinitionTarget {
                path: source.clone(),
                line: 0,
                character: 1,
            },
            document: None,
            alternatives: 0,
        });
        assert!(s.error.as_ref().unwrap().contains("invalid"));
        assert_eq!(
            s.workspace
                .as_ref()
                .unwrap()
                .active()
                .unwrap()
                .document
                .path,
            source
        );
    }
}

#[cfg(test)]
mod same_file_test {
    use super::*;
    use kedr_workbench::lsp::DefinitionTarget;
    #[test]
    fn same_file_definition_is_an_explicit_reversible_checkpoint() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/studio")
            .canonicalize()
            .unwrap();
        let mut s = Shell::new("CodeCedar", Settings::default());
        s.set_workspace(Workspace::scan(&root).unwrap());
        let path = root.join("src/main.rs");
        s.accept_document(Document::open(&path).unwrap());
        s.apply_definition(DefinitionOutcome {
            target: DefinitionTarget {
                path: path.clone(),
                line: 20,
                character: 4,
            },
            document: None,
            alternatives: 0,
        });
        assert_eq!(
            s.workspace.as_ref().unwrap().active().unwrap().position(),
            (20, 4)
        );
        s.action(Action::Back);
        assert_eq!(
            s.workspace.as_ref().unwrap().active().unwrap().position(),
            (0, 0)
        );
        s.action(Action::Forward);
        assert_eq!(
            s.workspace.as_ref().unwrap().active().unwrap().position(),
            (20, 4)
        );
    }
}

#[cfg(test)]
mod usage_tests {
    use super::*;
    use kedr_workbench::lsp::{DefinitionTarget, ReferenceHit};
    fn shell() -> Shell {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/studio");
        let mut s = Shell::new("CodeCedar", Settings::default());
        s.set_workspace(Workspace::scan(root).unwrap());
        let path = s
            .workspace
            .as_ref()
            .unwrap()
            .project
            .root
            .join("src/main.rs");
        s.accept_document(Document::open(path).unwrap());
        s
    }
    fn results(s: &mut Shell, path: PathBuf, line: usize, document: Option<Document>) {
        s.usages = Some(Usages {
            query: s.definition_query().unwrap(),
            result: ReferenceOutcome {
                hits: vec![ReferenceHit {
                    target: DefinitionTarget {
                        path,
                        line,
                        character: 0,
                    },
                    preview: "preview".into(),
                }],
                documents: document.into_iter().collect(),
            },
            selected: 0,
            fixture: false,
        });
        s.dialog = Some(Dialog::Usages);
    }
    #[test]
    fn references_use_separate_language_permission_and_correct_effect() {
        let mut s = shell();
        s.trusted = true;
        s.action(Action::References);
        assert!(matches!(s.dialog, Some(Dialog::LanguageTrust)));
        assert!(s.effects.is_empty());
        assert!(matches!(s.language_effect(), Effect::References));
        s.dialog = None;
        s.language_trusted = true;
        s.action(Action::References);
        assert!(matches!(s.effects.pop(), Some(Effect::References)));
    }
    #[test]
    fn usage_selection_loads_closed_target_and_records_back_history() {
        let mut s = shell();
        let root = s.workspace.as_ref().unwrap().project.root.clone();
        let source = root.join("src/main.rs");
        let target = root.join("src/garden.rs");
        results(
            &mut s,
            target.clone(),
            5,
            Some(Document::open(&target).unwrap()),
        );
        s.open_usage(0);
        assert!(s.usages.is_none());
        assert!(s.dialog.is_none());
        assert_eq!(
            s.workspace
                .as_ref()
                .unwrap()
                .active()
                .unwrap()
                .document
                .path,
            target
        );
        assert_eq!(
            s.workspace.as_ref().unwrap().active().unwrap().position(),
            (5, 0)
        );
        s.action(Action::Back);
        assert_eq!(
            s.workspace
                .as_ref()
                .unwrap()
                .active()
                .unwrap()
                .document
                .path,
            source
        );
    }
    #[test]
    fn usage_selection_preserves_dirty_buffer_and_rejects_stale_results() {
        let mut s = shell();
        let path = s
            .workspace
            .as_ref()
            .unwrap()
            .active()
            .unwrap()
            .document
            .path
            .clone();
        s.workspace
            .as_mut()
            .unwrap()
            .active_mut()
            .unwrap()
            .document
            .buffer
            .replace(0..0, "// unsaved\n")
            .unwrap();
        results(&mut s, path.clone(), 1, None);
        s.open_usage(0);
        assert!(
            s.workspace
                .as_ref()
                .unwrap()
                .active()
                .unwrap()
                .document
                .is_dirty()
        );
        results(&mut s, path, 1, None);
        s.workspace
            .as_mut()
            .unwrap()
            .active_mut()
            .unwrap()
            .document
            .buffer
            .replace(0..0, "// changed\n")
            .unwrap();
        s.poll_references(&Context::default());
        assert!(s.usages.is_none());
        assert!(s.dialog.is_none());
        assert!(s.status.contains("discarded"));
    }
    #[test]
    fn usage_selection_revalidates_even_without_frame_poll_and_checks_index() {
        let mut s = shell();
        let path = s
            .workspace
            .as_ref()
            .unwrap()
            .active()
            .unwrap()
            .document
            .path
            .clone();
        results(&mut s, path.clone(), 1, None);
        s.workspace
            .as_mut()
            .unwrap()
            .active_mut()
            .unwrap()
            .set_cursor(1, false);
        s.open_usage(0);
        assert!(s.error.take().unwrap().contains("stale"));
        results(&mut s, path, 1, None);
        s.open_usage(99);
        assert!(s.error.take().unwrap().contains("no longer"));
    }
}

#[cfg(test)]
mod usage_input_tests {
    use super::*;
    fn shell() -> Shell {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/studio");
        let mut s = Shell::new("CodeCedar", Settings::default());
        s.set_workspace(Workspace::scan(root).unwrap());
        let path = s
            .workspace
            .as_ref()
            .unwrap()
            .project
            .root
            .join("src/main.rs");
        s.accept_document(Document::open(path).unwrap());
        s
    }
    fn frame(s: &mut Shell, ctx: &Context, key: Option<Key>) {
        let events = key
            .map(|key| egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            })
            .into_iter()
            .collect();
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(760., 520.),
                )),
                events,
                focused: true,
                ..Default::default()
            },
            |ctx| {
                s.show(ctx);
            },
        );
    }
    #[test]
    fn usage_modal_arrows_and_enter_select_actual_target() {
        let mut s = shell();
        s.capture_state("usages");
        let ctx = Context::default();
        frame(&mut s, &ctx, None);
        frame(&mut s, &ctx, Some(Key::ArrowDown));
        assert_eq!(s.usages.as_ref().unwrap().selected, 1);
        frame(&mut s, &ctx, Some(Key::Enter));
        assert!(s.usages.is_none());
        assert!(s.dialog.is_none());
        assert_eq!(
            s.workspace.as_ref().unwrap().active().unwrap().position(),
            (21, 25)
        );
        s.action(Action::Back);
        assert_eq!(
            s.workspace.as_ref().unwrap().active().unwrap().position(),
            (0, 0)
        );
    }
    #[test]
    fn empty_usage_modal_ignores_enter_and_escape_releases_snapshot() {
        let mut s = shell();
        s.capture_state("usages-empty");
        let ctx = Context::default();
        frame(&mut s, &ctx, None);
        frame(&mut s, &ctx, Some(Key::Enter));
        assert!(matches!(s.dialog, Some(Dialog::Usages)));
        frame(&mut s, &ctx, Some(Key::Escape));
        assert!(s.dialog.is_none());
        assert!(s.usages.is_none());
    }
}
