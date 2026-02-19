use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use jwalk::WalkDir;
use log::{debug, warn};

use crate::types::{DepDir, StaleProject, dir_size};
use crate::projects::all_project_types;

// System paths to protect from accidental deletion
const PROTECTED_PATHS: &[&str] = &[
    "/",
    "/usr",
    "/bin",
    "/sbin",
    "/etc",
    "/var",
    "/boot",
    "/root",
    "/lib",
    "/lib64",
    "/opt",
    "C:\\",
    "C:\\Windows",
    "C:\\Program Files",
    "C:\\Program Files (x86)",
];

fn is_safe_to_scan(path: &Path) -> bool {
    let canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return false, // Path doesn't exist or can't be read
    };

    for protected in PROTECTED_PATHS {
        // Match exact and subdirectories (case-insensitive on Windows)
        #[cfg(windows)]
        {
            let canon_str = canonical.to_string_lossy().to_lowercase();
            let prot_str = protected.to_lowercase().trim_end_matches('\\').to_string();
            // Exact match
            if canon_str == prot_str {
                return false;
            }
            // Subdirectories (but not for roots like C:\ else it blocks everything)
            if prot_str.len() > 3 && canon_str.starts_with(&format!("{}\\", prot_str)) {
                return false;
            }
        }

        #[cfg(not(windows))]
        {
            let protected_path = Path::new(protected);
            if canonical == protected_path {
                return false;
            }
            // Check if subdirectory using path components (safer than strings)
            if canonical.starts_with(protected_path) && protected != &"/" {
                return false;
            }
        }
    }

    // Heuristic warning for paths with few components (e.g., /home/user)
    let component_count = canonical.components().count();
    if component_count <= 2 {
        warn!(
            "Path with few components ({}). Verify if intentional: {}",
            component_count,
            canonical.display()
        );
    }

    true
}

fn latest_source_mtime(project_dir: &Path) -> Option<SystemTime> {
    let skip_dirs: &[&str] = &[
        "node_modules", "target", ".next", "dist", "build",
        ".git", "venv", ".venv", "vendor",
    ];

    let latest = Arc::new(Mutex::new(None::<SystemTime>));
    let latest_clone = latest.clone();

    // Use process_read_dir to effectively skip descending into ignored directories
    // avoiding the overhead of walking huge dependency trees just to ignore them later.
    WalkDir::new(project_dir)
        .skip_hidden(false)
        .follow_links(false)
        .process_read_dir(move |_depth, _path, _read_dir_state, children| {
            // 1. Process files in current directory to update mtime
            for entry_result in children.iter() {
                if let Ok(entry) = entry_result {
                    if !entry.file_type().is_dir() {
                        if let Ok(meta) = entry.metadata() {
                            if let Ok(mtime) = meta.modified() {
                                let mut guard = latest_clone.lock().unwrap();
                                *guard = Some(match *guard {
                                    Some(current) => current.max(mtime),
                                    None => mtime,
                                });
                            }
                        }
                    }
                }
            }

            // 2. Filter directories to descend into
            children.retain(|dir_entry_result| {
                 dir_entry_result.as_ref().map(|e| {
                    if !e.file_type().is_dir() { return true; }
                    let name = e.file_name().to_string_lossy();
                    !skip_dirs.contains(&name.as_ref())
                }).unwrap_or(false)
            });
        })
        .into_iter()
        .for_each(|_| {}); 

    let res = *latest.lock().unwrap();
    res
}

