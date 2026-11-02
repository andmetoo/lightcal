#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use calendar_core::{CalendarState, LocalDate, Weekday};
use chrono::{Datelike, Timelike};
use lightcal::i18n::{
    catalog, format_active_calendars_heading, format_calendar_count, format_date_line,
    format_event_count, format_month_year, format_sync_status, format_update_status, weekday_short,
};
#[cfg(windows)]
use lightcal::platform::windows::render_native_tray_icon;
use lightcal::platform::windows::{
    bring_to_foreground, primary_work_area, query_calendar_provider, render_squircle_icon_rgba_64,
    system_prefers_dark_mode, ShortcutManager, SingleInstance, StartupManager,
};
use lightcal::platform::ProviderSnapshot;
use lightcal::preferences::{
    LocaleCode, Preferences, PreferencesStore, ThemePreference, TrayIconMode,
};
mod calendar_view;
mod event_colors;
mod snapshot_render;
#[cfg(test)]
mod ui_tests;
use event_colors::mark_color;
use snapshot_render::render_snapshots_to_dir;

use lightcal::updater::{is_allowed_external_link, UpdateCheckState};
static SIGN_IN_GATE: lightcal::calendars::sign_in::SignInGate =
    lightcal::calendars::sign_in::SignInGate::new();

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActiveSignIn {
    None,
    Microsoft,
    Google,
}
use slint::{
    ComponentHandle, Image, LogicalSize, ModelRc, PhysicalPosition, Rgba8Pixel, SharedPixelBuffer,
    SharedString, VecModel,
};
use std::cell::RefCell;
use std::env;
use std::path::Path;
use std::rc::Rc;

slint::include_modules!();

/// Keeps the UI on Segoe UI when Segoe UI Variable is missing (Windows 10).
/// Slint 1.18 falls back to the generic sans-serif family (Arial on Windows)
/// for an unknown name, so the static face is selected explicitly.
pub(crate) fn apply_system_font(ui: &AppWindow) {
    if !segoe_ui_variable_installed() {
        ui.global::<Theme>().set_font_family("Segoe UI".into());
    }
}

#[cfg(windows)]
fn segoe_ui_variable_installed() -> bool {
    lightcal::platform::windows::system_fonts_dir()
        .is_some_and(|fonts| fonts.join("SegUIVar.ttf").is_file())
}

/// Off Windows the named family is absent and Slint uses its sans-serif fallback.
#[cfg(not(windows))]
fn segoe_ui_variable_installed() -> bool {
    true
}

pub(crate) struct AppController {
    pub(crate) store: PreferencesStore,
    pub(crate) prefs: Preferences,
    pub(crate) cal_state: CalendarState,
    pub(crate) provider: ProviderSnapshot,
    pub(crate) provider_request: u64,
    pub(crate) connection_busy: bool,
    pub(crate) connection_message: String,
    /// Preferences recovery warning; shown on the General tab until dismissed.
    pub(crate) startup_warning: String,
    pub(crate) microsoft_account: lightcal::calendars::account::MicrosoftAccountState,
    pub(crate) provider_loading: bool,
    /// Account state to restore if a load ends without a successful result.
    pub(crate) account_before_check: Option<lightcal::calendars::account::MicrosoftAccountState>,
    pub(crate) install_busy: bool,
    pub(crate) install_status: String,
    pub(crate) startup_mgr: StartupManager,
    pub(crate) shortcut_mgr: ShortcutManager,
    pub(crate) shortcut_error: String,
    pub(crate) startup_error: String,
    pub(crate) conflicting_shortcut: Option<String>,
    pub(crate) update_state: UpdateCheckState,
    pub(crate) settings_open: bool,
    pub(crate) settings_tab: i32,
    #[cfg(windows)]
    pub(crate) tray_icon: Option<tray_icon::TrayIcon>,
    /// Tray menu items, relabeled when the language changes.
    #[cfg(windows)]
    pub(crate) tray_menu: TrayMenuItems,
    pub(crate) language_dropdown_open: bool,
    pub(crate) theme_dropdown_open: bool,
    pub(crate) first_day_dropdown_open: bool,
    pub(crate) update_channel_dropdown_open: bool,
    pub(crate) calendars_access_denied: bool,
    pub(crate) calendars_is_empty: bool,
    pub(crate) add_account_open: bool,
    pub(crate) ics_dialog_open: bool,
    pub(crate) sign_in_cancelled: bool,
    pub(crate) active_sign_in: ActiveSignIn,
    pub(crate) held_microsoft: lightcal::calendars::account::MicrosoftAccountState,
    pub(crate) held_google_connected: bool,
    pub(crate) held_google_needs_signin: bool,
    pub(crate) held_google_label: String,
    pub(crate) held_google_error: String,
    pub(crate) google_connected: bool,
    pub(crate) google_needs_signin: bool,
    pub(crate) google_label: String,
    pub(crate) google_error: String,
    pub(crate) ics_error: String,
    /// Calendar popup agenda: expanded and copied events.
    pub(crate) agenda: calendar_view::AgendaUiState,
    /// When calendars last loaded successfully; drives "Updated … ago" in Accounts.
    pub(crate) last_refreshed: Option<chrono::DateTime<chrono::Local>>,
}

thread_local! {
    /// Last browser sign-in address, for "Open browser again" while signing in.
    static LAST_SIGN_IN_URL: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn remember_sign_in_url(url: &str) {
    LAST_SIGN_IN_URL.with(|slot| *slot.borrow_mut() = Some(url.to_string()));
}

/// Forgets the consent address so "Open browser again" never reopens an old sign-in.
fn forget_sign_in_url() {
    LAST_SIGN_IN_URL.with(|slot| *slot.borrow_mut() = None);
}

/// Builds a normalized shortcut ("Ctrl + Alt + K") from a recorded key press.
/// Returns `None` for keys Lightcal cannot register or presses without Ctrl, Alt or Win.
#[cfg(test)]
fn shortcut_from_key(text: &str, ctrl: bool, alt: bool, shift: bool, meta: bool) -> Option<String> {
    shortcut_from_press(None, text, ctrl, alt, shift, meta)
}

/// The main key held down right now (A–Z, 0–9, F1–F12, Space), read from the keyboard
/// state. Unlike the produced text it does not change with Ctrl, Shift or the layout:
/// Ctrl+M and Enter both type "\r", and Shift+2 types "@" on a US layout.
#[cfg(windows)]
fn pressed_main_key() -> Option<String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    // SAFETY: GetAsyncKeyState only reads the current key state.
    let down = |vk: i32| (unsafe { GetAsyncKeyState(vk) } as u16) & 0x8000 != 0;
    (b'A'..=b'Z')
        .chain(b'0'..=b'9')
        .find(|&vk| down(i32::from(vk)))
        .map(|vk| char::from(vk).to_string())
        .or_else(|| (1..=12).find(|n| down(0x6F + n)).map(|n| format!("F{n}")))
        .or_else(|| down(0x20).then(|| "Space".to_string()))
}

/// Like [`shortcut_from_key`], but `pressed` (from [`pressed_main_key`]) wins over the text.
fn shortcut_from_press(
    pressed: Option<&str>,
    text: &str,
    ctrl: bool,
    alt: bool,
    shift: bool,
    meta: bool,
) -> Option<String> {
    if !(ctrl || alt || meta) {
        return None;
    }
    if let Some(key) = pressed {
        return Some(shortcut_label(key.to_string(), ctrl, alt, shift, meta));
    }
    let mut chars = text.chars();
    let first = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    let function_keys = [
        slint::platform::Key::F1,
        slint::platform::Key::F2,
        slint::platform::Key::F3,
        slint::platform::Key::F4,
        slint::platform::Key::F5,
        slint::platform::Key::F6,
        slint::platform::Key::F7,
        slint::platform::Key::F8,
        slint::platform::Key::F9,
        slint::platform::Key::F10,
        slint::platform::Key::F11,
        slint::platform::Key::F12,
    ];
    let key = if let Some(index) = function_keys.iter().position(|k| char::from(*k) == first) {
        format!("F{}", index + 1)
    } else if first == ' ' {
        "Space".to_string()
    } else if matches!(first, '\u{8}' | '\t' | '\n' | '\r') {
        // Backspace, Tab and Enter share codes with Ctrl+H/I/J/M; never guess a letter.
        return None;
    } else if ('\u{1}'..='\u{1a}').contains(&first) {
        // Ctrl+letter can arrive as an ASCII control character.
        char::from(b'A' + (first as u8) - 1).to_string()
    } else if first.is_ascii_alphanumeric() {
        first.to_ascii_uppercase().to_string()
    } else {
        return None;
    };
    Some(shortcut_label(key, ctrl, alt, shift, meta))
}

fn shortcut_label(key: String, ctrl: bool, alt: bool, shift: bool, meta: bool) -> String {
    let mut parts = Vec::new();
    if ctrl {
        parts.push("Ctrl".to_string());
    }
    if alt {
        parts.push("Alt".to_string());
    }
    if shift {
        parts.push("Shift".to_string());
    }
    if meta {
        parts.push("Win".to_string());
    }
    parts.push(key);
    parts.join(" + ")
}

/// Registers `candidate`; on failure the previous binding stays and the conflict is shown.
fn apply_shortcut(c: &mut AppController, candidate: &str) {
    match c.shortcut_mgr.bind(Some(candidate)) {
        Ok(bound) => {
            c.prefs.global_shortcut = bound;
            c.shortcut_error.clear();
            c.conflicting_shortcut = None;
            c.persist();
        }
        Err(err) => {
            // ShortcutManager restores the previous registration itself.
            c.shortcut_error = err;
            c.conflicting_shortcut = Some(candidate.to_string());
        }
    }
}

/// A display name for a one-field ICS subscription: the address host.
fn ics_name_from_address(address: &str) -> String {
    let trimmed = address.trim();
    let without_scheme = trimmed
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(trimmed);
    let host = without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('@')
        .next()
        .unwrap_or_default();
    let host = host.split(':').next().unwrap_or_default();
    if host.is_empty() {
        "Calendar".to_string()
    } else {
        host.chars().take(100).collect()
    }
}

fn skipped_google_events(locale: LocaleCode, count: usize) -> String {
    match locale {
        LocaleCode::Ru => format!("Не прочитано событий Google: {count}"),
        LocaleCode::Es => format!("Eventos de Google no leídos: {count}"),
        LocaleCode::Ja => format!("読み取れなかった Google の予定: {count}"),
        LocaleCode::En => format!("Google events not read: {count}"),
    }
}

fn google_status_line(
    locale: LocaleCode,
    connected: bool,
    needs_signin: bool,
    label: &str,
    error: &str,
) -> String {
    let connected_text = match locale {
        LocaleCode::Ru => "Google подключён",
        LocaleCode::Es => "Google conectado",
        LocaleCode::Ja => "Google に接続済み",
        LocaleCode::En => "Google connected",
    };
    let expired = match locale {
        LocaleCode::Ru => "Сеанс Google истёк — войдите повторно",
        LocaleCode::Es => "La sesión de Google caducó; vuelve a iniciar sesión",
        LocaleCode::Ja => "Google のセッションが切れました。再度サインインしてください",
        LocaleCode::En => "Google session expired — sign in again",
    };
    if needs_signin {
        expired.into()
    } else if connected && !error.is_empty() {
        let base = if label.is_empty() {
            connected_text.to_string()
        } else {
            format!("{connected_text} · {label}")
        };
        format!("{base} — {error}")
    } else if !error.is_empty() {
        error.to_string()
    } else if connected && !label.is_empty() {
        format!("{connected_text} · {label}")
    } else if connected {
        connected_text.into()
    } else {
        String::new()
    }
}

/// Calendar sources by provider, as Settings lists them.
#[derive(Default)]
struct SourceGroups {
    outlook: Vec<CalendarSourceItem>,
    google: Vec<CalendarSourceItem>,
    ics: Vec<CalendarSourceItem>,
    windows: Vec<CalendarSourceItem>,
}

fn split_sources(items: &[CalendarSourceItem]) -> SourceGroups {
    let mut groups = SourceGroups::default();
    for item in items {
        let group = if item.id.starts_with("graph:") {
            &mut groups.outlook
        } else if item.id.starts_with("google:") {
            &mut groups.google
        } else if item.id.starts_with("ics:") {
            &mut groups.ics
        } else {
            &mut groups.windows
        };
        group.push(item.clone());
    }
    groups
}

fn current_local_date() -> LocalDate {
    let now = chrono::Local::now().date_naive();
    LocalDate::new(now.year(), now.month() as u8, now.day() as u8)
        .unwrap_or_else(|| LocalDate::new(2026, 10, 29).unwrap())
}

fn resolve_dark_mode(theme: ThemePreference) -> bool {
    match theme {
        ThemePreference::Dark => true,
        ThemePreference::Light => false,
        ThemePreference::System => system_prefers_dark_mode(),
    }
}

fn target_logical_size(settings_open: bool) -> (f32, f32) {
    if settings_open {
        (448.0, 360.0)
    } else {
        (352.0, 432.0)
    }
}

fn sync_window_geometry(ui: &AppWindow, settings_open: bool) {
    let (logical_w, logical_h) = target_logical_size(settings_open);
    ui.window().set_size(LogicalSize::new(logical_w, logical_h));

    #[cfg(windows)]
    {
        let hwnd = lightcal::platform::windows::find_lightcal_hwnd();
        if !hwnd.is_null() {
            unsafe {
                lightcal::platform::windows::position_window_above_tray(hwnd, logical_w, logical_h);
            }
            apply_window_chrome(ui);
            return;
        }
    }

    let scale = ui.window().scale_factor().max(1.0);
    // Frameless window geometry anchored cleanly right above the Windows Taskbar / System Tray.
    let phys_w = (logical_w * scale).round() as i32;
    let phys_h = (logical_h * scale).round() as i32;
    let margin_phys = (12.0 * scale).round() as i32;
    let work_area = primary_work_area();
    let (x, y) = work_area.anchor_above_tray(phys_w, phys_h, margin_phys);
    ui.window().set_position(PhysicalPosition::new(x, y));
}

fn format_compact_summary(locale: LocaleCode, date: LocalDate, count: usize) -> String {
    let count_str = format_event_count(locale, count);
    match locale {
        LocaleCode::En => {
            let wd = match date.weekday() {
                Weekday::Monday => "MON",
                Weekday::Tuesday => "TUE",
                Weekday::Wednesday => "WED",
                Weekday::Thursday => "THU",
                Weekday::Friday => "FRI",
                Weekday::Saturday => "SAT",
                Weekday::Sunday => "SUN",
            };
            let m_short = match date.month {
                1 => "JAN",
                2 => "FEB",
                3 => "MAR",
                4 => "APR",
                5 => "MAY",
                6 => "JUN",
                7 => "JUL",
                8 => "AUG",
                9 => "SEP",
                10 => "OCT",
                11 => "NOV",
                12 => "DEC",
                _ => "OCT",
            };
            let c_upper = count_str.to_uppercase();
            format!("{wd}, {m_short} {} · {c_upper}", date.day)
        }
        LocaleCode::Ru => {
            let wd = match date.weekday() {
                Weekday::Monday => "ПН",
                Weekday::Tuesday => "ВТ",
                Weekday::Wednesday => "СР",
                Weekday::Thursday => "ЧТ",
                Weekday::Friday => "ПТ",
                Weekday::Saturday => "СБ",
                Weekday::Sunday => "ВС",
            };
            let m_short = match date.month {
                1 => "ЯНВ",
                2 => "ФЕВ",
                3 => "МАР",
                4 => "АПР",
                5 => "МАЯ",
                6 => "ИЮН",
                7 => "ИЮЛ",
                8 => "АВГ",
                9 => "СЕН",
                10 => "ОКТ",
                11 => "НОЯ",
                12 => "ДЕК",
                _ => "ОКТ",
            };
            let c_upper = count_str.to_uppercase();
            format!("{wd}, {} {m_short} · {c_upper}", date.day)
        }
        LocaleCode::Es => {
            let wd = match date.weekday() {
                Weekday::Monday => "LUN",
                Weekday::Tuesday => "MAR",
                Weekday::Wednesday => "MIÉ",
                Weekday::Thursday => "JUE",
                Weekday::Friday => "VIE",
                Weekday::Saturday => "SÁB",
                Weekday::Sunday => "DOM",
            };
            let m_short = match date.month {
                1 => "ENE",
                2 => "FEB",
                3 => "MAR",
                4 => "ABR",
                5 => "MAY",
                6 => "JUN",
                7 => "JUL",
                8 => "AGO",
                9 => "SEP",
                10 => "OCT",
                11 => "NOV",
                12 => "DIC",
                _ => "OCT",
            };
            let c_upper = count_str.to_uppercase();
            format!("{wd}, {} {m_short} · {c_upper}", date.day)
        }
        LocaleCode::Ja => {
            let wd = weekday_short(locale, date.weekday());
            format!("{}月{}日 ({wd}) · {count_str}", date.month, date.day)
        }
    }
}

