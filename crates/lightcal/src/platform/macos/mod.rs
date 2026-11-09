//! Native macOS services. All AppKit entry points run on Slint's main thread.
use crate::{
    platform::{ProviderSnapshot, ProviderStatus},
    preferences::{MenuWeekday, Preferences},
};
use calendar_core::{CalendarOccurrence, CalendarSource, LocalDate};
use chrono::{Datelike, TimeZone, Timelike};
use std::{
    ffi::{c_char, CStr, CString},
    path::Path,
};
unsafe extern "C" {
    fn lc_credentials_read(output: *mut *mut c_char) -> i32;
    fn lc_credentials_write(json: *const c_char, remove: bool) -> i32;
    fn lc_labels(json: *const c_char);
    fn lc_init(callback: extern "C" fn(i32));
    fn lc_update(
        text: *const c_char,
        day: i32,
        mono: bool,
        settings: bool,
        automatic: bool,
        download: bool,
    );
    fn lc_visible() -> i32;
    fn lc_attach(view: *mut std::ffi::c_void);
    fn lc_place();
    fn lc_begin_transition(duration: f64);
    fn lc_finish_transition();
    fn lc_cancel_transition();
    fn lc_show();
    fn lc_open(url: *const c_char);
    fn lc_calendar();
    fn lc_permission();
    fn lc_dark() -> bool;
    fn lc_motion() -> bool;
    fn lc_startup(enable: bool) -> i32;
    fn lc_startup_status() -> i32;
    fn lc_shortcut(code: i32, modifiers: u32) -> bool;
    fn lc_updater_ready() -> bool;
    fn lc_check_updates();
    fn lc_copy(text: *const c_char);
    fn lc_events(from: f64, to: f64) -> *mut c_char;
    fn lc_free(value: *mut c_char);
}
pub fn initialize(callback: extern "C" fn(i32)) {
    unsafe { lc_init(callback) }
}
pub fn menu_text(prefs: &Preferences, date: LocalDate) -> String {
    let weekday = match prefs.mac_weekday {
        MenuWeekday::Off => "",
        MenuWeekday::Short => crate::i18n::weekday_short(prefs.locale(), date.weekday()),
        MenuWeekday::Full => crate::i18n::weekday_full(prefs.locale(), date.weekday()),
    };
    if prefs.mac_show_month {
        format!(
            "{weekday} {}",
            crate::i18n::month_short(prefs.locale(), date.month)
        )
        .trim()
        .into()
    } else {
        weekday.into()
    }
}
pub fn sync(prefs: &Preferences, settings: bool) {
    let labels = crate::i18n::mac_labels(prefs.locale());
    let tray = crate::i18n::tray_menu_copy(prefs.locale());
    let commands = crate::i18n::mac_menu_labels(prefs.locale());
    let texts = [
        tray.open,
        tray.settings,
        commands[0],
        commands[1],
        labels[1],
        labels[2],
        labels[3],
        commands[2],
        commands[3],
        commands[4],
        labels[0],
    ];
    let payload = serde_json::json!({"titles":texts,"weekday":match prefs.mac_weekday { MenuWeekday::Off=>0,MenuWeekday::Short=>1,MenuWeekday::Full=>2 },"show_month":prefs.mac_show_month});
    let json = CString::new(serde_json::to_vec(&payload).unwrap()).unwrap();
    unsafe { lc_labels(json.as_ptr()) };
    let now = chrono::Local::now();
    let date = LocalDate::new(now.year(), now.month() as u8, now.day() as u8).unwrap();
    let text = CString::new(menu_text(prefs, date)).unwrap();
    unsafe {
        lc_update(
            text.as_ptr(),
            date.day.into(),
            prefs.tray_icon_monochrome,
            settings,
            prefs.auto_check_updates,
            prefs.auto_install_updates,
        )
    }
}
/// Attach the exact Slint-owned native window instead of relying on its title.
pub fn attach(window: &slint::Window) -> bool {
    use slint::winit_030::{
        winit::raw_window_handle::{HasWindowHandle, RawWindowHandle},
        WinitWindowAccessor,
    };
    window
        .with_winit_window(|native| {
            if let Ok(handle) = native.window_handle() {
                if let RawWindowHandle::AppKit(handle) = handle.as_raw() {
                    unsafe { lc_attach(handle.ns_view.as_ptr()) };
                    return true;
                }
            }
            false
        })
        .unwrap_or(false)
}
pub fn watch(window: &slint::Window) {
    use slint::winit_030::{winit::event::WindowEvent, EventResult, WinitWindowAccessor};
    let mut first_frame = true;
    window.on_winit_window_event(move |window, event| {
        let first_redraw = first_frame && matches!(event, WindowEvent::RedrawRequested);
        if first_redraw {
            first_frame = false;
        }
        if first_redraw
            || matches!(
                event,
                WindowEvent::Resized(_)
                    | WindowEvent::Moved(_)
                    | WindowEvent::Focused(true)
                    | WindowEvent::ScaleFactorChanged { .. }
            )
        {
            place(window);
        }
        EventResult::Propagate
    });
}
pub fn place(window: &slint::Window) {
    if attach(window) {
        unsafe { lc_place() }
    }
}
pub fn begin_transition(window: &slint::Window, duration: i64) {
    if attach(window) {
        unsafe { lc_begin_transition(duration as f64 / 1000.0) }
    }
}
pub fn finish_transition() {
    unsafe { lc_finish_transition() }
}
pub fn cancel_transition(window: &slint::Window) {
    if attach(window) {
        unsafe { lc_cancel_transition() }
    }
}
pub fn bring_to_foreground() {
    unsafe { lc_show() }
}
pub fn system_prefers_dark_mode() -> bool {
    unsafe { lc_dark() }
}
pub fn window_view() -> (bool, bool) {
    let state = unsafe { lc_visible() };
    (state > 0, state == 2)
}
pub fn motion_enabled() -> bool {
    unsafe { lc_motion() }
}
pub fn open_url(url: &str) {
    if let Ok(url) = CString::new(url) {
        unsafe { lc_open(url.as_ptr()) }
    }
}
pub fn open_calendar() {
    unsafe { lc_calendar() }
}
pub fn request_access() {
    unsafe { lc_permission() }
}
pub fn updates_available() -> bool {
    unsafe { lc_updater_ready() }
}
pub fn check_updates() {
    unsafe { lc_check_updates() }
}
pub fn copy_text(text: &str) -> Result<(), String> {
    let text = CString::new(text).map_err(|e| e.to_string())?;
    unsafe { lc_copy(text.as_ptr()) }
    Ok(())
}
pub fn query_calendar_provider(anchor: LocalDate, synthetic: bool) -> ProviderSnapshot {
    if synthetic {
        ProviderSnapshot::synthetic(anchor)
    } else {
        load(anchor, true).snapshot
    }
}
pub struct StartupManager {
    isolated: bool,
}
impl StartupManager {
    pub fn from_env(directory: &Path) -> Self {
        Self {
            isolated: cfg!(test)
                || std::env::var_os("LIGHTCAL_DATA_DIR").is_some()
                || directory != crate::preferences::resolve_data_dir(),
        }
    }
    pub fn set_enabled(&self, enable: bool, _: &Path) -> Result<(), String> {
        if self.isolated {
            return Err("Login registration is disabled with isolated application data".into());
        }
        if unsafe { lc_startup(enable) } == 0 {
            Ok(())
        } else {
            Err("Check Lightcal in System Settings → General → Login Items".into())
        }
    }
}
pub fn startup_enabled() -> bool {
    unsafe { lc_startup_status() != 0 }
}
#[derive(Default)]
pub struct ShortcutManager {
    binding: Option<String>,
    isolated: bool,
}
impl ShortcutManager {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn isolated() -> Self {
        Self {
            isolated: true,
            ..Self::default()
        }
    }
    pub fn listen_for_trigger<F: Fn() + Send + 'static>(&self, _: F) { /* Native callback posts event 0. */
    }
    pub fn bind(&mut self, requested: Option<&str>) -> Result<Option<String>, String> {
        let (code, mods) = if let Some(binding) = requested {
            parse_shortcut(binding)?
        } else {
            (-1, 0)
        };
        if !self.isolated
            && !cfg!(test)
            && std::env::var_os("LIGHTCAL_DATA_DIR").is_none()
            && !unsafe { lc_shortcut(code, mods) }
        {
            return Err("Shortcut is already in use".into());
        }
        self.binding = requested.map(str::to_string);
        Ok(self.binding.clone())
    }
}
fn parse_shortcut(binding: &str) -> Result<(i32, u32), String> {
    let mut modifiers = 0;
    let mut key = None;
    // Physical ANSI positions from HIToolbox/Events.h. Carbon codes are sparse,
    // so an index into a string of characters cannot represent this mapping.
    for part in binding.split('+') {
        match part.trim().to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= 4096,
            "alt" | "option" => modifiers |= 2048,
            "shift" => modifiers |= 512,
            "win" | "meta" | "cmd" | "super" | "command" => modifiers |= 256,
            value => {
                let code = match value {
                    "a" => 0x00,
                    "s" => 0x01,
                    "d" => 0x02,
                    "f" => 0x03,
                    "h" => 0x04,
                    "g" => 0x05,
                    "z" => 0x06,
                    "x" => 0x07,
                    "c" => 0x08,
                    "v" => 0x09,
                    "b" => 0x0B,
                    "q" => 0x0C,
                    "w" => 0x0D,
                    "e" => 0x0E,
                    "r" => 0x0F,
                    "y" => 0x10,
                    "t" => 0x11,
                    "1" => 0x12,
                    "2" => 0x13,
                    "3" => 0x14,
                    "4" => 0x15,
                    "6" => 0x16,
                    "5" => 0x17,
                    "=" => 0x18,
                    "9" => 0x19,
                    "7" => 0x1A,
                    "-" => 0x1B,
                    "8" => 0x1C,
                    "0" => 0x1D,
                    "]" => 0x1E,
                    "o" => 0x1F,
                    "u" => 0x20,
                    "[" => 0x21,
                    "i" => 0x22,
                    "p" => 0x23,
                    "l" => 0x25,
                    "j" => 0x26,
                    "'" => 0x27,
                    "k" => 0x28,
                    ";" => 0x29,
                    "\\" => 0x2A,
                    "," => 0x2B,
                    "/" => 0x2C,
                    "n" => 0x2D,
                    "m" => 0x2E,
                    "." => 0x2F,
                    "space" => 0x31,
                    "`" => 0x32,
                    "f1" => 0x7A,
                    "f2" => 0x78,
                    "f3" => 0x63,
                    "f4" => 0x76,
                    "f5" => 0x60,
                    "f6" => 0x61,
                    "f7" => 0x62,
                    "f8" => 0x64,
                    "f9" => 0x65,
                    "f10" => 0x6D,
                    "f11" => 0x67,
                    "f12" => 0x6F,
                    _ => return Err("Unsupported shortcut key".into()),
                };
                if key.replace(code).is_some() {
                    return Err("Choose only one shortcut key".into());
                }
            }
        }
    }
    if modifiers == 0 {
        return Err("Choose at least one modifier".into());
    }
    key.map(|key| (key, modifiers))
        .ok_or_else(|| "Choose a shortcut key".into())
}
pub enum SingleInstance {
    FirstInstance(SingleInstanceGuard),
    AlreadyRunning,
}
pub struct SingleInstanceGuard {
    _file: std::fs::File,
}
impl SingleInstance {
    pub fn acquire(app_id: &str) -> Self {
        Self::acquire_at(
            app_id,
            &crate::preferences::resolve_data_dir(),
            std::env::var_os("LIGHTCAL_DATA_DIR").is_none(),
        )
    }

