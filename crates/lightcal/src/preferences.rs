//! Versioned JSON preferences with atomic replacement and isolated test directories.

use calendar_core::{Weekday, WeekdaySet};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrayIconMode {
    #[default]
    DateOnly,
    WeekdayAndDate,
    TimeAndDate,
}

impl TrayIconMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DateOnly => "date_only",
            Self::WeekdayAndDate => "weekday_and_date",
            Self::TimeAndDate => "time_and_date",
        }
    }

    pub fn from_str_lossy(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "weekday_and_date" | "weekday" | "weekday-date" => Self::WeekdayAndDate,
            "time_and_date" | "time" | "time-date" => Self::TimeAndDate,
            _ => Self::DateOnly,
        }
    }
}

impl ThemePreference {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn from_str_lossy(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }
}

/// Supported UI locales (`en` is always the default and fallback).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocaleCode {
    #[serde(rename = "en")]
    #[default]
    En,
    #[serde(rename = "ru")]
    Ru,
    #[serde(rename = "es")]
    Es,
    #[serde(rename = "ja")]
    Ja,
}

impl LocaleCode {
    pub const ALL: [LocaleCode; 4] = [Self::En, Self::Ru, Self::Es, Self::Ja];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Ru => "ru",
            Self::Es => "es",
            Self::Ja => "ja",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Ru => "Русский",
            Self::Es => "Español",
            Self::Ja => "日本語",
        }
    }

    pub fn from_code_or_default(code: &str) -> Self {
        match code.trim().to_ascii_lowercase().as_str() {
            "ru" => Self::Ru,
            "es" => Self::Es,
            "ja" => Self::Ja,
            _ => Self::En,
        }
    }
}

const fn default_true() -> bool {
    true
}

fn default_update_channel() -> String {
    "stable".to_string()
}

fn default_microsoft_client_id() -> String {
    option_env!("LIGHTCAL_MICROSOFT_CLIENT_ID")
        .unwrap_or("")
        .to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    pub schema_version: u32,
    pub language: String,
    pub theme: ThemePreference,
    pub first_weekday: Weekday,
    pub gray_weekdays_bits: u8,
    pub show_weekday: bool,
    pub show_week_numbers: bool,
    #[serde(default)]
    pub tray_icon_mode: TrayIconMode,
    #[serde(default)]
    pub tray_icon_monochrome: bool,
    pub start_with_windows: bool,
    pub global_shortcut: Option<String>,
    pub selected_calendar_ids: BTreeSet<String>,
    pub taskbar_window_pos: Option<(i32, i32)>,
    #[serde(default = "default_true")]
    pub highlight_current_month: bool,
    #[serde(default = "default_true")]
    pub show_event_dots: bool,
    #[serde(default)]
    pub auto_check_updates: bool,
    #[serde(default = "default_update_channel")]
    pub update_channel: String,
    #[serde(skip, default = "default_microsoft_client_id")]
    pub microsoft_client_id: String,
    #[serde(default)]
    pub windows_calendars_enabled: bool,
    #[serde(default)]
    pub auto_install_updates: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            // Required by docs/features.md: default to English ("en") on a fresh installation,
            // even on a non-English OS.
            language: LocaleCode::En.as_str().to_string(),
            theme: ThemePreference::System,
            first_weekday: Weekday::Monday,
            gray_weekdays_bits: WeekdaySet::default_weekends().bits(),
            show_weekday: true,
            show_week_numbers: true,
            tray_icon_mode: TrayIconMode::DateOnly,
            tray_icon_monochrome: false,
            start_with_windows: false,
            global_shortcut: None,
            // First connection selects none until the user chooses.
            selected_calendar_ids: BTreeSet::new(),
            taskbar_window_pos: None,
            highlight_current_month: true,
            show_event_dots: true,
            auto_check_updates: false,
            update_channel: "stable".to_string(),
            microsoft_client_id: default_microsoft_client_id(),
            windows_calendars_enabled: false,
            auto_install_updates: false,
        }
    }
}

impl Preferences {
    pub fn locale(&self) -> LocaleCode {
        LocaleCode::from_code_or_default(&self.language)
    }

    pub fn set_locale(&mut self, locale: LocaleCode) {
        self.language = locale.as_str().to_string();
    }

    pub fn gray_weekdays(&self) -> WeekdaySet {
        WeekdaySet::from_bits(self.gray_weekdays_bits)
    }

