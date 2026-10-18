use super::*;
use kedr_core::PositionEncoding;
use kedr_workbench::lsp::{DefinitionOutcome, DefinitionQuery, DefinitionRunner};
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
        if self.definition.take().is_some() {
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
