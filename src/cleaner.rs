use anyhow::Error;
use std::fs;
use std::path::PathBuf;

use indicatif::{ProgressBar, ProgressStyle};

use crate::types::StaleProject;

#[derive(Debug)]
pub struct CleanResult {
    pub total_freed: u64,
    pub dirs_removed: usize,
    pub errors: Vec<(PathBuf, Error)>,
}

pub fn clean_projects(projects: &[StaleProject], dry_run: bool, verbose: bool) -> CleanResult {
    let total_dirs: usize = projects.iter().map(|p| p.dep_dirs.len()).sum();

    let pb = ProgressBar::new(total_dirs as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "  {spinner:.green} [{bar:30.cyan/dim}] {pos}/{len} pastas {msg}",
        )
        .unwrap()
        .progress_chars("█▓░"),
    );

    let mut result = CleanResult {
        total_freed: 0,
        dirs_removed: 0,
        errors: Vec::new(),
    };

    for project in projects {
        for dep in &project.dep_dirs {
            pb.set_message(format!("removendo {}", dep.kind));

            if verbose {
                pb.println(format!("    → {}", dep.path.display()));
            }

            if dry_run {
                result.total_freed += dep.size;
                result.dirs_removed += 1;
            } else {
                match remove_dir_all_with_retry(&dep.path) {
                    Ok(_) => {
                        result.total_freed += dep.size;
                        result.dirs_removed += 1;
                    }
                    Err(e) => {
                        result.errors.push((dep.path.clone(), e));
                    }
                }
            }

            pb.inc(1);
        }
    }

    pb.finish_and_clear();
    result
}

fn remove_dir_all_with_retry(path: &std::path::Path) -> Result<(), Error> {
    #[cfg(not(windows))]
    {
        fs::remove_dir_all(path).map_err(|e| e.into())
    }

    #[cfg(windows)]
    {
        use std::thread;
        use std::time::Duration;
        use std::io::ErrorKind;

        let mut last_err = None;
        for i in 0..5 {
            match fs::remove_dir_all(path) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    // Só faz retry para erros de lock (antivírus, processo aberto)
                    // Outros erros (NotFound, etc.) retornam imediatamente
                    if e.kind() != ErrorKind::PermissionDenied {
                        return Err(e.into());
                    }
                    if i < 4 {
                        thread::sleep(Duration::from_millis(100 * 2u64.pow(i as u32)));
                    }
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DepDir, DepKind};
    use std::time::SystemTime;

    #[test]
    fn test_clean_projects_dry_run() {
        let temp = std::env::temp_dir().join(format!("test_clean_dry_{}", std::process::id()));
        fs::create_dir_all(&temp).unwrap();

        let dep_path = temp.join("node_modules");
        fs::create_dir(&dep_path).unwrap();

        let project = StaleProject {
            name: "test".to_string(),
            path: temp.clone(),
            dep_dirs: vec![DepDir {
                path: dep_path.clone(),
                size: 100,
                kind: DepKind::NodeModules,
            }],
            last_modified: SystemTime::now(),
        };

        let result = clean_projects(&[project], true, false);

        assert_eq!(result.total_freed, 100);
        assert_eq!(result.dirs_removed, 1);
        assert_eq!(result.errors.len(), 0);
        
        // Assert directory STILL EXISTS
        assert!(dep_path.exists());

        fs::remove_dir_all(&temp).unwrap();
    }

    #[test]
    fn test_clean_projects_real() {
        let temp = std::env::temp_dir().join(format!("test_clean_real_{}", std::process::id()));
        fs::create_dir_all(&temp).unwrap();

        let dep_path = temp.join("node_modules");
        fs::create_dir(&dep_path).unwrap();

        let project = StaleProject {
            name: "test".to_string(),
            path: temp.clone(),
            dep_dirs: vec![DepDir {
                path: dep_path.clone(),
                size: 200,
                kind: DepKind::NodeModules,
            }],
            last_modified: SystemTime::now(),
        };

        // Run actual clean
        let result = clean_projects(&[project], false, false);

        assert_eq!(result.total_freed, 200);
        assert_eq!(result.dirs_removed, 1);
        assert_eq!(result.errors.len(), 0);
        
        // Assert directory GONE
        assert!(!dep_path.exists());

        fs::remove_dir_all(&temp).unwrap();
    }
}
