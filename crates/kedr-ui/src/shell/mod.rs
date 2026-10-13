mod chrome;
mod overlays;
mod panels;
use crate::{
    EditorCache, Icon, Palette, Theme, apply_theme, editor, icon_button, singleline, wordmark,
};
use egui::{Align, Align2, Color32, Context, FontId, Id, Key, Layout, Modifiers, RichText, Vec2};
use kedr_core::document::Document;
use kedr_workbench::{
    FileNode, SearchHit, SearchReport, TaskKind, TaskPhase, TaskRunner, Workspace,
};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Project,
    Replace,
    FindNext,
    FindPrevious,
    NextTab,
    PreviousTab,
    SelectAll,
    Duplicate,
    DeleteLine,
    GoToLine,
    Keybindings,
    OpenProject,
    QuickOpen,
    Commands,
    Save,
    SaveAll,
    Undo,
    Redo,
    Find,
    Search,
    Check,
    Test,
    CloseTab,
    Sidebar,
    Output,
    Settings,
}
impl Action {
    pub const ALL: [Self; 26] = [
        Self::Project,
        Self::Replace,
        Self::FindNext,
        Self::FindPrevious,
        Self::NextTab,
        Self::PreviousTab,
        Self::SelectAll,
        Self::Duplicate,
        Self::DeleteLine,
        Self::GoToLine,
        Self::Keybindings,
        Self::OpenProject,
        Self::QuickOpen,
        Self::Save,
        Self::SaveAll,
        Self::Undo,
        Self::Redo,
        Self::Find,
        Self::Search,
        Self::Check,
        Self::Test,
        Self::CloseTab,
        Self::Sidebar,
        Self::Output,
        Self::Settings,
        Self::Commands,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Project => "Show project files",
            Self::Replace => "Replace in file",
            Self::FindNext => "Find next occurrence",
            Self::FindPrevious => "Find previous occurrence",
            Self::NextTab => "Next editor tab",
            Self::PreviousTab => "Previous editor tab",
            Self::SelectAll => "Select all",
            Self::Duplicate => "Duplicate line or selection",
            Self::DeleteLine => "Delete current line",
            Self::GoToLine => "Go to line / column",
            Self::Keybindings => "Keyboard shortcuts",
            Self::OpenProject => "Open workspace",
            Self::QuickOpen => "Go to file",
            Self::Commands => "Command palette",
            Self::Save => "Save file",
            Self::SaveAll => "Save all files",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::Find => "Find in file",
            Self::Search => "Search saved project files",
            Self::Check => "Cargo check",
            Self::Test => "Cargo test",
            Self::CloseTab => "Close current tab",
            Self::Sidebar => "Toggle sidebar",
            Self::Output => "Toggle output panel",
            Self::Settings => "Appearance & settings",
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::Project => "workspace.project",
            Self::OpenProject => "workspace.open",
            Self::QuickOpen => "navigation.file",
            Self::Commands => "commands.search",
            Self::Save => "file.save",
            Self::SaveAll => "file.save_all",
            Self::Undo => "edit.undo",
            Self::Redo => "edit.redo",
            Self::Find => "search.find",
            Self::Search => "search.project",
            Self::Check => "cargo.check",
            Self::Test => "cargo.test",
            Self::CloseTab => "tab.close",
            Self::Sidebar => "view.sidebar",
            Self::Output => "view.output",
            Self::Settings => "settings.appearance",
            Self::Replace => "search.replace",
            Self::FindNext => "search.next",
            Self::FindPrevious => "search.previous",
            Self::NextTab => "tab.next",
            Self::PreviousTab => "tab.previous",
            Self::SelectAll => "edit.select_all",
            Self::Duplicate => "edit.duplicate",
            Self::DeleteLine => "edit.delete_line",
            Self::GoToLine => "navigation.line",
            Self::Keybindings => "settings.keymap",
        }
    }
}
#[derive(Debug)]
pub enum Effect {
    PickProject,
    OpenProject(PathBuf),
    OpenFile(PathBuf),
    Search(String),
    RunTask(TaskKind),
    Quit,
    PreferencesChanged,
}
#[derive(Clone)]
pub struct Settings {
    pub keymap: crate::Keymap,
    pub shortcuts: crate::ShortcutOverrides,
    pub theme: Theme,
    pub font_size: f32,
    pub sidebar_width: f32,
    pub show_sidebar: bool,
    pub show_output: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            keymap: crate::Keymap::default(),
            shortcuts: Default::default(),
            theme: Theme::Dark,
            font_size: 15.,
            sidebar_width: 278.,
            show_sidebar: true,
            show_output: false,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pane {
    Files,
    Search,
}
#[derive(Clone)]
struct NavEntry {
    label: String,
    path: PathBuf,
    depth: usize,
    folder: bool,
}
#[derive(Clone)]
enum Dialog {
    Settings,
    Keymap,
    EditShortcut(Action),
    GoToLine,
    Close(usize),
    Switch(PathBuf),
    Quit,
    Trust(TaskKind),
}
struct PaletteState {
    files: bool,
    query: String,
    selected: usize,
    focus: bool,
}
pub struct Shell {
    pub product: &'static str,
    pub workspace: Option<Workspace>,
    pub settings: Settings,
    pub recent: Vec<PathBuf>,
    pub loading: bool,
    pub search_loading: bool,
    pub search_report: Option<SearchReport>,
    pub task: Option<TaskRunner>,
    pub editor_cache: EditorCache,
    effects: Vec<Effect>,
    resolved_profile: crate::Keymap,
    resolved_overrides: crate::ShortcutOverrides,
    resolved_keys: Vec<crate::Binding>,
    pane: Pane,
    nav: Vec<NavEntry>,
    expanded: BTreeSet<PathBuf>,
    file_filter: String,
    palette: Option<PaletteState>,
    dialog: Option<Dialog>,
    find_open: bool,
    find: String,
    goto: String,
    shortcut_filter: String,
    shortcut_draft: String,
    shortcut_error: Option<String>,
    record_shortcut: bool,
    goto_focus: bool,
    replacement: String,
    find_focus: bool,
    search: String,
    search_focus: bool,
    status: String,
    error: Option<String>,
    trusted: bool,
    pending_hit: Option<SearchHit>,
    active_path: Option<PathBuf>,
}
impl Shell {
    pub fn new(product: &'static str, mut settings: Settings) -> Self {
        let shortcut_error = crate::validate_overrides(&settings.shortcuts).err();
        if shortcut_error.is_some() {
            settings.shortcuts.clear();
        }
        let resolved_profile = settings.keymap;
        let resolved_overrides = settings.shortcuts.clone();
        let resolved_keys = crate::resolved_bindings(
            resolved_profile,
            cfg!(target_os = "macos"),
            &resolved_overrides,
        )
        .unwrap_or_else(|_| crate::bindings(resolved_profile, cfg!(target_os = "macos")));
        Self {
            product,
            workspace: None,
            settings,
            recent: Vec::new(),
            loading: false,
            search_loading: false,
            search_report: None,
            task: None,
            editor_cache: EditorCache::default(),
            effects: Vec::new(),
            resolved_profile,
            resolved_overrides,
            resolved_keys,
            pane: Pane::Files,
            nav: Vec::new(),
            expanded: BTreeSet::new(),
            file_filter: String::new(),
            palette: None,
            dialog: None,
            find_open: false,
            find: String::new(),
            goto: String::new(),
            shortcut_filter: String::new(),
            shortcut_draft: String::new(),
            shortcut_error: None,
            record_shortcut: false,
            goto_focus: false,
            replacement: String::new(),
            find_focus: false,
            search: String::new(),
            search_focus: false,
            status: "Ready".into(),
            error: shortcut_error
                .map(|e| format!("Invalid custom shortcuts; presets restored: {e}")),
            trusted: false,
            pending_hit: None,
            active_path: None,
        }
    }
    pub fn error(&mut self, message: impl Into<String>) {
        self.error = Some(message.into());
    }
    pub fn set_workspace(&mut self, w: Workspace) {
        self.workspace = Some(w);
        self.trusted = false;
        self.task = None;
        self.loading = false;
        self.search_report = None;
        self.search_loading = false;
        self.expanded.clear();
        self.file_filter.clear();
        self.editor_cache.clear();
        self.active_path = None;
        self.rebuild_nav();
        self.status = "Workspace opened".into();
    }
    pub fn accept_document(&mut self, document: Document) {
        let path = document.path.clone();
        if let Some(w) = &mut self.workspace
            && let Err(e) = w.accept_document(document)
        {
            self.error(e.to_string());
            return;
        }
        if let Some(w) = &self.workspace {
            for parent in path
                .ancestors()
                .skip(1)
                .take_while(|p| p.starts_with(&w.project.root) && *p != w.project.root)
            {
                self.expanded.insert(parent.to_path_buf());
            }
        }
        self.rebuild_nav();
        self.finish_navigation();
        self.editor_cache.focus_next = true;
    }
    pub fn activate_path(&mut self, path: &Path) -> bool {
        let found = self
            .workspace
            .as_mut()
            .is_some_and(|w| w.activate_path(path));
        if found {
            self.finish_navigation();
            self.editor_cache.focus_next = true;
        }
        found
    }
    fn finish_navigation(&mut self) {
        if let Some(hit) = self.pending_hit.take()
            && let Some(tab) = self.workspace.as_mut().and_then(Workspace::active_mut)
        {
            if tab.document.path == hit.path
                && tab.document.buffer.slice(hit.range.clone()).ok().as_deref() == Some(&hit.needle)
            {
                tab.anchor = hit.range.start;
                tab.cursor = hit.range.end;
                tab.follow_cursor = true;
            } else {
                self.error("Search result changed; search again before navigating");
            }
        }
    }
    pub fn request_project(&mut self, path: PathBuf) {
        if self.workspace.as_mut().is_some_and(|w| w.dirty_count() > 0) {
            self.dialog = Some(Dialog::Switch(path));
        } else {
            self.effects.push(Effect::OpenProject(path));
        }
    }
    pub fn request_quit(&mut self) {
        if self.workspace.as_mut().is_some_and(|w| w.dirty_count() > 0) {
            self.dialog = Some(Dialog::Quit);
        } else {
            self.effects.push(Effect::Quit);
        }
    }
    pub fn action(&mut self, a: Action) {
        match a {
            Action::Keybindings => self.dialog = Some(Dialog::Keymap),
            Action::GoToLine => {
                self.goto.clear();
                self.goto_focus = true;
                self.dialog = Some(Dialog::GoToLine);
            }
            Action::Project => {
                self.pane = Pane::Files;
                self.settings.show_sidebar = true;
            }
            Action::Replace => {
                self.find_open = true;
                self.find_focus = true;
            }
            Action::FindNext | Action::FindPrevious => {
                if self.find.is_empty() {
                    self.find_open = true;
                    self.find_focus = true;
                } else if let Some(t) = self.workspace.as_mut().and_then(Workspace::active_mut) {
                    let found = if a == Action::FindNext {
                        t.find_next(&self.find)
                    } else {
                        t.find_previous(&self.find)
                    };
                    self.status = if found { "Match selected" } else { "No match" }.into();
                }
            }
            Action::NextTab | Action::PreviousTab => {
                if let Some(w) = &mut self.workspace
                    && !w.tabs.is_empty()
                {
                    let n = w.tabs.len();
                    w.active = (w.active + if a == Action::NextTab { 1 } else { n - 1 }) % n;
                    w.tabs[w.active].follow_cursor = true;
                    self.editor_cache.focus_next = true;
                }
            }
            Action::SelectAll | Action::Duplicate | Action::DeleteLine => {
                if let Some(t) = self.workspace.as_mut().and_then(Workspace::active_mut) {
                    let result = match a {
                        Action::SelectAll => {
                            t.select_all();
                            Ok(())
                        }
                        Action::Duplicate => t.duplicate(),
                        _ => t.delete_line(),
                    };
                    if let Err(e) = result {
                        self.error(e);
                    }
                }
            }
            Action::OpenProject => self.effects.push(Effect::PickProject),
            Action::QuickOpen | Action::Commands => {
                self.palette = Some(PaletteState {
                    files: a == Action::QuickOpen,
                    query: String::new(),
                    selected: 0,
                    focus: true,
                })
            }
            Action::Save => {
                if let Some(t) = self.workspace.as_mut().and_then(Workspace::active_mut) {
                    match t.save() {
                        Ok(()) => self.status = "Saved".into(),
                        Err(e) => self.error(e.to_string()),
                    }
                }
            }
            Action::SaveAll => {
                if let Some(w) = &mut self.workspace {
                    match w.save_all() {
                        Ok(()) => self.status = "All files saved".into(),
                        Err(e) => self.error(e.to_string()),
                    }
                }
            }
            Action::Undo | Action::Redo => {
                if let Some(t) = self.workspace.as_mut().and_then(Workspace::active_mut)
                    && let Err(e) = t.undo(a == Action::Redo)
                {
                    self.error(e)
                }
            }
            Action::Find => {
                self.find_open = !self.find_open;
                self.find_focus = self.find_open;
            }
            Action::Search => {
                self.pane = Pane::Search;
                self.settings.show_sidebar = true;
                self.search_focus = true;
            }
            Action::Check | Action::Test => {
                if self.workspace.is_none() {
                    self.error("Open a Cargo workspace first");
                } else if self
                    .task
                    .as_ref()
                    .is_some_and(|t| t.phase() == TaskPhase::Running)
                {
                    self.error("A task is already running");
                } else {
                    let kind = if a == Action::Check {
                        TaskKind::Check
                    } else {
                        TaskKind::Test
                    };
                    if self.trusted {
                        self.effects.push(Effect::RunTask(kind));
                    } else {
                        self.dialog = Some(Dialog::Trust(kind));
                    }
                }
            }
            Action::CloseTab => {
                if let Some(w) = &mut self.workspace
                    && !w.tabs.is_empty()
                {
                    let i = w.active;
                    if w.close_clean(i).is_err() {
                        self.dialog = Some(Dialog::Close(i));
                    }
                }
            }
            Action::Sidebar => self.settings.show_sidebar = !self.settings.show_sidebar,
            Action::Output => self.settings.show_output = !self.settings.show_output,
            Action::Settings => self.dialog = Some(Dialog::Settings),
        }
    }
    fn rebuild_nav(&mut self) {
        fn walk(
            n: &FileNode,
            root: &Path,
            depth: usize,
            expanded: &BTreeSet<PathBuf>,
            filter: &str,
            out: &mut Vec<NavEntry>,
        ) {
            for (name, child) in &n.directories {
                let path = root.join(name);
                out.push(NavEntry {
                    label: name.clone(),
                    path: path.clone(),
                    depth,
                    folder: true,
                });
                if expanded.contains(&path) || !filter.is_empty() {
                    walk(child, &path, depth + 1, expanded, filter, out);
                }
            }
            for path in &n.files {
                let label = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                if filter.is_empty()
                    || path
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&filter.to_lowercase())
                {
                    out.push(NavEntry {
                        label,
                        path: path.clone(),
                        depth,
                        folder: false,
                    });
                }
            }
        }
        self.nav.clear();
        if let Some(w) = &self.workspace {
            walk(
                &w.tree,
                &w.project.root,
                0,
                &self.expanded,
                &self.file_filter,
                &mut self.nav,
            );
        }
    }
    pub(super) fn refresh_shortcuts(&mut self) {
        if self.resolved_profile != self.settings.keymap
            || self.resolved_overrides != self.settings.shortcuts
        {
            self.resolved_keys = crate::resolved_bindings(
                self.settings.keymap,
                cfg!(target_os = "macos"),
                &self.settings.shortcuts,
            )
            .unwrap_or_else(|_| crate::bindings(self.settings.keymap, cfg!(target_os = "macos")));
            self.resolved_profile = self.settings.keymap;
            self.resolved_overrides = self.settings.shortcuts.clone();
        }
    }
    pub(super) fn shortcut(&self, action: Action) -> String {
        self.resolved_keys
            .iter()
            .find(|b| b.action == action)
            .map_or_else(
                || "—".into(),
                |b| {
                    if b.hint.is_empty() {
                        crate::Chord {
                            key: b.key,
                            modifiers: b.modifiers,
                        }
                        .label(cfg!(target_os = "macos"))
                    } else {
                        b.hint.into()
                    }
                },
            )
    }
    pub fn remap_shortcut(&mut self, action: Action, value: Option<&str>) -> Result<(), String> {
        crate::set_override(
            &mut self.settings.shortcuts,
            self.settings.keymap,
            cfg!(target_os = "macos"),
            action,
            value,
        )?;
        self.refresh_shortcuts();
        self.effects.push(Effect::PreferencesChanged);
        Ok(())
    }
    fn shortcuts(&mut self, ctx: &Context) {
        if self.dialog.is_some() {
            return;
        }
        let editor_focused = self
            .workspace
            .as_ref()
            .and_then(Workspace::active)
            .is_some_and(|t| ctx.memory(|m| m.has_focus(Id::new(("kedr-code", &t.document.path)))));
        for binding in self.resolved_keys.clone() {
            if binding.editor_only && (!editor_focused || self.palette.is_some()) {
                continue;
            }
            if ctx.input_mut(|i| binding.consume(i)) {
                self.action(binding.action);
            }
        }
        if self.palette.is_some() && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.palette = None;
            self.editor_cache.focus_next = true;
        } else if self.find_open && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.find_open = false;
            self.editor_cache.focus_next = true;
        }
    }
    pub fn show(&mut self, ctx: &Context) -> Vec<Effect> {
        self.refresh_shortcuts();
        self.shortcuts(ctx);
        let p = self.settings.theme.palette();
        self.header(ctx, p);
        self.status_bar(ctx, p);
        self.activity(ctx, p);
        if self.settings.show_sidebar && self.workspace.is_some() {
            self.sidebar(ctx, p);
        }
        if self.settings.show_output {
            self.output(ctx, p);
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.canvas).inner_margin(0))
            .show(ctx, |ui| {
                if self.loading {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.3);
                        ui.spinner();
                        ui.heading("Opening workspace");
                        ui.label("Scanning files in the background. Project code is not executed.");
                    });
                    return;
                }
                if self.workspace.is_none() {
                    self.welcome(ui, p);
                    return;
                }
                self.tabs(ui, p);
                if self.find_open {
                    self.find_bar(ui, p);
                }
                let path = self
                    .workspace
                    .as_ref()
                    .and_then(Workspace::active)
                    .map(|t| t.document.path.clone());
                if path != self.active_path {
                    self.active_path = path;
                    self.editor_cache.clear();
                    self.editor_cache.focus_next = true;
                }
                let enabled = self.dialog.is_none() && self.palette.is_none();
                let root = self
                    .workspace
                    .as_ref()
                    .map(|w| w.project.root.clone())
                    .unwrap_or_default();
                if let Some(tab) = self.workspace.as_mut().and_then(Workspace::active_mut) {
                    let relative = tab
                        .document
                        .path
                        .strip_prefix(&root)
                        .unwrap_or(&tab.document.path)
                        .to_string_lossy()
                        .into_owned();
                    ui.horizontal(|ui| {
                        ui.add_space(20.);
                        ui.add(
                            egui::Label::new(RichText::new(&relative).color(p.muted)).truncate(),
                        )
                        .on_hover_text(relative);
                    });
                    ui.add_space(8.);
                    if let Some(e) = editor(
                        ui,
                        tab,
                        &mut self.editor_cache,
                        self.settings.font_size,
                        self.settings.theme,
                        enabled,
                    ) {
                        self.error(e);
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.3);
                        ui.heading("Make room for your next idea.");
                        ui.label("Choose a file in the sidebar or press Ctrl P.");
                        if ui.button("Go to file").clicked() {
                            self.action(Action::QuickOpen);
                        }
                    });
                }
            });
        self.palette_ui(ctx, p);
        self.dialog_ui(ctx, p);
        if let Some(message) = self.error.clone() {
            egui::Window::new("Something needs your attention")
                .id(Id::new("kedr-error"))
                .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-24., -48.))
                .resizable(false)
                .collapsible(false)
                .default_width(400.)
                .show(ctx, |ui| {
                    ui.label(RichText::new(message).color(p.amber));
                    if ui.button("Dismiss").clicked() {
                        self.error = None;
                    }
                });
        }
        std::mem::take(&mut self.effects)
    }
}
fn language(path: &Path) -> &'static str {
    match path.extension().and_then(|x| x.to_str()).unwrap_or("") {
        "rs" => "Rust",
        "py" => "Python",
        "js" | "jsx" => "JavaScript",
        "ts" | "tsx" => "TypeScript",
        "json" => "JSON",
        "toml" => "TOML",
        "md" => "Markdown",
        _ => "Plain text",
    }
}
