//! Internationalization (i18n) support for DiskRipper.
//!
//! Provides multi-language translation infrastructure.
//! Currently supports English, Spanish, French, with placeholders for German, Japanese, Chinese, Russian, Portuguese.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    English,
    Spanish,
    French,
    German,
    Japanese,
    Chinese,
    Russian,
    Portuguese,
}

impl Language {
    /// Get all supported languages
    pub fn all() -> Vec<Language> {
        vec![
            Language::English,
            Language::Spanish,
            Language::French,
            Language::German,
            Language::Japanese,
            Language::Chinese,
            Language::Russian,
            Language::Portuguese,
        ]
    }

    /// Get display name for the language
    pub fn display_name(&self) -> &str {
        match self {
            Language::English => "English",
            Language::Spanish => "Español",
            Language::French => "Français",
            Language::German => "Deutsch",
            Language::Japanese => "日本語",
            Language::Chinese => "中文",
            Language::Russian => "Русский",
            Language::Portuguese => "Português",
        }
    }

    /// Get language code (ISO 639-1)
    pub fn code(&self) -> &str {
        match self {
            Language::English => "en",
            Language::Spanish => "es",
            Language::French => "fr",
            Language::German => "de",
            Language::Japanese => "ja",
            Language::Chinese => "zh",
            Language::Russian => "ru",
            Language::Portuguese => "pt",
        }
    }
}

/// Translator for a specific language
pub struct Translator {
    language: Language,
    strings: HashMap<String, String>,
}

