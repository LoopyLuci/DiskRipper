use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
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

pub struct Translator {
    language: Language,
    strings: HashMap<String, String>,
}

impl Language {
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

impl Translator {
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
                strings.insert("no_drives".to_string(), "No se detectaron unidades".to_string());
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
                strings.insert("no_drives".to_string(), "Aucun lecteur optique".to_string());
            }
            _ => return Translator::new(Language::English),
        }

        Self { language, strings }
    }

    pub fn get(&self, key: &str) -> &str {
        self.strings.get(key).map(|s| s.as_str()).unwrap_or("")
    }
}

pub fn supported_languages() -> Vec<(String, String)> {
    Language::all().iter().map(|l| (l.code().to_string(), l.display_name().to_string())).collect()
}
