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
                match fs::remove_dir_all(&dep.path) {
                    Ok(_) => {
                        result.total_freed += dep.size;
                        result.dirs_removed += 1;
                    }
                    Err(e) => {
                        result.errors.push((dep.path.clone(), e.into()));
                    }
                }
            }

            pb.inc(1);
        }
    }

    pb.finish_and_clear();
    result
}
