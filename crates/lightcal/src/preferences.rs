//! Versioned JSON preferences with atomic replacement and isolated test directories.

use calendar_core::{Weekday, WeekdaySet};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
/// Maximum number of preserved copies of unreadable or newer-schema files.
const MAX_PRESERVED_FILES: usize = 5;

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

/// Size of the day numbers in the calendar grid (14, 16 or 18 logical px).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DateSize {
    #[default]
    Standard,
    Large,
    Larger,
}

fn lenient_date_size<'de, D>(deserializer: D) -> Result<DateSize, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::String(value) => DateSize::from_str_lossy(&value),
        _ => DateSize::default(),
    })
}

fn lenient_theme<'de, D>(deserializer: D) -> Result<ThemePreference, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::String(value) => ThemePreference::from_str_lossy(&value),
        _ => ThemePreference::default(),
    })
}

fn lenient_tray_mode<'de, D>(deserializer: D) -> Result<TrayIconMode, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::String(value) => TrayIconMode::from_str_lossy(&value),
        _ => TrayIconMode::default(),
    })
}

impl TrayIconMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DateOnly => "date_only",
            Self::WeekdayAndDate => "weekday_and_date",
            Self::TimeAndDate => "time_and_date",
        }
    }

    /// True when `value` is a name or alias this build understands (case-insensitive).
    fn recognises(value: &str) -> bool {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "date_only"
                | "weekday_and_date"
                | "weekday"
                | "weekday-date"
                | "time_and_date"
                | "time"
                | "time-date"
        )
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

    /// True when `value` is a name this build understands (case-insensitive).
    fn recognises(value: &str) -> bool {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "system" | "light" | "dark"
        )
    }

    pub fn from_str_lossy(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }
}

impl DateSize {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Large => "large",
            Self::Larger => "larger",
        }
    }

    /// True when `value` is a name this build understands (case-insensitive).
    fn recognises(value: &str) -> bool {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "standard" | "large" | "larger"
        )
    }

    pub fn from_str_lossy(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "large" => Self::Large,
            "larger" => Self::Larger,
            _ => Self::Standard,
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

fn default_google_client_id() -> String {
    option_env!("LIGHTCAL_GOOGLE_CLIENT_ID")
        .unwrap_or("")
        .to_string()
}

fn default_google_client_secret() -> String {
    String::new()
}

/// Read one assignment from a dotenv file. Values are not logged.
pub fn env_assignment(text: &str, key: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .find(|(name, _)| name.trim() == key)
        .map(|(_, value)| {
            value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string()
        })
        .filter(|value| !value.is_empty())
}

/// Process environment, then a `.env` file beside the working directory or the executable.
/// The value is returned in memory and is not written to logs.
pub fn local_env_value(key: &str) -> String {
    if let Ok(value) = std::env::var(key) {
        let value = value
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        if !value.is_empty() {
            return value;
        }
    }
    let mut roots = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            roots.push(parent.to_path_buf());
        }
    }
    for mut dir in roots {
        for _ in 0..6 {
            if let Ok(text) = std::fs::read_to_string(dir.join(".env")) {
                if let Some(value) = env_assignment(&text, key) {
                    return value;
                }
            }
            if !dir.pop() {
                break;
            }
        }
    }
    String::new()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub schema_version: u32,
    pub language: String,
    #[serde(deserialize_with = "lenient_theme")]
    pub theme: ThemePreference,
    pub first_weekday: Weekday,
    pub gray_weekdays_bits: u8,
    pub show_weekday: bool,
    pub show_week_numbers: bool,
    #[serde(deserialize_with = "lenient_tray_mode")]
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
    // Koi swimming behind the calendar grid (ADR 0013).
    #[serde(default)]
    pub show_koi: bool,
    #[serde(deserialize_with = "lenient_date_size")]
    pub date_size: DateSize,
    #[serde(default)]
    pub auto_check_updates: bool,
    #[serde(default = "default_update_channel")]
    pub update_channel: String,
    #[serde(skip, default = "default_microsoft_client_id")]
    pub microsoft_client_id: String,
    #[serde(skip, default = "default_google_client_id")]
    pub google_client_id: String,
    #[serde(skip, default = "default_google_client_secret")]
    pub google_client_secret: String,
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
            show_koi: false,
            date_size: DateSize::Standard,
            auto_check_updates: false,
            update_channel: "stable".to_string(),
            microsoft_client_id: default_microsoft_client_id(),
            google_client_id: default_google_client_id(),
            google_client_secret: default_google_client_secret(),
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

