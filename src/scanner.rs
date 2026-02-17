use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use jwalk::WalkDir;

use crate::types::{DepDir, StaleProject, dir_size};
use crate::projects::all_project_types;

const PROTECTED_PATHS: &[&str] = &[
    "/",
    "/usr",
    "/bin",
    "/sbin",
    "/etc",
    "/var",
    "/boot",
    "/root",
    "C:\\",
    "C:\\Windows",
    "C:\\Program Files",
    "C:\\Program Files (x86)",
];

fn is_safe_to_scan(path: &Path) -> bool {
    let canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return false, // Path doesn't exist or cant be read, so it's "safe" in the sense that we won't scan it anyway
    };
    
    let path_str = canonical.to_string_lossy();
    
    // Check exact matches or if it's a parent
    for protected in PROTECTED_PATHS {
        // Exact match
        if path_str == *protected {
            return false;
        }
        
        // On Windows, handle case-insensitivity roughly or just rely on canonicalization?
        // Canonicalization on Windows usually gives `\\?\C:\...` which complicates things.
        // For simplicity, let's just check if the path *starts with* a protected path?
        // No, we want to prevent scanning the *parent* of a protected path? No, we want to prevent scanning *inside* or *starting at* a protected path IF it makes sense.
        // Actually, the user requirement is "Prevent accidental deletion in critical system directories".
        // If I scan `/`, I might find `node_modules` in `/var/lib/...` which might be system stuff.
        // So I should block scanning IF `root` IS one of the protected paths.
        // Scanning `/home/user` is fine. Scanning `/` is not.
        
        #[cfg(windows)]
        {
             // Simple check for Windows roots
             if path_str.eq_ignore_ascii_case(protected) {
                 return false;
             }
             // Also block `C:` without backslash if canonical resolves to it
             if protected.ends_with('\\') && path_str.eq_ignore_ascii_case(protected.trim_end_matches('\\')) {
                 return false;
             }
        }
        
        #[cfg(not(windows))]
        {
            if path_str == *protected {
                return false;
            }
        }
    }
    
    true
}

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

pub fn scan_projects<F>(root: &Path, days: u64, ignored_paths: &[PathBuf], on_progress: Option<F>) -> Vec<StaleProject>
where
    F: Fn() + Send + Sync + 'static,
{
    if !is_safe_to_scan(root) {
        // In a real app we might want to return Result, but signature is Vec.
        // Returns empty and logs/prints warning?
        eprintln!("⚠️  Caminho protegido detectado: {}. Varredura abortada por segurança.", root.display());
        return Vec::new();
    }

    let threshold = SystemTime::now() - Duration::from_secs(days * 24 * 3600);
    let project_types = all_project_types();
    
    // Mutex para coletar resultados de threads paralelas
    let project_deps: Arc<Mutex<HashMap<PathBuf, Vec<DepDir>>>> = Arc::new(Mutex::new(HashMap::new()));
    let on_progress = Arc::new(on_progress);
    
    // Ignored paths shared across threads
    let ignored_paths_shared: Arc<Vec<PathBuf>> = Arc::new(ignored_paths.to_vec());

    WalkDir::new(root)
        .skip_hidden(false) 
        .follow_links(false)
        .process_read_dir(move |_depth, _path, _read_dir_state, children| {
            children.retain(|dir_entry_result| {
                dir_entry_result.as_ref().map(|entry| {
                    let name = entry.file_name().to_string_lossy();
                    if name == ".git" {
                        return false;
                    }
                    
                    let entry_path = entry.path();
                    for ignored in ignored_paths_shared.iter() {
                         if entry_path == *ignored {
                             return false; 
                         }
                    }
                    true
                }).unwrap_or(false)
            });
        })
        .into_iter()
        .for_each(|entry| {
            if let Some(cb) = on_progress.as_ref() {
                cb();
            }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn make_temp_dir() -> PathBuf {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "limpador_scan_test_{}_{}", std::process::id(), id
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_scan_projects_integration() {
        let root = make_temp_dir();
        let proj1 = root.join("proj_node");
        fs::create_dir_all(proj1.join("node_modules")).unwrap();
        fs::write(proj1.join("package.json"), "{}").unwrap();
        
        // Simula um callback
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        // Data de modificação antiga para ser considerado stale
        // Mas o dir_size pode falhar se não tiver arquivos. Vamos criar arquivos.
        fs::write(proj1.join("node_modules/lib.js"), "content").unwrap();

        // Para testar stale, precisamos manipular mtime, mas latest_source_mtime lê mtime.
        // Se acabamos de criar, é recente. scan_projects(..., 0) varre tudo (0 dias).
        
        let projects = scan_projects(&root, 0, &[], Some(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }));

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "proj_node");
        assert!(counter.load(Ordering::SeqCst) > 0);
        
        fs::remove_dir_all(root).unwrap();
    }
}
