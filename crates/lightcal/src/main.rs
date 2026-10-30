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
mod snapshot_render;
use snapshot_render::render_snapshots_to_dir;

use lightcal::updater::{is_allowed_external_link, UpdateCheckState};
use slint::{
    Color, ComponentHandle, Image, LogicalSize, ModelRc, PhysicalPosition, Rgba8Pixel,
    SharedPixelBuffer, SharedString, VecModel,
};
use std::cell::RefCell;
use std::env;
use std::path::Path;
#[cfg(target_os = "macos")]
use std::process::Command;
use std::rc::Rc;

slint::include_modules!();

pub(crate) struct AppController {
    pub(crate) store: PreferencesStore,
    pub(crate) prefs: Preferences,
    pub(crate) cal_state: CalendarState,
    pub(crate) provider: ProviderSnapshot,
    pub(crate) provider_request: u64,
    pub(crate) connection_busy: bool,
    pub(crate) connection_message: String,
    pub(crate) microsoft_account: lightcal::calendars::account::MicrosoftAccountState,
    pub(crate) provider_loading: bool,
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
    pub(crate) language_dropdown_open: bool,
    pub(crate) theme_dropdown_open: bool,
    pub(crate) first_day_dropdown_open: bool,
    pub(crate) update_channel_dropdown_open: bool,
    pub(crate) calendars_access_denied: bool,
    pub(crate) calendars_is_empty: bool,
}

fn current_local_date() -> LocalDate {
    let now = chrono::Local::now().date_naive();
    LocalDate::new(now.year(), now.month() as u8, now.day() as u8)
        .unwrap_or_else(|| LocalDate::new(2026, 10, 29).unwrap())
}