fn has_schema_version(value: &serde_json::Value) -> bool {
    value.get("schema_version").is_some()
}

#[derive(Default)]
struct Deviations {
    /// Keys this build does not write.
    unknown_keys: bool,
    /// Known keys whose value could not be understood and fell back to a default.
    /// Benign canonicalization (case, ordering, duplicates) is not counted.
    fallback_values: bool,
}

impl Deviations {
    fn any(&self) -> bool {
        self.unknown_keys || self.fallback_values
    }
}

/// True when `raw` differs from `canonical` only by normalization that loses nothing.
fn is_benign_normalization(
    key: &str,
    raw: &serde_json::Value,
    canonical: &serde_json::Value,
) -> bool {
    match key {
        "theme" => raw.as_str().is_some_and(ThemePreference::recognises),
        "tray_icon_mode" => raw.as_str().is_some_and(TrayIconMode::recognises),
        "date_size" => raw.as_str().is_some_and(DateSize::recognises),
        "selected_calendar_ids" => match (raw.as_array(), canonical.as_array()) {
            (Some(raw), Some(canonical)) => {
                let raw: BTreeSet<_> = raw.iter().map(|item| item.as_str()).collect();
                let canonical: BTreeSet<_> = canonical.iter().map(|item| item.as_str()).collect();
                !raw.contains(&None) && raw == canonical
            }
            _ => false,
        },
        _ => false,
    }
}

/// Compares the raw file against the canonical serialization of what it parsed to.
/// Keys missing from the file are fine; unknown keys and values that fell back to a default
/// are not an exact round trip.
fn deviations(raw: &serde_json::Value, parsed: &Preferences) -> Deviations {
    let mut result = Deviations::default();
    let (serde_json::Value::Object(file), Ok(serde_json::Value::Object(expected))) =
        (raw, serde_json::to_value(parsed))
    else {
        result.fallback_values = true;
        return result;
    };
    for (key, value) in file {
        match expected.get(key) {
            None => result.unknown_keys = true,
            Some(canonical)
                if canonical != value && !is_benign_normalization(key, value, canonical) =>
            {
                result.fallback_values = true;
            }
            Some(_) => {}
        }
    }
    result
}

/// Why a load produced a warning; rendered in English by `LoadOutcome::warning`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum WarningKind {
    Unreadable(String),
    NewerSchema(u64),
    MissingSchema,
    ResetValues,
    Malformed(String),
}

