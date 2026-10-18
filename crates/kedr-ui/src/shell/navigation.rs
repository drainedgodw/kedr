use super::*;
use kedr_workbench::Location;
impl Shell {
    fn location(&self) -> Option<Location> {
        let tab = self.workspace.as_ref()?.active()?;
        let (line, column) = tab.position();
        Some(Location {
            path: tab.document.path.clone(),
            line: line + 1,
            column: column + 1,
        })
    }
    pub(super) fn observe_location(&mut self) {
        if self.restoring.is_some() {
            return;
        }
        if let Some(location) = self.location() {
            self.touch_recent(&location.path);
            self.history.observe(location);
        }
    }
    pub(super) fn checkpoint(&mut self) {
        if let Some(location) = self.location() {
            self.history.visit(location);
        }
    }
    fn touch_recent(&mut self, path: &Path) {
        if self.recent_files.first().is_some_and(|p| p == path) {
            return;
        }
        self.recent_files.retain(|p| p != path);
        self.recent_files.insert(0, path.to_path_buf());
        self.recent_files.truncate(50);
    }
    pub(super) fn history_step(&mut self, forward: bool) {
        if self.restoring.is_some() {
            return;
        }
        let target = if forward {
            self.history.forward()
        } else {
            self.history.back()
        };
        let Some(target) = target else { return };
        let valid = self
            .workspace
            .as_ref()
            .is_some_and(|w| w.project.files.contains(&target.path));
        if !valid {
            self.error("History file is no longer in this workspace");
            return;
        }
        self.pending_hit = None;
        self.pending_diagnostic = None;
        self.restoring = Some(target.clone());
        if !self.activate_path(&target.path) {
            self.effects.push(Effect::OpenFile(target.path));
        }
    }
    pub(super) fn palette_files(&self, query: &str, recent: bool) -> Vec<PathBuf> {
        let Some(w) = &self.workspace else {
            return Vec::new();
        };
        if !recent && !query.trim().is_empty() {
            return w.quick_files(query, 50);
        }
        let mut paths: Vec<_> = self
            .recent_files
            .iter()
            .filter(|p| w.project.files.contains(p))
            .filter(|p| {
                p.strip_prefix(&w.project.root)
                    .ok()
                    .is_some_and(|relative| {
                        kedr_workbench::fuzzy_score(&relative.to_string_lossy(), query).is_some()
                    })
            })
            .cloned()
            .collect();
        if !recent {
            for path in w.quick_files(query, 50) {
                if !paths.contains(&path) {
                    paths.push(path);
                }
            }
        }
        paths.truncate(50);
        paths
    }
}
impl Shell {
    pub(super) fn action_available(&self, action: Action) -> bool {
        match action {
            Action::Definition => {
                self.definition.is_none()
                    && self
                        .workspace
                        .as_ref()
                        .and_then(Workspace::active)
                        .is_some_and(|t| {
                            t.document.path.extension().and_then(|s| s.to_str()) == Some("rs")
                        })
            }
            Action::CancelLanguage => self.definition.is_some(),
            Action::Back => self.history.can_back() && self.restoring.is_none(),
            Action::Forward => self.history.can_forward() && self.restoring.is_none(),
            Action::RecentFiles | Action::QuickOpen => self.workspace.is_some(),
            Action::Check | Action::Test => self
                .workspace
                .as_ref()
                .is_some_and(|w| w.project.files.contains(&w.project.root.join("Cargo.toml"))),
            _ => true,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Shell, PathBuf) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/studio")
            .canonicalize()
            .unwrap();
        let mut shell = Shell::new("CodeCedar", Settings::default());
        shell.set_workspace(Workspace::scan(&root).unwrap());
        (shell, root)
    }
    fn open(s: &mut Shell, path: &Path) {
        s.accept_document(Document::open(path).unwrap());
    }
    #[test]
    fn back_and_forward_restore_open_tab_positions() {
        let (mut s, root) = fixture();
        let a = root.join("src/main.rs");
        let b = root.join("src/garden.rs");
        open(&mut s, &a);
        s.workspace
            .as_mut()
            .unwrap()
            .active_mut()
            .unwrap()
            .goto(6, 4)
            .unwrap();
        s.observe_location();
        open(&mut s, &b);
        s.workspace
            .as_mut()
            .unwrap()
            .active_mut()
            .unwrap()
            .goto(8, 1)
            .unwrap();
        s.observe_location();
        s.action(Action::Back);
        let tab = s.workspace.as_ref().unwrap().active().unwrap();
        assert_eq!(tab.document.path, a);
        assert_eq!(tab.position(), (5, 3));
        s.action(Action::Forward);
        let tab = s.workspace.as_ref().unwrap().active().unwrap();
        assert_eq!(tab.document.path, b);
        assert_eq!(tab.position(), (7, 0));
    }
    #[test]
    fn closed_file_restoration_waits_for_background_document() {
        let (mut s, root) = fixture();
        let a = root.join("src/main.rs");
        let b = root.join("src/garden.rs");
        open(&mut s, &a);
        s.workspace
            .as_mut()
            .unwrap()
            .active_mut()
            .unwrap()
            .goto(6, 1)
            .unwrap();
        s.observe_location();
        open(&mut s, &b);
        s.workspace.as_mut().unwrap().close_clean(0).unwrap();
        s.action(Action::Back);
        assert!(s.restoring.is_some());
        assert!(matches!(s.effects.last(),Some(Effect::OpenFile(path)) if path==&a));
        s.observe_location();
        assert_eq!(s.history.current().unwrap().path, a);
        open(&mut s, &a);
        assert!(s.restoring.is_none());
        assert_eq!(
            s.workspace.as_ref().unwrap().active().unwrap().position(),
            (5, 0)
        );
    }
    #[test]
    fn diagnostic_same_file_jump_creates_checkpoint() {
        let (mut s, root) = fixture();
        let a = root.join("src/main.rs");
        open(&mut s, &a);
        s.workspace
            .as_mut()
            .unwrap()
            .active_mut()
            .unwrap()
            .goto(3, 1)
            .unwrap();
        s.observe_location();
        s.navigate_diagnostic(DiagnosticLocation {
            file: a.clone(),
            line: 12,
            column: 1,
        });
        assert!(s.activate_path(&a));
        s.action(Action::Back);
        assert_eq!(
            s.workspace.as_ref().unwrap().active().unwrap().position(),
            (2, 0)
        );
        s.action(Action::Forward);
        assert_eq!(
            s.workspace.as_ref().unwrap().active().unwrap().position(),
            (11, 0)
        );
    }
    #[test]
    fn recent_files_are_unique_filtered_and_workspace_local() {
        let (mut s, root) = fixture();
        let a = root.join("src/main.rs");
        let b = root.join("src/garden.rs");
        open(&mut s, &a);
        open(&mut s, &b);
        assert!(s.activate_path(&a));
        assert_eq!(s.palette_files("", true), vec![a.clone(), b.clone()]);
        assert_eq!(s.palette_files("grs", true), vec![b]);
        // Absolute checkout/build directories must not participate in matching.
        assert!(s.palette_files(&root.to_string_lossy(), true).is_empty());
        assert_eq!(s.palette_files("", false).first(), Some(&a));
        s.set_workspace(Workspace::scan(&root).unwrap());
        assert!(s.palette_files("", true).is_empty());
        assert!(!s.history.can_back());
    }
    #[test]
    fn history_does_not_discard_unsaved_edits() {
        let (mut s, root) = fixture();
        let a = root.join("src/main.rs");
        let b = root.join("src/garden.rs");
        open(&mut s, &a);
        s.workspace
            .as_mut()
            .unwrap()
            .active_mut()
            .unwrap()
            .insert("// local edit\n")
            .unwrap();
        s.observe_location();
        open(&mut s, &b);
        s.action(Action::Back);
        let tab = s.workspace.as_mut().unwrap().active_mut().unwrap();
        tab.reconcile_dirty();
        assert!(tab.cached_dirty);
        assert!(tab.document.buffer.to_text().starts_with("// local edit"));
    }
}
