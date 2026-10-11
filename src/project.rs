use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub max_files: usize,
    pub max_directories: usize,
    pub max_depth: usize,
}
impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_files: 5000,
            max_directories: 10000,
            max_depth: 32,
        }
    }
}
#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    pub files: Vec<PathBuf>,
    pub truncated: bool,
    pub skipped_entries: usize,
    pub directories_visited: usize,
}
impl Project {
    pub fn scan(root: impl AsRef<Path>, options: &ScanOptions) -> io::Result<Self> {
        let root = fs::canonicalize(root)?;
        if !root.is_dir() {
            return Err(io::Error::other("Not a project directory"));
        }
        let mut project = Self {
            root: root.clone(),
            files: Vec::new(),
            truncated: false,
            skipped_entries: 0,
            directories_visited: 0,
        };
        let mut pending = vec![(root, 0)];
        while let Some((dir, depth)) = pending.pop() {
            if project.files.len() >= options.max_files
                || project.directories_visited >= options.max_directories
            {
                project.truncated = true;
                break;
            }
            if depth > options.max_depth {
                project.truncated = true;
                continue;
            }
            project.directories_visited += 1;
            let entries = match fs::read_dir(dir) {
                Ok(entries) => entries,
                Err(_) => {
                    project.skipped_entries += 1;
                    continue;
                }
            };
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => {
                        project.skipped_entries += 1;
                        continue;
                    }
                };
                let kind = match entry.file_type() {
                    Ok(k) => k,
                    Err(_) => {
                        project.skipped_entries += 1;
                        continue;
                    }
                };
                if kind.is_symlink() {
                    continue;
                }
                if kind.is_dir() {
                    let name = entry.file_name();
                    if [
                        ".git",
                        ".venv",
                        "venv",
                        "__pycache__",
                        "node_modules",
                        "target",
                    ]
                    .contains(&name.to_string_lossy().as_ref())
                    {
                        continue;
                    }
                    if pending.len() + project.directories_visited >= options.max_directories {
                        project.truncated = true;
                        continue;
                    }
                    pending.push((entry.path(), depth + 1));
                } else if kind.is_file() {
                    if project.files.len() >= options.max_files {
                        project.truncated = true;
                        break;
                    }
                    project.files.push(entry.path());
                }
            }
        }
        project.files.sort();
        Ok(project)
    }
}