impl WarningKind {
    fn message(&self, locale: LocaleCode) -> String {
        use LocaleCode::{En, Es, Ja, Ru};
        let n = CURRENT_SCHEMA_VERSION;
        match (self, locale) {
            (Self::Unreadable(e), En) => format!("Could not read preferences file: {e}"),
            (Self::Unreadable(e), Ru) => format!("Не удалось прочитать файл настроек: {e}"),
            (Self::Unreadable(e), Es) => {
                format!("No se pudo leer el archivo de preferencias: {e}")
            }
            (Self::Unreadable(e), Ja) => format!("設定ファイルを読み込めませんでした: {e}"),
            (Self::NewerSchema(v), En) => format!(
                "Preferences schema v{v} is newer than supported v{n}; the original file is kept and safe defaults were loaded."
            ),
            (Self::NewerSchema(v), Ru) => format!(
                "Схема настроек v{v} новее поддерживаемой v{n}; исходный файл сохранён, загружены безопасные значения по умолчанию."
            ),
            (Self::NewerSchema(v), Es) => format!(
                "El esquema de preferencias v{v} es más reciente que el compatible v{n}; se conserva el archivo original y se cargaron valores predeterminados seguros."
            ),
            (Self::NewerSchema(v), Ja) => format!(
                "設定のスキーマ v{v} は対応バージョン v{n} より新しいため、元のファイルは保持し、安全な既定値を読み込みました。"
            ),
            (Self::MissingSchema, En) => "Preferences file has no schema_version; a copy is kept and safe defaults were loaded.".into(),
            (Self::MissingSchema, Ru) => "В файле настроек нет schema_version; копия сохранена, загружены безопасные значения по умолчанию.".into(),
            (Self::MissingSchema, Es) => "El archivo de preferencias no tiene schema_version; se conserva una copia y se cargaron valores predeterminados seguros.".into(),
            (Self::MissingSchema, Ja) => "設定ファイルに schema_version がないため、コピーを保持し、安全な既定値を読み込みました。".into(),
            (Self::ResetValues, En) => "Some settings were not recognised and were reset; the original file is kept.".into(),
            (Self::ResetValues, Ru) => "Некоторые настройки не распознаны и сброшены; исходный файл сохранён.".into(),
            (Self::ResetValues, Es) => "Algunos ajustes no se reconocieron y se restablecieron; se conserva el archivo original.".into(),
            (Self::ResetValues, Ja) => "認識できない設定があったため初期値に戻しました。元のファイルは保持されています。".into(),
            (Self::Malformed(e), En) => format!(
                "Malformed preferences file ({e}); a copy is kept and safe defaults were loaded."
            ),
            (Self::Malformed(e), Ru) => format!(
                "Файл настроек повреждён ({e}); копия сохранена, загружены безопасные значения по умолчанию."
            ),
            (Self::Malformed(e), Es) => format!(
                "Archivo de preferencias dañado ({e}); se conserva una copia y se cargaron valores predeterminados seguros."
            ),
            (Self::Malformed(e), Ja) => format!(
                "設定ファイルが壊れています ({e})。コピーを保持し、安全な既定値を読み込みました。"
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadOutcome {
    pub preferences: Preferences,
    /// English warning text, kept for compatibility; see `localized_warning`.
    pub warning: Option<String>,
    warning_kind: Option<WarningKind>,
    /// When a malformed or newer schema file was encountered, atomic saves must not
    /// silently clobber the user's original `preferences.json` unless explicitly updated.
    pub recovered_from_error: bool,
}

impl LoadOutcome {
    fn new(preferences: Preferences, kind: Option<WarningKind>, recovered: bool) -> Self {
        Self {
            preferences,
            warning: kind.as_ref().map(|kind| kind.message(LocaleCode::En)),
            warning_kind: kind,
            recovered_from_error: recovered,
        }
    }

    /// The same warning in the language of the loaded preferences. Detail text from the
    /// parser or the OS stays untranslated.
    pub fn localized_warning(&self) -> Option<String> {
        self.warning_kind
            .as_ref()
            .map(|kind| kind.message(self.preferences.locale()))
    }
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

    /// Existing preserved copies (`preferences.preserved-<unix-seconds>[-n].json`), oldest first.
    pub fn preserved_files(&self) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = fs::read_dir(&self.dir)
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                    .filter(|path| {
                        path.file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| {
                                name.starts_with("preferences.preserved-")
                                    && name.ends_with(".json")
                            })
                    })
                    .collect()
            })
            .unwrap_or_default();
        files.sort_by_key(|path| {
            fs::metadata(path)
                .and_then(|meta| meta.modified())
                .ok()
                .zip(path.file_name().map(|name| name.to_os_string()))
        });
        files
    }

    fn preserve_existing(&self, source: &Path) {
        let seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        for attempt in 0..1000u32 {
            let name = if attempt == 0 {
                format!("preferences.preserved-{seconds}.json")
            } else {
                format!("preferences.preserved-{seconds}-{attempt}.json")
            };
            let destination = self.dir.join(name);
            if destination.exists() {
                continue;
            }
            if fs::copy(source, &destination).is_ok() {
                break;
            }
            return;
        }
        let files = self.preserved_files();
        if files.len() > MAX_PRESERVED_FILES {
            for stale in &files[..files.len() - MAX_PRESERVED_FILES] {
                let _ = fs::remove_file(stale);
            }
        }
    }

    /// Loads preferences from disk. On missing file, returns defaults cleanly.
    /// On malformed JSON or unsupported newer `schema_version`, preserves the original
    /// file, attempts to read `preferences.last_good.json` or safe defaults, and returns
    /// a user-visible warning.
    pub fn load(&self) -> LoadOutcome {
        let path = self.file_path();
        if !path.exists() {
            return LoadOutcome::new(Preferences::default(), None, false);
        }

        let raw = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(err) => {
                return LoadOutcome::new(
                    Preferences::default(),
                    Some(WarningKind::Unreadable(err.to_string())),
                    true,
                );
            }
        };