fn parse_hex_color(hex: &str) -> Color {
    let h = hex.trim().trim_start_matches('#');
    if h.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&h[0..2], 16),
            u8::from_str_radix(&h[2..4], 16),
            u8::from_str_radix(&h[4..6], 16),
        ) {
            return Color::from_rgb_u8(r, g, b);
        }
    }
    Color::from_rgb_u8(94, 106, 210)
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
        (780.0, 620.0)
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

        ui.set_dark_mode(dark_mode);
        ui.set_settings_open(self.settings_open);
        ui.set_settings_tab(self.settings_tab);
        if application_window_is_open(ui) {
            sync_window_geometry(ui, self.settings_open);
        }

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
            dark_mode,
        );
        let pixel_buf = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&rgba64, 64, 64);
        ui.set_window_icon(Image::from_rgba8(pixel_buf));

        let grid = self.cal_state.build_grid(
            &self.provider.occurrences,
            &self.prefs.selected_calendar_ids,
        );

        ui.set_month_title(format_month_year(locale, grid.displayed_month).into());
        ui.set_is_current_month(grid.is_current_month);
        ui.set_current_month_badge(strings.current_month_badge.into());
        ui.set_today_date_line(
            format_date_line(locale, self.cal_state.today, self.prefs.show_weekday).into(),
        );
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

        let gray_cols: Vec<bool> = grid.gray_columns.to_vec();
        ui.set_gray_columns(ModelRc::new(VecModel::from(gray_cols)));

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
                let d1 = c
                    .dot_colors
                    .first()
                    .map(|s| parse_hex_color(s))
                    .unwrap_or_else(|| Color::from_argb_u8(0, 0, 0, 0));
                let d2 = c
                    .dot_colors
                    .get(1)
                    .map(|s| parse_hex_color(s))
                    .unwrap_or_else(|| Color::from_argb_u8(0, 0, 0, 0));
                let d3 = c
                    .dot_colors
                    .get(2)
                    .map(|s| parse_hex_color(s))
                    .unwrap_or_else(|| Color::from_argb_u8(0, 0, 0, 0));
                let date_desc = format_date_line(locale, c.date, true);
                let ev_desc = format_event_count(locale, c.total_event_count);
                DayCellData {
                    day_number: c.date.day.to_string().into(),
                    in_month: c.in_displayed_month,
                    is_today: c.is_today && c.in_displayed_month,
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
        ui.set_cells(ModelRc::new(VecModel::from(cell_items)));

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

        let day_events = self.cal_state.events_for_date(
            self.cal_state.selected_date,
            &self.provider.occurrences,
            &self.prefs.selected_calendar_ids,
        );
        let max_agenda_rows = 2;
        let agenda_items: Vec<AgendaEventItem> = day_events
            .into_iter()
            .take(max_agenda_rows)
            .map(|occ| {
                let cal_name = self
                    .provider
                    .sources
                    .iter()
                    .find(|s| s.id == occ.source_id)
                    .map(|s| s.name.as_str())
                    .unwrap_or("Calendar");
                let time_display = if occ.time_label.is_empty() {
                    "All day".to_string()
                } else {
                    occ.time_label.clone()
                };
                AgendaEventItem {
                    time_label: time_display.into(),
                    title: if occ.title.is_empty() {
                        "Event".into()
                    } else {
                        occ.title.into()
                    },
                    calendar_name: cal_name.into(),
                    swatch: parse_hex_color(&occ.color_hex),
                }
            })
            .collect();
        ui.set_selected_day_events(ModelRc::new(VecModel::from(agenda_items)));

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
        ui.set_install_status_text(self.install_status.clone().into());
        ui.set_connection_busy(self.connection_busy);
        ui.set_connection_message(self.connection_message.clone().into());
        let account_idle = !self.connection_busy && !self.provider_loading;
        ui.set_provider_loading(self.provider_loading);
        ui.set_account_status_text(self.microsoft_account.status_text(locale).into());
        ui.set_account_connected(matches!(
            self.microsoft_account,
            lightcal::calendars::account::MicrosoftAccountState::Connected { .. }
        ));
        ui.set_can_connect_microsoft(
            account_idle
                && self.microsoft_account.can_connect()
                && lightcal::calendars::microsoft::valid_client_id(&self.prefs.microsoft_client_id),
        );
        ui.set_can_disconnect_microsoft(account_idle && self.microsoft_account.can_disconnect());
        ui.set_can_refresh_calendars(
            account_idle
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
        ui.set_app_version_line_text(
            format!(
                "Lightcal {} · Rust + Slint · {}",
                env!("CARGO_PKG_VERSION"),
                std::env::consts::ARCH
            )
            .into(),
        );
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
        ui.set_update_ready_notes_text(strings.update_ready_notes.into());
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
        let chips: Vec<WeekdayChipItem> = Weekday::ALL
            .iter()
            .enumerate()
            .map(|(idx, &day)| WeekdayChipItem {
                index: idx as i32,
                label: weekday_short(locale, day).into(),
                active: gray_set.contains(day),
            })
            .collect();
        ui.set_gray_weekday_chips(ModelRc::new(VecModel::from(chips)));

        ui.set_start_with_windows(self.prefs.start_with_windows);

        let cal_items: Vec<CalendarSourceItem> = self
            .provider
            .sources
            .iter()
            .map(|s| CalendarSourceItem {
                id: s.id.clone().into(),
                name: s.name.clone().into(),
                account: s.account_label.clone().into(),
                swatch: parse_hex_color(&s.color_hex),
                checked: self.prefs.selected_calendar_ids.contains(&s.id),
                removable: s.id.starts_with("ics:"),
            })
            .collect();
        ui.set_calendar_sources(ModelRc::new(VecModel::from(cal_items)));

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
        invalidate_taskbar_preview();
    }

    fn persist(&self) {
        let _ = self.store.save(&self.prefs);
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
        }
        ensure_taskbar_preview(ui);
    }
    sync_window_geometry(ui, ui.get_settings_open());
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

#[cfg(windows)]
fn ensure_taskbar_preview(ui: &AppWindow) {
    TASKBAR_PREVIEW.with(|slot| {
        if slot.borrow().is_some() {
            return;
        }
        let weak = ui.as_weak();
        if let Ok(preview) =
            lightcal::platform::windows::taskbar::TaskbarPreview::install(move || {
                let ui = weak.upgrade()?;
                if ui.get_settings_open() {
                    return None;
                }
                ui.window().take_snapshot().ok()
            })
        {
            *slot.borrow_mut() = Some(preview);
        }
    });
}

#[cfg(windows)]
fn invalidate_taskbar_preview() {
    TASKBAR_PREVIEW.with(|slot| {
        if let Some(preview) = slot.borrow().as_ref() {
            preview.invalidate();
        }
    });
}

fn open_browser_link(url: &str) {
    if is_allowed_external_link(url) {
        #[cfg(any(windows, target_os = "macos"))]
        open_allowed_browser_link(url);
    }
}

#[cfg(any(windows, target_os = "macos"))]
fn open_allowed_browser_link(url: &str) {
    #[cfg(windows)]
    {
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
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(url).spawn();
    }
}

