use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use walkdir::WalkDir;

/// Tipo de pasta de dependência.
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

/// Uma pasta de dependência encontrada num projeto.
#[derive(Debug, Clone)]
pub struct DepDir {
    pub path: PathBuf,
    pub size: u64,
    pub kind: DepKind,
}

/// Um projeto considerado "inativo" com suas pastas de dependência.
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

fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

fn is_rust_target(target_path: &Path) -> bool {
    target_path
        .parent()
        .map(|p| p.join("Cargo.toml").exists())
        .unwrap_or(false)
}

fn is_node_project(nm_path: &Path) -> bool {
    nm_path
        .parent()
        .map(|p| p.join("package.json").exists())
        .unwrap_or(false)
}

fn is_next_project(next_path: &Path) -> bool {
    if let Some(parent) = next_path.parent() {
        if parent.join("package.json").exists() {
            return true;
        }
        for ext in &["js", "mjs", "ts"] {
            if parent.join(format!("next.config.{}", ext)).exists() {
                return true;
            }
        }
    }
    false
}

fn is_python_venv(venv_path: &Path) -> bool {
    venv_path.join("pyvenv.cfg").exists()
        || venv_path.join("bin/python").exists()
        || venv_path.join("Scripts/python.exe").exists()
}

fn is_go_vendor(vendor_path: &Path) -> bool {
    vendor_path
        .parent()
        .map(|p| p.join("go.mod").exists())
        .unwrap_or(false)
}

fn is_gradle_build(build_path: &Path) -> bool {
    if let Some(parent) = build_path.parent() {
        return parent.join("build.gradle").exists()
            || parent.join("build.gradle.kts").exists();
    }
    false
}

/// Retorna o mtime mais recente dos arquivos-fonte de um projeto,
/// ignorando pastas de dependências e `.git`.
fn latest_source_mtime(project_dir: &Path) -> Option<SystemTime> {
    let skip_dirs: Vec<&str> = vec![
        "node_modules", "target", ".next", "dist", "build",
        ".git", "venv", ".venv", "vendor",
    ];
    let mut latest: Option<SystemTime> = None;

    let config_files = [
        "Cargo.toml",
        "Cargo.lock",
        "package.json",
        "package-lock.json",
        "yarn.lock",
        "pnpm-lock.yaml",
        "tsconfig.json",
        "next.config.js",
        "next.config.mjs",
        "next.config.ts",
        "requirements.txt",
        "setup.py",
        "pyproject.toml",
        "go.mod",
        "go.sum",
        "build.gradle",
        "build.gradle.kts",
    ];

    for f in &config_files {
        let p = project_dir.join(f);
        if let Ok(meta) = fs::metadata(&p) {
            if let Ok(mtime) = meta.modified() {
                latest = Some(match latest {
                    Some(current) => current.max(mtime),
                    None => mtime,
                });
            }
        }
    }

    for entry in WalkDir::new(project_dir)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !skip_dirs.contains(&name.as_ref())
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(mtime) = meta.modified() {
                    latest = Some(match latest {
                        Some(current) => current.max(mtime),
                        None => mtime,
                    });
                }
            }
        }
    }

    latest
}

/// Varre recursivamente `root` e retorna projetos inativos há `days` dias.
pub fn scan_projects(root: &Path, days: u64) -> Vec<StaleProject> {
    let threshold = SystemTime::now() - Duration::from_secs(days * 24 * 3600);
    let mut project_deps: HashMap<PathBuf, Vec<DepDir>> = HashMap::new();

    let mut it = WalkDir::new(root).into_iter();

    while let Some(entry) = it.next() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.file_type().is_dir() {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy();
        let dir_path = entry.path().to_path_buf();

        if dir_name == ".git" {
            it.skip_current_dir();
            continue;
        }

        let dep = match dir_name.as_ref() {
            "node_modules" if is_node_project(&dir_path) => {
                Some((DepKind::NodeModules, dir_path.clone()))
            }
            "target" if is_rust_target(&dir_path) => {
                Some((DepKind::Target, dir_path.clone()))
            }
            ".next" if is_next_project(&dir_path) => {
                Some((DepKind::NextBuild, dir_path.clone()))
            }
            "venv" | ".venv" if is_python_venv(&dir_path) => {
                Some((DepKind::Venv, dir_path.clone()))
            }
            "vendor" if is_go_vendor(&dir_path) => {
                Some((DepKind::Vendor, dir_path.clone()))
            }
            "build" if is_gradle_build(&dir_path) => {
                Some((DepKind::Build, dir_path.clone()))
            }
            _ => None,
        };

        if let Some((kind, dep_path)) = dep {
            if let Some(parent) = dep_path.parent() {
                project_deps
                    .entry(parent.to_path_buf())
                    .or_default()
                    .push(DepDir {
                        path: dep_path,
                        size: 0,
                        kind,
                    });
            }
            it.skip_current_dir();
        }
    }

    let mut stale: Vec<StaleProject> = Vec::new();

    for (project_path, mut deps) in project_deps {
        let last_modified = match latest_source_mtime(&project_path) {
            Some(t) => t,
            None => continue,
        };

        if last_modified > threshold {
            continue;
        }

        for dep in &mut deps {
            dep.size = dir_size(&dep.path);
        }

        deps.sort_by(|a, b| a.path.cmp(&b.path));

        let name = project_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| project_path.display().to_string());

        stale.push(StaleProject {
            name,
            path: project_path,
            dep_dirs: deps,
            last_modified,
        });
    }

    stale.sort_by(|a, b| b.total_size().cmp(&a.total_size()));
    stale
}
