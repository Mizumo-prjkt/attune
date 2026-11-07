//! Localization (i18n) for Attune Desktop.
//!
//! Provides localized strings across English and Wikang Filipino (Tagalog),
//! and exports a clean translation dictionary template JSON so community
//! contributors can add new languages without hardcoding.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    English,
    Filipino,
}

impl Language {
    pub fn code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Filipino => "fil",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Language::English => "English (US)",
            Language::Filipino => "Wikang Filipino (Tagalog)",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code.trim().to_lowercase().as_str() {
            "fil" | "tagalog" | "tl" => Language::Filipino,
            _ => Language::English,
        }
    }
}

pub struct Strings {
    pub settings: &'static str,
    pub general: &'static str,
    pub audio: &'static str,
    pub language: &'static str,
    pub lyrics: &'static str,
    pub api_providers: &'static str,
    pub directory_and_indexes: &'static str,
    pub backup_and_restore: &'static str,
    pub developer_options: &'static str,
    pub save: &'static str,
    pub cancel: &'static str,
    pub reset: &'static str,
    pub open_in_window: &'static str,
    pub close: &'static str,
}

pub fn get_strings(lang: Language) -> Strings {
    match lang {
        Language::English => Strings {
            settings: "Settings & Preferences",
            general: "General",
            audio: "Audio",
            language: "Language",
            lyrics: "Lyrics",
            api_providers: "API Providers",
            directory_and_indexes: "Directory & Indexes",
            backup_and_restore: "Backup & Restore",
            developer_options: "Developer Options",
            save: "Save",
            cancel: "Cancel",
            reset: "Reset to Default",
            open_in_window: "Open in Separate Window",
            close: "Close Window",
        },
        Language::Filipino => Strings {
            settings: "Mga Setting at Kagustuhan",
            general: "Pangkalahatan",
            audio: "Tunog at Audio",
            language: "Wika",
            lyrics: "Titik ng Kanta",
            api_providers: "Mga Tagapagbigay ng API",
            directory_and_indexes: "Direktoryo at Index",
            backup_and_restore: "I-backup at Ibalik",
            developer_options: "Mga Opsyon ng Developer",
            save: "I-save",
            cancel: "Kanselahin",
            reset: "Ibalik sa Default",
            open_in_window: "Buksan sa Bagong Window",
            close: "Isara ang Window",
        },
    }
}

/// Generates a structured JSON template of all translation keys and English default strings.
pub fn export_language_template_json() -> String {
    let mut map = BTreeMap::new();
    let s = get_strings(Language::English);
    map.insert("settings", s.settings);
    map.insert("general", s.general);
    map.insert("audio", s.audio);
    map.insert("language", s.language);
    map.insert("lyrics", s.lyrics);
    map.insert("api_providers", s.api_providers);
    map.insert("directory_and_indexes", s.directory_and_indexes);
    map.insert("backup_and_restore", s.backup_and_restore);
    map.insert("developer_options", s.developer_options);
    map.insert("save", s.save);
    map.insert("cancel", s.cancel);
    map.insert("reset", s.reset);
    map.insert("open_in_window", s.open_in_window);
    map.insert("close", s.close);

    serde_json::to_string_pretty(&map).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_code() {
        assert_eq!(Language::from_code("en"), Language::English);
        assert_eq!(Language::from_code("fil"), Language::Filipino);
        assert_eq!(Language::from_code("tagalog"), Language::Filipino);
        assert_eq!(Language::from_code("unknown"), Language::English);
    }

    #[test]
    fn test_export_template_json_contains_keys() {
        let json = export_language_template_json();
        assert!(json.contains("\"general\""));
        assert!(json.contains("\"lyrics\""));
        assert!(json.contains("\"backup_and_restore\""));
    }
}