    pub fn set_gray_weekdays(&mut self, set: WeekdaySet) {
        self.gray_weekdays_bits = set.bits();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadOutcome {
    pub preferences: Preferences,
    pub warning: Option<String>,
    /// When a malformed or newer schema file was encountered, atomic saves must not
    /// silently clobber the user's original `preferences.json` unless explicitly updated.
    pub recovered_from_error: bool,
}

/// Resolves the writable data directory, honoring `LIGHTCAL_DATA_DIR` first.
pub fn resolve_data_dir() -> PathBuf {
    if let Ok(custom) = env::var("LIGHTCAL_DATA_DIR") {
        let trimmed = custom.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    #[cfg(windows)]
    {
        if let Ok(appdata) = env::var("APPDATA") {
            return PathBuf::from(appdata).join("Lightcal");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("Lightcal");
        }
    }
    env::temp_dir().join("lightcal-data")
}

pub struct PreferencesStore {
    dir: PathBuf,
}

impl PreferencesStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn from_env() -> Self {
        Self::new(resolve_data_dir())
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn file_path(&self) -> PathBuf {
        self.dir.join("preferences.json")
    }

    pub fn last_good_path(&self) -> PathBuf {
        self.dir.join("preferences.last_good.json")
    }

    pub fn preserved_corrupt_path(&self) -> PathBuf {
        self.dir.join("preferences.preserved.json")
    }

    /// Loads preferences from disk. On missing file, returns defaults cleanly.
    /// On malformed JSON or unsupported newer `schema_version`, preserves the original
    /// file, attempts to read `preferences.last_good.json` or safe defaults, and returns
    /// a user-visible warning.
    pub fn load(&self) -> LoadOutcome {
        let path = self.file_path();
        if !path.exists() {
            return LoadOutcome {
                preferences: Preferences::default(),
                warning: None,
                recovered_from_error: false,
            };
        }

        let raw = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(err) => {
                return LoadOutcome {
                    preferences: Preferences::default(),
                    warning: Some(format!("Could not read preferences file: {err}")),
                    recovered_from_error: true,
                };
            }
        };

        // First inspect schema_version if valid JSON object.
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
            if let Some(ver) = val.get("schema_version").and_then(|v| v.as_u64()) {
                if ver > CURRENT_SCHEMA_VERSION as u64 {
                    let _ = fs::copy(&path, self.preserved_corrupt_path());
                    return LoadOutcome {
                        preferences: self.load_last_good_or_default(),
                        warning: Some(format!(
                            "Preferences schema v{ver} is newer than supported v{CURRENT_SCHEMA_VERSION}; preserved original and loaded safe defaults."
                        )),
                        recovered_from_error: true,
                    };
                }
            }
        }

        match serde_json::from_str::<Preferences>(&raw) {
            Ok(mut prefs) => {
                // Normalize language code to supported set (fallback to "en").
                prefs.language = LocaleCode::from_code_or_default(&prefs.language)
                    .as_str()
                    .to_string();
                prefs.gray_weekdays_bits &= 0b0111_1111;
                LoadOutcome {
                    preferences: prefs,
                    warning: None,
                    recovered_from_error: false,
                }
            }
            Err(err) => {
                let _ = fs::copy(&path, self.preserved_corrupt_path());
                LoadOutcome {
                    preferences: self.load_last_good_or_default(),
                    warning: Some(format!(
                        "Malformed preferences file preserved ({err}); started with safe defaults."
                    )),
                    recovered_from_error: true,
                }
            }
        }
    }

    fn load_last_good_or_default(&self) -> Preferences {
        let backup = self.last_good_path();
        if let Ok(raw) = fs::read_to_string(backup) {
            if let Ok(mut prefs) = serde_json::from_str::<Preferences>(&raw) {
                prefs.language = LocaleCode::from_code_or_default(&prefs.language)
                    .as_str()
                    .to_string();
                prefs.gray_weekdays_bits &= 0b0111_1111;
                return prefs;
            }
        }
        Preferences::default()
    }

    /// Writes preferences atomically via a temporary file and retains `preferences.last_good.json`.
    pub fn save(&self, prefs: &Preferences) -> Result<(), String> {
        fs::create_dir_all(&self.dir)
            .map_err(|e| format!("Failed to create config directory: {e}"))?;

        let target = self.file_path();
        let tmp = self.dir.join("preferences.json.tmp");
        let last_good = self.last_good_path();

        let serialized = serde_json::to_string_pretty(prefs)
            .map_err(|e| format!("Failed to serialize preferences: {e}"))?;

        {
            let mut f = fs::File::create(&tmp)
                .map_err(|e| format!("Failed to create temporary preferences file: {e}"))?;
            f.write_all(serialized.as_bytes())
                .map_err(|e| format!("Failed to write temporary preferences file: {e}"))?;
            f.sync_all()
                .map_err(|e| format!("Failed to flush temporary preferences file: {e}"))?;
        }

        // If the existing target is valid JSON with supported schema, copy it to last_good before replacing.
        if target.exists() {
            if let Ok(existing_raw) = fs::read_to_string(&target) {
                if let Ok(existing_prefs) = serde_json::from_str::<Preferences>(&existing_raw) {
                    if existing_prefs.schema_version <= CURRENT_SCHEMA_VERSION {
                        let _ = fs::copy(&target, &last_good);
                    }
                } else if !self.preserved_corrupt_path().exists() {
                    let _ = fs::copy(&target, self.preserved_corrupt_path());
                }
            }
        }

        fs::rename(&tmp, &target)
            .map_err(|e| format!("Failed to replace preferences file: {e}"))?;
        let _ = fs::copy(&target, &last_good);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("lightcal-test-{label}-{nanos}"))
    }