    /// Explicit isolation for callers whose linked library is not built with
    /// cfg(test), including binary tests. Never activates a real installation.
    pub fn acquire_isolated(app_id: &str, directory: &Path) -> Self {
        Self::acquire_at(app_id, directory, false)
    }

    fn acquire_at(app_id: &str, dir: &Path, activate_existing: bool) -> Self {
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        app_id.hash(&mut hash);
        std::fs::create_dir_all(dir).expect("Cannot create Lightcal data directory");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(dir.join(format!("instance-{:016x}.lock", hash.finish())))
            .expect("Cannot open instance lock");
        if file.try_lock().is_err() {
            if activate_existing {
                open_calendar_instance();
            }
            Self::AlreadyRunning
        } else {
            Self::FirstInstance(SingleInstanceGuard { _file: file })
        }
    }
}
fn open_calendar_instance() {
    let _ = std::process::Command::new("open")
        .args(["-b", "app.koona.lightcal"])
        .spawn();
}
#[derive(serde::Deserialize)]
struct StoreData {
    denied: bool,
    sources: Vec<Source>,
    events: Vec<Event>,
}
#[derive(serde::Deserialize)]
struct Source {
    id: String,
    name: String,
    account: String,
    color: String,
}
#[derive(serde::Deserialize)]
struct Event {
    id: String,
    source: String,
    title: String,
    location: String,
    all_day: bool,
    start: f64,
    end: f64,
    #[serde(default)]
    start_date: Option<String>,
    #[serde(default)]
    end_date_exclusive: Option<String>,
}
fn snapshot(anchor: LocalDate, connected: bool) -> Result<ProviderSnapshot, String> {
    if !connected {
        return Ok(ProviderSnapshot {
            status: ProviderStatus::NoCalendars,
            sources: vec![],
            occurrences: vec![],
        });
    }
    let (from, to) = crate::calendars::range(anchor);
    let timestamp = |d: LocalDate| {
        chrono::Local
            .with_ymd_and_hms(d.year, d.month.into(), d.day.into(), 0, 0, 0)
            .earliest()
            .map(|t| t.timestamp() as f64)
            .ok_or("Invalid local date boundary")
    };
    let pointer = unsafe { lc_events(timestamp(from)?, timestamp(to)?) };
    if pointer.is_null() {
        return Err("macOS calendar query failed".into());
    }
    let bytes = unsafe { CStr::from_ptr(pointer) }.to_bytes().to_vec();
    unsafe { lc_free(pointer) };
    decode_snapshot(&bytes)
}