impl Translator {
    /// Create a new translator for the given language
    pub fn new(language: Language) -> Self {
        let mut strings = HashMap::new();

        match language {
            Language::English => {
                strings.insert("app_title".to_string(), "DiskRipper".to_string());
                strings.insert("drives".to_string(), "Drives".to_string());
                strings.insert("jobs".to_string(), "Jobs".to_string());
                strings.insert("settings".to_string(), "Settings".to_string());
                strings.insert("rip".to_string(), "Rip".to_string());
                strings.insert("completed".to_string(), "Completed".to_string());
                strings.insert("failed".to_string(), "Failed".to_string());
                strings.insert("running".to_string(), "Running".to_string());
                strings.insert("no_drives".to_string(), "No optical drives detected".to_string());
                strings.insert("disc_found".to_string(), "Disc found".to_string());
                strings.insert("disc_type".to_string(), "Disc Type".to_string());
                strings.insert("size".to_string(), "Size".to_string());
                strings.insert("filesystem".to_string(), "Filesystem".to_string());
                strings.insert("tracks".to_string(), "Tracks".to_string());
                strings.insert("progress".to_string(), "Progress".to_string());
                strings.insert("speed".to_string(), "Speed".to_string());
                strings.insert("eta".to_string(), "ETA".to_string());
                strings.insert("output".to_string(), "Output".to_string());
                strings.insert("theme".to_string(), "Theme".to_string());
                strings.insert("language".to_string(), "Language".to_string());
                strings.insert("dark".to_string(), "Dark".to_string());
                strings.insert("light".to_string(), "Light".to_string());
                strings.insert("auto_organize".to_string(), "Auto-organize".to_string());
                strings.insert("feedback".to_string(), "Feedback".to_string());
                strings.insert("submit".to_string(), "Submit".to_string());
                strings.insert("confidence".to_string(), "Confidence".to_string());
                strings.insert("source".to_string(), "Source".to_string());
                strings.insert("title".to_string(), "Title".to_string());
                strings.insert("artist".to_string(), "Artist".to_string());
                strings.insert("album".to_string(), "Album".to_string());
                strings.insert("genre".to_string(), "Genre".to_string());
                strings.insert("identify".to_string(), "Identify".to_string());
                strings.insert("ml_panel".to_string(), "ML Identify".to_string());
                strings.insert("how_it_works".to_string(), "How It Works".to_string());
            }
            Language::Spanish => {
                strings.insert("app_title".to_string(), "DiskRipper".to_string());
                strings.insert("drives".to_string(), "Unidades".to_string());
                strings.insert("jobs".to_string(), "Trabajos".to_string());
                strings.insert("settings".to_string(), "Configuración".to_string());
                strings.insert("rip".to_string(), "Extraer".to_string());
                strings.insert("completed".to_string(), "Completado".to_string());
                strings.insert("failed".to_string(), "Fallido".to_string());
                strings.insert("running".to_string(), "Ejecutando".to_string());
                strings.insert("no_drives".to_string(), "No se detectaron unidades ópticas".to_string());
                strings.insert("disc_found".to_string(), "Disco encontrado".to_string());
                strings.insert("disc_type".to_string(), "Tipo de Disco".to_string());
                strings.insert("size".to_string(), "Tamaño".to_string());
                strings.insert("filesystem".to_string(), "Sistema de Archivos".to_string());
                strings.insert("tracks".to_string(), "Pistas".to_string());
                strings.insert("progress".to_string(), "Progreso".to_string());
                strings.insert("speed".to_string(), "Velocidad".to_string());
                strings.insert("eta".to_string(), "Tiempo Restante".to_string());
                strings.insert("output".to_string(), "Salida".to_string());
                strings.insert("theme".to_string(), "Tema".to_string());
                strings.insert("language".to_string(), "Idioma".to_string());
                strings.insert("dark".to_string(), "Oscuro".to_string());
                strings.insert("light".to_string(), "Claro".to_string());
                strings.insert("auto_organize".to_string(), "Auto-organizar".to_string());
                strings.insert("feedback".to_string(), "Comentarios".to_string());
                strings.insert("submit".to_string(), "Enviar".to_string());
                strings.insert("confidence".to_string(), "Confianza".to_string());
                strings.insert("source".to_string(), "Fuente".to_string());
                strings.insert("title".to_string(), "Título".to_string());
                strings.insert("artist".to_string(), "Artista".to_string());
                strings.insert("album".to_string(), "Álbum".to_string());
                strings.insert("genre".to_string(), "Género".to_string());
                strings.insert("identify".to_string(), "Identificar".to_string());
                strings.insert("ml_panel".to_string(), "Identificación ML".to_string());
                strings.insert("how_it_works".to_string(), "Cómo Funciona".to_string());
            }
            Language::French => {
                strings.insert("app_title".to_string(), "DiskRipper".to_string());
                strings.insert("drives".to_string(), "Lecteurs".to_string());
                strings.insert("jobs".to_string(), "Tâches".to_string());
                strings.insert("settings".to_string(), "Paramètres".to_string());
                strings.insert("rip".to_string(), "Extraire".to_string());
                strings.insert("completed".to_string(), "Terminé".to_string());
                strings.insert("failed".to_string(), "Échoué".to_string());
                strings.insert("running".to_string(), "En cours".to_string());
                strings.insert("no_drives".to_string(), "Aucun lecteur optique détecté".to_string());
                strings.insert("disc_found".to_string(), "Disque trouvé".to_string());
                strings.insert("disc_type".to_string(), "Type de Disque".to_string());
                strings.insert("size".to_string(), "Taille".to_string());
                strings.insert("filesystem".to_string(), "Système de Fichiers".to_string());
                strings.insert("tracks".to_string(), "Pistes".to_string());
                strings.insert("progress".to_string(), "Progrès".to_string());
                strings.insert("speed".to_string(), "Vitesse".to_string());
                strings.insert("eta".to_string(), "Temps Restant".to_string());
                strings.insert("output".to_string(), "Sortie".to_string());
                strings.insert("theme".to_string(), "Thème".to_string());
                strings.insert("language".to_string(), "Langue".to_string());
                strings.insert("dark".to_string(), "Sombre".to_string());
                strings.insert("light".to_string(), "Clair".to_string());
                strings.insert("auto_organize".to_string(), "Auto-organiser".to_string());
                strings.insert("feedback".to_string(), "Commentaires".to_string());
                strings.insert("submit".to_string(), "Soumettre".to_string());
                strings.insert("confidence".to_string(), "Confiance".to_string());
                strings.insert("source".to_string(), "Source".to_string());
                strings.insert("title".to_string(), "Titre".to_string());
                strings.insert("artist".to_string(), "Artiste".to_string());
                strings.insert("album".to_string(), "Album".to_string());
                strings.insert("genre".to_string(), "Genre".to_string());
                strings.insert("identify".to_string(), "Identifier".to_string());
                strings.insert("ml_panel".to_string(), "Identification ML".to_string());
                strings.insert("how_it_works".to_string(), "Comment Ça Marche".to_string());
            }
            _ => {
                // Default to English for unsupported languages
                return Translator::new(Language::English);
            }
        }

        Self { language, strings }
    }

    /// Get a translated string for the given key
    pub fn get(&self, key: &str) -> &str {
        self.strings.get(key).map(|s| s.as_str()).unwrap_or("")
    }

    /// Get the current language
    pub fn language(&self) -> Language {
        self.language
    }
}

/// Get list of supported languages as (code, display_name) pairs
pub fn supported_languages() -> Vec<(String, String)> {
    Language::all()
        .iter()
        .map(|l| (l.code().to_string(), l.display_name().to_string()))
        .collect()
}
