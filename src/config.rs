use std::path::PathBuf;
use directories::ProjectDirs;
use serde::Deserialize;
use std::fs;
use std::fmt;

#[derive(Debug)]
pub enum ConfigError {
    NotFound,
    ParseError(PathBuf, String),
    IoError(std::io::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::NotFound => write!(f, "Arquivo de configuração não encontrado"),
            ConfigError::ParseError(path, msg) => write!(f, "Erro de sintaxe no arquivo '{}': {}", path.display(), msg),
            ConfigError::IoError(err) => write!(f, "Erro de I/O: {}", err),
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub days: Option<u64>,
    pub excluded_dirs: Option<Vec<String>>,
    pub auto_confirm: Option<bool>,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let proj_dirs = ProjectDirs::from("", "", "faxina-cli")
            .ok_or_else(|| ConfigError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory not found")))?;
            
        let config_path = proj_dirs.config_dir().join("config.toml");
        Self::load_from_path(config_path)
    }

    pub fn load_from_path<P: AsRef<std::path::Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
             return Err(ConfigError::NotFound);
        }

        let content = fs::read_to_string(path)
            .map_err(ConfigError::IoError)?;
            
        match toml::from_str(&content) {
            Ok(c) => Ok(c),
            Err(e) => Err(ConfigError::ParseError(path.to_path_buf(), e.to_string())),
        }
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
        let config_path = temp_dir.join("non_existent.toml");
        
        match Config::load_from_path(&config_path) {
            Err(ConfigError::NotFound) => (), // pass
            _ => panic!("Should return NotFound"),
        }
    }

    #[test]
    fn test_load_from_path_invalid_toml() {
        let temp_dir = std::env::temp_dir().join(format!("test_config_invalid_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let config_path = temp_dir.join("config.toml");
        
        {
            let mut f = fs::File::create(&config_path).unwrap();
            f.write_all(b"days = 'invalid_number'").unwrap(); // String instead of int
        }

        match Config::load_from_path(&config_path) {
            Err(ConfigError::ParseError(_, _)) => (), // pass
            _ => panic!("Should return ParseError"),
        }

        fs::remove_dir_all(&temp_dir).unwrap();
    }
}