impl AppController {
    pub(crate) fn sync_to_ui(&self, ui: &AppWindow) {
        let locale = self.prefs.locale();
        let active_count = self
            .provider
            .sources
            .iter()
            .filter(|source| self.prefs.selected_calendar_ids.contains(&source.id))
            .count();
        let strings = catalog(locale);
        let dark_mode = resolve_dark_mode(self.prefs.theme);
        let managed = lightcal::installer::package_managed();
        ui.set_package_managed_updates(managed);

        let theme_changed = ui.get_dark_mode() != dark_mode;
        ui.set_dark_mode(dark_mode);
        if theme_changed {
            apply_window_chrome(ui);
        }
        ui.set_settings_open(self.settings_open);
        ui.set_settings_tab(self.settings_tab);

        let now = chrono::Local::now();
        ui.set_live_day_text(self.cal_state.today.day.to_string().into());
        ui.set_live_weekday_date_text(
            format!(
                "{} {}",
                weekday_short(locale, self.cal_state.today.weekday()),
                self.cal_state.today.day
            )
            .into(),
        );
        ui.set_live_time_text(format!("{:02}:{:02}", now.hour(), now.minute()).into());
        for monochrome in [false, true] {
            let rgba = render_squircle_icon_rgba_64(
                self.prefs.tray_icon_mode,
                monochrome,
                self.cal_state.today.day,
                self.cal_state.today.weekday(),
                (now.hour() as u8, now.minute() as u8),
                dark_mode,
            );
            let image = Image::from_rgba8(SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
                &rgba, 64, 64,
            ));
            if monochrome {
                ui.set_monochrome_icon_preview(image);
            } else {
                ui.set_colored_icon_preview(image);
            }
        }
        let rgba64 = render_squircle_icon_rgba_64(
            self.prefs.tray_icon_mode,
            self.prefs.tray_icon_monochrome,
            self.cal_state.today.day,
            self.cal_state.today.weekday(),
            (now.hour() as u8, now.minute() as u8),
            lightcal::platform::windows::icon::taskbar_prefers_dark_mode(dark_mode),
        );
        let pixel_buf = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&rgba64, 64, 64);
        ui.set_window_icon(Image::from_rgba8(pixel_buf));

        let grid = self.cal_state.build_grid(
            &self.provider.occurrences,
            &self.prefs.selected_calendar_ids,
        );

        ui.set_is_current_month(grid.is_current_month);
        ui.set_current_month_badge(strings.current_month_badge.into());
        ui.set_today_button_label(strings.today_button.into());
        ui.set_week_header_label(strings.week_col_header.into());
        ui.set_show_week_numbers(self.prefs.show_week_numbers);
        ui.set_show_weekday_header(true);

        let wd_headers: Vec<SharedString> = grid
            .weekdays
            .iter()
            .map(|&d| SharedString::from(weekday_short(locale, d)))
            .collect();
        ui.set_weekday_headers(ModelRc::new(VecModel::from(wd_headers)));

        let iso_wks: Vec<SharedString> = grid
            .iso_weeks
            .iter()
            .map(|w| SharedString::from(w.to_string()))
            .collect();
        ui.set_iso_weeks(ModelRc::new(VecModel::from(iso_wks)));

        let mut selected_total_events = 0usize;
        let cell_items: Vec<DayCellData> = grid
            .cells
            .iter()
            .map(|c| {
                if c.is_selected {
                    selected_total_events = c.total_event_count;
                }
                let dot = |i: usize| {
                    c.dot_colors
                        .get(i)
                        .map(|hex| mark_color(hex))
                        .unwrap_or_default()
                };
                let (d1, d2, d3) = (dot(0), dot(1), dot(2));
                let date_desc = format_date_line(locale, c.date, true);
                let ev_desc = format_event_count(locale, c.total_event_count);
                DayCellData {
                    day_number: c.date.day.to_string().into(),
                    in_month: c.in_displayed_month,
                    is_today: c.is_today,
                    is_selected: c.is_selected,
                    is_gray_weekday: c.is_gray_weekday,
                    dot_count: c.dot_colors.len() as i32,
                    dot1: d1,
                    dot2: d2,
                    dot3: d3,
                    accessible_label: format!("{date_desc}, {ev_desc}").into(),
                }
            })
            .collect();
        ui.set_month_view(MonthViewData {
            title: format_month_year(locale, grid.displayed_month).into(),
            cells: ModelRc::new(VecModel::from(cell_items)),
        });

        ui.set_selected_date_line(
            format_date_line(
                locale,
                self.cal_state.selected_date,
                self.prefs.show_weekday,
            )
            .into(),
        );
        ui.set_selected_event_summary(format_event_count(locale, selected_total_events).into());
        ui.set_active_calendars_label(format_calendar_count(locale, active_count).into());
        ui.set_back_to_calendar_text(strings.back_to_calendar.into());
        self.sync_calendar_view(ui, &grid);

        let compact_events = if selected_total_events > 0 {
            selected_total_events.min(2)
        } else {
            0
        };
        let compact_summary =
            format_compact_summary(locale, self.cal_state.selected_date, compact_events);
        ui.set_compact_summary_header(compact_summary.into());

        ui.set_sync_status_text(format_sync_status(locale, active_count).into());

        // Settings properties
        ui.set_settings_title_text(strings.settings_title.into());
        ui.set_preferences_subtitle_text(strings.preferences_subtitle.into());
        ui.set_section_region_text(strings.section_region.into());
        ui.set_section_startup_text(strings.section_startup.into());
        ui.set_desc_auto_install_text(strings.desc_auto_install.into());
        ui.set_nav_general_text(strings.nav_general.into());
        ui.set_nav_appearance_text(strings.nav_appearance.into());
        ui.set_nav_calendars_text(strings.nav_calendars.into());
        ui.set_nav_about_text(strings.nav_about.into());
        ui.set_general_subtitle_text(strings.general_subtitle.into());
        ui.set_appearance_subtitle_text(strings.appearance_subtitle.into());
        ui.set_calendars_subtitle_text(strings.calendars_subtitle.into());
        ui.set_about_subtitle_text(strings.about_subtitle.into());
        ui.set_label_language_text(strings.label_language.into());
        ui.set_desc_language_text(strings.desc_language.into());
        ui.set_label_first_weekday_text(strings.label_first_weekday.into());
        ui.set_desc_first_weekday_text(strings.desc_first_weekday.into());
        ui.set_label_show_week_numbers_text(strings.label_show_week_numbers.into());
        ui.set_desc_show_week_numbers_text(strings.desc_show_week_numbers.into());
        let startup = lightcal::i18n::startup_copy(locale);
        ui.set_label_startup_text(startup.label.into());
        ui.set_desc_startup_text(startup.description.into());
        ui.set_label_shortcut_text(strings.label_shortcut.into());
        ui.set_desc_shortcut_text(strings.desc_shortcut.into());
        ui.set_label_theme_text(strings.label_theme.into());
        ui.set_desc_theme_text(strings.desc_theme.into());
        ui.set_label_gray_weekdays_text(strings.label_gray_weekdays.into());
        ui.set_desc_gray_weekdays_text(strings.desc_gray_weekdays.into());
        ui.set_label_highlight_current_month_text(strings.label_highlight_current_month.into());
        ui.set_desc_highlight_current_month_text(strings.desc_highlight_current_month.into());
        ui.set_label_show_event_dots_text(strings.label_show_event_dots.into());
        ui.set_desc_show_event_dots_text(strings.desc_show_event_dots.into());
        ui.set_label_tray_mode_text(strings.label_tray_mode.into());
        ui.set_desc_tray_mode_text(strings.desc_tray_mode.into());
        ui.set_label_tray_palette_text(strings.label_tray_palette.into());
        ui.set_desc_tray_palette_text(strings.desc_tray_palette.into());
        ui.set_palette_colored_text(strings.palette_colored.into());
        ui.set_palette_monochrome_text(strings.palette_monochrome.into());
        ui.set_provider_heading_text(strings.provider_heading.into());
        let labels = lightcal::i18n::connection_labels(locale);
        ui.set_connect_microsoft_label(labels[1].into());
        ui.set_disconnect_microsoft_label(labels[2].into());
        ui.set_ics_name_placeholder(labels[3].into());
        ui.set_ics_url_placeholder(labels[4].into());
        ui.set_add_ics_label(labels[5].into());
        ui.set_remove_source_label(labels[6].into());
        ui.set_legacy_windows_label(labels[7].into());
        ui.set_install_update_label(labels[8].into());
        ui.set_auto_install_label(labels[9].into());
        ui.set_legacy_windows_enabled(self.prefs.windows_calendars_enabled);
        ui.set_auto_install_updates(self.prefs.auto_install_updates);
        ui.set_install_busy(self.install_busy);
        ui.set_install_status_text(self.install_status.clone().into());
        ui.set_connection_busy(self.connection_busy);
        ui.set_connection_message(self.connection_message.clone().into());
        ui.set_startup_warning(self.startup_warning.clone().into());
        ui.set_dismiss_label_text(catalog(self.prefs.locale()).dismiss_label.into());
        let account_idle = !self.connection_busy && !self.provider_loading;
        ui.set_provider_loading(self.provider_loading);
        ui.set_account_status_text(self.microsoft_account.status_text(locale).into());
        ui.set_account_connected(matches!(
            self.microsoft_account,
            lightcal::calendars::account::MicrosoftAccountState::Connected { .. }
        ));
        ui.set_account_needs_signin(matches!(
            self.microsoft_account,
            lightcal::calendars::account::MicrosoftAccountState::NeedsSignIn
        ));
        ui.set_sign_in_cancelled(self.sign_in_cancelled);
        ui.set_add_account_open(self.add_account_open);
        ui.set_ics_dialog_open(self.ics_dialog_open);
        let (
            change,
            conflict_line,
            add_title,
            add_intro,
            google,
            google_desc,
            outlook_desc,
            ics_label,
            ics_desc,
            waiting,
            cancel,
            try_again,
            cancelled,
            stale,
            reconnect,
            no_cals,
            ics_title,
            privacy,
            stays,
            subscribe,
            google_missing,
        ) = match locale {
            LocaleCode::Ru => (
                "Изменить",
                "Занято другим приложением",
                "Добавить аккаунт",
                "Выберите источник только для чтения.",
                "Google Календарь",
                "Войти через аккаунт Google.",
                "Рабочий, учебный или личный аккаунт.",
                "Закрытая ссылка ICS",
                "Подписка по адресу календаря.",
                "Ожидание браузера…",
                "Отмена",
                "Ещё раз",
                "Вход отменён. Аккаунты не изменились.",
                "События могут быть устаревшими.",
                "Переподключить",
                "У этого аккаунта нет календарей.",
                "Добавить ссылку на календарь",
                "Кто знает адрес, тот читает календарь.",
                "Остаётся на этом компьютере.",
                "Подписаться",
                "Вход через Google в этой сборке не настроен.",
            ),
            LocaleCode::Es => (
                "Cambiar",
                "Lo usa otra aplicación",
                "Añadir una cuenta",
                "Elige una fuente de solo lectura.",
                "Google Calendar",
                "Conecta tu cuenta de Google.",
                "Cuenta personal, del trabajo o de la escuela.",
                "Enlace ICS privado",
                "Suscríbete con una dirección de calendario.",
                "Esperando al navegador…",
                "Cancelar",
                "Reintentar",
                "Inicio de sesión cancelado. Las cuentas no cambiaron.",
                "Los eventos pueden estar desactualizados.",
                "Reconectar",
                "No hay calendarios en esta cuenta.",
                "Añadir un enlace de calendario",
                "Quien tenga la dirección puede leer el calendario.",
                "Se queda en este equipo.",
                "Suscribirse",
                "El inicio de sesión de Google no está configurado en esta compilación.",
            ),
            LocaleCode::Ja => (
                "変更",
                "他のアプリが使用中",
                "アカウントを追加",
                "読み取り専用のソースを選びます。",
                "Google カレンダー",
                "Google アカウントで接続します。",
                "職場、学校、または個人のアカウント。",
                "非公開 ICS リンク",
                "カレンダーのアドレスで購読します。",
                "ブラウザーを待っています…",
                "キャンセル",
                "再試行",
                "サインインを取り消しました。アカウントは変更されていません。",
                "予定が最新ではない可能性があります。",
                "再接続",
                "このアカウントにカレンダーはありません。",
                "カレンダーのリンクを追加",
                "アドレスを知っている人はカレンダーを読めます。",
                "このコンピューターに残ります。",
                "購読",
                "このビルドでは Google サインインが設定されていません。",
            ),
            LocaleCode::En => (
                "Change",
                "Used by another app",
                "Add an account",
                "Choose a read-only source.",
                "Google Calendar",
                "Connect using your Google account.",
                "Work, school, or a personal account.",
                "Private ICS link",
                "Subscribe using a calendar address.",
                "Waiting for browser…",
                "Cancel",
                "Try again",
                "Sign-in cancelled. Your accounts are unchanged.",
                "Events may be out of date.",
                "Reconnect",
                "No calendars found for this account.",
                "Add a calendar link",
                "Anyone with the address can read this calendar.",
                "Stays on this computer.",
                "Subscribe",
                "Google sign-in is not configured in this build.",
            ),
        };
        ui.set_shortcut_change_label(change.into());
        ui.set_shortcut_conflict_line(conflict_line.into());
        ui.set_add_account_title(add_title.into());
        ui.set_add_account_intro(add_intro.into());
        ui.set_google_account_label(google.into());
        ui.set_google_account_desc(google_desc.into());
        ui.set_outlook_account_desc(outlook_desc.into());
        ui.set_ics_account_label(ics_label.into());
        ui.set_ics_account_desc(ics_desc.into());
        ui.set_waiting_browser_text(waiting.into());
        ui.set_cancel_sign_in_label(cancel.into());
        ui.set_try_again_label(try_again.into());
        ui.set_sign_in_cancelled_text(cancelled.into());
        ui.set_events_stale_text(stale.into());
        ui.set_reconnect_label(reconnect.into());
        ui.set_no_calendars_text(no_cals.into());
        ui.set_ics_dialog_title(ics_title.into());
        ui.set_ics_privacy_text(privacy.into());
        ui.set_ics_stays_local_text(stays.into());
        ui.set_subscribe_label(subscribe.into());
        ui.set_ics_loading_text(
            match locale {
                LocaleCode::Ru => "Проверка календаря…",
                LocaleCode::Es => "Comprobando el calendario…",
                LocaleCode::Ja => "カレンダーを確認中…",
                LocaleCode::En => "Checking calendar…",
            }
            .into(),
        );
        ui.set_google_unavailable_text(google_missing.into());
        ui.set_disconnect_google_label(
            match locale {
                LocaleCode::Ru => "Отключить Google",
                LocaleCode::Es => "Desconectar Google",
                LocaleCode::Ja => "Google を切断",
                LocaleCode::En => "Disconnect Google",
            }
            .into(),
        );
        ui.set_ics_status_text(self.ics_error.clone().into());
        ui.set_google_connected(self.google_connected);
        ui.set_google_needs_signin(self.google_needs_signin);
        ui.set_google_status_text(
            google_status_line(
                locale,
                self.google_connected,
                self.google_needs_signin,
                &self.google_label,
                &self.google_error,
            )
            .into(),
        );
        ui.set_sign_in_title(if self.active_sign_in == ActiveSignIn::Google {
            ui.get_google_account_label()
        } else {
            strings.outlook_section.into()
        });
        ui.set_add_account_enabled(account_idle);
        let refresh_ready = !self.connection_busy;
        ui.set_can_connect_microsoft(
            account_idle
                && self.microsoft_account.can_connect()
                && lightcal::calendars::microsoft::valid_client_id(&self.prefs.microsoft_client_id),
        );
        ui.set_can_disconnect_microsoft(account_idle && self.microsoft_account.can_disconnect());
        ui.set_can_refresh_calendars(
            refresh_ready
                && (self
                    .provider
                    .sources
                    .iter()
                    .any(|source| source.id.starts_with("ics:"))
                    || matches!(
                        self.microsoft_account,
                        lightcal::calendars::account::MicrosoftAccountState::Connected { .. }
                            | lightcal::calendars::account::MicrosoftAccountState::Unavailable {
                                saved_credentials: true
                            }
                    )
                    || self.google_connected
                    || self.google_needs_signin
                    || self.prefs.windows_calendars_enabled),
        );
        ui.set_outlook_section_text(strings.outlook_section.into());
        ui.set_ics_section_text(strings.ics_section.into());
        ui.set_ics_help_text(strings.ics_help.into());
        ui.set_calendars_dot_note_text(strings.calendars_dot_note.into());
        ui.set_provider_error_text(match &self.provider.status {
            lightcal::platform::ProviderStatus::ProviderUnavailable(message) => {
                message.clone().into()
            }
            _ => "".into(),
        });
        ui.set_refresh_button_text(strings.refresh_button.into());
        ui.set_updates_heading_text(strings.updates_heading.into());
        ui.set_about_link_text(strings.tray_about.into());
        ui.set_updates_check_button_text(strings.updates_check_button.into());
        ui.set_updates_open_release_button_text(strings.updates_open_release_button.into());
        ui.set_current_language_code(locale.as_str().into());
        ui.set_current_theme_code(self.prefs.theme.as_str().into());
        ui.set_current_tray_icon_mode(self.prefs.tray_icon_mode.as_str().into());
        ui.set_tray_icon_monochrome(self.prefs.tray_icon_monochrome);
        ui.set_first_weekday_monday(self.prefs.first_weekday == Weekday::Monday);
        ui.set_highlight_current_month(self.prefs.highlight_current_month);
        ui.set_show_event_dots(self.prefs.show_event_dots);
        ui.set_auto_check_updates(self.prefs.auto_check_updates);
        let (check_title, check_description) = match locale {
            LocaleCode::En => (
                "Check for updates automatically",
                "Check at startup and daily; installation is controlled separately",
            ),
            LocaleCode::Ru => (
                "Автоматически проверять обновления",
                "При запуске и раз в сутки; установка включается отдельно",
            ),
            LocaleCode::Es => (
                "Buscar actualizaciones automáticamente",
                "Al iniciar y cada día; la instalación se configura aparte",
            ),
            LocaleCode::Ja => (
                "更新を自動確認",
                "起動時と毎日確認します。インストールは別に設定します",
            ),
        };
        ui.set_label_auto_updates_text(check_title.into());
        ui.set_desc_auto_updates_text(check_description.into());
        ui.set_label_update_channel_text(strings.label_update_channel.into());
        ui.set_desc_update_channel_text(strings.desc_update_channel.into());
        ui.set_label_license_source_text(strings.label_license_source.into());
        ui.set_desc_license_source_text(strings.desc_license_source.into());
        ui.set_app_version_line_text(lightcal::version::VERSION.into());
        ui.set_about_version_text(
            lightcal::i18n::about_version_line(locale, lightcal::version::VERSION).into(),
        );
        ui.set_app_name_text(strings.app_name.into());
        ui.set_provider_readonly_tag_text(strings.provider_readonly_tag.into());
        ui.set_shortcut_unset_text(strings.shortcut_unset.into());
        ui.set_license_short_text(strings.license_short.into());
        ui.set_shortcut_conflict_title(strings.shortcut_conflict_title.into());
        ui.set_shortcut_conflict_hint(strings.shortcut_conflict_hint.into());
        ui.set_shortcut_reset_label(strings.shortcut_reset_button.into());
        ui.set_calendars_denied_title_text(strings.calendars_denied_title.into());
        ui.set_calendars_denied_desc_text(strings.access_denied_desc.into());
        ui.set_open_privacy_settings_label(strings.open_privacy_settings_button.into());
        ui.set_retry_sync_label(strings.retry_sync_button.into());
        ui.set_calendars_empty_title_text(strings.calendars_empty_title.into());
        ui.set_calendars_empty_desc_text(strings.calendars_unavailable_desc.into());
        ui.set_add_calendar_account_label(strings.add_calendar_account_button.into());
        ui.set_add_calendar_label(strings.add_calendar_button.into());
        ui.set_checking_updates_label(strings.checking_updates_label.into());
        if let UpdateCheckState::UpdateAvailable { latest_version, .. } = &self.update_state {
            ui.set_update_available_badge_text(format!("v{latest_version}").into());
            let title = match locale {
                LocaleCode::En => format!("Version {latest_version} is available"),
                LocaleCode::Ru => format!("Доступна версия {latest_version}"),
                LocaleCode::Es => format!("La versión {latest_version} está disponible"),
                LocaleCode::Ja => format!("バージョン {latest_version} が利用できます"),
            };
            ui.set_update_ready_title_text(title.into());
        } else {
            ui.set_update_available_badge_text(strings.update_available_badge.into());
            ui.set_update_ready_title_text(strings.update_ready_title.into());
        }
        let update_available =
            matches!(self.update_state, UpdateCheckState::UpdateAvailable { .. });
        if lightcal::installer::automatic_install_configured() {
            ui.set_install_update_label(labels[8].into());
            ui.set_update_ready_notes_text(strings.update_ready_notes.into());
            ui.set_desc_auto_install_text(strings.desc_auto_install.into());
        } else {
            ui.set_desc_auto_install_text(strings.manual_install_status.into());
            if update_available {
                ui.set_install_update_label(strings.download_release_button.into());
                ui.set_update_ready_notes_text(strings.manual_update_notes.into());
            } else {
                ui.set_install_update_label(labels[8].into());
                ui.set_update_ready_notes_text(strings.update_ready_notes.into());
            }
        }
        ui.set_release_notes_button_text(strings.release_notes_button.into());

        let lang_disp = match locale {
            LocaleCode::En => strings.lang_en_us,
            LocaleCode::Ru => strings.lang_ru,
            LocaleCode::Es => strings.lang_es,
            LocaleCode::Ja => strings.lang_ja,
        };
        ui.set_current_language_display(lang_disp.into());

        let theme_disp = match self.prefs.theme {
            ThemePreference::Light => strings.theme_light_obsidian,
            ThemePreference::Dark => strings.theme_dark_obsidian,
            ThemePreference::System => strings.theme_system_obsidian,
        };
        ui.set_current_theme_display(theme_disp.into());

        let fday_disp = if self.prefs.first_weekday == Weekday::Monday {
            strings.start_of_week_monday
        } else {
            strings.start_of_week_sunday
        };
        ui.set_current_first_weekday_display(fday_disp.into());

        let chan_disp = if self.prefs.update_channel == "preview" {
            strings.channel_preview
        } else {
            strings.channel_stable
        };
        ui.set_current_update_channel_display(chan_disp.into());

        ui.set_active_calendars_count_label(
            format_active_calendars_heading(locale, active_count).into(),
        );

        ui.set_language_dropdown_open(self.language_dropdown_open);
        ui.set_theme_dropdown_open(self.theme_dropdown_open);
        ui.set_first_day_dropdown_open(self.first_day_dropdown_open);
        ui.set_update_channel_dropdown_open(self.update_channel_dropdown_open);

        let has_conflict = !self.shortcut_error.is_empty();
        ui.set_shortcut_has_conflict(has_conflict);
        ui.set_shortcut_can_clear(self.prefs.global_shortcut.is_some() || has_conflict);
        ui.set_shortcut_clear_label(strings.shortcut_clear.into());
        if let Some(conflict) = &self.conflicting_shortcut {
            ui.set_global_shortcut_display(conflict.clone().into());
        } else {
            ui.set_global_shortcut_display(
                self.prefs
                    .global_shortcut
                    .clone()
                    .unwrap_or_else(|| strings.shortcut_unset.to_string())
                    .into(),
            );
        }
        ui.set_shortcut_error_message(self.shortcut_error.clone().into());
        ui.set_startup_status_text(self.startup_error.clone().into());
        ui.set_shortcut_record_label(strings.shortcut_record.into());
        ui.set_lang_en_us_label(strings.lang_en_us.into());
        ui.set_lang_ru_label(strings.lang_ru.into());
        ui.set_lang_es_label(strings.lang_es.into());
        ui.set_lang_ja_label(strings.lang_ja.into());
        ui.set_weekday_monday_label(strings.start_of_week_monday.into());
        ui.set_weekday_sunday_label(strings.start_of_week_sunday.into());
        ui.set_theme_light_label(strings.theme_light_obsidian.into());
        ui.set_theme_dark_label(strings.theme_dark_obsidian.into());
        ui.set_theme_system_label(strings.theme_system_obsidian.into());
        ui.set_channel_stable_label(strings.channel_stable.into());
        ui.set_channel_preview_label(strings.channel_preview.into());
        ui.set_update_channel_is_preview(self.prefs.update_channel == "preview");
        ui.set_github_button_text(strings.github_repo_button.into());
        ui.set_koona_button_text(strings.koona_app_button.into());

        ui.set_calendars_access_denied(self.calendars_access_denied);
        ui.set_calendars_is_empty(self.calendars_is_empty);

        let is_checking = matches!(self.update_state, UpdateCheckState::Checking);
        let is_avail = matches!(self.update_state, UpdateCheckState::UpdateAvailable { .. });
        ui.set_update_is_checking(is_checking);
        ui.set_update_is_available(is_avail);

        let gray_set = self.prefs.gray_weekdays();
        // One letter per day, in calendar column order; `index` stays Monday-based.
        let first = self.prefs.first_weekday.monday_index();
        let chips: Vec<WeekdayChipItem> = (0..7u8)
            .map(|offset| Weekday::from_monday_index((first + offset) % 7))
            .map(|day| WeekdayChipItem {
                index: day.monday_index() as i32,
                label: lightcal::i18n::weekday_letter(locale, day).into(),
                active: gray_set.contains(day),
            })
            .collect();
        ui.set_gray_weekday_chips(ModelRc::new(VecModel::from(chips)));

        ui.set_start_with_windows(self.prefs.start_with_windows);

        let mut upd_text = format_update_status(locale, &self.update_state);
        if self.update_state == UpdateCheckState::Idle {
            upd_text=match locale {
                LocaleCode::En=>"Check stable GitHub releases. Installation requires a configured signing certificate.",
                LocaleCode::Ru=>"Проверка стабильных релизов GitHub. Для установки нужен настроенный сертификат подписи.",
                LocaleCode::Es=>"Buscar versiones estables en GitHub. La instalación requiere un certificado configurado.",
                LocaleCode::Ja=>"GitHub の安定版を確認します。インストールには署名証明書の設定が必要です。",
            }.into();
        }
        if managed {
            upd_text=match locale {
                LocaleCode::En=>"MSIX updates are managed by Microsoft Store or the package deployment channel.",
                LocaleCode::Ru=>"Обновления MSIX выполняет Microsoft Store или канал установки пакета.",
                LocaleCode::Es=>"Microsoft Store o el canal de distribución administra las actualizaciones MSIX.",
                LocaleCode::Ja=>"MSIX の更新は Microsoft Store またはパッケージ配布元が管理します。",
            }.into();
        }
        ui.set_update_status_text(upd_text.into());
        self.sync_settings_view(ui);

        #[cfg(windows)]
        if let Some(tray) = &self.tray_icon {
            let now = chrono::Local::now();
            let (rgba, icon_size) = render_native_tray_icon(
                self.prefs.tray_icon_mode,
                self.prefs.tray_icon_monochrome,
                self.cal_state.today.day,
                self.cal_state.today.weekday(),
                (now.hour() as u8, now.minute() as u8),
                dark_mode,
            );
            if let Ok(icon) = tray_icon::Icon::from_rgba(rgba, icon_size, icon_size) {
                let _ = tray.set_icon(Some(icon));
            }
            let tip = format!(
                "Lightcal · {}",
                format_date_line(locale, self.cal_state.today, true)
            );
            let _ = tray.set_tooltip(Some(&tip));
        }
        #[cfg(windows)]
        self.tray_menu.relabel(locale);

        #[cfg(windows)]
        invalidate_taskbar_preview();
    }

    fn persist(&self) {
        let _ = self.store.save(&self.prefs);
    }
}

