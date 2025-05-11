use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use log::{info, warn, error};
use crate::config::language::LanguageConfig;

#[derive(Debug, Error)]
pub enum TranslationError {
    #[error("Language not supported: {0}")]
    UnsupportedLanguage(String),
    #[error("Translation key not found: {0} in language {1}")]
    KeyNotFound(String, String),
    #[error("Failed to load translations: {0}")]
    LoadError(String),
    #[error("Invalid translation file format: {0}")]
    FormatError(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Placeholder error: {0}")]
    PlaceholderError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TranslationFile {
    translations: HashMap<String, String>,
    #[serde(default)]
    metadata: TranslationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TranslationMetadata {
    #[serde(default)]
    last_modified: Option<u64>,
}

struct TranslationCache {
    translations: HashMap<String, TranslationFile>,
    base_path: PathBuf,
    last_checked: std::time::SystemTime,
}

static TRANSLATION_CACHE: Lazy<Arc<RwLock<TranslationCache>>> = Lazy::new(|| {
    Arc::new(RwLock::new(TranslationCache {
        translations: HashMap::new(),
        base_path: PathBuf::new(),
        last_checked: std::time::SystemTime::now(),
    }))
});

/// Initialize translation system with base path
pub fn init_translations(base_path: &str) -> Result<(), TranslationError> {
    let mut cache = TRANSLATION_CACHE.write().unwrap();
    cache.base_path = PathBuf::from(base_path);
    reload_translations(&mut cache)?;
    Ok(())
}

/// Reload translations if files changed
pub fn reload_if_changed() -> Result<bool, TranslationError> {
    let mut cache = TRANSLATION_CACHE.write().unwrap();
    let needs_reload = check_for_updates(&cache)?;
    
    if needs_reload {
        info!("Reloading translations due to changes");
        reload_translations(&mut cache)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn reload_translations(cache: &mut TranslationCache) -> Result<(), TranslationError> {
    let translations_dir = cache.base_path.join("languages");
    info!("Loading translations from: {}", translations_dir.display());

    if !translations_dir.exists() {
        return Err(TranslationError::LoadError(format!(
            "Languages directory not found at {}",
            translations_dir.display()
        )));
    }

    let mut translations = HashMap::new();
    let mut loaded_languages = 0;

    for entry in fs::read_dir(&translations_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Some(language_code) = path.file_stem().and_then(|s| s.to_str()) {
                match load_translation_file(&path) {
                    Ok(translation_file) => {
                        translations.insert(language_code.to_string(), translation_file);
                        loaded_languages += 1;
                    }
                    Err(e) => {
                        error!("Failed to load {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    cache.translations = translations;
    cache.last_checked = std::time::SystemTime::now();
    info!("Loaded {} language(s)", loaded_languages);
    Ok(())
}

fn load_translation_file(path: &Path) -> Result<TranslationFile, TranslationError> {
    let content = fs::read_to_string(path)?;
    let translation_file: TranslationFile = serde_json::from_str(&content)?;
    Ok(translation_file)
}

fn check_for_updates(cache: &TranslationCache) -> Result<bool, TranslationError> {
    let translations_dir = cache.base_path.join("languages");
    let mut needs_reload = false;

    for (lang_code, translation_file) in &cache.translations {
        let file_path = translations_dir.join(format!("{}.json", lang_code));
        let metadata = fs::metadata(&file_path)?;

        if let Ok(modified_time) = metadata.modified() {
            if let Some(cached_time) = translation_file.metadata.last_modified {
                let cached_time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(cached_time);
                if modified_time > cached_time {
                    needs_reload = true;
                    break;
                }
            } else if modified_time > cache.last_checked {
                needs_reload = true;
                break;
            }
        }
    }

    Ok(needs_reload)
}

pub fn get_translation(key: &str) -> Result<String, TranslationError> {
    let lang_config = crate::config::language::load_or_create_config()
        .map_err(|e| TranslationError::LoadError(e.to_string()))?;
    
    let domain = key.split('.').next().unwrap_or("");
    let lang = lang_config.overrides.get(domain)
        .unwrap_or(&lang_config.preferred);

    let cache = TRANSLATION_CACHE.read().unwrap();
    let translation_file = cache.translations
        .get(lang)
        .ok_or_else(|| TranslationError::UnsupportedLanguage(lang.to_string()))?;

    // Check the main language first
    if let Some(value) = translation_file.translations.get(key) {
        return Ok(value.to_string());
    }

    // Check fallback language if configured
    if let Some(fallback) = &lang_config.fallback {
        if let Some(fallback_translation) = cache.translations.get(fallback) {
            if let Some(value) = fallback_translation.translations.get(key) {
                return Ok(value.to_string());
            }
        }
    }

    // Return error if key not found in both main and fallback
    Err(TranslationError::KeyNotFound(key.to_string(), lang.to_string()))
}

pub fn get_translation_with_params(
    key: &str,
    params: &HashMap<&str, &str>,
) -> Result<String, TranslationError> {
    let translation = get_translation(key)?;
    let mut result = translation.clone();

    for (param, value) in params {
        result = result.replace(&format!("{{{}}}", param), value);
    }

    Ok(result)
}

pub fn supported_languages() -> Vec<String> {
    let cache = TRANSLATION_CACHE.read().unwrap();
    cache.translations.keys().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    fn create_test_translations(dir: &Path) {
        fs::create_dir_all(dir.join("languages")).unwrap();

        let en_path = dir.join("languages/en.json");
        let mut en_file = File::create(en_path).unwrap();
        writeln!(en_file, r#"
        {{
            "translations": {{
                "greeting": "Hello",
                "welcome": "Welcome, {name}",
                "menu.save": "Save",
                "menu.save.world": "Save World"
            }}
        }}"#).unwrap();

        let es_path = dir.join("languages/es.json");
        let mut es_file = File::create(es_path).unwrap();
        writeln!(es_file, r#"
        {{
            "translations": {{
                "greeting": "Hola",
                "welcome": "Bienvenido, {name}",
                "menu.save": "Guardar",
                "menu.save.world": "Guardar Mundo"
            }}
        }}"#).unwrap();
    }

    #[test]
    fn test_translation_loading() {
        let dir = tempdir().unwrap();
        create_test_translations(dir.path());

        init_translations(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(supported_languages().len(), 2);

        // Mock config
        let mut config = crate::config::language::LanguageConfig::default();
        config.preferred = "es".to_string();
        
        assert_eq!(get_translation("greeting").unwrap(), "Hola");
        
        let mut params = HashMap::new();
        params.insert("name", "Juan");
        assert_eq!(
            get_translation_with_params("welcome", &params).unwrap(),
            "Bienvenido, Juan"
        );
    }
}
