use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use directories::ProjectDirs;
use anyhow::{Context, Result};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LanguageConfig {
    pub preferred: String,
    pub fallback: Option<String>,
    #[serde(default)]
    pub overrides: HashMap<String, String>,
}

impl Default for LanguageConfig {
    fn default() -> Self {
        Self {
            preferred: "en".to_string(),
            fallback: Some("en".to_string()),
            overrides: HashMap::new(),
        }
    }
}

pub fn load_or_create_config() -> Result<LanguageConfig> {
    let config_path = get_config_path()?;
    
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .context("Failed to create config directory")?;
    }

    if !config_path.exists() {
        let default_config = LanguageConfig::default();
        let toml_content = toml::to_string_pretty(&default_config)?;
        std::fs::write(&config_path, toml_content)
            .context("Failed to write default config")?;
        return Ok(default_config);
    }

    let content = std::fs::read_to_string(&config_path)
        .context("Failed to read config file")?;
    toml::from_str(&content)
        .context("Failed to parse config file")
}

fn get_config_path() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "your_company", "Your_Game")
        .context("Couldn't determine project directory")?;
    Ok(proj_dirs.config_dir().join("language.toml"))
}