fn decode_snapshot(bytes: &[u8]) -> Result<ProviderSnapshot, String> {
    decode_snapshot_in_timezone(bytes, &chrono::Local)
}

fn decode_snapshot_in_timezone<Tz: TimeZone>(
    bytes: &[u8],
    zone: &Tz,
) -> Result<ProviderSnapshot, String>
where
    Tz::Offset: std::fmt::Display,
{
    let data: StoreData = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let sources: Vec<_> = data
        .sources
        .into_iter()
        .map(|s| CalendarSource {
            id: format!("mac:{}", s.id),
            name: s.name,
            account_label: s.account,
            color_hex: s.color,
        })
        .collect();
    let mut occurrences = Vec::new();
    for event in data.events {
        let (first, end_date, time_label, start_minutes, end_minutes) = if event.all_day {
            let date = |text: Option<&str>| {
                let date = chrono::NaiveDate::parse_from_str(text?, "%Y-%m-%d").ok()?;
                LocalDate::new(date.year(), date.month() as u8, date.day() as u8)
            };
            let (Some(first), Some(last)) = (
                date(event.start_date.as_deref()),
                date(event.end_date_exclusive.as_deref()),
            ) else {
                continue;
            };
            if last <= first {
                continue;
            }
            (first, last, String::new(), None, None)
        } else {
            let timestamp = |seconds: f64| {
                seconds
                    .is_finite()
                    .then(|| chrono::DateTime::from_timestamp(seconds as i64, 0))
                    .flatten()
            };
            let (Some(start), Some(end)) = (timestamp(event.start), timestamp(event.end)) else {
                continue;
            };
            if end <= start {
                continue;
            }
            let start = start.with_timezone(zone);
            let end = end.with_timezone(zone);
            let date =
                |t: &chrono::DateTime<Tz>| LocalDate::new(t.year(), t.month() as u8, t.day() as u8);
            let (Some(first), Some(last)) = (date(&start), date(&end)) else {
                continue;
            };
            let end_date = if end.hour() == 0 && end.minute() == 0 && end.second() == 0 {
                last
            } else {
                last.add_days(1)
            };
            (
                first,
                end_date.max(first.add_days(1)),
                start.format("%H:%M").to_string(),
                Some(calendar_core::local_minutes(
                    first,
                    start.hour() * 60 + start.minute(),
                )),
                Some(calendar_core::local_minutes(
                    last,
                    end.hour() * 60 + end.minute(),
                )),
            )
        };
        let source_id = format!("mac:{}", event.source);
        let color = sources
            .iter()
            .find(|s| s.id == source_id)
            .map(|s| s.color_hex.clone())
            .unwrap_or_default();
        occurrences.push(CalendarOccurrence {
            source_id,
            event_id: format!("{}:{}", event.id, event.start),
            color_hex: color,
            start_date: first,
            end_date_exclusive: end_date,
            is_all_day: event.all_day,
            title: event.title,
            time_label,
            start_minutes,
            end_minutes,
            location: (!event.location.is_empty()).then_some(event.location),
            ..Default::default()
        });
    }
    Ok(ProviderSnapshot {
        status: if data.denied {
            ProviderStatus::AccessDenied
        } else if sources.is_empty() {
            ProviderStatus::NoCalendars
        } else {
            ProviderStatus::Available
        },
        sources,
        occurrences,
    })
}
pub fn load(anchor: LocalDate, connected: bool) -> crate::calendars::CalendarLoad {
    let snapshot = snapshot(anchor, connected).unwrap_or_else(|error| ProviderSnapshot {
        status: ProviderStatus::ProviderUnavailable(error),
        sources: vec![],
        occurrences: vec![],
    });
    crate::calendars::CalendarLoad {
        snapshot,
        microsoft: crate::calendars::account::MicrosoftAccountState::Disconnected,
        microsoft_error: String::new(),
        google_connected: false,
        google_needs_signin: false,
        google_configuration: None,
        google_label: None,
        google_error: String::new(),
        google_skipped: 0,
        ics_feed_errors: Default::default(),
    }
}