/// Settings and About (ADR 0011): copy, account cards and update status.
impl AppController {
    fn sync_settings_view(&self, ui: &AppWindow) {
        use lightcal::calendars::account::MicrosoftAccountState as Ms;
        use lightcal::i18n::{settings_text, updated_ago};
        let locale = self.prefs.locale();
        let st = settings_text(locale);
        let strings = catalog(locale);
        let now = chrono::Local::now();
        let updated = self.last_refreshed.map(|at| {
            updated_ago(
                locale,
                (now - at).num_minutes(),
                &at.format("%H:%M").to_string(),
            )
        });
        let offline_text = match self.last_refreshed {
            Some(at) => lightcal::i18n::offline_saved_at(locale, &at.format("%H:%M").to_string()),
            None => st.offline_plain.to_string(),
        };
        let join = |parts: &[&str]| {
            parts
                .iter()
                .filter(|p| !p.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join(" · ")
        };
        let initial = |text: &str, fallback: &str| {
            text.chars()
                .find(|c| c.is_alphanumeric())
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_else(|| fallback.to_string())
        };

        let items: Vec<CalendarSourceItem> = self
            .provider
            .sources
            .iter()
            .map(|s| CalendarSourceItem {
                id: s.id.clone().into(),
                name: s.name.clone().into(),
                account: s.account_label.clone().into(),
                swatch: mark_color(&s.color_hex),
                checked: self.prefs.selected_calendar_ids.contains(&s.id),
                removable: s.id.starts_with("ics:"),
            })
            .collect();
        let SourceGroups {
            outlook,
            google,
            ics,
            windows,
        } = split_sources(&items);
        let can_refresh = ui.get_can_refresh_calendars();
        let idle = !self.connection_busy && !self.provider_loading;
        let model = |list: Vec<CalendarSourceItem>| ModelRc::new(VecModel::from(list));
        let blank = AccountCardItem::default();
        let mut cards = Vec::new();

        // Outlook: a check in progress keeps showing the state it started from.
        let ms = match (&self.microsoft_account, &self.account_before_check) {
            (Ms::Checking, Some(before)) => before,
            (state, _) => state,
        };
        let outlook_visible = matches!(
            ms,
            Ms::Connected { .. }
                | Ms::NeedsSignIn
                | Ms::Unavailable {
                    saved_credentials: true
                }
        ) || !outlook.is_empty();
        if outlook_visible {
            let label = match ms {
                Ms::Connected { label: Some(label) } => label.as_str(),
                _ => "",
            };
            let checking = matches!(self.microsoft_account, Ms::Checking);
            let fresh = if matches!(ms, Ms::Connected { .. }) {
                updated.as_deref().unwrap_or("")
            } else {
                ""
            };
            let (notice_kind, notice, action) = match ms {
                Ms::NeedsSignIn => (2, st.access_expired.to_string(), st.sign_in),
                Ms::Unavailable { .. } => (1, offline_text.clone(), ""),
                _ => (0, String::new(), ""),
            };
            cards.push(AccountCardItem {
                id: "outlook".into(),
                title: st.outlook_title.into(),
                initial: initial(label, "O").into(),
                detail: if checking {
                    join(&[label, st.checking_account])
                } else {
                    join(&[label, fresh])
                }
                .into(),
                notice: notice.into(),
                notice_kind,
                notice_offline: notice_kind == 1,
                action_label: action.into(),
                can_refresh,
                can_disconnect: ui.get_can_disconnect_microsoft(),
                disconnect_label: st.disconnect.into(),
                empty_text: if outlook.is_empty() && matches!(ms, Ms::Connected { .. }) {
                    st.no_calendars.into()
                } else {
                    "".into()
                },
                sources: model(outlook.clone()),
            });
        }

        let google_visible = self.google_connected
            || self.google_needs_signin
            || !google.is_empty()
            || !self.google_error.is_empty();
        if google_visible {
            let fresh = if self.google_connected {
                updated.as_deref().unwrap_or("")
            } else {
                ""
            };
            let lower = self.google_error.to_lowercase();
            let offline = ["network", "offline", "connect", "timed out", "dns"]
                .iter()
                .any(|needle| lower.contains(needle));
            let (notice_kind, notice, action) = if self.google_needs_signin {
                (2, st.access_expired.to_string(), st.sign_in)
            } else if !self.google_error.is_empty() {
                (1, self.google_error.clone(), "")
            } else {
                (0, String::new(), "")
            };
            cards.push(AccountCardItem {
                id: "google".into(),
                title: st.google_title.into(),
                initial: initial(&self.google_label, "G").into(),
                detail: join(&[&self.google_label, fresh]).into(),
                notice: notice.into(),
                notice_kind,
                notice_offline: offline,
                action_label: action.into(),
                can_refresh: can_refresh && (self.google_connected || self.google_needs_signin),
                can_disconnect: idle,
                disconnect_label: st.disconnect.into(),
                empty_text: if google.is_empty() && self.google_connected {
                    st.no_calendars.into()
                } else {
                    "".into()
                },
                sources: model(google.clone()),
            });
        }

        for (index, source) in ics.iter().enumerate() {
            let error = if index == 0 {
                self.ics_error.as_str()
            } else {
                ""
            };
            cards.push(AccountCardItem {
                id: source.id.clone(),
                title: source.name.clone(),
                initial: initial(source.name.as_str(), "I").into(),
                detail: join(&[st.ics_detail, updated.as_deref().unwrap_or("")]).into(),
                notice: error.into(),
                notice_kind: if error.is_empty() { 0 } else { 1 },
                action_label: "".into(),
                can_refresh,
                can_disconnect: idle,
                disconnect_label: st.disconnect.into(),
                sources: model(vec![source.clone()]),
                ..blank.clone()
            });
        }

        if self.prefs.windows_calendars_enabled
            || !windows.is_empty()
            || self.calendars_access_denied
        {
            let denied = self.calendars_access_denied;
            cards.push(AccountCardItem {
                id: "windows".into(),
                title: st.windows_title.into(),
                initial: "W".into(),
                detail: join(&[
                    st.windows_detail,
                    if denied {
                        ""
                    } else {
                        updated.as_deref().unwrap_or("")
                    },
                ])
                .into(),
                notice: if denied { st.access_denied } else { "" }.into(),
                notice_kind: if denied { 2 } else { 0 },
                action_label: st.open_privacy.into(),
                can_refresh,
                can_disconnect: idle,
                disconnect_label: st.turn_off.into(),
                empty_text: if windows.is_empty() && !denied {
                    st.no_calendars.into()
                } else {
                    "".into()
                },
                sources: model(windows.clone()),
                ..blank.clone()
            });
        }

        // A provider-wide failure (slow refresh, worker error) is a quiet line.
        if let lightcal::platform::ProviderStatus::ProviderUnavailable(message) =
            &self.provider.status
        {
            if let Some(card) = cards.iter_mut().find(|card| card.notice_kind == 0) {
                card.notice = message.clone().into();
                card.notice_kind = 1;
            }
        }
        ui.set_account_cards(ModelRc::new(VecModel::from(cards)));

        let keys: Vec<SharedString> = self
            .conflicting_shortcut
            .as_deref()
            .or(self.prefs.global_shortcut.as_deref())
            .map(|s| s.split(" + ").map(SharedString::from).collect())
            .unwrap_or_default();
        ui.set_shortcut_keys(ModelRc::new(VecModel::from(keys)));
        let shortcut_error = if self.shortcut_error.is_empty() {
            String::new()
        } else if let Some(candidate) = &self.conflicting_shortcut {
            lightcal::i18n::shortcut_conflict_text(
                locale,
                candidate,
                self.prefs.global_shortcut.as_deref(),
                self.shortcut_error.contains("in use"),
            )
        } else {
            self.shortcut_error.clone()
        };

        let configured = lightcal::installer::automatic_install_configured();
        ui.set_install_configured(configured);
        let quiet_note = if self.prefs.auto_check_updates {
            st.auto_note
        } else {
            st.manual_note
        };
        let (kind, short, title, note) = match &self.update_state {
            UpdateCheckState::Idle => (
                0,
                st.not_checked_short.to_string(),
                st.idle_title.to_string(),
                quiet_note.to_string(),
            ),
            UpdateCheckState::Checking => (
                2,
                st.checking_short.to_string(),
                st.checking_title.to_string(),
                quiet_note.to_string(),
            ),
            UpdateCheckState::UpToDate { .. } => (
                1,
                st.up_to_date_short.to_string(),
                st.up_to_date_title.to_string(),
                quiet_note.to_string(),
            ),
            UpdateCheckState::UpdateAvailable { latest_version, .. } => (
                3,
                lightcal::i18n::update_available_short(locale, latest_version),
                ui.get_update_ready_title_text().to_string(),
                if configured {
                    strings.update_ready_notes.to_string()
                } else {
                    st.release_page_note.to_string()
                },
            ),
            UpdateCheckState::NoStableRelease => (
                0,
                st.not_checked_short.to_string(),
                st.idle_title.to_string(),
                format_update_status(locale, &self.update_state),
            ),
            UpdateCheckState::Offline => (
                4,
                st.offline_short.to_string(),
                st.offline_title.to_string(),
                format_update_status(locale, &self.update_state),
            ),
            UpdateCheckState::Timeout
            | UpdateCheckState::RateLimited
            | UpdateCheckState::InvalidResponse(_) => (
                5,
                st.problem_short.to_string(),
                st.problem_title.to_string(),
                format_update_status(locale, &self.update_state),
            ),
        };
        let note = if lightcal::installer::package_managed() {
            ui.get_update_status_text().to_string()
        } else {
            note
        };

        ui.set_settings_strings(SettingsStrings {
            window_title: st.window_title.into(),
            close_label: st.close.into(),
            connected_accounts: st.connected_accounts.into(),
            add_account: st.add_account.into(),
            read_only_note: st.read_only_note.into(),
            connect_heading: st.connect_heading.into(),
            connect_note: st.connect_note.into(),
            google_title: st.google_title.into(),
            google_desc: st.google_desc.into(),
            outlook_title: st.outlook_title.into(),
            outlook_desc: st.outlook_desc.into(),
            ics_title: st.ics_title.into(),
            ics_desc: st.ics_desc.into(),
            windows_title: st.windows_title.into(),
            windows_desc: st.windows_desc.into(),
            signing_title: lightcal::i18n::signing_in_title(
                locale,
                if self.active_sign_in == ActiveSignIn::Google {
                    "Google"
                } else {
                    "Outlook"
                },
            )
            .into(),
            signing_note: st.signing_note.into(),
            open_browser_again: st.open_browser_again.into(),
            cancel: st.cancel.into(),
            ics_heading: st.ics_heading.into(),
            ics_field_label: st.ics_field_label.into(),
            ics_placeholder: st.ics_placeholder.into(),
            ics_privacy: st.ics_privacy.into(),
            subscribe: st.subscribe.into(),
            refresh_label: st.refresh.into(),
            more_label: st.more.into(),
            back_label: st.back.into(),
            no_selection_hint: if !items.is_empty() && items.iter().all(|item| !item.checked) {
                st.no_selection_hint.into()
            } else {
                "".into()
            },
            cancelled_text: st.cancelled_text.into(),
            try_again: st.try_again.into(),
            shortcut_recording: st.shortcut_recording.into(),
            shortcut_clear_label: st.shortcut_clear.into(),
            shortcut_error: shortcut_error.into(),
            update_short: short.into(),
            update_kind: kind,
            app_update_button: if kind == 3 { st.details } else { st.check_now }.into(),
            about_tagline: st.about_tagline.into(),
            about_status_title: title.into(),
            about_status_note: note.into(),
            view_release: st.view_release.into(),
            check_now: st.check_now.into(),
            github_title: st.github_title.into(),
            github_desc: st.github_desc.into(),
            koona_title: st.koona_title.into(),
            koona_desc: st.koona_desc.into(),
            sponsor_label: st.sponsor_label.into(),
            sponsor_monthly_title: st.sponsor_monthly_title.into(),
            sponsor_monthly_desc: st.sponsor_monthly_desc.into(),
            sponsor_once_title: st.sponsor_once_title.into(),
            sponsor_once_desc: st.sponsor_once_desc.into(),
            license_title: st.license_title.into(),
            license_desc: st.license_desc.into(),
            release_notes_title: st.release_notes_title.into(),
            release_notes_desc: st.release_notes_desc.into(),
            auto_check_label: st.auto_check_label.into(),
        });
    }
}

fn show_application_window(ui: &AppWindow) {
    let _ = ui.show();
    #[cfg(windows)]
    {
        let (lw, lh) = target_logical_size(ui.get_settings_open());
        let hwnd = lightcal::platform::windows::find_lightcal_hwnd();
        if !hwnd.is_null() {
            unsafe {
                lightcal::platform::windows::show_and_restore_window(hwnd, lw, lh);
            }
            ensure_window_hooks(ui, hwnd);
        }
    }
    sync_window_geometry(ui, ui.get_settings_open());
    ui.invoke_focus_calendar_keys();
}

fn hide_application_window(ui: &AppWindow) {
    // Preserve the known working hide/show path for Slint's software renderer.
    let _ = ui.hide();
    #[cfg(windows)]
    {
        let hwnd = lightcal::platform::windows::find_lightcal_hwnd();
        if !hwnd.is_null() {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                    hwnd,
                    windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE,
                );
            }
        }
    }
}