pub fn scan_projects<F>(root: &Path, days: u64, ignored_paths: &[PathBuf], on_progress: Option<F>) -> Vec<StaleProject>
where
    F: Fn() + Send + Sync + 'static,
{
    if !is_safe_to_scan(root) {
        warn!("Protected path detected: {}. Scan aborted for safety.", root.display());
        return Vec::new();
    }

    let threshold = SystemTime::now() - Duration::from_secs(days * 24 * 3600);
    let project_types = Arc::new(all_project_types());
    
    let ignored_paths_canonical: Vec<PathBuf> = ignored_paths.iter()
        .filter_map(|p| p.canonicalize().ok().or_else(|| Some(p.clone())))
        .collect();
    let ignored_paths_shared: Arc<Vec<PathBuf>> = Arc::new(ignored_paths_canonical);

    // Pass 1: Scan file system to find ALL projects and their dependencies
    let findings: Arc<Mutex<HashMap<PathBuf, Vec<DepDir>>>> = Arc::new(Mutex::new(HashMap::new()));
    let findings_clone = findings.clone();
    let pt_clone = project_types.clone();
    let ign_clone = ignored_paths_shared.clone();
    
    WalkDir::new(root)
        .skip_hidden(false)
        .follow_links(false)
        .process_read_dir(move |_depth, _path, _read_dir_state, children| {
             children.retain(|dir_entry_result| {
                let entry = match dir_entry_result.as_ref() {
                    Ok(e) => e,
                    Err(_) => return false,
                };
                let entry_path = entry.path();
                let name = entry.file_name().to_string_lossy();

                for ignored in ign_clone.iter() {
                    if entry_path == *ignored { return false; }
                }

                if entry.file_type().is_dir() {
                    // 1. Dependency Detection
                    for proj_type in pt_clone.iter() {
                        if proj_type.is_dependency_dir(&entry_path) {
                            if let Some(parent) = entry_path.parent() {
                                let mut map = findings_clone.lock().unwrap();
                                map.entry(parent.to_path_buf())
                                   .or_default()
                                   .push(DepDir {
                                       path: entry_path.clone(),
                                       size: 0,
                                       kind: proj_type.dep_kind(),
                                   });
                            }
                            return false; // Don't descend into dep dirs
                        }
                    }

                    // 2. Skip common non-project hidden/cache dirs
                    const SKIP_DIRS: &[&str] = &[
                        ".git", ".next", ".vscode", ".cursor", ".idea", ".eclipse",
                        ".local", ".cache", ".cargo", ".rustup", ".npm", ".nvm",
                        ".gradle", ".m2", ".sdkman", ".config", ".Trash", 
                        ".pyenv", ".rbenv", "Library", "AppData",
                    ];
                    if SKIP_DIRS.contains(&name.as_ref()) {
                        return false;
                    }
                }
                true
             });
        })
        .into_iter()
        .for_each(move |entry| {
             if let Some(cb) = on_progress.as_ref() {
                cb();
             }
             let _ = entry;
        });

    // Pass 2: Calculate mtimes and Identify Active Roots
    struct ProjectInfo {
        path: PathBuf,
        deps: Vec<DepDir>,
        last_modified: SystemTime,
    }

    let raw_projects = {
        let mut guard = findings.lock().unwrap();
        std::mem::take(&mut *guard)
    };

    let mut project_infos: Vec<ProjectInfo> = Vec::with_capacity(raw_projects.len());
    let mut active_roots: Vec<PathBuf> = Vec::new();

    for (path, deps) in raw_projects {
        let last_modified = match latest_source_mtime(&path) {
            Some(t) => t,
            None => {
                debug!("Could not read mtime for {}; ignoring.", path.display());
                continue; 
            }
        };

        if last_modified >= threshold {
            active_roots.push(path.clone());
        }

        project_infos.push(ProjectInfo {
            path,
            deps,
            last_modified,
        });
    }

    // Pass 3: Filter Stale Projects (Bidirectional Protection)
    // - Protect if project itself is active (already handled by mtime check)
    // - Protect if project is child of Active Root
    // - Protect if project contains an Active Root
    
    let mut stale: Vec<StaleProject> = Vec::new();

    for proj in project_infos {
        // Condition 1: Must be old
        if proj.last_modified >= threshold {
            continue;
        }

        // Condition 2: Must NOT be inside an Active Root (active parent protects child)
        if let Some(parent_root) = active_roots.iter().find(|root| proj.path.starts_with(root) && &proj.path != *root) {
            debug!("Protected child project: {} (Parent {} is active)", proj.path.display(), parent_root.display());
            continue;
        }

        // Condition 3: Must NOT contain an Active Root (active child protects parent)
        // e.g. Monorepo (Stale) -> Package (Active). don't delete Monorepo node_modules.
        if let Some(child_root) = active_roots.iter().find(|root| root.starts_with(&proj.path) && *root != &proj.path) {
             debug!("Protected parent project: {} (Child {} is active)", proj.path.display(), child_root.display());
             continue;
        }

        let name = proj.path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| proj.path.display().to_string());

        stale.push(StaleProject {
            name,
            path: proj.path,
            dep_dirs: proj.deps,
            last_modified: proj.last_modified,
        });
    }

    stale.sort_by(|a, b| a.name.cmp(&b.name));
    stale
}

