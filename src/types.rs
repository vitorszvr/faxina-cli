use std::path::{Path, PathBuf};
use std::time::SystemTime;
use jwalk::WalkDir;

#[derive(Debug, Clone)]
pub enum DepKind {
    NodeModules,
    Target,
    NextBuild,
    Venv,
    Vendor,
    Build,
}

impl std::fmt::Display for DepKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DepKind::NodeModules => write!(f, "node_modules"),
            DepKind::Target => write!(f, "target"),
            DepKind::NextBuild => write!(f, ".next"),
            DepKind::Venv => write!(f, "venv"),
            DepKind::Vendor => write!(f, "vendor"),
            DepKind::Build => write!(f, "build"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DepDir {
    pub path: PathBuf,
    pub size: u64,
    pub kind: DepKind,
}

#[derive(Debug, Clone)]
pub struct StaleProject {
    pub name: String,
    pub path: PathBuf,
    pub dep_dirs: Vec<DepDir>,
    pub last_modified: SystemTime,
}

impl StaleProject {
    pub fn total_size(&self) -> u64 {
        self.dep_dirs.iter().map(|d| d.size).sum()
    }
}

pub fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .skip_hidden(false)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}