thread_local! {
    #[cfg(windows)]
    static TASKBAR_PREVIEW: RefCell<Option<lightcal::platform::windows::taskbar::TaskbarPreview>> = const { RefCell::new(None) };
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

fn apply_loaded_provider(snapshot: ProviderSnapshot, ui_weak: slint::Weak<AppWindow>) {
    with_active_controller(&ui_weak, |ctrl, ui| {
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
        ctrl.provider = snapshot;
        ctrl.sync_to_ui(ui);
    });
}

fn request_update_check(ui_weak: slint::Weak<AppWindow>) {
    if lightcal::installer::package_managed() {
        return;
    }
    let mut start = false;
    with_active_controller(&ui_weak, |ctrl, ui| {
        if ctrl.update_state != UpdateCheckState::Checking {
            ctrl.update_state = UpdateCheckState::Checking;
            ctrl.sync_to_ui(ui);
            start = true;
        }
    });
    if start {
        std::thread::spawn(move || {
            let state = lightcal::updater::check_latest_release(env!("CARGO_PKG_VERSION"));
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
    with_active_controller(&ui_weak, |ctrl, ui| {
        if !ctrl.install_busy {
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
        let result = lightcal::installer::stage(&directory)
            .and_then(|path| lightcal::installer::launch_helper(&path));
        let _ = slint::invoke_from_event_loop(move || match result {
            Ok(()) => {
                let _ = slint::quit_event_loop();
            }
            Err(error) => with_active_controller(&ui_weak, |ctrl, ui| {
                ctrl.install_busy = false;
                ctrl.install_status = error;
                ctrl.sync_to_ui(ui);
            }),
        });
    });
}

fn request_visible_calendars(ui_weak: slint::Weak<AppWindow>, use_synthetic: bool) {
    let mut anchor = None;
    let mut request_id = 0;
    let mut config = None;
    with_active_controller(&ui_weak, |ctrl, ui| {
        if ctrl.connection_busy {
            return;
        }
        ctrl.provider_request += 1;
        request_id = ctrl.provider_request;
        let month = ctrl.cal_state.displayed_month;
        anchor = LocalDate::new(month.year, month.month, 1);
        config = Some((ctrl.store.dir().to_path_buf(), ctrl.prefs.clone()));
        ctrl.provider_loading = true;
        if ctrl.microsoft_account
            != lightcal::calendars::account::MicrosoftAccountState::Disconnected
        {
            ctrl.microsoft_account = lightcal::calendars::account::MicrosoftAccountState::Checking;
        }
        ctrl.sync_to_ui(ui);
    });
    if let (Some(anchor), Some((dir, prefs))) = (anchor, config) {
        std::thread::spawn(move || {
            let data = if use_synthetic {
                lightcal::calendars::CalendarLoad {
                    snapshot: ProviderSnapshot::synthetic(anchor),
                    microsoft: lightcal::calendars::account::MicrosoftAccountState::Disconnected,
                }
            } else {
                lightcal::calendars::query(
                    anchor,
                    &dir,
                    &prefs.microsoft_client_id,
                    &prefs.selected_calendar_ids,
                    prefs.windows_calendars_enabled,
                )
            };
            let _ = slint::invoke_from_event_loop(move || {
                let mut still_visible = false;
                with_active_controller(&ui_weak, |ctrl, _| {
                    still_visible = ctrl.provider_request == request_id
                        && ctrl.cal_state.displayed_month == anchor.year_month();
                });
                if still_visible {
                    with_active_controller(&ui_weak, |ctrl, _| {
                        ctrl.microsoft_account = data.microsoft;
                        ctrl.provider_loading = false;
                    });
                    apply_loaded_provider(data.snapshot, ui_weak);
                }
            });
        });
    }
}

enum CalendarAction {
    ConnectMicrosoft(String),
    DisconnectMicrosoft,
    AddIcs(String, String),
    Remove(String),
}

fn run_calendar_action(ui_weak: slint::Weak<AppWindow>, action: CalendarAction, synthetic: bool) {
    let mut config = None;
    with_active_controller(&ui_weak, |ctrl, ui| {
        let allowed = match &action {
            CalendarAction::ConnectMicrosoft(_) => ctrl.microsoft_account.can_connect(),
            CalendarAction::DisconnectMicrosoft => ctrl.microsoft_account.can_disconnect(),
            CalendarAction::Remove(id) if id.starts_with("graph:") => {
                ctrl.microsoft_account.can_disconnect()
            }
            _ => true,
        };
        if !ctrl.connection_busy && !ctrl.provider_loading && allowed {
            ctrl.connection_busy = true;
            ctrl.connection_message.clear();
            ctrl.provider_request += 1;
            config = Some((
                ctrl.store.dir().to_path_buf(),
                ctrl.cal_state.today,
                ctrl.microsoft_account.clone(),
            ));
            if matches!(action, CalendarAction::ConnectMicrosoft(_)) {
                ctrl.microsoft_account =
                    lightcal::calendars::account::MicrosoftAccountState::Connecting;
            }
            ctrl.sync_to_ui(ui);
        }
    });
    let Some((dir, anchor, previous_account)) = config else {
        return;
    };
    std::thread::spawn(move || {
        let operation = || -> Result<Option<String>, String> {
            // Browser consent must not hold the credential lock for its whole lifetime.
            let refresh = if let CalendarAction::ConnectMicrosoft(client) = &action {
                Some(lightcal::calendars::microsoft::connect(client, |url| {
                    let _ = slint::invoke_from_event_loop(move || open_browser_link(&url));
                })?)
            } else {
                None
            };
            let _guard = lightcal::secure_store::credential_lock()?;
            let store = lightcal::secure_store::SecureStore::new(&dir);
            let mut credentials = store.load()?;
            let mut removed = None;
            match &action {
                CalendarAction::ConnectMicrosoft(client) => {
                    credentials.microsoft_refresh_token = refresh;
                    credentials.microsoft_client_id = Some(client.clone());
                }
                CalendarAction::DisconnectMicrosoft => {
                    credentials.microsoft_refresh_token = None;
                    credentials.microsoft_client_id = None;
                    removed = Some("graph:".to_string());
                }
                CalendarAction::AddIcs(name, address) => {
                    let id = lightcal::calendars::add_feed(&mut credentials, name, address)?;
                    let feed = credentials
                        .ics
                        .iter()
                        .find(|f| f.id == id)
                        .ok_or("Could not prepare ICS subscription")?;
                    lightcal::calendars::ics::fetch(feed, anchor)?;
                }
                CalendarAction::Remove(id) => {
                    if id.starts_with("graph:") {
                        credentials.microsoft_refresh_token = None;
                        credentials.microsoft_client_id = None;
                        removed = Some("graph:".to_string());
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
        let result = operation();
        let added_ics = matches!(action, CalendarAction::AddIcs(..));
        let connected_client = if let CalendarAction::ConnectMicrosoft(client) = action {
            Some(client)
        } else {
            None
        };
        let _ = slint::invoke_from_event_loop(move || {
            let success = result.is_ok();
            with_active_controller(&ui_weak, |ctrl, ui| {
                ctrl.connection_busy = false;
                ctrl.connection_message = match result {
                    Ok(removed) => {
                        if let Some(client) = connected_client {
                            ctrl.prefs.microsoft_client_id = client;
                            ctrl.persist();
                            ctrl.microsoft_account =
                                lightcal::calendars::account::MicrosoftAccountState::Checking;
                        }
                        if let Some(prefix) = removed {
                            if prefix == "graph:" {
                                ctrl.microsoft_account=lightcal::calendars::account::MicrosoftAccountState::Disconnected;
                            }
                            ctrl.prefs
                                .selected_calendar_ids
                                .retain(|id| !id.starts_with(&prefix));
                            ctrl.persist();
                        }
                        if added_ics {
                            ui.set_ics_source_url("".into());
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
    let mut prefs = loaded.preferences;
    if prefs.microsoft_client_id.is_empty() {
        prefs.microsoft_client_id = option_env!("LIGHTCAL_MICROSOFT_CLIENT_ID")
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
    if let Some(binding) = prefs.global_shortcut.as_deref() {
        let _ = shortcut_mgr.bind(Some(binding));
    }

    let ui = AppWindow::new().expect("Failed to create Lightcal window");

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
    let (tray_icon, open_item_id, today_item_id, settings_item_id, quit_item_id) = {
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
        let open_item = muda::MenuItem::new("Open Lightcal", true, None);
        let today_item = muda::MenuItem::new("Today", true, None);
        let settings_item = muda::MenuItem::new("Settings…", true, None);
        let quit_item = muda::MenuItem::new("Exit", true, None);
        let open_id = open_item.id().clone();
        let today_id = today_item.id().clone();
        let settings_id = settings_item.id().clone();
        let quit_id = quit_item.id().clone();
        let _ = menu.append(&open_item);
        let _ = menu.append(&today_item);
        let _ = menu.append(&settings_item);
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
        (tray, open_id, today_id, settings_id, quit_id)
    };

    let ctrl = Rc::new(RefCell::new(AppController {
        store,
        prefs,
        cal_state,
        provider,
        provider_request: 0,
        connection_busy: false,
        connection_message: String::new(),
        microsoft_account: lightcal::calendars::account::MicrosoftAccountState::Checking,
        provider_loading: false,
        install_busy: false,
        install_status: String::new(),
        startup_mgr,
        shortcut_mgr,
        shortcut_error: String::new(),
        startup_error: String::new(),
        conflicting_shortcut: None,
        update_state: UpdateCheckState::Idle,
        settings_open: false,
        settings_tab: 0,
        #[cfg(windows)]
        tray_icon,
        language_dropdown_open: false,
        theme_dropdown_open: false,
        first_day_dropdown_open: false,
        update_channel_dropdown_open: false,
        calendars_access_denied: false,
        calendars_is_empty: false,
    }));

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
                        ensure_taskbar_preview(&w);
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
        let quit_id = quit_item_id.clone();

        muda::MenuEvent::set_event_handler(Some(move |ev: muda::MenuEvent| {
            let id = ev.id;
            let ui_w = ui_weak_menu.clone();
            let open_id = open_id.clone();
            let today_id = today_id.clone();
            let settings_id = settings_id.clone();
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
                        c.settings_tab = 0;
                        c.sync_to_ui(w);
                        show_application_window(w);
                        bring_to_foreground();
                    });
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

    {
        let ctrl = Rc::clone(&ctrl);
        let ui_weak = ui.as_weak();
        ui.on_open_settings(move |tab| {
            let mut c = ctrl.borrow_mut();
            c.settings_open = true;
            c.settings_tab = tab;
            if let Some(w) = ui_weak.upgrade() {
                c.sync_to_ui(&w);
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
        ui.on_cycle_shortcut_preset(move || {
            let mut c = ctrl.borrow_mut();
            let candidate = match c.prefs.global_shortcut.as_deref() {
                None => "Alt + Shift + C",
                Some("Alt + Shift + C") => "Ctrl + Alt + C", // Demonstrates conflict rollback!
                _ => "Ctrl + Shift + L",
            };
            match c.shortcut_mgr.bind(Some(candidate)) {
                Ok(bound) => {
                    c.prefs.global_shortcut = bound;
                    c.shortcut_error.clear();
                    c.conflicting_shortcut = None;
                    c.persist();
                }
                Err(err) => {
                    c.shortcut_error = err;
                    c.conflicting_shortcut = Some(candidate.to_string());
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
                CalendarAction::AddIcs(name.to_string(), address.to_string()),
                use_synthetic,
            )
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_remove_calendar_source(move |id| {
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
        ui.on_install_update(move || request_install(weak.clone()));
    }
    {
        let ctrl = Rc::clone(&ctrl);
        let weak = ui.as_weak();
        ui.on_toggle_auto_install_updates(move || {
            let mut c = ctrl.borrow_mut();
            let desired = !c.prefs.auto_install_updates;
            if desired {
                if let Err(error) = lightcal::installer::certificate_pin() {
                    c.install_status = error;
                    if let Some(w) = weak.upgrade() {
                        c.sync_to_ui(&w);
                    }
                    return;
                }
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
            #[cfg(target_os = "macos")]
            open_browser_link(lightcal::updater::MACOS_CALENDAR_PRIVACY_URL);
            #[cfg(not(target_os = "macos"))]
            open_browser_link("ms-settings:privacy-calendar");
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
            if let Some(w) = weak.upgrade() {
                w.invoke_connect_microsoft();
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        ui.on_minimize_window(move || {
            if let Some(w) = ui_weak.upgrade() {
                hide_application_window(&w);
            }
        });
    }

    slint::run_event_loop_until_quit().expect("Failed to run Slint event loop");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_logical_size_matches_compact_and_settings_specs() {
        assert_eq!(target_logical_size(false), (352.0, 432.0));
        assert_eq!(target_logical_size(true), (780.0, 620.0));
        assert_eq!(target_logical_size(true), (780.0, 620.0));
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
