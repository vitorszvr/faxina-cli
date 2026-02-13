use std::path::Path;
use crate::types::DepKind;

pub trait ProjectType: Send + Sync {
    fn name(&self) -> &'static str;
    fn dep_kind(&self) -> DepKind;
    
    // Retorna true se o diretório passado (ex: path/to/node_modules)
    // é uma pasta de dependência válida deste tipo de projeto.
    fn is_dependency_dir(&self, dir_path: &Path) -> bool;
}

pub struct NodeProject;
impl ProjectType for NodeProject {
    fn name(&self) -> &'static str { "Node.js" }
    fn dep_kind(&self) -> DepKind { DepKind::NodeModules }
    fn is_dependency_dir(&self, dir_path: &Path) -> bool {
        dir_path.file_name() == Some(std::ffi::OsStr::new("node_modules"))
            && dir_path.parent()
                .map(|p| p.join("package.json").exists())
                .unwrap_or(false)
    }
}

pub struct RustProject;
impl ProjectType for RustProject {
    fn name(&self) -> &'static str { "Rust" }
    fn dep_kind(&self) -> DepKind { DepKind::Target }
    fn is_dependency_dir(&self, dir_path: &Path) -> bool {
        dir_path.file_name() == Some(std::ffi::OsStr::new("target"))
            && dir_path.parent()
                .map(|p| p.join("Cargo.toml").exists())
                .unwrap_or(false)
    }
}

pub struct NextProject;
impl ProjectType for NextProject {
    fn name(&self) -> &'static str { "Next.js" }
    fn dep_kind(&self) -> DepKind { DepKind::NextBuild }
    fn is_dependency_dir(&self, dir_path: &Path) -> bool {
        if dir_path.file_name() != Some(std::ffi::OsStr::new(".next")) {
            return false;
        }
        let parent = match dir_path.parent() {
            Some(p) => p,
            None => return false,
        };
        
        if parent.join("package.json").exists() {
            return true;
        }
        for ext in &["js", "mjs", "ts"] {
            if parent.join(format!("next.config.{}", ext)).exists() {
                return true;
            }
        }
        false
    }
}

pub struct PythonProject;
impl ProjectType for PythonProject {
    fn name(&self) -> &'static str { "Python (venv)" }
    fn dep_kind(&self) -> DepKind { DepKind::Venv }
    fn is_dependency_dir(&self, dir_path: &Path) -> bool {
        let name = match dir_path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => return false,
        };
        
        if name != "venv" && name != ".venv" {
            return false;
        }

        // Verifica se é um venv válido verificando conteúdo INTERNO
        dir_path.join("pyvenv.cfg").exists()
            || dir_path.join("bin/python").exists()
            || dir_path.join("Scripts/python.exe").exists()
    }
}

pub struct GoProject;
impl ProjectType for GoProject {
    fn name(&self) -> &'static str { "Go" }
    fn dep_kind(&self) -> DepKind { DepKind::Vendor }
    fn is_dependency_dir(&self, dir_path: &Path) -> bool {
        dir_path.file_name() == Some(std::ffi::OsStr::new("vendor"))
            && dir_path.parent()
                .map(|p| p.join("go.mod").exists())
                .unwrap_or(false)
    }
}

pub struct GradleProject;
impl ProjectType for GradleProject {
    fn name(&self) -> &'static str { "Gradle" }
    fn dep_kind(&self) -> DepKind { DepKind::Build }
    fn is_dependency_dir(&self, dir_path: &Path) -> bool {
        dir_path.file_name() == Some(std::ffi::OsStr::new("build"))
            && dir_path.parent()
                .map(|p| p.join("build.gradle").exists() || p.join("build.gradle.kts").exists())
                .unwrap_or(false)
    }
}

pub fn all_project_types() -> Vec<Box<dyn ProjectType>> {
    vec![
        Box::new(NodeProject),
        Box::new(RustProject),
        Box::new(NextProject),
        Box::new(PythonProject),
        Box::new(GoProject),
        Box::new(GradleProject),
    ]
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
            "limpador_test_projects_{}_{}", std::process::id(), id
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_node_project() {
        let dir = make_temp_dir();
        fs::create_dir(dir.join("node_modules")).unwrap();
        fs::write(dir.join("package.json"), "{}").unwrap();
        
        let proj = NodeProject;
        assert!(proj.is_dependency_dir(&dir.join("node_modules")));
        
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_rust_project() {
        let dir = make_temp_dir();
        fs::create_dir(dir.join("target")).unwrap();
        fs::write(dir.join("Cargo.toml"), "").unwrap();
        
        let proj = RustProject;
        assert!(proj.is_dependency_dir(&dir.join("target")));
        
        fs::remove_dir_all(dir).unwrap();
    }
    
    #[test]
    fn test_python_project() {
        let dir = make_temp_dir();
        let venv = dir.join("venv");
        fs::create_dir(&venv).unwrap();
        fs::write(venv.join("pyvenv.cfg"), "").unwrap();
        
        let proj = PythonProject;
        assert!(proj.is_dependency_dir(&venv));
        
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_python_project_dot_venv() {
        let dir = make_temp_dir();
        let venv = dir.join(".venv");
        fs::create_dir(&venv).unwrap();
        
        // Cria bin primeiro
        fs::create_dir(venv.join("bin")).unwrap();
        fs::write(venv.join("bin/python"), "").unwrap();

        let proj = PythonProject;
        assert!(proj.is_dependency_dir(&venv));
        
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_go_project() {
        let dir = make_temp_dir();
        fs::create_dir(dir.join("vendor")).unwrap();
        fs::write(dir.join("go.mod"), "").unwrap();
        
        let proj = GoProject;
        assert!(proj.is_dependency_dir(&dir.join("vendor")));
        
        fs::remove_dir_all(dir).unwrap();
    }
    
    #[test]
    fn test_gradle_project() {
        let dir = make_temp_dir();
        fs::create_dir(dir.join("build")).unwrap();
        fs::write(dir.join("build.gradle.kts"), "").unwrap();
        
        let proj = GradleProject;
        assert!(proj.is_dependency_dir(&dir.join("build")));
        
        fs::remove_dir_all(dir).unwrap();
    }
    
    #[test]
    fn test_next_project() {
        let dir = make_temp_dir();
        fs::create_dir(dir.join(".next")).unwrap();
        fs::write(dir.join("next.config.js"), "").unwrap();
        
        let proj = NextProject;
        assert!(proj.is_dependency_dir(&dir.join(".next")));
        
        fs::remove_dir_all(dir).unwrap();
    }
}