fn application_window_is_open(ui: &AppWindow) -> bool {
    #[cfg(windows)]
    if let Some(open) = lightcal::platform::windows::main_window_is_open() {
        return open;
    }
    ui.window().is_visible() && !ui.window().is_minimized()
}

/// Attaches the restore repaint and the taskbar preview (`WindowHooks`) to the native window,
/// and asks DWM for custom previews again on each show in case it refused or forgot them.
#[cfg(windows)]
fn ensure_window_hooks(ui: &AppWindow, hwnd: windows_sys::Win32::Foundation::HWND) {
    if hwnd.is_null() {
        return;
    }
    WINDOW_HOOKS.with(|slot| {
        let attached = slot
            .borrow()
            .as_ref()
            .is_some_and(|hooks| hooks.is_attached_to(hwnd));
        if !attached {
            let restore_ui = ui.as_weak();
            let snapshot_ui = ui.as_weak();
            // SAFETY: `hwnd` is this process's live application window, owned by the UI thread.
            let hooks = unsafe {
                lightcal::platform::windows::taskbar::WindowHooks::install(
                    hwnd,
                    move || {
                        // Runs before the restored window paints, so its first frame is complete.
                        if let Some(ui) = restore_ui.upgrade() {
                            request_full_repaint(&ui);
                        }
                    },
                    move || {
                        let ui = snapshot_ui.upgrade()?;
                        if ui.get_settings_open() {
                            return None;
                        }
                        ui.window().take_snapshot().ok()
                    },
                )
            };
            *slot.borrow_mut() = hooks.ok();
        }
        if let Some(hooks) = slot.borrow().as_ref() {
            hooks.request_previews();
        }
    });
}

/// Makes the next frame repaint and present every pixel, not only the dirty regions.
#[cfg(any(windows, test))]
fn request_full_repaint(ui: &AppWindow) {
    ui.set_repaint_generation(ui.get_repaint_generation().wrapping_add(1) & i32::MAX);
    ui.window().request_redraw();
}

/// Requests DWM rounded corners, the system shadow and, with `LIGHTCAL_MICA=1`, the Mica
/// backdrop prototype (ADR 0011). Silently does nothing before the native window exists.
fn apply_window_chrome(ui: &AppWindow) {
    #[cfg(windows)]
    {
        use lightcal::platform::windows::{find_lightcal_hwnd, mica_requested, WindowChrome};
        use windows_sys::Win32::UI::WindowsAndMessaging::IsWindow;
        thread_local! {
            /// Last window handle and the (dark, mica) state applied to it.
            static APPLIED: std::cell::Cell<(isize, Option<(bool, bool)>)> =
                const { std::cell::Cell::new((0, None)) };
        }
        let (cached, applied) = APPLIED.with(|slot| slot.get());
        let mut hwnd = cached as windows_sys::Win32::Foundation::HWND;
        // SAFETY: IsWindow only checks whether the handle still names a window.
        if hwnd.is_null() || unsafe { IsWindow(hwnd) } == 0 {
            hwnd = find_lightcal_hwnd();
        }
        if hwnd.is_null() {
            return;
        }
        let state = (ui.get_dark_mode(), mica_requested());
        if hwnd as isize == cached && applied == Some(state) {
            return;
        }
        let chrome = WindowChrome {
            dark_mode: state.0,
            mica: state.1,
        };
        unsafe {
            lightcal::platform::windows::apply_window_chrome(hwnd, chrome);
        }
        APPLIED.with(|slot| slot.set((hwnd as isize, Some(state))));
    }
    #[cfg(not(windows))]
    let _ = ui;
}

#[cfg(windows)]
fn invalidate_taskbar_preview() {
    WINDOW_HOOKS.with(|slot| {
        if let Some(hooks) = slot.borrow().as_ref() {
            hooks.invalidate_preview();
        }
    });
}

fn ctrl_release_url(ui_weak: &slint::Weak<AppWindow>) -> String {
    let mut url = lightcal::updater::DEFAULT_RELEASE_PAGE.to_string();
    with_active_controller(ui_weak, |ctrl, _| {
        if let UpdateCheckState::UpdateAvailable { release_url, .. } = &ctrl.update_state {
            url = release_url.clone();
        }
    });
    url
}

fn open_browser_link(url: &str) {
    if is_allowed_external_link(url) {
        #[cfg(windows)]
        open_allowed_browser_link(url);
    }
}

#[cfg(windows)]
fn open_allowed_browser_link(url: &str) {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let wide_url: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
    let wide_open: Vec<u16> = "open\0".encode_utf16().collect();
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            wide_open.as_ptr(),
            wide_url.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}