    #[test]
    fn fresh_install_defaults_to_english_and_weekends() {
        let dir = unique_temp_dir("fresh");
        let store = PreferencesStore::new(&dir);
        let outcome = store.load();
        assert_eq!(outcome.preferences.language, "en");
        assert_eq!(outcome.preferences.locale(), LocaleCode::En);
        assert!(!outcome.preferences.start_with_windows);
        assert!(outcome.preferences.global_shortcut.is_none());
        assert!(outcome.preferences.selected_calendar_ids.is_empty());
        assert_eq!(
            outcome.preferences.gray_weekdays(),
            WeekdaySet::default_weekends()
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn roundtrip_save_and_reload() {
        let dir = unique_temp_dir("roundtrip");
        let store = PreferencesStore::new(&dir);
        let mut prefs = Preferences::default();
        prefs.set_locale(LocaleCode::Ja);
        prefs.theme = ThemePreference::Dark;
        prefs.first_weekday = Weekday::Sunday;
        prefs.set_gray_weekdays(WeekdaySet::from_slice(&[
            Weekday::Friday,
            Weekday::Saturday,
        ]));
        prefs.selected_calendar_ids.insert("cal-work".into());

        store.save(&prefs).unwrap();
        let loaded = store.load();
        assert!(loaded.warning.is_none());
        assert_eq!(loaded.preferences, prefs);
        assert!(store.last_good_path().exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_or_newer_schema_preserves_original_and_recovers() {
        let dir = unique_temp_dir("corrupt");
        fs::create_dir_all(&dir).unwrap();
        let store = PreferencesStore::new(&dir);

        // Save a valid last-known-good state first.
        let mut good = Preferences::default();
        good.set_locale(LocaleCode::Ru);
        store.save(&good).unwrap();

        // Corrupt preferences.json.
        fs::write(store.file_path(), "{ not valid json ").unwrap();
        let outcome = store.load();
        assert!(outcome.recovered_from_error);
        assert!(outcome.warning.is_some());
        assert_eq!(outcome.preferences.locale(), LocaleCode::Ru);
        assert!(store.preserved_corrupt_path().exists());

        // Newer schema version.
        fs::write(
            store.file_path(),
            r#"{"schema_version": 99, "language": "es"}"#,
        )
        .unwrap();
        let newer_outcome = store.load();
        assert!(newer_outcome.recovered_from_error);
        assert!(newer_outcome.warning.unwrap().contains("schema v99"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn tray_mode_ui_values_survive_persistence() {
        let dir = unique_temp_dir("tray_modes");
        let store = PreferencesStore::new(&dir);
        for (value, expected) in [
            ("date", TrayIconMode::DateOnly),
            ("weekday-date", TrayIconMode::WeekdayAndDate),
            ("time-date", TrayIconMode::TimeAndDate),
        ] {
            let prefs = Preferences {
                tray_icon_mode: TrayIconMode::from_str_lossy(value),
                ..Preferences::default()
            };
            assert_eq!(prefs.tray_icon_mode, expected);
            store.save(&prefs).unwrap();
            assert_eq!(store.load().preferences.tray_icon_mode, expected);
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn new_preferences_backward_compatibility_and_roundtrip() {
        let dir = unique_temp_dir("new_prefs");
        fs::create_dir_all(&dir).unwrap();
        let store = PreferencesStore::new(&dir);

        // Obsolete view selection is ignored while other preferences survive.
        let legacy_json = r#"{
            "schema_version": 1,
            "language": "en",
            "theme": "dark",
            "first_weekday": "monday",
            "gray_weekdays_bits": 96,
            "show_weekday": true,
            "show_week_numbers": true,
            "start_with_windows": false,
            "global_shortcut": null,
            "default_view_mode": "expanded",
            "microsoft_client_id": "obsolete-user-editable-value",
            "selected_calendar_ids": []
        }"#;
        fs::write(store.file_path(), legacy_json).unwrap();
        let loaded = store.load();
        assert!(loaded.warning.is_none());
        assert!(loaded.preferences.highlight_current_month);
        assert_eq!(
            loaded.preferences.microsoft_client_id,
            default_microsoft_client_id()
        );
        assert!(loaded.preferences.show_event_dots);
        assert!(!loaded.preferences.auto_check_updates);
        assert_eq!(loaded.preferences.update_channel, "stable");

        // Now modify and roundtrip
        let mut modified = loaded.preferences;
        modified.highlight_current_month = false;
        modified.show_event_dots = false;
        modified.auto_check_updates = true;
        modified.update_channel = "preview".to_string();

        store.save(&modified).unwrap();
        let reloaded = store.load();
        assert_eq!(reloaded.preferences, modified);
        assert!(!fs::read_to_string(store.file_path())
            .unwrap()
            .contains("default_view_mode"));
        assert!(!fs::read_to_string(store.file_path())
            .unwrap()
            .contains("microsoft_client_id"));

        let _ = fs::remove_dir_all(&dir);
    }
}
