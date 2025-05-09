use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use log::{info, warn};

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TranslationFile {
    translations: HashMap<String, String>,
}

static TRANSLATIONS: OnceLock<HashMap<String, TranslationFile>> = OnceLock::new();

/// Initialize the translation system
pub fn init_translations(base_path: &str) -> Result<(), TranslationError> {
    let translations = load_all_translations(base_path)?;
    TRANSLATIONS.set(translations).map_err(|_| TranslationError::LoadError("Translations already initialized".into()))?;
    Ok(())
}

fn load_all_translations(base_path: &str) -> Result<HashMap<String, TranslationFile>, TranslationError> {
    let translations_dir = Path::new(base_path).join("languages");
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
                        info!("Loaded language: {}", language_code);
                    }
                    Err(e) => {
                        warn!("Failed to load {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    info!("Loaded {} language(s)", loaded_languages);
    Ok(translations)
}

fn load_translation_file(path: &Path) -> Result<TranslationFile, TranslationError> {
    let content = fs::read_to_string(path)?;
    let translation_file: TranslationFile = serde_json::from_str(&content)?;
    Ok(translation_file)
}

pub fn get_translation(language: &str, key: &str) -> Result<String, TranslationError> {
    let translations = TRANSLATIONS.get().ok_or_else(|| TranslationError::LoadError("Translations not initialized".into()))?;

    let translation_file = translations
        .get(language)
        .ok_or_else(|| TranslationError::UnsupportedLanguage(language.to_string()))?;

    translation_file.translations
        .get(key)
        .map(|s| s.to_string())
        .ok_or_else(|| TranslationError::KeyNotFound(key.to_string(), language.to_string()))
}

pub fn get_translation_with_params(
    language: &str,
    key: &str,
    params: &HashMap<&str, &str>,
) -> Result<String, TranslationError> {
    let translation = get_translation(language, key)?;
    let mut result = translation.clone();

    for (param, value) in params {
        result = result.replace(&format!("{{{}}}", param), value);
    }

    Ok(result)
}

pub fn supported_languages() -> Vec<String> {
    TRANSLATIONS.get()
        .map(|t| t.keys().cloned().collect())
        .unwrap_or_default()
}

pub fn get_system_language() -> String {
    std::env::var("LANG")
        .unwrap_or_else(|_| "en".to_string())
        .split('.')
        .next()
        .and_then(|s| s.split('_').next())
        .unwrap_or("en")
        .to_lowercase()
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
                "messages.one": "You have 1 message",
                "messages.other": "You have {count} messages"
            }}
        }}"#).unwrap();

        let es_path = dir.join("languages/es.json");
        let mut es_file = File::create(es_path).unwrap();
        writeln!(es_file, r#"
        {{
            "translations": {{
                "greeting": "Hola",
                "welcome": "Bienvenido, {name}"
            }}
        }}"#).unwrap();
    }

    #[test]
    fn test_translation_loading() {
        let dir = tempdir().unwrap();
        create_test_translations(dir.path());

        init_translations(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(supported_languages().len(), 2);

        assert_eq!(get_translation("en", "greeting").unwrap(), "Hello");
        assert_eq!(get_translation("es", "greeting").unwrap(), "Hola");
        
        let mut params = HashMap::new();
        params.insert("name", "John");
        assert_eq!(
            get_translation_with_params("en", "welcome", &params).unwrap(),
            "Welcome, John"
        );
    }

    #[test]
    fn test_plural_translation() {
        let dir = tempdir().unwrap();
        create_test_translations(dir.path());
        init_translations(dir.path().to_str().unwrap()).unwrap();

        // Test plural variants directly
        assert_eq!(
            get_translation("en", "messages.one").unwrap(),
            "You have 1 message"
        );
        
        let mut params = HashMap::new();
        params.insert("count", "5");
        assert_eq!(
            get_translation_with_params("en", "messages.other", &params).unwrap(),
            "You have 5 messages"
        );
    }
}

/* USAGE

// Initialize once at app start
init_translations("path/to/translations").unwrap();

// Simple lookup
get_translation("en", "greeting").unwrap();

// With parameters
let mut params = HashMap::new();
params.insert("name", "Alice");
get_translation_with_params("en", "welcome", &params).unwrap();

// Plural handling (directly use your keys)
get_translation("en", "messages.one").unwrap();  // "You have 1 message"
get_translation("en", "messages.many").unwrap(); // "You have {count} messages" 

*/