/// Handles to the tray context-menu items, so a language change can relabel them.
#[cfg(windows)]
pub(crate) struct TrayMenuItems {
    open: muda::MenuItem,
    today: muda::MenuItem,
    settings: muda::MenuItem,
    sponsor: muda::MenuItem,
    exit: muda::MenuItem,
    /// Locale the labels currently show; relabeling is skipped while it matches.
    locale: std::cell::Cell<LocaleCode>,
}

#[cfg(windows)]
impl TrayMenuItems {
    /// Items labeled for `locale`, not yet attached to a menu (tests and headless renders).
    pub(crate) fn new(locale: LocaleCode) -> Self {
        let labels = lightcal::i18n::tray_menu_copy(locale);
        Self {
            open: muda::MenuItem::new(labels.open, true, None),
            today: muda::MenuItem::new(lightcal::i18n::catalog(locale).today_button, true, None),
            settings: muda::MenuItem::new(labels.settings, true, None),
            sponsor: muda::MenuItem::new(labels.sponsor, true, None),
            exit: muda::MenuItem::new(labels.exit, true, None),
            locale: std::cell::Cell::new(locale),
        }
    }

    fn relabel(&self, locale: LocaleCode) {
        if self.locale.get() == locale {
            return;
        }
        let labels = lightcal::i18n::tray_menu_copy(locale);
        self.open.set_text(labels.open);
        self.today
            .set_text(lightcal::i18n::catalog(locale).today_button);
        self.settings.set_text(labels.settings);
        self.sponsor.set_text(labels.sponsor);
        self.exit.set_text(labels.exit);
        self.locale.set(locale);
    }
}

thread_local! {
    #[cfg(windows)]
    static WINDOW_HOOKS: RefCell<Option<lightcal::platform::windows::taskbar::WindowHooks>> = const { RefCell::new(None) };
    static ACTIVE_CONTROLLER: std::cell::RefCell<Option<std::rc::Weak<std::cell::RefCell<AppController>>>> = const { std::cell::RefCell::new(None) };
    static ACTIVE_UI: std::cell::RefCell<Option<slint::Weak<AppWindow>>> = const { std::cell::RefCell::new(None) };
}

fn with_active_controller<F>(ui_weak: &slint::Weak<AppWindow>, f: F)
where
    F: FnOnce(&mut AppController, &AppWindow),
{
    ACTIVE_CONTROLLER.with(|c| {
        if let Some(weak_ctrl) = c.borrow().as_ref() {
            if let Some(ctrl) = weak_ctrl.upgrade() {
                if let Some(ui) = ui_weak.upgrade() {
                    f(&mut ctrl.borrow_mut(), &ui);
                }
            }
        }
    });
}

fn apply_loaded_provider(data: lightcal::calendars::CalendarLoad, ui_weak: slint::Weak<AppWindow>) {
    let snapshot = data.snapshot;
    with_active_controller(&ui_weak, |ctrl, ui| {
        ctrl.microsoft_account = data.microsoft;
        ctrl.account_before_check = None;
        ctrl.google_connected = data.google_connected;
        ctrl.google_needs_signin = data.google_needs_signin;
        if let Some(configuration) = data.google_configuration {
            ctrl.prefs.google_client_id = configuration.client_id;
            ctrl.prefs.google_client_secret = configuration.client_secret;
        }
        ctrl.google_label = data.google_label.unwrap_or_default();
        ctrl.google_error = data.google_error;
        if data.google_skipped > 0 && ctrl.google_error.is_empty() {
            ctrl.google_error = skipped_google_events(ctrl.prefs.locale(), data.google_skipped);
        }
        ctrl.ics_error = data.ics_error;
        ctrl.provider_loading = false;
        match &snapshot.status {
            lightcal::platform::ProviderStatus::AccessDenied => {
                ctrl.calendars_access_denied = true;
                ctrl.calendars_is_empty = snapshot.sources.is_empty();
            }
            lightcal::platform::ProviderStatus::NoCalendars => {
                ctrl.calendars_access_denied = false;
                ctrl.calendars_is_empty = true;
            }
            lightcal::platform::ProviderStatus::ProviderUnavailable(_) => {
                ctrl.calendars_access_denied = false;
                ctrl.calendars_is_empty = false;
            }
            _ => {
                ctrl.calendars_access_denied = false;
                ctrl.calendars_is_empty = snapshot.sources.is_empty();
            }
        }
        if matches!(
            snapshot.status,
            lightcal::platform::ProviderStatus::Available
                | lightcal::platform::ProviderStatus::NoCalendars
        ) {
            ctrl.last_refreshed = Some(chrono::Local::now());
        }
        ctrl.provider = snapshot;
        ctrl.sync_to_ui(ui);
    });
}

fn request_update_check(ui_weak: slint::Weak<AppWindow>) {
    if lightcal::installer::package_managed() {
        return;
    }
    let mut start = false;
    let mut failure_text = "";
    with_active_controller(&ui_weak, |ctrl, ui| {
        if ctrl.update_state != UpdateCheckState::Checking {
            ctrl.update_state = UpdateCheckState::Checking;
            failure_text = catalog(ctrl.prefs.locale()).worker_failed_error;
            ctrl.sync_to_ui(ui);
            start = true;
        }
    });
    if start {
        std::thread::spawn(move || {
            let state =
                run_guarded(|| lightcal::updater::check_latest_release(lightcal::version::VERSION))
                    .unwrap_or_else(|_| {
                        UpdateCheckState::InvalidResponse(failure_text.to_string())
                    });
            let _ = slint::invoke_from_event_loop(move || {
                with_active_controller(&ui_weak, |ctrl, ui| {
                    ctrl.update_state = state;
                    ctrl.sync_to_ui(ui);
                });
                let mut auto_install = false;
                with_active_controller(&ui_weak, |ctrl, _| {
                    auto_install = ctrl.prefs.auto_install_updates
                        && matches!(ctrl.update_state, UpdateCheckState::UpdateAvailable { .. });
                });
                if auto_install {
                    request_install(ui_weak);
                }
            });
        });
    }
}

fn request_install(ui_weak: slint::Weak<AppWindow>) {
    if lightcal::installer::package_managed() {
        return;
    }
    let mut directory = None;
    let mut failure_text = "";
    with_active_controller(&ui_weak, |ctrl, ui| {
        if !ctrl.install_busy {
            failure_text = catalog(ctrl.prefs.locale()).worker_failed_error;
            ctrl.install_busy = true;
            ctrl.install_status = "Downloading and verifying the signed update…".into();
            directory = Some(ctrl.store.dir().to_path_buf());
            ctrl.sync_to_ui(ui);
        }
    });
    let Some(directory) = directory else {
        return;
    };
    std::thread::spawn(move || {
        let result = run_guarded(|| {
            lightcal::installer::stage(&directory)
                .and_then(|path| lightcal::installer::launch_helper(&path))
        })
        .unwrap_or_else(|_| Err(failure_text.to_string()));
        let _ = slint::invoke_from_event_loop(move || match result {
            Ok(()) => {
                let _ = slint::quit_event_loop();
            }
            Err(error) => with_active_controller(&ui_weak, |ctrl, ui| {
                ctrl.install_busy = false;
                ctrl.install_status = if error.contains("LIGHTCAL_UPDATE_CERT_SHA256") {
                    catalog(ctrl.prefs.locale())
                        .manual_install_status
                        .to_string()
                } else {
                    error
                };
                ctrl.sync_to_ui(ui);
            }),
        });
    });
}

thread_local! {
    /// A calendar reload was requested while another load or account operation was running.
    static RELOAD_PENDING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Whether the last reload used the synthetic fixture (replayed reloads reuse it).
    static RELOAD_SYNTHETIC: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs a worker closure, converting a panic into an error so the completion callback
/// always runs and the busy flags it owns are always cleared.
fn run_guarded<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        .map_err(|_| "worker panicked".to_string())
}

/// Runs a calendar load, converting a panic into an error message so the completion path
/// always runs and `provider_loading` is always cleared.
fn run_load_guarded<F>(load: F) -> Result<lightcal::calendars::CalendarLoad, String>
where
    F: FnOnce() -> lightcal::calendars::CalendarLoad,
{
    run_guarded(load)
        .map_err(|_| "Calendar refresh failed unexpectedly. It will be retried.".to_string())
}

/// Restores the account state saved before a check began. Only a still-`Checking` state is
/// replaced, so a state set by another operation is never overwritten.
fn restore_account_after_check(
    account: &mut lightcal::calendars::account::MicrosoftAccountState,
    before: &mut Option<lightcal::calendars::account::MicrosoftAccountState>,
) {
    let saved = before.take();
    if matches!(
        account,
        lightcal::calendars::account::MicrosoftAccountState::Checking
    ) {
        if let Some(previous) = saved {
            *account = previous;
        }
    }
}

/// A load result is applied only while its request is still current and the displayed month
/// is the one that was loaded. Watchdog expiry does not change the request, so a healthy slow
/// result still qualifies.
fn result_is_visible(current_request: u64, request_id: u64, month_matches: bool) -> bool {
    current_request == request_id && month_matches
}

/// Applies watchdog expiry to the shared state. The request id is left alone so a late result
/// is still accepted. Returns true when the expiry applied.
fn expire_load(
    current_request: u64,
    request_id: u64,
    loading: &mut bool,
    account: &mut lightcal::calendars::account::MicrosoftAccountState,
    before: &mut Option<lightcal::calendars::account::MicrosoftAccountState>,
) -> bool {
    if !watchdog_should_expire(current_request, request_id, *loading) {
        return false;
    }
    *loading = false;
    restore_account_after_check(account, before);
    true
}

/// Returns true when a load may start now; otherwise records that one reload is pending.
fn start_or_defer_reload(busy: bool) -> bool {
    if busy {
        RELOAD_PENDING.with(|p| p.set(true));
        false
    } else {
        RELOAD_PENDING.with(|p| p.set(false));
        true
    }
}

fn take_pending_reload() -> bool {
    RELOAD_PENDING.with(|p| p.replace(false))
}

/// How long a calendar load may run before the UI stops waiting for it.
const LOAD_WATCHDOG_SECS: u64 = 90;

thread_local! {
    /// Watchdog for the in-flight load: (request id, timer). Dropping the timer stops it.
    static LOAD_WATCHDOG: std::cell::RefCell<Option<(u64, slint::Timer)>> =
        const { std::cell::RefCell::new(None) };
}

/// A watchdog fires only for the request that is still current and still loading.
fn watchdog_should_expire(current_request: u64, request_id: u64, loading: bool) -> bool {
    loading && current_request == request_id
}

/// A result that is not visible may release the in-flight guard only when it belongs to the
/// current request; a newer in-flight request owns the flag.
fn stale_result_releases_guard(current_request: u64, request_id: u64) -> bool {
    current_request == request_id
}

fn cancel_load_watchdog(request_id: u64) {
    LOAD_WATCHDOG.with(|w| {
        let mut slot = w.borrow_mut();
        if matches!(&*slot, Some((id, _)) if *id == request_id) {
            *slot = None;
        }
    });
}

fn start_load_watchdog(ui_weak: slint::Weak<AppWindow>, request_id: u64) {
    let timer = slint::Timer::default();
    let weak = ui_weak;
    timer.start(
        slint::TimerMode::SingleShot,
        std::time::Duration::from_secs(LOAD_WATCHDOG_SECS),
        move || {
            let mut expired = false;
            with_active_controller(&weak, |ctrl, ui| {
                if expire_load(
                    ctrl.provider_request,
                    request_id,
                    &mut ctrl.provider_loading,
                    &mut ctrl.microsoft_account,
                    &mut ctrl.account_before_check,
                ) {
                    expired = true;
                    // The request stays alive: a late result for it is still applied.
                    ctrl.provider.status = lightcal::platform::ProviderStatus::ProviderUnavailable(
                        catalog(ctrl.prefs.locale()).refresh_slow_status.to_string(),
                    );
                    ctrl.sync_to_ui(ui);
                }
            });
            if expired {
                // Only replays when a reload is actually pending.
                replay_pending_reload(weak.clone());
            }
        },
    );
    LOAD_WATCHDOG.with(|w| *w.borrow_mut() = Some((request_id, timer)));
}

fn replay_pending_reload(ui_weak: slint::Weak<AppWindow>) {
    if take_pending_reload() {
        let synthetic = RELOAD_SYNTHETIC.with(|s| s.get());
        request_visible_calendars(ui_weak, synthetic);
    }
}

fn request_visible_calendars(ui_weak: slint::Weak<AppWindow>, use_synthetic: bool) {
    RELOAD_SYNTHETIC.with(|s| s.set(use_synthetic));
    let mut anchor = None;
    let mut request_id = 0;
    let mut config = None;
    with_active_controller(&ui_weak, |ctrl, ui| {
        if !start_or_defer_reload(ctrl.connection_busy || ctrl.provider_loading) {
            return;
        }
        ctrl.provider_request += 1;
        request_id = ctrl.provider_request;
        let month = ctrl.cal_state.displayed_month;
        anchor = LocalDate::new(month.year, month.month, 1);
        config = Some((ctrl.store.dir().to_path_buf(), ctrl.prefs.clone()));
        ctrl.provider_loading = true;
        if !matches!(
            ctrl.microsoft_account,
            lightcal::calendars::account::MicrosoftAccountState::Disconnected
                | lightcal::calendars::account::MicrosoftAccountState::Connected { .. }
                | lightcal::calendars::account::MicrosoftAccountState::Checking
        ) {
            ctrl.account_before_check = Some(ctrl.microsoft_account.clone());
            ctrl.microsoft_account = lightcal::calendars::account::MicrosoftAccountState::Checking;
        }
        ctrl.sync_to_ui(ui);
    });
    if let (Some(anchor), Some((dir, prefs))) = (anchor, config) {
        start_load_watchdog(ui_weak.clone(), request_id);
        std::thread::spawn(move || {
            let result = run_load_guarded(|| {
                if use_synthetic {
                    lightcal::calendars::CalendarLoad {
                        snapshot: ProviderSnapshot::synthetic(anchor),
                        microsoft:
                            lightcal::calendars::account::MicrosoftAccountState::Disconnected,
                        google_connected: false,
                        google_needs_signin: false,
                        google_configuration: None,
                        google_label: None,
                        google_error: String::new(),
                        google_skipped: 0,
                        ics_error: String::new(),
                    }
                } else {
                    lightcal::calendars::query(
                        anchor,
                        &dir,
                        &prefs.microsoft_client_id,
                        &prefs.google_client_id,
                        &prefs.google_client_secret,
                        &prefs.selected_calendar_ids,
                        prefs.windows_calendars_enabled,
                        prefs.locale(),
                    )
                }
            });
            let _ = slint::invoke_from_event_loop(move || {
                cancel_load_watchdog(request_id);
                let mut still_visible = false;
                with_active_controller(&ui_weak, |ctrl, _| {
                    still_visible = result_is_visible(
                        ctrl.provider_request,
                        request_id,
                        ctrl.cal_state.displayed_month == anchor.year_month(),
                    );
                    if !still_visible
                        && stale_result_releases_guard(ctrl.provider_request, request_id)
                    {
                        // A stale result is dropped, but this request's in-flight guard must
                        // be released and its account check ended. A newer request owns its
                        // own flag and saved state.
                        ctrl.provider_loading = false;
                        restore_account_after_check(
                            &mut ctrl.microsoft_account,
                            &mut ctrl.account_before_check,
                        );
                    }
                });
                match result {
                    Ok(data) if still_visible => apply_loaded_provider(data, ui_weak.clone()),
                    Ok(_) => {}
                    Err(_) if !still_visible => {}
                    Err(message) => with_active_controller(&ui_weak, |ctrl, ui| {
                        ctrl.provider_loading = false;
                        restore_account_after_check(
                            &mut ctrl.microsoft_account,
                            &mut ctrl.account_before_check,
                        );
                        ctrl.provider.status =
                            lightcal::platform::ProviderStatus::ProviderUnavailable(message);
                        ctrl.sync_to_ui(ui);
                    }),
                }
                replay_pending_reload(ui_weak);
            });
        });
    }
}

