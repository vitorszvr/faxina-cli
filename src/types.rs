use std::path::{Path, PathBuf};
use std::time::SystemTime;
use jwalk::WalkDir;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum DepKind {
    NodeModules,
    Target,
    NextBuild,
    Venv,
    Vendor,
    Build,
}

impl DepKind {
    pub fn icon(&self) -> &'static str {
        match self {
            DepKind::NodeModules => "📦",
            DepKind::Target => "🦀",
            DepKind::NextBuild => "▲ ",
            DepKind::Venv => "🐍",
            DepKind::Vendor => "🐹",
            DepKind::Build => "☕",
        }
    }
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
    /// Tamanho em bytes. Inicializado como `0` pelo scanner — valor real
    /// preenchido por `scanner::calculate_sizes()` antes de qualquer exibição.
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
        .filter(|e| !e.file_type().is_dir())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_dir_size() {
        let id = std::process::id();
        let temp_dir = std::env::temp_dir().join(format!("test_dir_size_{}", id));
        let _ = fs::remove_dir_all(&temp_dir); // Ensure clean start
        fs::create_dir_all(&temp_dir).unwrap();

        let file_a = temp_dir.join("a.txt");
        {
            let mut f = fs::File::create(&file_a).unwrap();
            f.write_all(&[0u8; 100]).unwrap(); // 100 bytes
        }

        let subdir = temp_dir.join("subdir");
        fs::create_dir(&subdir).unwrap();
        let file_b = subdir.join("b.txt");
        {
            let mut f = fs::File::create(&file_b).unwrap();
            f.write_all(&[0u8; 200]).unwrap(); // 200 bytes
        }

        let size = dir_size(&temp_dir);
        assert_eq!(size, 300, "Expected 300 bytes, got {}", size);

        let sub_size = dir_size(&subdir);
        assert_eq!(sub_size, 200, "Expected 200 bytes for subdir, got {}", sub_size);

        fs::remove_dir_all(&temp_dir).unwrap();
    }
}
