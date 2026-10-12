use crate::EditorTab;
use kedr_core::{
    document::Document,
    project::{Project, ScanOptions},
};
use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
};
#[derive(Debug, Default)]
pub struct FileNode {
    pub directories: BTreeMap<String, FileNode>,
    pub files: Vec<PathBuf>,
}
#[derive(Debug)]
pub struct Workspace {
    pub project: Project,
    pub tree: FileNode,
    pub tabs: Vec<EditorTab>,
    pub active: usize,
}
impl Workspace {
    pub fn scan(path: impl AsRef<Path>) -> io::Result<Self> {
        let project = Project::scan(path, &ScanOptions::default())?;
        let mut tree = FileNode::default();
        for file in &project.files {
            let relative = file.strip_prefix(&project.root).unwrap();
            let mut node = &mut tree;
            if let Some(parent) = relative.parent() {
                for part in parent.components() {
                    node = node
                        .directories
                        .entry(part.as_os_str().to_string_lossy().into())
                        .or_default();
                }
            }
            node.files.push(file.clone());
        }
        Ok(Self {
            project,
            tree,
            tabs: Vec::new(),
            active: 0,
        })
    }
    pub fn name(&self) -> String {
        self.project
            .root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }
    pub fn active(&self) -> Option<&EditorTab> {
        self.tabs.get(self.active)
    }
    pub fn active_mut(&mut self) -> Option<&mut EditorTab> {
        self.tabs.get_mut(self.active)
    }
    pub fn activate_path(&mut self, path: &Path) -> bool {
        if let Some(i) = self.tabs.iter().position(|t| t.document.path == path) {
            self.active = i;
            self.tabs[i].follow_cursor = true;
            true
        } else {
            false
        }
    }
    pub fn accept_document(&mut self, doc: Document) -> io::Result<()> {
        if !doc.path.starts_with(&self.project.root) {
            return Err(io::Error::other("File is outside the current workspace"));
        }
        if !self.activate_path(&doc.path) {
            if self.tabs.len() >= 32 {
                return Err(io::Error::other(
                    "Close another file: this preview supports 32 open tabs",
                ));
            }
            self.tabs.push(EditorTab::from_document(doc));
            self.active = self.tabs.len() - 1;
        }
        Ok(())
    }
    pub fn open(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = std::fs::canonicalize(path)?;
        if !path.starts_with(&self.project.root) {
            return Err(io::Error::other("File is outside the current workspace"));
        }
        if self.activate_path(&path) {
            return Ok(());
        }
        self.accept_document(Document::open(path)?)
    }
    pub fn close_clean(&mut self, index: usize) -> Result<(), String> {
        let tab = self.tabs.get_mut(index).ok_or("Unknown tab")?;
        tab.reconcile_dirty();
        if tab.cached_dirty {
            return Err("Unsaved changes require an explicit decision".into());
        }
        self.discard(index);
        Ok(())
    }
    pub fn discard(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        if self.active > index {
            self.active -= 1;
        }
        self.active = self.active.min(self.tabs.len().saturating_sub(1));
    }
    pub fn dirty_count(&mut self) -> usize {
        for tab in &mut self.tabs {
            tab.reconcile_dirty();
        }
        self.tabs.iter().filter(|t| t.cached_dirty).count()
    }
    pub fn save_all(&mut self) -> io::Result<()> {
        for tab in &mut self.tabs {
            tab.reconcile_dirty();
            if tab.cached_dirty {
                tab.save()?;
            }
        }
        Ok(())
    }
    pub fn quick_files(&self, query: &str, limit: usize) -> Vec<PathBuf> {
        let mut found: Vec<_> = self
            .project
            .files
            .iter()
            .filter_map(|p| {
                fuzzy_score(
                    &p.strip_prefix(&self.project.root)
                        .unwrap()
                        .to_string_lossy(),
                    query,
                )
                .map(|score| (score, p))
            })
            .collect();
        found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
        found
            .into_iter()
            .take(limit)
            .map(|(_, p)| p.clone())
            .collect()
    }
}
pub fn fuzzy_score(text: &str, query: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }
    let text = text.to_lowercase();
    let query = query.to_lowercase();
    if let Some(at) = text.find(&query) {
        return Some(10000 - at as i64 - text.len() as i64);
    }
    let mut wanted = query.chars().peekable();
    let (mut score, mut last) = (0, None);
    for (i, c) in text.chars().enumerate() {
        if wanted.peek() == Some(&c) {
            wanted.next();
            score += if last == Some(i.saturating_sub(1)) {
                8
            } else {
                1
            };
            last = Some(i);
        }
    }
    wanted.next().is_none().then_some(score - text.len() as i64)
}