/// Calculate sizes lazily
pub fn calculate_sizes(projects: &mut [StaleProject]) {
    for project in projects.iter_mut() {
        for dep in &mut project.dep_dirs {
            dep.size = dir_size(&dep.path);
        }
    }
    projects.sort_by(|a, b| b.total_size().cmp(&a.total_size()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use filetime::{FileTime, set_file_mtime};

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
    fn test_nested_bidirectional_protection() {
        let root = make_temp_dir();
        
        let parent = root.join("parent");
        let child = parent.join("child");
        
        fs::create_dir_all(&child).unwrap();
        
        // Setup deps
        fs::create_dir_all(parent.join("node_modules")).unwrap();
        fs::write(parent.join("package.json"), "{}").unwrap();
        
        fs::create_dir_all(child.join("node_modules")).unwrap();
        fs::write(child.join("package.json"), "{}").unwrap();
        
        // Helper to set time
        let set_age = |path: &Path, days: u64| {
            let now = SystemTime::now();
            let past = now - Duration::from_secs(days * 24 * 3600 + 3600); // +1h margin
            let ft = FileTime::from_system_time(past);
            set_file_mtime(path, ft).unwrap();
        };

        // Case 1: Parent Active, Child Stale -> Child Protected
        set_age(&parent.join("package.json"), 1); // 1 day old (Active)
        set_age(&child.join("package.json"), 60); // 60 days old (Stale)
        
        let projects = scan_projects(&root, 30, &[], None::<fn()>);
        // Expect: ZERO projects because parent is active (not stale) and child is protected by parent.
        assert_eq!(projects.len(), 0, "Child should be protected by active parent");

        // Case 2: Parent Stale, Child Active -> Parent Protected
        set_age(&parent.join("package.json"), 60); // Stale
        set_age(&child.join("package.json"), 1);   // Active
        
        let projects = scan_projects(&root, 30, &[], None::<fn()>);
        // Expect: ZERO projects because child is active (not stale) and parent is protected by child.
        assert_eq!(projects.len(), 0, "Parent should be protected by active child");

        // Case 3: Both Stale -> Both Cleanable
        set_age(&parent.join("package.json"), 60);
        set_age(&child.join("package.json"), 60);
        
        let projects = scan_projects(&root, 30, &[], None::<fn()>);
        assert_eq!(projects.len(), 2, "Both should be stale");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_is_safe_to_scan() {
        assert!(!is_safe_to_scan(Path::new("/")));
        assert!(!is_safe_to_scan(Path::new("/usr")));
        
        if let Some(home) = std::env::var_os("HOME") {
            let home_path = PathBuf::from(home);
            if home_path.exists() {
                assert!(is_safe_to_scan(&home_path));
            }
        }

        let temp = std::env::temp_dir();
        assert!(is_safe_to_scan(&temp));
        
        #[cfg(windows)]
        {
            assert!(!is_safe_to_scan(Path::new("C:\\")));
        }
    }
}