        let value = serde_json::from_str::<serde_json::Value>(&raw).ok();

        // First inspect schema_version if valid JSON object.
        if let Some(ver) = value
            .as_ref()
            .and_then(|val| val.get("schema_version"))
            .and_then(|v| v.as_u64())
        {
            if ver > CURRENT_SCHEMA_VERSION as u64 {
                return LoadOutcome::new(
                    self.load_last_good_or_default(),
                    Some(WarningKind::NewerSchema(ver)),
                    true,
                );
            }
        }

        // A JSON document without `schema_version` is not a preferences file (for example `{}`).
        if value.as_ref().is_some_and(|val| !has_schema_version(val)) {
            return LoadOutcome::new(
                self.load_last_good_or_default(),
                Some(WarningKind::MissingSchema),
                true,
            );
        }

        match serde_json::from_str::<Preferences>(&raw) {
            Ok(mut prefs) => {
                let fell_back = value
                    .as_ref()
                    .is_some_and(|val| deviations(val, &prefs).fallback_values);
                // Normalize language code to supported set (fallback to "en").
                prefs.language = LocaleCode::from_code_or_default(&prefs.language)
                    .as_str()
                    .to_string();
                prefs.gray_weekdays_bits &= 0b0111_1111;
                LoadOutcome::new(prefs, fell_back.then_some(WarningKind::ResetValues), false)
            }
            Err(err) => LoadOutcome::new(
                self.load_last_good_or_default(),
                Some(WarningKind::Malformed(err.to_string())),
                true,
            ),
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
        let tmp = self.dir.join(format!(
            "preferences.json.{}-{}.tmp",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
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

        // Keep the user's original file when it cannot be read as current preferences, so
        // recovering with defaults never silently destroys it. Otherwise refresh last_good.
        if target.exists() {
            if let Ok(existing_raw) = fs::read_to_string(&target) {
                let exact = serde_json::from_str::<serde_json::Value>(&existing_raw)
                    .ok()
                    .zip(serde_json::from_str::<Preferences>(&existing_raw).ok())
                    .is_some_and(|(value, existing)| {
                        existing.schema_version <= CURRENT_SCHEMA_VERSION
                            && has_schema_version(&value)
                            && !deviations(&value, &existing).any()
                    });
                if exact {
                    let _ = fs::copy(&target, &last_good);
                } else {
                    self.preserve_existing(&target);
                }
            } else {
                self.preserve_existing(&target);
            }
        }

        fs::rename(&tmp, &target).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            format!("Failed to replace preferences file: {e}")
        })?;
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
        // Loading never rewrites the file; the next save preserves it.
        assert_eq!(
            fs::read_to_string(store.file_path()).unwrap(),
            "{ not valid json "
        );

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
    fn env_assignment_reads_the_named_value_and_ignores_comments() {
        let text = "\
OTHER=1
# LIGHTCAL_GOOGLE_CLIENT_SECRET=commented
LIGHTCAL_GOOGLE_CLIENT_SECRET=\"desktop-secret\"
";
        assert_eq!(
            env_assignment(text, "LIGHTCAL_GOOGLE_CLIENT_SECRET").as_deref(),
            Some("desktop-secret")
        );
        assert!(env_assignment(text, "MISSING").is_none());
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
        assert!(!loaded.preferences.show_koi);
        assert_eq!(loaded.preferences.date_size, DateSize::Standard);
        assert!(!loaded.preferences.auto_check_updates);
        assert_eq!(loaded.preferences.update_channel, "stable");

        // Now modify and roundtrip
        let mut modified = loaded.preferences;
        modified.highlight_current_month = false;
        modified.show_event_dots = false;
        modified.show_koi = true;
        modified.date_size = DateSize::Larger;
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

    #[test]
    fn save_after_recovery_preserves_original_under_unique_names() {
        let dir = unique_temp_dir("preserve");
        fs::create_dir_all(&dir).unwrap();
        let store = PreferencesStore::new(&dir);
        fs::write(store.file_path(), "{ broken original").unwrap();
        let loaded = store.load();
        assert!(loaded.recovered_from_error);
        store.save(&loaded.preferences).unwrap();
        let preserved = store.preserved_files();
        assert_eq!(preserved.len(), 1);
        assert_eq!(
            fs::read_to_string(&preserved[0]).unwrap(),
            "{ broken original"
        );

        // Newer schema, saved repeatedly, is preserved without clobbering and capped at 5.
        for index in 0..8 {
            fs::write(
                store.file_path(),
                format!(r#"{{"schema_version": 99, "n": {index}}}"#),
            )
            .unwrap();
            store.save(&Preferences::default()).unwrap();
        }
        let preserved = store.preserved_files();
        assert_eq!(preserved.len(), MAX_PRESERVED_FILES);
        assert!(preserved
            .iter()
            .any(|path| fs::read_to_string(path).unwrap().contains(r#""n": 7"#)));
        assert!(fs::read_dir(&dir).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_enum_values_and_missing_fields_fall_back_per_field() {
        let dir = unique_temp_dir("lenient");
        fs::create_dir_all(&dir).unwrap();
        let store = PreferencesStore::new(&dir);
        fs::write(
            store.file_path(),
            r#"{"schema_version": 1, "theme": "sepia", "tray_icon_mode": 7, "language": "ja", "show_event_dots": false, "date_size": "huge"}"#,
        )
        .unwrap();
        let loaded = store.load();
        assert!(loaded.warning.is_some());
        assert!(!loaded.recovered_from_error);
        assert_eq!(loaded.preferences.theme, ThemePreference::System);
        assert_eq!(loaded.preferences.tray_icon_mode, TrayIconMode::DateOnly);
        assert_eq!(loaded.preferences.date_size, DateSize::Standard);
        assert_eq!(loaded.preferences.locale(), LocaleCode::Ja);
        assert!(!loaded.preferences.show_event_dots);
        assert!(loaded.preferences.highlight_current_month);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn date_size_round_trips_and_loads_case_insensitively() {
        for size in [DateSize::Standard, DateSize::Large, DateSize::Larger] {
            assert_eq!(DateSize::from_str_lossy(size.as_str()), size);
        }
        assert_eq!(DateSize::from_str_lossy("huge"), DateSize::Standard);

        let dir = unique_temp_dir("date-size");
        fs::create_dir_all(&dir).unwrap();
        let store = PreferencesStore::new(&dir);
        fs::write(
            store.file_path(),
            r#"{"schema_version": 1, "date_size": "LARGE"}"#,
        )
        .unwrap();
        let loaded = store.load();
        assert!(loaded.warning.is_none());
        assert_eq!(loaded.preferences.date_size, DateSize::Large);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_enum_value_warns_and_original_is_preserved_on_save() {
        let dir = unique_temp_dir("unknown_enum");
        fs::create_dir_all(&dir).unwrap();
        let store = PreferencesStore::new(&dir);
        let original = r#"{"schema_version": 1, "theme": "sepia"}"#;
        fs::write(store.file_path(), original).unwrap();
        let loaded = store.load();
        assert!(loaded.warning.is_some());
        assert!(!loaded.recovered_from_error);
        assert_eq!(loaded.preferences.theme, ThemePreference::System);
        store.save(&loaded.preferences).unwrap();
        let preserved = store.preserved_files();
        assert_eq!(preserved.len(), 1);
        assert_eq!(fs::read_to_string(&preserved[0]).unwrap(), original);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_key_is_preserved_on_save_without_load_warning() {
        let dir = unique_temp_dir("unknown_key");
        fs::create_dir_all(&dir).unwrap();
        let store = PreferencesStore::new(&dir);
        fs::write(
            store.file_path(),
            r#"{"schema_version": 1, "from_newer_build": true}"#,
        )
        .unwrap();
        let loaded = store.load();
        assert!(loaded.warning.is_none());
        store.save(&loaded.preferences).unwrap();
        assert_eq!(store.preserved_files().len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn object_without_schema_version_warns_and_is_preserved() {
        let dir = unique_temp_dir("no_schema");
        fs::create_dir_all(&dir).unwrap();
        let store = PreferencesStore::new(&dir);
        for original in ["{}", r#"{"unrelated": 1}"#] {
            fs::write(store.file_path(), original).unwrap();
            let loaded = store.load();
            assert!(loaded.warning.is_some(), "{original}");
            assert!(loaded.recovered_from_error);
            store.save(&loaded.preferences).unwrap();
            assert!(store
                .preserved_files()
                .iter()
                .any(|path| fs::read_to_string(path).unwrap() == original));
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_alias_and_unsorted_ids_load_without_warning_or_preservation() {
        let dir = unique_temp_dir("benign");
        fs::create_dir_all(&dir).unwrap();
        let store = PreferencesStore::new(&dir);
        fs::write(
            store.file_path(),
            r#"{"schema_version": 1, "theme": "Dark", "tray_icon_mode": "WEEKDAY", "selected_calendar_ids": ["b", "a", "b"]}"#,
        )
        .unwrap();
        let loaded = store.load();
        assert!(loaded.warning.is_none());
        assert!(loaded.localized_warning().is_none());
        assert_eq!(loaded.preferences.theme, ThemePreference::Dark);
        assert_eq!(loaded.preferences.selected_calendar_ids.len(), 2);
        store.save(&loaded.preferences).unwrap();
        assert!(store.preserved_files().is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn wrong_typed_enum_value_warns() {
        let dir = unique_temp_dir("wrong_type");
        fs::create_dir_all(&dir).unwrap();
        let store = PreferencesStore::new(&dir);
        fs::write(
            store.file_path(),
            r#"{"schema_version": 1, "theme": 3, "selected_calendar_ids": ["a"]}"#,
        )
        .unwrap();
        assert!(store.load().warning.is_some());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn localized_warning_follows_the_loaded_language() {
        let dir = unique_temp_dir("localized_warning");
        fs::create_dir_all(&dir).unwrap();
        let store = PreferencesStore::new(&dir);
        for (code, expected) in [
            ("ru", "Некоторые настройки"),
            ("es", "Algunos ajustes"),
            ("ja", "認識できない設定"),
            ("en", "Some settings were not recognised"),
        ] {
            fs::write(
                store.file_path(),
                format!(r#"{{"schema_version": 1, "language": "{code}", "theme": "sepia"}}"#),
            )
            .unwrap();
            let loaded = store.load();
            assert!(loaded.localized_warning().unwrap().starts_with(expected));
            assert!(loaded
                .warning
                .as_deref()
                .unwrap()
                .starts_with("Some settings were not recognised"));
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_warning_kind_has_text_in_every_locale() {
        let kinds = [
            WarningKind::Unreadable("e".into()),
            WarningKind::NewerSchema(99),
            WarningKind::MissingSchema,
            WarningKind::ResetValues,
            WarningKind::Malformed("e".into()),
        ];
        for kind in &kinds {
            let english = kind.message(LocaleCode::En);
            for locale in LocaleCode::ALL {
                let text = kind.message(locale);
                assert!(!text.is_empty());
                if locale != LocaleCode::En {
                    assert_ne!(text, english);
                }
            }
        }
        assert!(WarningKind::NewerSchema(99)
            .message(LocaleCode::Ru)
            .contains("v99"));
    }

    #[test]
    fn exact_valid_file_has_no_warning_and_no_preserved_copy() {
        let dir = unique_temp_dir("exact");
        let store = PreferencesStore::new(&dir);
        let prefs = Preferences {
            theme: ThemePreference::Dark,
            selected_calendar_ids: BTreeSet::from(["a".to_string()]),
            ..Preferences::default()
        };
        store.save(&prefs).unwrap();
        let loaded = store.load();
        assert!(loaded.warning.is_none());
        assert!(!loaded.recovered_from_error);
        store.save(&loaded.preferences).unwrap();
        store.save(&loaded.preferences).unwrap();
        assert!(store.preserved_files().is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn preserved_cap_holds_for_unknown_enum_files() {
        let dir = unique_temp_dir("enum_cap");
        fs::create_dir_all(&dir).unwrap();
        let store = PreferencesStore::new(&dir);
        for index in 0..8 {
            fs::write(
                store.file_path(),
                format!(r#"{{"schema_version": 1, "theme": "sepia{index}"}}"#),
            )
            .unwrap();
            store.save(&Preferences::default()).unwrap();
        }
        assert_eq!(store.preserved_files().len(), MAX_PRESERVED_FILES);
        let _ = fs::remove_dir_all(&dir);
    }
}
