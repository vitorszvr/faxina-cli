use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use jwalk::WalkDir;

use crate::types::{DepDir, StaleProject, dir_size};
use crate::projects::all_project_types;

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

     for entry in walkdir::WalkDir::new(project_dir)
        .follow_links(false)
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

pub fn scan_projects(root: &Path, days: u64) -> Vec<StaleProject> {
    let threshold = SystemTime::now() - Duration::from_secs(days * 24 * 3600);
    let project_types = all_project_types();
    
    // Mutex para coletar resultados de threads paralelas
    let project_deps: Arc<Mutex<HashMap<PathBuf, Vec<DepDir>>>> = Arc::new(Mutex::new(HashMap::new()));
    
    WalkDir::new(root)
        .skip_hidden(false) 
        .follow_links(false)
        .process_read_dir(move |_depth, _path, _read_dir_state, children| {
            children.retain(|dir_entry_result| {
                dir_entry_result.as_ref().map(|entry| {
                    let name = entry.file_name().to_string_lossy();
                    name != ".git"
                }).unwrap_or(false)
            });
        })
        .into_iter()
        .for_each(|entry| {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => return,
            };

            if !entry.file_type().is_dir() {
                return;
            }

            let dir_path = entry.path();
            
            // Lógica de detecção dinâmica via Traits
            let mut detected = None;
            for proj_type in &project_types {
                 if proj_type.is_dependency_dir(&dir_path) {
                     detected = Some(proj_type.dep_kind());
                     break;
                 }
            }

            if let Some(kind) = detected {
                if let Some(parent) = dir_path.parent() {
                    let mut map = project_deps.lock().unwrap();
                    map
                        .entry(parent.to_path_buf())
                        .or_default()
                        .push(DepDir {
                            path: dir_path,
                            size: 0,
                            kind,
                        });
                }
            }
        });

    let mut stale: Vec<StaleProject> = Vec::new();

    let map = Arc::try_unwrap(project_deps).unwrap().into_inner().unwrap();

    for (project_path, mut deps) in map {
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
