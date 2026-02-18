use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub days: Option<u64>,
    pub excluded_dirs: Option<Vec<String>>,
    pub auto_confirm: Option<bool>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let proj_dirs = ProjectDirs::from("", "", "faxina-cli")
            .context("Não foi possível determinar o diretório de configuração do sistema")?;
            
        let config_path = proj_dirs.config_dir().join("config.toml");
        Self::load_from_path(config_path)
    }

    pub fn load_from_path<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
             return Ok(Config {
                days: None,
                excluded_dirs: None,
                auto_confirm: None,
            });
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Falha ao ler arquivo de configuração: {}", path.display()))?;
            
        toml::from_str(&content)
            .with_context(|| format!("Falha ao fazer parse do config: {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_load_from_path_valid() {
        let temp_dir = std::env::temp_dir().join(format!("test_config_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let config_path = temp_dir.join("config.toml");

        let content = r#"
            days = 45
            excluded_dirs = ["/tmp", "node_modules"]
            auto_confirm = true
        "#;
        
        {
            let mut f = fs::File::create(&config_path).unwrap();
            f.write_all(content.as_bytes()).unwrap();
        }

        let config = Config::load_from_path(&config_path).unwrap();
        
        assert_eq!(config.days, Some(45));
        assert_eq!(config.auto_confirm, Some(true));
        assert_eq!(config.excluded_dirs.unwrap().len(), 2);

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_load_from_path_missing() {
        let temp_dir = std::env::temp_dir().join(format!("test_config_missing_{}", std::process::id()));
        // Ensure dir doesn't exist or file doesn't
        let config_path = temp_dir.join("non_existent.toml");
        
        let config = Config::load_from_path(&config_path).unwrap();
        assert!(config.days.is_none());
        
        // Cleanup not needed if we didn't create
    }
}
