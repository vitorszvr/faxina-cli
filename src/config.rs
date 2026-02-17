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
        
        if !config_path.exists() {
            return Ok(Config {
                days: None,
                excluded_dirs: None,
                auto_confirm: None,
            });
        }
        
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Falha ao ler arquivo de configuração: {}", config_path.display()))?;
            
        toml::from_str(&content)
            .with_context(|| format!("Falha ao fazer parse do config: {}", config_path.display()))
    }
}
