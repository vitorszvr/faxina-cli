use std::collections::HashMap;
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

    // Usa comparação baseada em componentes de Path, que é mais segura
    // do que comparação de strings pois lida com separadores e normalização do OS
    for protected in PROTECTED_PATHS {
        let protected_path = Path::new(protected);

        // Match exato (case-insensitive no Windows)
        #[cfg(windows)]
        {
            let canon_str = canonical.to_string_lossy().to_lowercase();
            let prot_str = protected.to_lowercase().trim_end_matches('\\').to_string();
            if canon_str == prot_str || canon_str.starts_with(&format!("{}\\", prot_str)) {
                return false;
            }
        }

        #[cfg(not(windows))]
        {
            if canonical == protected_path {
                return false;
            }
            // Verifica se é subdiretório usando starts_with de Path 
            // (mais seguro que strings: /usr não bloqueia /usr-local)
            if canonical.starts_with(protected_path) && protected != &"/" {
                return false;
            }
        }
    }

    // Nota: $HOME (ex: /home/vitor) NÃO é bloqueado — é o uso mais comum da ferramenta.
    // O aviso de poucos componentes abaixo serve como alerta suave.

    // Alerta para caminhos com poucos componentes (ex: /home/user, C:\Users)
    // Não bloqueia, mas é uma heurística de segurança
    let component_count = canonical.components().count();
    if component_count <= 2 {
        eprintln!(
            "⚠️  Aviso: caminho com poucos componentes ({}). Verifique se é intencional: {}",
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

    WalkDir::new(project_dir)
        .skip_hidden(false)
        .follow_links(false)
        .process_read_dir(move |_depth, _path, _read_dir_state, children| {
            // Atualiza o mtime mais recente para arquivos encontrados neste diretório
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

            // Filtra diretórios de dependência para não descer neles
            children.retain(|dir_entry_result| {
                dir_entry_result.as_ref().map(|e| {
                    if !e.file_type().is_dir() {
                        return true; // Mantém arquivos (já processados acima, mas jwalk precisa)
                    }
                    let name = e.file_name().to_string_lossy();
                    !skip_dirs.contains(&name.as_ref())
                }).unwrap_or(false)
            });
        })
        .into_iter()
        .for_each(|_| {}); // Consome o iterador para garantir que a varredura complete

    let guard = latest.lock().unwrap();
    *guard
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
    let project_types = Arc::new(all_project_types());
    
    // Mutex para coletar resultados de threads paralelas
    let project_deps: Arc<Mutex<HashMap<PathBuf, Vec<DepDir>>>> = Arc::new(Mutex::new(HashMap::new()));
    let on_progress = Arc::new(on_progress);
    
    // Ignored paths shared across threads
    let ignored_paths_shared: Arc<Vec<PathBuf>> = Arc::new(ignored_paths.to_vec());

    // Clones para o closure do process_read_dir
    let project_deps_clone = project_deps.clone();
    let project_types_clone = project_types.clone();
    let ignored_paths_clone = ignored_paths_shared.clone();

    // Cache de projetos já confirmados para evitar re-verificar config files
    let confirmed_projects: Arc<Mutex<std::collections::HashSet<PathBuf>>> = 
        Arc::new(Mutex::new(std::collections::HashSet::new()));
    let confirmed_projects_clone = confirmed_projects.clone();

    WalkDir::new(root)
        .skip_hidden(false) 
        .follow_links(false)
        .process_read_dir(move |_depth, _path, _read_dir_state, children| {
            children.retain(|dir_entry_result| {
                let entry = match dir_entry_result.as_ref() {
                    Ok(e) => e,
                    Err(_) => return false,
                };
                
                let name = entry.file_name().to_string_lossy();
                if name == ".git" {
                    return false;
                }
                
                let entry_path = entry.path();
                for ignored in ignored_paths_clone.iter() {
                        if entry_path == *ignored {
                            return false; 
                        }
                }

                if entry.file_type().is_dir() {
                    // Verifica se o parent já foi confirmado como projeto
                    let parent_confirmed = entry_path.parent()
                        .map(|p| confirmed_projects_clone.lock().unwrap().contains(p))
                        .unwrap_or(false);

                    for proj_type in project_types_clone.iter() {
                        // Se parent já confirmado, basta checar se o nome do dir 
                        // corresponde ao tipo (evita re-verificar package.json etc.)
                        let is_dep = if parent_confirmed {
                            // Verifica apenas o nome do diretório
                            let dep_name = proj_type.dep_kind().to_string();
                            name == dep_name.as_str()
                        } else {
                            proj_type.is_dependency_dir(&entry_path)
                        };

                        if is_dep {
                            if let Some(parent) = entry_path.parent() {
                                let parent_owned = parent.to_path_buf();
                                let mut map = project_deps_clone.lock().unwrap();
                                map
                                    .entry(parent_owned.clone())
                                    .or_default()
                                    .push(DepDir {
                                        path: entry_path,
                                        size: 0, // calculado depois
                                        kind: proj_type.dep_kind(),
                                    });
                                // Registra o parent como projeto confirmado
                                confirmed_projects_clone.lock().unwrap().insert(parent_owned);
                            }
                            return false; // Skip recursion!
                        }
                    }
                }

                true
            });
        })
        .into_iter()
        .for_each(|entry| {
            if let Some(cb) = on_progress.as_ref() {
                cb();
            }
            // A detecção já aconteceu no process_read_dir
             let _ = entry; 
        });

    let mut stale: Vec<StaleProject> = Vec::new();

    let map = {
        let mut guard = project_deps.lock().unwrap();
        std::mem::take(&mut *guard)
    };

    for (project_path, mut deps) in map {
        let last_modified = match latest_source_mtime(&project_path) {
            Some(t) => t,
            None => continue,
        };

        if last_modified > threshold {
            continue;
        }

        // Tamanhos calculados depois por calculate_sizes() — lazy evaluation

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

    stale.sort_by(|a, b| a.name.cmp(&b.name)); // Ordena por nome antes do cálculo de tamanho
    stale
}

/// Calcula o tamanho de cada diretório de dependência dos projetos.
/// Deve ser chamado após a seleção do usuário para evitar I/O desnecessário.
pub fn calculate_sizes(projects: &mut [StaleProject]) {
    for project in projects.iter_mut() {
        for dep in &mut project.dep_dirs {
            dep.size = dir_size(&dep.path);
        }
    }
    // Re-ordena por tamanho total (maior primeiro)
    projects.sort_by(|a, b| b.total_size().cmp(&a.total_size()));
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
    #[test]
    fn test_is_safe_to_scan() {
        // Caminhos protegidos básicos
        assert!(!is_safe_to_scan(Path::new("/")));
        assert!(!is_safe_to_scan(Path::new("/usr")));
        assert!(!is_safe_to_scan(Path::new("/bin")));
        
        // /home NÃO é protegido (é onde ficam os projetos do usuário)
        // assert!(is_safe_to_scan(Path::new("/home"))); // pode não existir em todos os ambientes

        // $HOME direto deve ser PERMITIDO (é o uso mais comum)
        if let Some(home) = std::env::var_os("HOME") {
            let home_path = PathBuf::from(home);
            if home_path.exists() {
                assert!(is_safe_to_scan(&home_path), "$HOME deve ser permitido");
            }
        }

        // Caminhos seguros (temp dir)
        let temp = std::env::temp_dir();
        assert!(is_safe_to_scan(&temp));

        // Subdiretório de temp deve ser seguro
        let safe_dir = make_temp_dir();
        assert!(is_safe_to_scan(&safe_dir));
        fs::remove_dir_all(safe_dir).unwrap();

        // Caminho inexistente deve retornar false (não pode canonicalizar)
        assert!(!is_safe_to_scan(Path::new("/caminho/que/nao/existe/xyzzy")));

        #[cfg(windows)]
        {
            assert!(!is_safe_to_scan(Path::new("C:\\")));
            assert!(!is_safe_to_scan(Path::new("C:\\Windows")));
        }
    }
}