enum CalendarAction {
    ConnectMicrosoft(String),
    ConnectGoogle {
        client_id: String,
        client_secret: String,
    },
    DisconnectMicrosoft,
    DisconnectGoogle,
    AddIcs(String, String),
    Remove(String),
}

fn run_calendar_action(ui_weak: slint::Weak<AppWindow>, action: CalendarAction, synthetic: bool) {
    if matches!(
        action,
        CalendarAction::ConnectMicrosoft(_) | CalendarAction::ConnectGoogle { .. }
    ) {
        forget_sign_in_url();
    }
    let mut config = None;
    with_active_controller(&ui_weak, |ctrl, ui| {
        let allowed = match &action {
            CalendarAction::DisconnectMicrosoft => ctrl.microsoft_account.can_disconnect(),
            CalendarAction::DisconnectGoogle => {
                ctrl.google_connected || ctrl.google_needs_signin || !ctrl.google_error.is_empty()
            }
            CalendarAction::Remove(id) if id.starts_with("graph:") => {
                ctrl.microsoft_account.can_disconnect()
            }
            CalendarAction::Remove(id) if id.starts_with("google:") => {
                ctrl.google_connected || ctrl.google_needs_signin || !ctrl.google_error.is_empty()
            }
            _ => true,
        };
        if !ctrl.connection_busy && !ctrl.provider_loading && allowed {
            let sign_in_epoch = if matches!(
                action,
                CalendarAction::ConnectMicrosoft(_) | CalendarAction::ConnectGoogle { .. }
            ) {
                match SIGN_IN_GATE.begin() {
                    Ok(epoch) => epoch,
                    Err(error) => {
                        ctrl.connection_message = error;
                        ctrl.sync_to_ui(ui);
                        return;
                    }
                }
            } else {
                0
            };
            ctrl.connection_busy = true;
            ctrl.connection_message.clear();
            ctrl.provider_request += 1;
            config = Some((
                ctrl.store.dir().to_path_buf(),
                ctrl.cal_state.today,
                ctrl.microsoft_account.clone(),
                sign_in_epoch,
                ctrl.prefs.locale(),
            ));
            if matches!(action, CalendarAction::ConnectMicrosoft(_)) {
                ctrl.held_microsoft = ctrl.microsoft_account.clone();
                ctrl.active_sign_in = ActiveSignIn::Microsoft;
                ctrl.microsoft_account =
                    lightcal::calendars::account::MicrosoftAccountState::Connecting;
                ctrl.sign_in_cancelled = false;
                ctrl.add_account_open = false;
                ctrl.ics_dialog_open = false;
            }
            if matches!(action, CalendarAction::ConnectGoogle { .. }) {
                ctrl.held_google_connected = ctrl.google_connected;
                ctrl.held_google_needs_signin = ctrl.google_needs_signin;
                ctrl.held_google_label = ctrl.google_label.clone();
                ctrl.held_google_error = ctrl.google_error.clone();
                ctrl.active_sign_in = ActiveSignIn::Google;
                ctrl.google_error.clear();
                ctrl.sign_in_cancelled = false;
                ctrl.add_account_open = false;
                ctrl.ics_dialog_open = false;
            }
            ctrl.sync_to_ui(ui);
        }
    });
    let Some((dir, anchor, previous_account, sign_in_epoch, worker_locale)) = config else {
        return;
    };
    std::thread::spawn(move || {
        let operation = || -> Result<Option<String>, String> {
            // Browser consent must not hold the credential lock for its whole lifetime.
            let mut microsoft_token = None;
            let mut google_token = None;
            match &action {
                CalendarAction::ConnectMicrosoft(client) => {
                    let token = lightcal::calendars::microsoft::connect(client, |url| {
                        let _ = slint::invoke_from_event_loop(move || {
                            remember_sign_in_url(&url);
                            open_browser_link(&url);
                        });
                    })?;
                    if !SIGN_IN_GATE.is_current(sign_in_epoch) {
                        return Err("Sign-in cancelled".into());
                    }
                    microsoft_token = Some(token);
                }
                CalendarAction::ConnectGoogle {
                    client_id,
                    client_secret,
                } => {
                    let token =
                        lightcal::calendars::google::connect(client_id, client_secret, |url| {
                            let _ = slint::invoke_from_event_loop(move || {
                                remember_sign_in_url(&url);
                                open_browser_link(&url);
                            });
                        })?;
                    if !SIGN_IN_GATE.is_current(sign_in_epoch) {
                        return Err("Sign-in cancelled".into());
                    }
                    google_token = Some(token);
                }
                _ => {}
            }
            if let CalendarAction::AddIcs(name, address) = &action {
                // Validate and download without holding the credential lock; the lock is
                // taken only briefly to read the current subscriptions.
                let mut probe = {
                    let _guard = lightcal::secure_store::credential_lock()?;
                    lightcal::secure_store::SecureStore::new(&dir).load()?
                };
                let id = lightcal::calendars::add_feed(&mut probe, name, address)?;
                let feed = probe
                    .ics
                    .iter()
                    .find(|f| f.id == id)
                    .ok_or("Could not prepare ICS subscription")?;
                lightcal::calendars::ics::fetch(feed, anchor)?;
            }
            let _guard = lightcal::secure_store::credential_lock()?;
            let save = || -> Result<Option<String>, String> {
                let store = lightcal::secure_store::SecureStore::new(&dir);
                let mut credentials = store.load()?;
                let mut removed = None;
                match &action {
                    CalendarAction::ConnectMicrosoft(client) => {
                        credentials.microsoft_refresh_token = microsoft_token;
                        credentials.microsoft_client_id = Some(client.clone());
                    }
                    CalendarAction::ConnectGoogle {
                        client_id,
                        client_secret,
                    } => {
                        credentials.google_refresh_token = google_token;
                        credentials.google_client_id = Some(client_id.clone());
                        credentials.google_client_secret = Some(client_secret.clone());
                    }
                    CalendarAction::DisconnectMicrosoft => {
                        credentials.microsoft_refresh_token = None;
                        credentials.microsoft_client_id = None;
                        removed = Some("graph:".to_string());
                    }
                    CalendarAction::DisconnectGoogle => {
                        credentials.google_refresh_token = None;
                        credentials.google_client_id = None;
                        credentials.google_client_secret = None;
                        removed = Some("google:".to_string());
                    }
                    CalendarAction::AddIcs(name, address) => {
                        // The feed was already downloaded and validated without the lock.
                        lightcal::calendars::add_feed(&mut credentials, name, address)?;
                    }
                    CalendarAction::Remove(id) => {
                        if id.starts_with("graph:") {
                            credentials.microsoft_refresh_token = None;
                            credentials.microsoft_client_id = None;
                            removed = Some("graph:".to_string());
                        } else if id.starts_with("google:") {
                            credentials.google_refresh_token = None;
                            credentials.google_client_id = None;
                            credentials.google_client_secret = None;
                            removed = Some("google:".to_string());
                        } else if id.starts_with("ics:") {
                            credentials.ics.retain(|f| f.id != *id);
                            removed = Some(id.clone());
                        } else {
                            return Err("Disable legacy Windows calendars using the switch".into());
                        }
                    }
                }
                store.save(&credentials)?;
                Ok(removed)
            };
            if sign_in_epoch != 0 {
                SIGN_IN_GATE.commit(sign_in_epoch, save)
            } else {
                save()
            }
        };
        let result = run_guarded(operation)
            .unwrap_or_else(|_| Err(catalog(worker_locale).worker_failed_error.to_string()));
        let added_ics = matches!(action, CalendarAction::AddIcs(..));
        let (connected_client, connected_google) = match action {
            CalendarAction::ConnectMicrosoft(client) => (Some(client), None),
            CalendarAction::ConnectGoogle {
                client_id,
                client_secret,
            } => (None, Some((client_id, client_secret))),
            _ => (None, None),
        };
        let _ = slint::invoke_from_event_loop(move || {
            if sign_in_epoch != 0 && !SIGN_IN_GATE.is_current(sign_in_epoch) {
                return;
            }
            let success = result.is_ok();
            with_active_controller(&ui_weak, |ctrl, ui| {
                ctrl.connection_busy = false;
                forget_sign_in_url();
                ctrl.connection_message = match result {
                    Ok(removed) => {
                        if let Some(client) = connected_client {
                            ctrl.prefs.microsoft_client_id = client;
                            ctrl.persist();
                            ctrl.microsoft_account =
                                lightcal::calendars::account::MicrosoftAccountState::Checking;
                        }
                        if let Some((client, secret)) = connected_google {
                            ctrl.prefs.google_client_id = client;
                            ctrl.prefs.google_client_secret = secret;
                            ctrl.persist();
                            ctrl.google_connected = true;
                            ctrl.google_needs_signin = false;
                            ctrl.google_error.clear();
                            ctrl.sign_in_cancelled = false;
                        }
                        if let Some(prefix) = removed {
                            if prefix == "graph:" {
                                ctrl.microsoft_account=lightcal::calendars::account::MicrosoftAccountState::Disconnected;
                            }
                            if prefix == "google:" {
                                ctrl.google_connected = false;
                                ctrl.google_needs_signin = false;
                                ctrl.google_label.clear();
                                ctrl.google_error.clear();
                            }
                            ctrl.prefs
                                .selected_calendar_ids
                                .retain(|id| !id.starts_with(&prefix));
                            ctrl.persist();
                        }
                        if added_ics {
                            ui.set_ics_source_url("".into());
                            ui.set_ics_source_name("".into());
                            ctrl.ics_dialog_open = false;
                        }
                        String::new()
                    }
                    Err(error) => {
                        ctrl.microsoft_account = previous_account;
                        error
                    }
                };
                ctrl.sync_to_ui(ui);
            });
            if success {
                request_visible_calendars(ui_weak, synthetic);
            } else {
                replay_pending_reload(ui_weak);
            }
        });
    });
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.get(1).map(String::as_str) == Some("--export-app-icon") {
        let path = args
            .get(2)
            .map(Path::new)
            .expect("Provide an output ICO path");
        std::fs::write(
            path,
            lightcal::platform::windows::icon::generate_application_ico(),
        )
        .expect("Could not write application icon");
        return;
    }
    if args.get(1).map(String::as_str) == Some("--export-icon-review") {
        let dir = args
            .get(2)
            .map(Path::new)
            .expect("Provide an output directory");
        let written = lightcal::platform::windows::icon::export_review_set(dir)
            .expect("Could not write icon review images");
        println!(
            "Wrote {} icon review images to {} (numerals: {})",
            written.len(),
            dir.display(),
            lightcal::platform::windows::icon::numeral_font_source()
        );
        return;
    }
    if args.get(1).map(String::as_str) == Some("--apply-update") {
        if let Some(path) = args.get(2).map(Path::new) {
            if let Err(error) = lightcal::installer::apply(path) {
                let _ = std::fs::write(path.with_extension("result.txt"), error);
                lightcal::installer::recover_after_failure(path);
                std::process::exit(1);
            }
        }
        return;
    }
    if let Some(pos) = args.iter().position(|a| a == "--render-snapshots") {
        let out_dir = args
            .get(pos + 1)
            .map(Path::new)
            .unwrap_or_else(|| Path::new("target/snapshots"));
        render_snapshots_to_dir(out_dir).expect("Failed to render UI snapshots");
        return;
    }

    let instance_id = env::var_os("LIGHTCAL_DATA_DIR")
        .map(|path| {
            use std::hash::{Hash, Hasher};
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            path.hash(&mut hash);
            format!("Lightcal.Isolated.{:016x}", hash.finish())
        })
        .unwrap_or_else(|| "Lightcal".into());
    let _single_instance = match SingleInstance::acquire(&instance_id) {
        SingleInstance::FirstInstance(guard) => guard,
        SingleInstance::AlreadyRunning => {
            // Secondary launch signaled the primary instance and exits immediately.
            return;
        }
    };

    #[cfg(windows)]
    lightcal::platform::windows::taskbar::initialize_identity();

    let start_in_tray = args.iter().any(|a| a == "--tray" || a == "--background");
    let use_synthetic = args.iter().any(|a| a == "--synthetic-calendars");

    let store = PreferencesStore::from_env();
    let loaded = store.load();
    let loaded_warning = loaded.localized_warning();
    let mut prefs = loaded.preferences;
    if prefs.microsoft_client_id.is_empty() {
        prefs.microsoft_client_id = option_env!("LIGHTCAL_MICROSOFT_CLIENT_ID")
            .unwrap_or("")
            .to_string();
    }
    if prefs.google_client_id.is_empty() {
        prefs.google_client_id = option_env!("LIGHTCAL_GOOGLE_CLIENT_ID")
            .unwrap_or("")
            .to_string();
    }
    if prefs.google_client_secret.is_empty() {
        // A runtime value (environment or a local `.env`) overrides the embedded one.
        prefs.google_client_secret =
            lightcal::preferences::local_env_value("LIGHTCAL_GOOGLE_CLIENT_SECRET");
    }
    if prefs.google_client_secret.is_empty() {
        prefs.google_client_secret = option_env!("LIGHTCAL_GOOGLE_CLIENT_SECRET")
            .unwrap_or("")
            .to_string();
    }
    if use_synthetic && prefs.selected_calendar_ids.is_empty() {
        prefs.selected_calendar_ids.insert("win-work".to_string());
        prefs
            .selected_calendar_ids
            .insert("win-personal".to_string());
    }

    let today = current_local_date();
    let mut cal_state = CalendarState::new(today, prefs.first_weekday, prefs.gray_weekdays());
    cal_state.select_date(today);

    // Initial snapshot: synthetic fixture when requested, or immediate empty container while
    // background worker loads real WinRT calendars without blocking GUI startup.
    let provider = if use_synthetic {
        query_calendar_provider(today, true)
    } else {
        ProviderSnapshot {
            status: lightcal::platform::ProviderStatus::Available,
            sources: Vec::new(),
            occurrences: Vec::new(),
        }
    };

    let startup_mgr = StartupManager::from_env(store.dir());
    let mut shortcut_mgr = ShortcutManager::new();
    let mut startup_shortcut_error = String::new();
    let mut startup_conflicting_shortcut = None;
    if let Some(binding) = prefs.global_shortcut.as_deref() {
        if let Err(error) = shortcut_mgr.bind(Some(binding)) {
            startup_shortcut_error = error;
            startup_conflicting_shortcut = Some(binding.to_string());
        }
    }

    let ui = AppWindow::new().expect("Failed to create Lightcal window");
    apply_system_font(&ui);

    ACTIVE_UI.with(|u| {
        *u.borrow_mut() = Some(ui.as_weak());
    });

    let ui_weak_shortcut = ui.as_weak();
    shortcut_mgr.listen_for_trigger(move || {
        let ui_w = ui_weak_shortcut.clone();
        let _ = slint::invoke_from_event_loop(move || {
            with_active_controller(&ui_w, |c, w| {
                if !c.settings_open && application_window_is_open(w) {
                    hide_application_window(w);
                } else {
                    c.settings_open = false;
                    c.sync_to_ui(w);
                    show_application_window(w);
                    bring_to_foreground();
                }
            });
        });
    });
    #[cfg(windows)]
    let (
        tray_icon,
        tray_menu,
        open_item_id,
        today_item_id,
        settings_item_id,
        sponsor_item_id,
        quit_item_id,
    ) = {
        let now = chrono::Local::now();
        let dark_mode = resolve_dark_mode(prefs.theme);
        let (rgba, icon_size) = render_native_tray_icon(
            prefs.tray_icon_mode,
            prefs.tray_icon_monochrome,
            today.day,
            today.weekday(),
            (now.hour() as u8, now.minute() as u8),
            dark_mode,
        );
        let menu = muda::Menu::new();
        let labels = lightcal::i18n::tray_menu_copy(prefs.locale());
        let open_item = muda::MenuItem::new(labels.open, true, None);
        let today_item = muda::MenuItem::new(
            lightcal::i18n::catalog(prefs.locale()).today_button,
            true,
            None,
        );
        let settings_item = muda::MenuItem::new(labels.settings, true, None);
        let sponsor_item = muda::MenuItem::new(labels.sponsor, true, None);
        let quit_item = muda::MenuItem::new(labels.exit, true, None);
        let open_id = open_item.id().clone();
        let today_id = today_item.id().clone();
        let settings_id = settings_item.id().clone();
        let sponsor_id = sponsor_item.id().clone();
        let quit_id = quit_item.id().clone();
        let _ = menu.append(&open_item);
        let _ = menu.append(&today_item);
        let _ = menu.append(&settings_item);
        let _ = menu.append(&sponsor_item);
        let _ = menu.append(&muda::PredefinedMenuItem::separator());
        let _ = menu.append(&quit_item);
        let initial_tip = format!(
            "Lightcal · {}",
            format_date_line(prefs.locale(), today, true)
        );
        let tray = tray_icon::Icon::from_rgba(rgba, icon_size, icon_size)
            .ok()
            .and_then(|icon| {
                tray_icon::TrayIconBuilder::new()
                    .with_menu(Box::new(menu))
                    // Left clicks toggle the calendar; only right clicks open this menu.
                    .with_menu_on_left_click(false)
                    .with_tooltip(&initial_tip)
                    .with_icon(icon)
                    .build()
                    .ok()
            });
        let items = TrayMenuItems {
            open: open_item.clone(),
            today: today_item.clone(),
            settings: settings_item.clone(),
            sponsor: sponsor_item.clone(),
            exit: quit_item.clone(),
            locale: std::cell::Cell::new(prefs.locale()),
        };
        (
            tray,
            items,
            open_id,
            today_id,
            settings_id,
            sponsor_id,
            quit_id,
        )
    };

    let ctrl = Rc::new(RefCell::new(AppController {
        store,
        prefs,
        cal_state,
        provider,
        provider_request: 0,
        connection_busy: false,
        connection_message: String::new(),
        startup_warning: String::new(),
        microsoft_account: lightcal::calendars::account::MicrosoftAccountState::Checking,
        provider_loading: false,
        account_before_check: None,
        install_busy: false,
        install_status: String::new(),
        startup_mgr,
        shortcut_mgr,
        shortcut_error: startup_shortcut_error,
        startup_error: String::new(),
        conflicting_shortcut: startup_conflicting_shortcut,
        update_state: UpdateCheckState::Idle,
        settings_open: false,
        settings_tab: 0,
        #[cfg(windows)]
        tray_icon,
        #[cfg(windows)]
        tray_menu,
        language_dropdown_open: false,
        theme_dropdown_open: false,
        first_day_dropdown_open: false,
        update_channel_dropdown_open: false,
        calendars_access_denied: false,
        calendars_is_empty: false,
        add_account_open: false,
        ics_dialog_open: false,
        sign_in_cancelled: false,
        active_sign_in: ActiveSignIn::None,
        held_microsoft: lightcal::calendars::account::MicrosoftAccountState::Disconnected,
        held_google_connected: false,
        held_google_needs_signin: false,
        held_google_label: String::new(),
        held_google_error: String::new(),
        google_connected: false,
        google_needs_signin: false,
        google_label: String::new(),
        google_error: String::new(),
        ics_error: String::new(),
        agenda: Default::default(),
        last_refreshed: None,
    }));
    if let Some(warning) = loaded_warning {
        ctrl.borrow_mut().startup_warning = warning;
    }

    ACTIVE_CONTROLLER.with(|c| {
        *c.borrow_mut() = Some(Rc::downgrade(&ctrl));
    });

    ctrl.borrow().sync_to_ui(&ui);

    // If starting in tray, hide the window initially; otherwise show it.
    if !start_in_tray {
        show_application_window(&ui);
    }

    // Load real WinRT calendar data off the UI thread so startup is never blocked
    if !use_synthetic {
        request_visible_calendars(ui.as_weak(), false);
    }

    // Explicit opt-in: check at startup and once per day, never install updates.
    if ctrl.borrow().prefs.auto_check_updates && !use_synthetic {
        request_update_check(ui.as_weak());
    }
    let update_timer = slint::Timer::default();
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        update_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_secs(24 * 60 * 60),
            move || {
                if ctrl.borrow().prefs.auto_check_updates && !use_synthetic {
                    request_update_check(ui_weak.clone());
                }
            },
        );
    }
    let calendar_timer = slint::Timer::default();
    {
        let weak = ui.as_weak();
        calendar_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_secs(15 * 60),
            move || request_visible_calendars(weak.clone(), use_synthetic),
        );
    }

    // Ensure post-show DPI scale factor and bottom-right taskbar anchoring are applied cleanly
    let startup_position_timer = slint::Timer::default();
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        let is_visible = !start_in_tray;
        startup_position_timer.start(
            slint::TimerMode::SingleShot,
            std::time::Duration::from_millis(25),
            move || {
                if is_visible {
                    if let Some(w) = ui_weak.upgrade() {
                        let c = ctrl.borrow();
                        if application_window_is_open(&w) {
                            sync_window_geometry(&w, c.settings_open);
                        }
                        #[cfg(windows)]
                        ensure_window_hooks(&w, lightcal::platform::windows::find_lightcal_hwnd());
                    }
                }
            },
        );
    }

    #[cfg(windows)]
    {
        // 1. Single-instance activation listener sleeping on Win32 kernel event (0.0% CPU)
        let ui_weak_act = ui.as_weak();
        _single_instance.listen_for_activation(move || {
            let ui_w = ui_weak_act.clone();
            let _ = slint::invoke_from_event_loop(move || {
                with_active_controller(&ui_w, |c, w| {
                    c.settings_open = false;
                    c.sync_to_ui(w);
                    show_application_window(w);
                    bring_to_foreground();
                });
            });
        });

        // 2. Event-driven system tray clicks (dispatches directly on mouse events, no busy polling)
        let ui_weak_tray = ui.as_weak();
        tray_icon::TrayIconEvent::set_event_handler(Some(move |ev| {
            if let tray_icon::TrayIconEvent::Click {
                button: tray_icon::MouseButton::Left,
                button_state: tray_icon::MouseButtonState::Up,
                ..
            } = ev
            {
                let ui_w = ui_weak_tray.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    with_active_controller(&ui_w, |c, w| {
                        if !c.settings_open && application_window_is_open(w) {
                            hide_application_window(w);
                        } else {
                            c.settings_open = false;
                            c.sync_to_ui(w);
                            show_application_window(w);
                            bring_to_foreground();
                        }
                    });
                });
            }
        }));

        // 3. Event-driven right-click tray context menu actions
        let ui_weak_menu = ui.as_weak();
        let open_id = open_item_id.clone();
        let today_id = today_item_id.clone();
        let settings_id = settings_item_id.clone();
        let sponsor_id = sponsor_item_id.clone();
        let quit_id = quit_item_id.clone();

        muda::MenuEvent::set_event_handler(Some(move |ev: muda::MenuEvent| {
            let id = ev.id;
            let ui_w = ui_weak_menu.clone();
            let open_id = open_id.clone();
            let today_id = today_id.clone();
            let settings_id = settings_id.clone();
            let sponsor_id = sponsor_id.clone();
            let quit_id = quit_id.clone();

            let _ = slint::invoke_from_event_loop(move || {
                if id == open_id {
                    with_active_controller(&ui_w, |c, w| {
                        c.sync_to_ui(w);
                        show_application_window(w);
                        bring_to_foreground();
                    });
                } else if id == today_id {
                    with_active_controller(&ui_w, |c, w| {
                        c.cal_state.update_today(current_local_date());
                        c.cal_state.return_to_today();
                        c.settings_open = false;
                        c.sync_to_ui(w);
                        show_application_window(w);
                        bring_to_foreground();
                    });
                    request_visible_calendars(ui_w.clone(), use_synthetic);
                } else if id == settings_id {
                    with_active_controller(&ui_w, |c, w| {
                        c.settings_open = true;
                        c.settings_tab = 2; // App tab (language, startup, shortcut)
                        c.sync_to_ui(w);
                        show_application_window(w);
                        bring_to_foreground();
                    });
                } else if id == sponsor_id {
                    // Opens only the allowlisted GitHub Sponsors page in the browser.
                    open_browser_link(lightcal::updater::SPONSOR_URL);
                } else if id == quit_id {
                    let _ = slint::quit_event_loop();
                }
            });
        }));
    }

    #[cfg(windows)]
    let mut last_today = today;
    #[cfg(windows)]
    let mut last_minute = chrono::Local::now().timestamp() / 60;

    // 4. Low-frequency 1-second timer for local midnight rollover and minute-based tray clock updates
    #[cfg(windows)]
    let clock_rollover_timer = slint::Timer::default();
    #[cfg(windows)]
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        clock_rollover_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(1000),
            move || {
                let now = chrono::Local::now();
                let now_date = current_local_date();
                let date_changed = now_date != last_today;
                let mut refresh_tray = false;

                if now_date != last_today {
                    last_today = now_date;
                    let mut c = ctrl.borrow_mut();
                    c.cal_state.update_today(now_date);
                    refresh_tray = true;
                }

                if now.timestamp() / 60 != last_minute {
                    last_minute = now.timestamp() / 60;
                    refresh_tray = true;
                }

                if refresh_tray {
                    if let Some(w) = ui_weak.upgrade() {
                        let c = ctrl.borrow();
                        c.sync_to_ui(&w);
                    }
                }
                if date_changed {
                    request_visible_calendars(ui_weak.clone(), use_synthetic);
                }
            },
        );
    }

    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_prev_month(move || {
            let mut c = ctrl.borrow_mut();
            c.cal_state.prev_month();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
            drop(c);
            request_visible_calendars(ui_weak.clone(), use_synthetic);
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_next_month(move || {
            let mut c = ctrl.borrow_mut();
            c.cal_state.next_month();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
            drop(c);
            request_visible_calendars(ui_weak.clone(), use_synthetic);
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_return_to_today(move || {
            let mut c = ctrl.borrow_mut();
            c.cal_state.update_today(current_local_date());
            c.cal_state.return_to_today();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
            drop(c);
            request_visible_calendars(ui_weak.clone(), use_synthetic);
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_select_cell(move |idx| {
            let mut c = ctrl.borrow_mut();
            let previous_month = c.cal_state.displayed_month;
            let grid = c
                .cal_state
                .build_grid(&c.provider.occurrences, &c.prefs.selected_calendar_ids);
            if let Some(cell) = grid.cells.get(idx as usize) {
                c.cal_state.select_date(cell.date);
            }
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
            let changed_month = previous_month != c.cal_state.displayed_month;
            drop(c);
            if changed_month {
                request_visible_calendars(ui_weak.clone(), use_synthetic);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_move_selection_by(move |delta| {
            let mut c = ctrl.borrow_mut();
            let previous_month = c.cal_state.displayed_month;
            c.cal_state.move_selection_by_days(delta);
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
            let changed_month = previous_month != c.cal_state.displayed_month;
            drop(c);
            if changed_month {
                request_visible_calendars(ui_weak.clone(), use_synthetic);
            }
        });
    }
    calendar_view::connect_agenda_callbacks(&ui, &ctrl, move |weak| {
        request_visible_calendars(weak, use_synthetic)
    });

    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_open_settings(move |tab| {
            let mut c = ctrl.borrow_mut();
            c.settings_open = true;
            c.settings_tab = tab;
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
                sync_window_geometry(&w, true);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_close_settings(move || {
            let mut c = ctrl.borrow_mut();
            c.settings_open = false;
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
                sync_window_geometry(&w, false);
                w.invoke_focus_calendar_keys();
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_select_language(move |code| {
            let mut c = ctrl.borrow_mut();
            let loc = LocaleCode::from_code_or_default(code.as_str());
            c.prefs.set_locale(loc);
            c.language_dropdown_open = false;
            c.persist();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_select_theme(move |mode| {
            let mut c = ctrl.borrow_mut();
            c.prefs.theme = ThemePreference::from_str_lossy(mode.as_str());
            c.theme_dropdown_open = false;
            c.persist();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_select_tray_icon_mode(move |mode| {
            let mut c = ctrl.borrow_mut();
            c.prefs.tray_icon_mode = TrayIconMode::from_str_lossy(mode.as_str());
            c.persist();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_toggle_tray_icon_monochrome(move || {
            let mut c = ctrl.borrow_mut();
            c.prefs.tray_icon_monochrome = !c.prefs.tray_icon_monochrome;
            c.persist();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_toggle_first_weekday(move || {
            let mut c = ctrl.borrow_mut();
            let next = if c.prefs.first_weekday == Weekday::Monday {
                Weekday::Sunday
            } else {
                Weekday::Monday
            };
            c.prefs.first_weekday = next;
            c.cal_state.first_weekday = next;
            c.persist();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_toggle_gray_weekday(move |idx| {
            let mut c = ctrl.borrow_mut();
            let day = Weekday::from_monday_index(idx as u8);
            let mut set = c.prefs.gray_weekdays();
            set.toggle(day);
            c.prefs.set_gray_weekdays(set);
            c.cal_state.gray_weekdays = set;
            c.persist();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_toggle_show_week_numbers(move || {
            let mut c = ctrl.borrow_mut();
            c.prefs.show_week_numbers = !c.prefs.show_week_numbers;
            c.persist();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_toggle_show_weekday(move || {
            let mut c = ctrl.borrow_mut();
            c.prefs.show_weekday = !c.prefs.show_weekday;
            c.persist();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_toggle_start_with_windows(move || {
            let mut c = ctrl.borrow_mut();
            let desired = !c.prefs.start_with_windows;
            let exe = env::current_exe().unwrap_or_default();
            match c.startup_mgr.set_enabled(desired, &exe) {
                Ok(()) => {
                    c.prefs.start_with_windows = desired;
                    c.startup_error.clear();
                    c.persist();
                }
                Err(err) => {
                    let prefix = lightcal::i18n::startup_copy(c.prefs.locale()).error;
                    c.startup_error = if err.is_empty() {
                        prefix.to_string()
                    } else {
                        format!("{prefix} {err}")
                    };
                }
            }
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_record_shortcut(move |text, control, alt, shift, meta| {
            let mut c = ctrl.borrow_mut();
            #[cfg(windows)]
            let pressed = pressed_main_key();
            #[cfg(not(windows))]
            let pressed: Option<String> = None;
            if let Some(candidate) =
                shortcut_from_press(pressed.as_deref(), text.as_str(), control, alt, shift, meta)
            {
                apply_shortcut(&mut c, &candidate);
            }
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        ui.on_reopen_sign_in_browser(move || {
            if let Some(url) = LAST_SIGN_IN_URL.with(|slot| slot.borrow().clone()) {
                open_browser_link(&url);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_clear_shortcut(move || {
            let mut c = ctrl.borrow_mut();
            let _ = c.shortcut_mgr.bind(None);
            c.prefs.global_shortcut = None;
            c.shortcut_error.clear();
            c.conflicting_shortcut = None;
            c.persist();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_toggle_calendar_source(move |id| {
            let mut c = ctrl.borrow_mut();
            let key = id.to_string();
            if !c.prefs.selected_calendar_ids.remove(&key) {
                c.prefs.selected_calendar_ids.insert(key);
            }
            c.persist();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
            drop(c);
            request_visible_calendars(ui_weak.clone(), use_synthetic);
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let weak = ui.as_weak();
        ui.on_connect_microsoft(move || {
            let client = ctrl.borrow().prefs.microsoft_client_id.clone();
            if !lightcal::calendars::microsoft::valid_client_id(&client) {
                let mut controller = ctrl.borrow_mut();
                controller.connection_message =
                    "Outlook sign-in is not configured in this build".into();
                if let Some(window) = weak.upgrade() {
                    controller.sync_to_ui(&window);
                }
                return;
            }
            run_calendar_action(
                weak.clone(),
                CalendarAction::ConnectMicrosoft(client),
                use_synthetic,
            );
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_disconnect_microsoft(move || {
            run_calendar_action(
                weak.clone(),
                CalendarAction::DisconnectMicrosoft,
                use_synthetic,
            )
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_add_ics_source(move |name, address| {
            run_calendar_action(
                weak.clone(),
                CalendarAction::AddIcs(
                    if name.trim().is_empty() {
                        ics_name_from_address(address.as_str())
                    } else {
                        name.to_string()
                    },
                    address.to_string(),
                ),
                use_synthetic,
            )
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_remove_calendar_source(move |id| {
            // Legacy Windows store sources have no per-calendar delete in the dense UI.
            // Treat remove as turning the preference off so users are never stuck.
            if id.as_str().starts_with("win:") {
                with_active_controller(&weak, |c, w| {
                    c.prefs.windows_calendars_enabled = false;
                    c.persist();
                    c.sync_to_ui(w);
                });
                request_visible_calendars(weak.clone(), use_synthetic);
                return;
            }
            run_calendar_action(
                weak.clone(),
                CalendarAction::Remove(id.to_string()),
                use_synthetic,
            )
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let weak = ui.as_weak();
        ui.on_toggle_legacy_windows(move || {
            {
                let mut c = ctrl.borrow_mut();
                c.prefs.windows_calendars_enabled = !c.prefs.windows_calendars_enabled;
                c.persist();
                if let Some(w) = weak.upgrade() {
                    c.sync_to_ui(&w);
                }
            }
            request_visible_calendars(weak.clone(), use_synthetic);
        });
    }
    {
        let ui_weak = ui.as_weak();
        ui.on_refresh_calendars(move || {
            request_visible_calendars(ui_weak.clone(), use_synthetic);
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_install_update(move || {
            if lightcal::installer::automatic_install_configured() {
                request_install(weak.clone());
                return;
            }
            let url = ctrl_release_url(&weak);
            open_browser_link(&url);
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let weak = ui.as_weak();
        ui.on_toggle_auto_install_updates(move || {
            let mut c = ctrl.borrow_mut();
            let desired = !c.prefs.auto_install_updates;
            if desired && !lightcal::installer::automatic_install_configured() {
                c.install_status = catalog(c.prefs.locale()).manual_install_status.to_string();
                if let Some(w) = weak.upgrade() {
                    c.sync_to_ui(&w);
                }
                return;
            }
            c.prefs.auto_install_updates = desired;
            if desired {
                c.prefs.auto_check_updates = true;
            }
            c.persist();
            if let Some(w) = weak.upgrade() {
                c.sync_to_ui(&w);
            }
            drop(c);
            if desired {
                request_update_check(weak.clone());
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_check_updates(move || {
            let c = ctrl.borrow();
            if let UpdateCheckState::UpdateAvailable { release_url, .. } = &c.update_state {
                open_browser_link(release_url);
                return;
            }
            drop(c);
            request_update_check(ui_weak.clone());
        });
    }
    {
        ui.on_open_external_url(move |url| {
            open_browser_link(url.as_str());
        });
        ui.on_open_sponsor(|one_time| {
            open_browser_link(if one_time {
                lightcal::updater::SPONSOR_ONCE_URL
            } else {
                lightcal::updater::SPONSOR_URL
            });
        });
    }
    {
        let ui_weak = ui.as_weak();
        ui.on_dismiss_compact(move || {
            if let Some(w) = ui_weak.upgrade() {
                hide_application_window(&w);
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        ui.window().on_close_requested(move || {
            if let Some(w) = ui_weak.upgrade() {
                hide_application_window(&w);
            }
            slint::CloseRequestResponse::KeepWindowShown
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_toggle_language_dropdown(move || {
            let mut c = ctrl.borrow_mut();
            c.language_dropdown_open = !c.language_dropdown_open;
            c.theme_dropdown_open = false;
            c.first_day_dropdown_open = false;
            c.update_channel_dropdown_open = false;
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_toggle_theme_dropdown(move || {
            let mut c = ctrl.borrow_mut();
            c.theme_dropdown_open = !c.theme_dropdown_open;
            c.language_dropdown_open = false;
            c.first_day_dropdown_open = false;
            c.update_channel_dropdown_open = false;
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }

    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_toggle_first_day_dropdown(move || {
            let mut c = ctrl.borrow_mut();
            c.first_day_dropdown_open = !c.first_day_dropdown_open;
            c.language_dropdown_open = false;
            c.theme_dropdown_open = false;
            c.update_channel_dropdown_open = false;
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_toggle_update_channel_dropdown(move || {
            let mut c = ctrl.borrow_mut();
            c.update_channel_dropdown_open = !c.update_channel_dropdown_open;
            c.language_dropdown_open = false;
            c.theme_dropdown_open = false;
            c.first_day_dropdown_open = false;
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }

    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_select_first_weekday_option(move |monday| {
            let mut c = ctrl.borrow_mut();
            let day = if monday {
                Weekday::Monday
            } else {
                Weekday::Sunday
            };
            c.prefs.first_weekday = day;
            c.cal_state.first_weekday = day;
            c.first_day_dropdown_open = false;
            c.persist();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_select_update_channel(move |ch| {
            let mut c = ctrl.borrow_mut();
            c.prefs.update_channel = ch.to_ascii_lowercase().to_string();
            c.update_channel_dropdown_open = false;
            c.persist();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_toggle_highlight_current_month(move || {
            let mut c = ctrl.borrow_mut();
            c.prefs.highlight_current_month = !c.prefs.highlight_current_month;
            c.persist();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_toggle_show_event_dots(move || {
            let mut c = ctrl.borrow_mut();
            c.prefs.show_event_dots = !c.prefs.show_event_dots;
            c.persist();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_toggle_auto_check_updates(move || {
            let mut c = ctrl.borrow_mut();
            c.prefs.auto_check_updates = !c.prefs.auto_check_updates;
            c.persist();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
            let enabled = c.prefs.auto_check_updates;
            drop(c);
            if enabled && !use_synthetic {
                request_update_check(ui_weak.clone());
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_reset_shortcut(move || {
            let mut c = ctrl.borrow_mut();
            let _ = c.shortcut_mgr.bind(None);
            c.prefs.global_shortcut = None;
            c.shortcut_error.clear();
            c.conflicting_shortcut = None;
            c.persist();
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        ui.on_open_privacy_settings(move || {
            open_browser_link("ms-settings:privacy-calendar");
        });
    }
    {
        let ui_weak = ui.as_weak();
        ui.on_dismiss_startup_warning(move || {
            with_active_controller(&ui_weak, |ctrl, ui| {
                ctrl.startup_warning.clear();
                ctrl.sync_to_ui(ui);
            });
        });
    }
    {
        let ui_weak = ui.as_weak();
        ui.on_retry_calendar_sync(move || {
            request_visible_calendars(ui_weak.clone(), use_synthetic);
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_add_calendar_account(move || {
            with_active_controller(&weak, |c, w| {
                c.add_account_open = true;
                c.ics_dialog_open = false;
                c.sync_to_ui(w);
            });
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_close_account_dialogs(move || {
            with_active_controller(&weak, |c, w| {
                if c.ics_dialog_open && c.connection_busy {
                    return;
                }
                c.add_account_open = false;
                c.ics_dialog_open = false;
                c.sync_to_ui(w);
                w.invoke_focus_calendar_keys();
            });
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_choose_outlook_account(move || {
            if let Some(w) = weak.upgrade() {
                w.invoke_connect_microsoft();
            }
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let weak = ui.as_weak();
        ui.on_choose_google_account(move || {
            let client = ctrl.borrow().prefs.google_client_id.clone();
            let secret = ctrl.borrow().prefs.google_client_secret.clone();
            if !lightcal::calendars::google::valid_client_id(&client)
                || !lightcal::calendars::google::valid_client_secret(&secret)
            {
                with_active_controller(&weak, |c, w| {
                    c.add_account_open = false;
                    c.connection_message = if lightcal::calendars::google::valid_client_id(&client)
                    {
                        match c.prefs.locale() {
                            LocaleCode::Ru => {
                                "Секрет клиента Google не задан в этой сборке.".into()
                            }
                            LocaleCode::Es => {
                                "El secreto de cliente de Google no está en esta compilación."
                                    .into()
                            }
                            LocaleCode::Ja => {
                                "このビルドに Google のクライアント シークレットがありません。"
                                    .into()
                            }
                            LocaleCode::En => {
                                "The Google client secret is not set in this build.".into()
                            }
                        }
                    } else {
                        w.get_google_unavailable_text().to_string()
                    };
                    c.sync_to_ui(w);
                });
                return;
            }
            run_calendar_action(
                weak.clone(),
                CalendarAction::ConnectGoogle {
                    client_id: client,
                    client_secret: secret,
                },
                use_synthetic,
            );
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_disconnect_google(move || {
            run_calendar_action(
                weak.clone(),
                CalendarAction::DisconnectGoogle,
                use_synthetic,
            );
        });
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let weak = ui.as_weak();
        ui.on_retry_sign_in(move || {
            let kind = ctrl.borrow().active_sign_in;
            match kind {
                ActiveSignIn::Google => {
                    if let Some(window) = weak.upgrade() {
                        window.invoke_choose_google_account();
                    }
                }
                _ => {
                    if let Some(window) = weak.upgrade() {
                        window.invoke_connect_microsoft();
                    }
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_choose_ics_account(move || {
            with_active_controller(&weak, |c, w| {
                c.add_account_open = false;
                c.ics_dialog_open = true;
                c.connection_message.clear();
                c.sync_to_ui(w);
            });
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_cancel_account_sign_in(move || {
            match SIGN_IN_GATE.cancel() {
                Ok(true) => {}
                Ok(false) => return,
                Err(error) => {
                    with_active_controller(&weak, |c, w| {
                        c.connection_message = error;
                        c.sync_to_ui(w);
                    });
                    return;
                }
            }
            with_active_controller(&weak, |c, w| {
                c.connection_busy = false;
                c.sign_in_cancelled = true;
                forget_sign_in_url();
                match c.active_sign_in {
                    ActiveSignIn::Microsoft => {
                        c.microsoft_account = c.held_microsoft.clone();
                    }
                    ActiveSignIn::Google => {
                        c.google_connected = c.held_google_connected;
                        c.google_needs_signin = c.held_google_needs_signin;
                        c.google_label = c.held_google_label.clone();
                        c.google_error = c.held_google_error.clone();
                    }
                    ActiveSignIn::None => {}
                }
                c.connection_message.clear();
                c.sync_to_ui(w);
            });
            replay_pending_reload(weak.clone());
        });
    }

    slint::run_event_loop_until_quit().expect("Failed to run Slint event loop");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panicking_calendar_load_becomes_an_error_instead_of_unwinding() {
        let result = run_load_guarded(|| panic!("provider exploded"));
        assert!(result.is_err());
        let ok = run_load_guarded(|| lightcal::calendars::CalendarLoad {
            snapshot: ProviderSnapshot::synthetic(LocalDate::new(2026, 1, 1).unwrap()),
            microsoft: lightcal::calendars::account::MicrosoftAccountState::Disconnected,
            google_connected: false,
            google_needs_signin: false,
            google_configuration: None,
            google_label: None,
            google_error: String::new(),
            google_skipped: 0,
            ics_error: String::new(),
        });
        assert!(ok.is_ok());
    }

    #[test]
    fn reload_requested_while_busy_is_remembered_once_and_replayed() {
        assert!(!take_pending_reload());
        assert!(!start_or_defer_reload(true));
        assert!(!start_or_defer_reload(true));
        assert!(take_pending_reload());
        assert!(!take_pending_reload());
        assert!(!start_or_defer_reload(true));
        // Starting a load consumes the pending request.
        assert!(start_or_defer_reload(false));
        assert!(!take_pending_reload());
    }

    #[test]
    fn watchdog_expires_only_the_current_loading_request() {
        assert!(watchdog_should_expire(5, 5, true));
        assert!(!watchdog_should_expire(6, 5, true));
        assert!(!watchdog_should_expire(5, 5, false));
    }

    #[test]
    fn stale_result_never_releases_a_newer_requests_guard() {
        assert!(stale_result_releases_guard(4, 4));
        assert!(!stale_result_releases_guard(5, 4));
    }

    fn unavailable() -> lightcal::calendars::account::MicrosoftAccountState {
        lightcal::calendars::account::MicrosoftAccountState::Unavailable {
            saved_credentials: true,
        }
    }

    #[test]
    fn expiry_unlocks_restores_account_and_keeps_the_request_alive() {
        use lightcal::calendars::account::MicrosoftAccountState as S;
        let mut loading = true;
        let mut account = S::Checking;
        let mut before = Some(unavailable());
        let request = 7;
        assert!(expire_load(
            request,
            request,
            &mut loading,
            &mut account,
            &mut before
        ));
        assert!(!loading);
        assert_eq!(account, unavailable());
        assert!(before.is_none());
        // Late result after expiry: the request id is unchanged, so it is still applied.
        assert!(result_is_visible(request, request, true));
        // A second expiry for the same request is a no-op.
        assert!(!expire_load(
            request,
            request,
            &mut loading,
            &mut account,
            &mut before
        ));
    }

    #[test]
    fn late_result_is_dropped_when_a_newer_request_exists() {
        // Request 7 expired, then request 8 started: 7's late result is stale and must not
        // release request 8's guard.
        assert!(!result_is_visible(8, 7, true));
        assert!(!stale_result_releases_guard(8, 7));
        // Same request but the user moved to another month: dropped, guard released.
        assert!(!result_is_visible(7, 7, false));
        assert!(stale_result_releases_guard(7, 7));
    }

    #[test]
    fn restore_only_replaces_a_checking_account() {
        use lightcal::calendars::account::MicrosoftAccountState as S;
        let mut account = S::Checking;
        let mut before = Some(unavailable());
        restore_account_after_check(&mut account, &mut before);
        assert_eq!(account, unavailable());
        assert!(before.is_none());

        let mut account = S::Disconnected;
        let mut before = Some(unavailable());
        restore_account_after_check(&mut account, &mut before);
        assert_eq!(account, S::Disconnected);
        assert!(before.is_none());
    }

    #[test]
    fn panicking_worker_yields_an_error() {
        let result: Result<(), String> = run_guarded(|| panic!("worker exploded"));
        assert!(result.is_err());
        assert_eq!(run_guarded(|| 5).unwrap(), 5);
    }

    #[test]
    fn target_logical_size_matches_compact_and_settings_specs() {
        assert_eq!(target_logical_size(false), (352.0, 432.0));
        assert_eq!(target_logical_size(true), (448.0, 360.0));
    }

    #[test]
    fn work_area_anchors_all_window_sizes_above_taskbar_without_clipping() {
        let area = lightcal::platform::WorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1032,
        };
        for settings in [false, true] {
            let (lw, lh) = target_logical_size(settings);
            let pw = (lw as i32) + 16;
            let ph = (lh as i32) + 44;
            let (x, y) = area.anchor_above_tray(pw, ph, 12);
            assert!(x >= area.x + 8);
            assert!(y >= area.y + 8);
            assert!(x + pw <= area.x + area.width - 8);
            assert!(y + ph <= area.y + area.height - 8);
        }
    }
}