pub fn load_credentials() -> Result<crate::secure_store::Credentials, String> {
    let mut pointer = std::ptr::null_mut();
    let status = unsafe { lc_credentials_read(&mut pointer) };
    if status != 0 {
        return Err(format!(
            "macOS Keychain could not unlock Lightcal credentials (error {status})"
        ));
    }
    if pointer.is_null() {
        return Ok(Default::default());
    }
    let parsed = serde_json::from_slice(unsafe { CStr::from_ptr(pointer) }.to_bytes());
    unsafe { lc_free(pointer) };
    parsed.map_err(|_| "Stored Keychain credentials are invalid; reconnect the account".into())
}
pub fn save_credentials(credentials: &crate::secure_store::Credentials) -> Result<(), String> {
    let json =
        CString::new(serde_json::to_vec(credentials).map_err(|_| "Could not encode credentials")?)
            .map_err(|_| "Invalid credentials")?;
    let remove = credentials.microsoft_refresh_token.is_none()
        && credentials.google_refresh_token.is_none()
        && credentials.ics.is_empty();
    let status = unsafe { lc_credentials_write(json.as_ptr(), remove) };
    if status == 0 {
        Ok(())
    } else {
        Err(format!(
            "macOS Keychain could not save Lightcal credentials (error {status})"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_preferences_preserve_app_locale() {
        let date = LocalDate::new(2026, 11, 9).unwrap();
        let mut prefs = Preferences {
            mac_show_month: false,
            ..Preferences::default()
        };
        assert_eq!(menu_text(&prefs, date), "Mo");
        prefs.mac_weekday = MenuWeekday::Full;
        assert_eq!(menu_text(&prefs, date), "Monday");
        prefs.language = "ru".into();
        assert_eq!(menu_text(&prefs, date), "Понедельник");
        prefs.mac_weekday = MenuWeekday::Off;
        prefs.mac_show_month = true;
        assert_eq!(menu_text(&prefs, date), "нояб.");
    }
    #[test]
    fn provider_maps_local_boundaries_and_distinguishes_permissions() {
        assert_eq!(
            decode_snapshot(br#"{"denied":true,"sources":[],"events":[]}"#)
                .unwrap()
                .status,
            ProviderStatus::AccessDenied
        );
        assert_eq!(
            decode_snapshot(br#"{"denied":false,"sources":[],"events":[]}"#)
                .unwrap()
                .status,
            ProviderStatus::NoCalendars
        );
        assert!(decode_snapshot(b"invalid").is_err());
        let start = chrono::Local
            .with_ymd_and_hms(2026, 11, 9, 23, 0, 0)
            .unwrap()
            .timestamp();
        let end = chrono::Local
            .with_ymd_and_hms(2026, 11, 10, 0, 0, 0)
            .unwrap()
            .timestamp();
        let json = serde_json::json!({"denied":false,"sources":[{"id":"one","name":"Personal","account":"iCloud","color":"#008573"}],"events":[{"id":"event","source":"one","title":"Overnight","location":"Home","all_day":false,"start":start,"end":end}]});
        let snapshot = decode_snapshot(&serde_json::to_vec(&json).unwrap()).unwrap();
        let event = &snapshot.occurrences[0];
        assert_eq!(event.source_id, "mac:one");
        assert_eq!(event.start_date, LocalDate::new(2026, 11, 9).unwrap());
        assert_eq!(
            event.end_date_exclusive,
            LocalDate::new(2026, 11, 10).unwrap()
        );
        assert_eq!(
            event.end_minutes.unwrap() - event.start_minutes.unwrap(),
            60
        );
        assert_eq!(event.location.as_deref(), Some("Home"));
        assert!(event.web_link.is_none());
    }

    #[test]
    fn menu_text_supports_all_app_languages() {
        let date = LocalDate::new(2026, 11, 9).unwrap();
        for code in [
            "en", "de", "es", "fr", "pt-BR", "ru", "uk", "ja", "ko", "zh-Hans",
        ] {
            let mut prefs = Preferences {
                language: code.into(),
                ..Preferences::default()
            };
            assert!(!menu_text(&prefs, date).is_empty());
            prefs.mac_weekday = MenuWeekday::Full;
            prefs.mac_show_month = true;
            assert!(menu_text(&prefs, date)
                .ends_with(crate::i18n::month_short(prefs.locale(), date.month)));
            assert_eq!(prefs.language, code);
        }
    }

    #[test]
    fn shortcut_requires_modifier_and_known_key() {
        assert_eq!(parse_shortcut("Ctrl+Alt+Space").unwrap(), (49, 6144));
        assert!(parse_shortcut("Space").is_err());
        assert!(parse_shortcut("Ctrl+Unknown").is_err());
        assert!(parse_shortcut("Ctrl+B+1").is_err());
    }

    #[test]
    fn shortcuts_use_sparse_carbon_ansi_positions() {
        for (key, code) in [
            ("B", 0x0B),
            ("Q", 0x0C),
            ("Y", 0x10),
            ("1", 0x12),
            ("6", 0x16),
            ("5", 0x17),
            ("0", 0x1D),
            ("]", 0x1E),
            ("P", 0x23),
            ("L", 0x25),
            ("M", 0x2E),
            (".", 0x2F),
            ("`", 0x32),
        ] {
            assert_eq!(
                parse_shortcut(&format!("Ctrl+Alt+{key}")).unwrap(),
                (code, 6144),
                "{key}"
            );
            assert_eq!(
                parse_shortcut(&format!("Cmd+Shift+{}", key.to_lowercase())).unwrap(),
                (code, 768),
                "{key}"
            );
        }
    }

    #[test]
    fn shortcuts_accept_every_function_key_supported_by_the_recorder() {
        let codes = [
            0x7A, 0x78, 0x63, 0x76, 0x60, 0x61, 0x62, 0x64, 0x65, 0x6D, 0x67, 0x6F,
        ];
        for (index, code) in codes.into_iter().enumerate() {
            assert_eq!(
                parse_shortcut(&format!("Ctrl + F{}", index + 1)).unwrap(),
                (code, 4096)
            );
        }
        assert!(parse_shortcut("Ctrl + F13").is_err());
    }

    #[test]
    fn all_day_civil_dates_do_not_shift_with_display_timezone() {
        let start = chrono::Utc
            .with_ymd_and_hms(2026, 11, 9, 0, 0, 0)
            .unwrap()
            .timestamp();
        let sources = serde_json::json!([{"id":"one","name":"Holidays","account":"iCloud","color":"#008573"}]);
        let all_day = serde_json::json!({"id":"holiday","source":"one","title":"Holiday","location":"","all_day":true,"start":start,"end":start+172800,"start_date":"2026-11-09","end_date_exclusive":"2026-11-11"});
        for offset in [-8 * 3600, 14 * 3600] {
            let zone = chrono::FixedOffset::east_opt(offset).unwrap();
            let bytes = serde_json::to_vec(
                &serde_json::json!({"denied":false,"sources":sources,"events":[all_day]}),
            )
            .unwrap();
            let snapshot = decode_snapshot_in_timezone(&bytes, &zone).unwrap();
            let event = &snapshot.occurrences[0];
            assert_eq!(event.start_date, LocalDate::new(2026, 11, 9).unwrap());
            assert_eq!(
                event.end_date_exclusive,
                LocalDate::new(2026, 11, 11).unwrap()
            );
            assert!(event.is_all_day && event.time_label.is_empty());
            assert!(event.start_minutes.is_none() && event.end_minutes.is_none());
        }
        // Timed events at the same UTC instant must still follow the display zone.
        let mut timed = all_day.clone();
        timed["all_day"] = serde_json::json!(false);
        timed["end"] = serde_json::json!(start + 3600);
        let bytes = serde_json::to_vec(
            &serde_json::json!({"denied":false,"sources":sources,"events":[timed]}),
        )
        .unwrap();
        let snapshot =
            decode_snapshot_in_timezone(&bytes, &chrono::FixedOffset::west_opt(8 * 3600).unwrap())
                .unwrap();
        assert_eq!(
            snapshot.occurrences[0].start_date,
            LocalDate::new(2026, 11, 8).unwrap()
        );
        assert_eq!(snapshot.occurrences[0].time_label, "16:00");

        // A malformed civil span must never fall back to the shifted timestamp.
        for (first, last) in [
            (None, None),
            (Some("2026-02-30"), Some("2026-03-01")),
            (Some("2026-11-09"), Some("2026-11-09")),
        ] {
            let mut invalid = all_day.clone();
            invalid["start_date"] = serde_json::json!(first);
            invalid["end_date_exclusive"] = serde_json::json!(last);
            let bytes = serde_json::to_vec(
                &serde_json::json!({"denied":false,"sources":sources,"events":[invalid]}),
            )
            .unwrap();
            assert!(decode_snapshot(&bytes).unwrap().occurrences.is_empty());
        }
    }
}
