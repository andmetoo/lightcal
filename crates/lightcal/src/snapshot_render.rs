//! Headless UI snapshot rendering pipeline for design system and visual regression validation.

use crate::{AppController, AppWindow};
use calendar_core::{CalendarOccurrence, CalendarState, LocalDate, Weekday};
use lightcal::platform::windows::{render_squircle_icon_rgba_64, ShortcutManager, StartupManager};
use lightcal::platform::ProviderSnapshot;
use lightcal::preferences::{
    DateSize, LocaleCode, Preferences, PreferencesStore, ThemePreference, TrayIconMode,
};
use lightcal::snapshot::{write_rgba_png, HeadlessPlatform};
use lightcal::updater::UpdateCheckState;
use slint::ComponentHandle;
use slint::Model;
use std::env;
use std::path::Path;

pub fn render_snapshots_to_dir(out_dir: &Path) -> std::io::Result<()> {
    let platform = HeadlessPlatform::new();
    let window_handle = platform.window.clone();
    let _ = slint::platform::set_platform(Box::new(platform));

    let anchor = LocalDate::new(2026, 10, 29).unwrap();
    let temp_dir = env::temp_dir().join("lightcal-snapshots-prefs");
    let store = PreferencesStore::new(temp_dir.clone());
    let mut prefs = Preferences::default();
    prefs.selected_calendar_ids.insert("win-work".to_string());
    prefs
        .selected_calendar_ids
        .insert("win-personal".to_string());
    prefs
        .selected_calendar_ids
        .insert("win-holidays".to_string());

    let mut cal_state = CalendarState::new(anchor, prefs.first_weekday, prefs.gray_weekdays());
    cal_state.select_date(anchor.add_days(1)); // Friday, Oct 30 selected; Oct 29 is Today
    let mut provider = ProviderSnapshot::synthetic(anchor);
    if let Some(occ) = provider
        .occurrences
        .iter_mut()
        .find(|o| o.event_id == "syn-8")
    {
        occ.start_date = anchor.add_days(1);
        occ.end_date_exclusive = anchor.add_days(2);
        occ.title = "Founder summit prep".to_string();
        occ.time_label = "All day".to_string();
    }
    let startup_mgr = StartupManager::from_env(&temp_dir);
    let shortcut_mgr = ShortcutManager::new();

    let ui = AppWindow::new().expect("Failed to create headless AppWindow");
    crate::apply_system_font(&ui, LocaleCode::En);
    ui.show().expect("Failed to show headless AppWindow");

    let mut ctrl = AppController {
        store,
        prefs,
        cal_state,
        provider,
        provider_request: 0,
        connection_busy: false,
        connection_message: String::new(),
        startup_warning: String::new(),
        microsoft_account: lightcal::calendars::account::MicrosoftAccountState::Disconnected,
        provider_loading: false,
        account_before_check: None,
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
        tray_icon: None,
        #[cfg(windows)]
        tray_menu: crate::TrayMenuItems::new(LocaleCode::En),
        language_dropdown_open: false,
        theme_dropdown_open: false,
        first_day_dropdown_open: false,
        update_channel_dropdown_open: false,
        calendars_access_denied: false,
        calendars_is_empty: false,
        add_account_open: false,
        ics_dialog_open: false,
        sign_in_cancelled: false,
        active_sign_in: crate::ActiveSignIn::None,
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
        // Snapshots are never on screen, so headers render settled.
        on_screen: || false,
        animation_effects: || false,
    };

    let helper = HeadlessPlatform {
        window: window_handle,
    };

    // 1. Compact Dark (352×432)
    ctrl.prefs.theme = ThemePreference::Dark;
    ctrl.settings_open = false;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("01-compact-dark.png"), 352, 432)?;

    // 2. Compact Light (352×432)
    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("02-compact-light.png"), 352, 432)?;

    render_settings_snapshots(&mut ctrl, &ui, &helper, out_dir)?;

    // 10..13. Rendered 64×64 Taskbar / Tray Icons
    let icon_colored_wk = render_squircle_icon_rgba_64(
        TrayIconMode::WeekdayAndDate,
        false,
        29,
        Weekday::Thursday,
        (22, 1),
        true,
    );
    write_rgba_png(
        &out_dir.join("10-icon-colored-weekday-64.png"),
        64,
        64,
        &icon_colored_wk,
    )?;

    let icon_mono_dark = render_squircle_icon_rgba_64(
        TrayIconMode::WeekdayAndDate,
        true,
        29,
        Weekday::Thursday,
        (22, 1),
        true,
    );
    write_rgba_png(
        &out_dir.join("11-icon-mono-weekday-dark-64.png"),
        64,
        64,
        &icon_mono_dark,
    )?;

    let icon_mono_light = render_squircle_icon_rgba_64(
        TrayIconMode::WeekdayAndDate,
        true,
        29,
        Weekday::Thursday,
        (22, 1),
        false,
    );
    write_rgba_png(
        &out_dir.join("12-icon-mono-weekday-light-64.png"),
        64,
        64,
        &icon_mono_light,
    )?;

    let icon_colored_time = render_squircle_icon_rgba_64(
        TrayIconMode::TimeAndDate,
        false,
        29,
        Weekday::Thursday,
        (22, 1),
        true,
    );
    write_rgba_png(
        &out_dir.join("13-icon-colored-time-64.png"),
        64,
        64,
        &icon_colored_time,
    )?;

    // November 2026 events matching Penpot NOVEMBER_2026
    let nov = |day: u8| LocalDate::new(2026, 11, day).unwrap();
    let nov_occurrences = vec![
        CalendarOccurrence {
            source_id: "win-work".to_string(),
            event_id: "nov-2".to_string(),
            color_hex: "#008573".to_string(),
            start_date: nov(2),
            end_date_exclusive: nov(3),
            is_all_day: false,
            title: "Sprint planning".to_string(),
            time_label: "10:00".to_string(),
            ..Default::default()
        },
        CalendarOccurrence {
            source_id: "win-work".to_string(),
            event_id: "nov-4".to_string(),
            color_hex: "#008573".to_string(),
            start_date: nov(4),
            end_date_exclusive: nov(5),
            is_all_day: false,
            title: "Tech sync".to_string(),
            time_label: "14:00".to_string(),
            ..Default::default()
        },
        CalendarOccurrence {
            source_id: "win-personal".to_string(),
            event_id: "nov-6".to_string(),
            color_hex: "#F59E0B".to_string(),
            start_date: nov(6),
            end_date_exclusive: nov(7),
            is_all_day: false,
            title: "Gym session".to_string(),
            time_label: "18:00".to_string(),
            ..Default::default()
        },
        CalendarOccurrence {
            source_id: "win-work".to_string(),
            event_id: "nov-10".to_string(),
            color_hex: "#008573".to_string(),
            start_date: nov(10),
            end_date_exclusive: nov(11),
            is_all_day: false,
            title: "Design review".to_string(),
            time_label: "11:00".to_string(),
            ..Default::default()
        },
        CalendarOccurrence {
            source_id: "win-holidays".to_string(),
            event_id: "nov-11".to_string(),
            color_hex: "#10B981".to_string(),
            start_date: nov(11),
            end_date_exclusive: nov(12),
            is_all_day: true,
            title: "Veterans Day".to_string(),
            time_label: "All day".to_string(),
            ..Default::default()
        },
        CalendarOccurrence {
            source_id: "win-work".to_string(),
            event_id: "nov-12-w".to_string(),
            color_hex: "#008573".to_string(),
            start_date: nov(12),
            end_date_exclusive: nov(13),
            is_all_day: false,
            title: "Q4 roadmap review".to_string(),
            time_label: "09:30".to_string(),
            ..Default::default()
        },
        CalendarOccurrence {
            source_id: "win-personal".to_string(),
            event_id: "nov-12-p".to_string(),
            color_hex: "#F59E0B".to_string(),
            start_date: nov(12),
            end_date_exclusive: nov(13),
            is_all_day: false,
            title: "Dinner with Alex".to_string(),
            time_label: "18:00".to_string(),
            ..Default::default()
        },
        CalendarOccurrence {
            source_id: "win-work".to_string(),
            event_id: "nov-18".to_string(),
            color_hex: "#008573".to_string(),
            start_date: nov(18),
            end_date_exclusive: nov(19),
            is_all_day: false,
            title: "Backend sync".to_string(),
            time_label: "15:00".to_string(),
            ..Default::default()
        },
        CalendarOccurrence {
            source_id: "win-work".to_string(),
            event_id: "nov-20".to_string(),
            color_hex: "#008573".to_string(),
            start_date: nov(20),
            end_date_exclusive: nov(21),
            is_all_day: false,
            title: "Release cutoff".to_string(),
            time_label: "17:00".to_string(),
            ..Default::default()
        },
        CalendarOccurrence {
            source_id: "win-personal".to_string(),
            event_id: "nov-24".to_string(),
            color_hex: "#F59E0B".to_string(),
            start_date: nov(24),
            end_date_exclusive: nov(25),
            is_all_day: false,
            title: "Dentist follow-up".to_string(),
            time_label: "16:00".to_string(),
            ..Default::default()
        },
        CalendarOccurrence {
            source_id: "win-holidays".to_string(),
            event_id: "nov-26".to_string(),
            color_hex: "#10B981".to_string(),
            start_date: nov(26),
            end_date_exclusive: nov(27),
            is_all_day: true,
            title: "Thanksgiving".to_string(),
            time_label: "All day".to_string(),
            ..Default::default()
        },
        CalendarOccurrence {
            source_id: "win-work".to_string(),
            event_id: "nov-30".to_string(),
            color_hex: "#008573".to_string(),
            start_date: nov(30),
            end_date_exclusive: LocalDate::new(2026, 12, 1).unwrap(),
            is_all_day: false,
            title: "Monthly wrap-up".to_string(),
            time_label: "16:30".to_string(),
            ..Default::default()
        },
    ];
    ctrl.provider.occurrences.extend(nov_occurrences);

    // 14. Compact Dark Next Month (Nov 2026, 352×432)
    ctrl.prefs.set_locale(LocaleCode::En);
    ctrl.prefs.theme = ThemePreference::Dark;
    ctrl.settings_open = false;
    ctrl.cal_state.navigate_months(1);
    ctrl.cal_state
        .select_date(LocalDate::new(2026, 11, 12).unwrap());
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("14-compact-dark-next-month.png"), 352, 432)?;

    // 15. Compact Light Next Month (Nov 2026, 352×432)
    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("15-compact-light-next-month.png"), 352, 432)?;

    // Reset back to October 2026
    ctrl.cal_state.navigate_months(-1);
    ctrl.cal_state.select_date(anchor.add_days(1));

    // 16. Compact Dark in Russian (352×432)
    ctrl.prefs.set_locale(LocaleCode::Ru);
    ctrl.prefs.theme = ThemePreference::Dark;
    if let Some(occ) = ctrl
        .provider
        .occurrences
        .iter_mut()
        .find(|o| o.event_id == "syn-6")
    {
        occ.title = "Архитектурный синк".to_string();
    }
    if let Some(occ) = ctrl
        .provider
        .occurrences
        .iter_mut()
        .find(|o| o.event_id == "syn-7")
    {
        occ.title = "Визит к стоматологу".to_string();
    }
    if let Some(src) = ctrl
        .provider
        .sources
        .iter_mut()
        .find(|s| s.id == "win-work")
    {
        src.name = "Работа".to_string();
    }
    if let Some(src) = ctrl
        .provider
        .sources
        .iter_mut()
        .find(|s| s.id == "win-personal")
    {
        src.name = "Личное".to_string();
    }
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("16-compact-ru-dark.png"), 352, 432)?;

    // 17. Compact Dark in Japanese (352×432)
    ctrl.prefs.set_locale(LocaleCode::Ja);
    if let Some(occ) = ctrl
        .provider
        .occurrences
        .iter_mut()
        .find(|o| o.event_id == "syn-6")
    {
        occ.title = "設計レビュー".to_string();
    }
    if let Some(occ) = ctrl
        .provider
        .occurrences
        .iter_mut()
        .find(|o| o.event_id == "syn-7")
    {
        occ.title = "歯科定期検診".to_string();
    }
    if let Some(src) = ctrl
        .provider
        .sources
        .iter_mut()
        .find(|s| s.id == "win-work")
    {
        src.name = "仕事".to_string();
    }
    if let Some(src) = ctrl
        .provider
        .sources
        .iter_mut()
        .find(|s| s.id == "win-personal")
    {
        src.name = "個人".to_string();
    }
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("17-compact-ja-dark.png"), 352, 432)?;

    // 18. Compact Light in Spanish (352×432)
    ctrl.prefs.set_locale(LocaleCode::Es);
    ctrl.prefs.theme = ThemePreference::Light;
    if let Some(occ) = ctrl
        .provider
        .occurrences
        .iter_mut()
        .find(|o| o.event_id == "syn-6")
    {
        occ.title = "Revisión de arquitectura".to_string();
    }
    if let Some(occ) = ctrl
        .provider
        .occurrences
        .iter_mut()
        .find(|o| o.event_id == "syn-7")
    {
        occ.title = "Consulta dental".to_string();
    }
    if let Some(src) = ctrl
        .provider
        .sources
        .iter_mut()
        .find(|s| s.id == "win-work")
    {
        src.name = "Trabajo".to_string();
    }
    if let Some(src) = ctrl
        .provider
        .sources
        .iter_mut()
        .find(|s| s.id == "win-personal")
    {
        src.name = "Personal".to_string();
    }
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("18-compact-es-light.png"), 352, 432)?;

    // Regression evidence: all three settings modes use the actual shell renderer.
    ctrl.prefs.set_locale(LocaleCode::Ru);
    ctrl.settings_open = true;
    ctrl.settings_tab = 0;
    ctrl.cal_state
        .update_today(LocalDate::new(2026, 10, 30).unwrap());
    for theme in [ThemePreference::Light, ThemePreference::Dark] {
        ctrl.prefs.theme = theme;
        ctrl.prefs.tray_icon_monochrome = false;
        ctrl.sync_to_ui(&ui);
        helper.render_to_png(
            &out_dir.join(format!("19-calendar-settings-{}.png", theme.as_str())),
            448,
            360,
        )?;
        ctrl.prefs.tray_icon_monochrome = true;
        ctrl.sync_to_ui(&ui);
        helper.render_to_png(
            &out_dir.join(format!("19-calendar-settings-{}-mono.png", theme.as_str())),
            448,
            360,
        )?;
    }

    ctrl.settings_open = true;
    ctrl.settings_tab = 1;
    let account_sources = ctrl.provider.sources.clone();
    ctrl.prefs.set_locale(LocaleCode::Ru);
    ctrl.prefs.microsoft_client_id = "00000000-0000-0000-0000-000000000000".into();
    for theme in [ThemePreference::Light, ThemePreference::Dark] {
        ctrl.prefs.theme = theme;
        for (name, state) in [
            (
                "signed-out",
                lightcal::calendars::account::MicrosoftAccountState::Disconnected,
            ),
            (
                "connecting",
                lightcal::calendars::account::MicrosoftAccountState::Connecting,
            ),
            (
                "connected",
                lightcal::calendars::account::MicrosoftAccountState::Connected {
                    label: Some("demo@example.invalid".into()),
                },
            ),
            (
                "reauth",
                lightcal::calendars::account::MicrosoftAccountState::NeedsSignIn,
            ),
        ] {
            let connected = matches!(
                state,
                lightcal::calendars::account::MicrosoftAccountState::Connected { .. }
            );
            ctrl.provider.sources = if connected {
                account_sources
                    .iter()
                    .enumerate()
                    .map(|(i, source)| {
                        let mut source = source.clone();
                        source.id = format!("graph:fixture-{i}");
                        source.account_label = "Outlook".into();
                        source
                    })
                    .collect()
            } else {
                Vec::new()
            };
            ctrl.calendars_is_empty = !connected;
            ctrl.calendars_access_denied = false;
            ctrl.microsoft_account = state;
            ctrl.connection_busy = name == "connecting";
            ctrl.sync_to_ui(&ui);
            assert_eq!(
                ui.get_can_disconnect_microsoft(),
                matches!(name, "connected" | "reauth")
            );
            assert_eq!(
                ui.get_can_connect_microsoft(),
                matches!(name, "signed-out" | "reauth")
            );
            helper.render_to_png(
                &out_dir.join(format!("20-account-{}-{name}.png", theme.as_str())),
                448,
                360,
            )?;
        }
    }
    // Permission recovery must coexist with readable calendars from other providers.
    ctrl.settings_open = true;
    ctrl.settings_tab = 1;
    ctrl.connection_busy = false;
    ctrl.provider_loading = false;
    ctrl.prefs.windows_calendars_enabled = true;
    for theme in [ThemePreference::Light, ThemePreference::Dark] {
        ctrl.prefs.theme = theme;
        for partial in [false, true] {
            ctrl.provider = ProviderSnapshot::synthetic(anchor);
            ctrl.provider.status = lightcal::platform::ProviderStatus::AccessDenied;
            ctrl.provider.occurrences.clear();
            if partial {
                for (i, source) in ctrl.provider.sources.iter_mut().enumerate() {
                    source.id = format!("graph:permission-fixture-{i}");
                    source.account_label = "Outlook".into();
                }
                ctrl.microsoft_account =
                    lightcal::calendars::account::MicrosoftAccountState::Connected {
                        label: Some("demo@example.invalid".into()),
                    };
            } else {
                ctrl.provider.sources.clear();
                ctrl.microsoft_account =
                    lightcal::calendars::account::MicrosoftAccountState::Disconnected;
            }
            ctrl.calendars_access_denied = true;
            ctrl.calendars_is_empty = !partial;
            ctrl.sync_to_ui(&ui);
            assert!(ui.get_calendars_access_denied());
            let listed_sources = ui
                .get_account_cards()
                .iter()
                .map(|card| card.sources.row_count())
                .sum::<usize>();
            assert_eq!(listed_sources > 0, partial);
            helper.render_to_png(
                &out_dir.join(format!(
                    "21-permission-{}-{}.png",
                    theme.as_str(),
                    if partial { "partial" } else { "denied" }
                )),
                448,
                360,
            )?;
        }
    }
    render_calendar_boards(&mut ctrl, &ui, &helper, out_dir)?;
    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(())
}

/// Sample month from the design canvas boards (design/canvas/Main.dc.html):
/// today is Sunday 1 November 2026 at 14:05 with Google calendars.
pub(crate) fn canvas_month() -> ProviderSnapshot {
    use calendar_core::{local_minutes, CalendarSource};
    let sources = [
        ("google:personal", "Personal", "#D9822B"),
        ("google:work", "Work", "#3B82C4"),
        ("google:family", "Family", "#2E9D5B"),
    ]
    .map(|(id, name, color)| CalendarSource {
        id: id.into(),
        name: name.into(),
        account_label: "Google Calendar".into(),
        color_hex: color.into(),
    });
    /// Month, day, start and end, title, source index, location.
    type CanvasEvent<'a> = (
        u8,
        u8,
        Option<(&'a str, &'a str)>,
        &'a str,
        usize,
        Option<&'a str>,
    );
    let events: &[CanvasEvent] = &[
        (10, 27, Some(("09:30", "09:45")), "Standup", 1, None),
        (10, 29, Some(("12:00", "13:00")), "Lunch with Leo", 0, None),
        (10, 30, Some(("09:30", "09:45")), "Standup", 1, None),
        (10, 30, Some(("16:00", "17:00")), "Retro", 1, None),
        (11, 1, None, "Book fair", 2, None),
        (
            11,
            1,
            Some(("10:00", "11:00")),
            "Coffee with Maya",
            0,
            Some("Café Lumen"),
        ),
        (
            11,
            1,
            Some(("13:30", "15:00")),
            "Weekend walk",
            2,
            Some("Riverside park"),
        ),
        (
            11,
            1,
            Some(("18:00", "19:30")),
            "Dinner at Ana’s",
            0,
            Some("Ana’s place"),
        ),
        (11, 3, Some(("09:30", "09:45")), "Standup", 1, None),
        (11, 4, Some(("08:40", "09:20")), "Dentist", 0, None),
        (11, 4, Some(("11:00", "12:00")), "Sprint planning", 1, None),
        (11, 6, None, "Release 0.3.1", 1, None),
        (11, 10, Some(("09:30", "09:45")), "Standup", 1, None),
        (
            11,
            13,
            Some(("15:00", "16:00")),
            "Design review",
            1,
            Some("Room 4 and video call"),
        ),
        (11, 17, Some(("09:30", "09:45")), "Standup", 1, None),
        (11, 17, Some(("14:00", "14:30")), "1:1 with Sam", 1, None),
        (11, 17, Some(("19:00", "20:00")), "Gym", 0, None),
        (11, 19, Some(("19:00", "20:00")), "Gym", 0, None),
        (11, 21, Some(("20:00", "22:30")), "Concert", 0, None),
        (11, 24, Some(("09:30", "09:45")), "Standup", 1, None),
        (11, 26, Some(("10:00", "11:00")), "Sprint review", 1, None),
        (11, 26, Some(("12:30", "13:30")), "Team lunch", 1, None),
        (11, 26, Some(("16:30", "17:00")), "School pickup", 2, None),
        (11, 26, Some(("19:00", "19:30")), "Call Mum", 0, None),
        (11, 28, None, "Hike", 2, None),
        (12, 1, Some(("09:30", "09:45")), "Standup", 1, None),
        (12, 3, Some(("18:00", "19:00")), "Book club", 0, None),
    ];
    let minutes = |raw: &str| {
        let (h, m) = raw.split_once(':').unwrap();
        h.parse::<u32>().unwrap() * 60 + m.parse::<u32>().unwrap()
    };
    let occurrences = events
        .iter()
        .enumerate()
        .map(|(i, &(month, day, time, title, source, location))| {
            let date = LocalDate::new(2026, month, day).unwrap();
            let span = time.map(|(start, end)| {
                (
                    local_minutes(date, minutes(start)),
                    local_minutes(date, minutes(end)),
                )
            });
            CalendarOccurrence {
                source_id: sources[source].id.clone(),
                event_id: format!("canvas-{i}"),
                color_hex: sources[source].color_hex.clone(),
                start_date: date,
                end_date_exclusive: date.add_days(1),
                is_all_day: time.is_none(),
                title: title.into(),
                time_label: time.map(|(start, _)| start.to_string()).unwrap_or_default(),
                start_minutes: span.map(|(start, _)| start),
                end_minutes: span.map(|(_, end)| end),
                location: location.map(String::from),
                web_link: Some(format!(
                    "https://www.google.com/calendar/event?eid=Y2FudmFzLXtp{i}"
                )),
            }
        })
        .collect();
    ProviderSnapshot {
        status: lightcal::platform::ProviderStatus::Available,
        sources: sources.to_vec(),
        occurrences,
    }
}

/// Calendar popup states from the Main, CalendarDetail and CalendarDark boards.
fn render_calendar_boards(
    ctrl: &mut AppController,
    ui: &AppWindow,
    helper: &HeadlessPlatform,
    out_dir: &Path,
) -> std::io::Result<()> {
    let today = LocalDate::new(2026, 11, 1).unwrap();
    ctrl.provider = canvas_month();
    ctrl.prefs.selected_calendar_ids = ctrl
        .provider
        .sources
        .iter()
        .map(|source| source.id.clone())
        .collect();
    ctrl.prefs.show_week_numbers = true;
    ctrl.prefs.show_event_dots = true;
    ctrl.settings_open = false;
    ctrl.cal_state =
        CalendarState::new(today, ctrl.prefs.first_weekday, ctrl.prefs.gray_weekdays());
    ctrl.agenda = Default::default();
    ctrl.agenda.now_override = Some(calendar_core::local_minutes(today, 14 * 60 + 5));
    let shot = |ctrl: &AppController, name: &str| {
        ctrl.sync_to_ui(ui);
        helper.render_to_png(&out_dir.join(format!("calendar-{name}.png")), 352, 432)
    };

    ctrl.prefs.set_locale(LocaleCode::En);
    for theme in [ThemePreference::Light, ThemePreference::Dark] {
        ctrl.prefs.theme = theme;
        shot(ctrl, &format!("today-{}", theme.as_str()))?;
    }
    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.prefs.show_week_numbers = false;
    shot(ctrl, "today-no-weeks-light")?;
    ctrl.prefs.show_week_numbers = true;

    // Koi pond behind the grid, at a fixed pond time so the boards are reproducible. At 260 s
    // the three koi are spread over the grid, which the README screenshot uses.
    ctrl.prefs.show_koi = true;
    ui.set_koi_clock(260.0);
    for theme in [ThemePreference::Light, ThemePreference::Dark] {
        ctrl.prefs.theme = theme;
        shot(ctrl, &format!("koi-{}", theme.as_str()))?;
    }
    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.prefs.show_koi = false;

    // Event details: Friday 13 November, Design review expanded.
    ctrl.cal_state
        .select_date(LocalDate::new(2026, 11, 13).unwrap());
    {
        let key = ctrl.agenda_key_at(0);
        ctrl.toggle_agenda_event(&key);
    }
    for theme in [ThemePreference::Light, ThemePreference::Dark] {
        ctrl.prefs.theme = theme;
        shot(ctrl, &format!("detail-{}", theme.as_str()))?;
    }
    ctrl.prefs.theme = ThemePreference::Light;
    {
        let key = ctrl.agenda_key_at(0);
        ctrl.copy_agenda_event(&key);
    }
    shot(ctrl, "detail-copied-light")?;
    ctrl.agenda.expanded = None;
    ctrl.agenda.copied = None;

    // Empty day with the Next link: Thursday 12 November.
    ctrl.cal_state
        .select_date(LocalDate::new(2026, 11, 12).unwrap());
    for theme in [ThemePreference::Dark, ThemePreference::Light] {
        ctrl.prefs.theme = theme;
        shot(ctrl, &format!("empty-{}", theme.as_str()))?;
    }

    // Away from today, the Today button returns.
    ctrl.cal_state.return_to_today();
    ctrl.cal_state.navigate_months(1);
    ctrl.prefs.theme = ThemePreference::Light;
    shot(ctrl, "december-light")?;
    ctrl.cal_state.return_to_today();

    // Longest header: September with the Today button, next to settings and hide-to-tray.
    ctrl.cal_state.navigate_months(-2);
    for locale in [LocaleCode::En, LocaleCode::Ru, LocaleCode::Es] {
        ctrl.prefs.set_locale(locale);
        shot(ctrl, &format!("september-header-{}-light", locale.as_str()))?;
    }
    ctrl.prefs.set_locale(LocaleCode::En);
    ctrl.cal_state.return_to_today();

    // Keyboard focus ring on the selected day after arrow-key navigation.
    ctrl.cal_state.move_selection_by_days(3);
    ui.set_calendar_keyboard_nav(true);
    shot(ctrl, "keyboard-focus-light")?;
    ui.set_calendar_keyboard_nav(false);
    ctrl.cal_state.return_to_today();

    // Expanded all-day chip on today.
    {
        let key = ctrl.agenda_key_at(0);
        ctrl.toggle_agenda_event(&key);
    }
    shot(ctrl, "chip-detail-light")?;
    ctrl.agenda.expanded = None;

    // Date size (DateSize boards): Large keeps 352 × 432, Larger is 352 × 444.
    for (size, theme) in [
        (DateSize::Large, ThemePreference::Light),
        (DateSize::Larger, ThemePreference::Light),
        (DateSize::Larger, ThemePreference::Dark),
    ] {
        ctrl.prefs.date_size = size;
        ctrl.prefs.theme = theme;
        ctrl.sync_to_ui(ui);
        helper.render_to_png(
            &out_dir.join(format!("calendar-{}-{}.png", size.as_str(), theme.as_str())),
            352,
            crate::calendar_height(ui) as u32,
        )?;
    }
    ctrl.prefs.date_size = DateSize::Standard;
    ctrl.prefs.theme = ThemePreference::Light;

    for (locale, theme) in [
        (LocaleCode::Ru, ThemePreference::Light),
        (LocaleCode::Ja, ThemePreference::Dark),
        (LocaleCode::Es, ThemePreference::Light),
        (LocaleCode::De, ThemePreference::Light),
        (LocaleCode::Fr, ThemePreference::Dark),
        (LocaleCode::PtBr, ThemePreference::Light),
        (LocaleCode::Uk, ThemePreference::Dark),
        (LocaleCode::ZhHans, ThemePreference::Light),
        (LocaleCode::Ko, ThemePreference::Dark),
    ] {
        ctrl.prefs.set_locale(locale);
        ctrl.prefs.theme = theme;
        ctrl.cal_state.return_to_today();
        shot(
            ctrl,
            &format!("today-{}-{}", locale.as_str(), theme.as_str()),
        )?;
        ctrl.cal_state
            .select_date(LocalDate::new(2026, 11, 13).unwrap());
        {
            let key = ctrl.agenda_key_at(0);
            ctrl.toggle_agenda_event(&key);
        }
        shot(
            ctrl,
            &format!("detail-{}-{}", locale.as_str(), theme.as_str()),
        )?;
        ctrl.agenda.expanded = None;
        ctrl.cal_state
            .select_date(LocalDate::new(2026, 11, 12).unwrap());
        shot(
            ctrl,
            &format!("empty-{}-{}", locale.as_str(), theme.as_str()),
        )?;
    }
    ctrl.prefs.set_locale(LocaleCode::En);
    Ok(())
}

/// Settings, Accounts states and About (ADR 0011), light and dark, plus ru/es/ja.
/// Files are named `settings-<area>-<state>-<theme>[-<locale>].png`.
fn render_settings_snapshots(
    ctrl: &mut AppController,
    ui: &AppWindow,
    helper: &HeadlessPlatform,
    out_dir: &Path,
) -> std::io::Result<()> {
    use calendar_core::CalendarSource;
    use lightcal::calendars::account::MicrosoftAccountState as Ms;
    use lightcal::platform::ProviderStatus;

    let base_provider = ctrl.provider.clone();
    let base_selection = ctrl.prefs.selected_calendar_ids.clone();
    let source = |id: &str, name: &str, color: &str| CalendarSource {
        id: id.to_string(),
        name: name.to_string(),
        account_label: String::new(),
        color_hex: color.to_string(),
    };
    let reset = |ctrl: &mut AppController| {
        ctrl.prefs.set_locale(LocaleCode::En);
        ctrl.settings_open = true;
        ctrl.provider = base_provider.clone();
        ctrl.provider.status = ProviderStatus::Available;
        ctrl.microsoft_account = Ms::Disconnected;
        ctrl.account_before_check = None;
        ctrl.google_connected = false;
        ctrl.google_needs_signin = false;
        ctrl.google_label.clear();
        ctrl.google_error.clear();
        ctrl.ics_error.clear();
        ctrl.connection_busy = false;
        ctrl.connection_message.clear();
        ctrl.provider_loading = false;
        ctrl.sign_in_cancelled = false;
        ctrl.add_account_open = false;
        ctrl.ics_dialog_open = false;
        ctrl.active_sign_in = crate::ActiveSignIn::None;
        ctrl.calendars_access_denied = false;
        ctrl.calendars_is_empty = false;
        ctrl.prefs.windows_calendars_enabled = false;
        ctrl.shortcut_error.clear();
        ctrl.conflicting_shortcut = None;
        ctrl.prefs.global_shortcut = Some("Ctrl + Alt + C".into());
        ctrl.update_state = UpdateCheckState::Idle;
        ctrl.install_status.clear();
        ctrl.install_busy = false;
        ctrl.startup_warning.clear();
        ctrl.startup_error.clear();
        ctrl.language_dropdown_open = false;
        ctrl.first_day_dropdown_open = false;
        ctrl.last_refreshed = Some(chrono::Local::now() - chrono::Duration::minutes(2));
    };
    // Connected Google (three calendars, two checked) and Outlook accounts.
    let connect = |ctrl: &mut AppController| {
        ctrl.provider.sources = vec![
            source("google:personal", "Personal", "#D9822B"),
            source("google:work", "Work", "#3B82C4"),
            source("google:holidays", "Public holidays", "#2E9D5B"),
        ];
        ctrl.prefs.selected_calendar_ids.clear();
        ctrl.prefs
            .selected_calendar_ids
            .insert("google:personal".into());
        ctrl.prefs
            .selected_calendar_ids
            .insert("google:work".into());
        ctrl.google_connected = true;
        ctrl.google_label = "alex@example.com".into();
    };
    let themes = [ThemePreference::Light, ThemePreference::Dark];
    let shot = |ctrl: &mut AppController, name: &str| -> std::io::Result<()> {
        for theme in themes {
            ctrl.prefs.theme = theme;
            ctrl.sync_to_ui(ui);
            helper.render_to_png(
                &out_dir.join(format!("settings-{name}-{}.png", theme.as_str())),
                448,
                360,
            )?;
        }
        Ok(())
    };

    // Tabs.
    reset(ctrl);
    connect(ctrl);
    ctrl.settings_tab = 0;
    shot(ctrl, "calendar")?;
    ctrl.first_day_dropdown_open = true;
    shot(ctrl, "calendar-week-menu")?;
    ctrl.first_day_dropdown_open = false;
    ctrl.settings_tab = 1;
    shot(ctrl, "accounts-connected")?;
    ctrl.settings_tab = 2;
    shot(ctrl, "app")?;
    ctrl.prefs.tray_icon_monochrome = true;
    shot(ctrl, "app-mono")?;
    ctrl.prefs.tray_icon_monochrome = false;
    ctrl.language_dropdown_open = true;
    shot(ctrl, "app-language-menu")?;
    ctrl.language_dropdown_open = false;

    // Shortcut conflict: the previous shortcut stays.
    ctrl.shortcut_error =
        "Shortcut 'Ctrl + Alt + K' is in use by another application or reserved (error 1409)."
            .into();
    ctrl.conflicting_shortcut = Some("Ctrl + Alt + K".into());
    shot(ctrl, "app-shortcut-conflict")?;
    ctrl.shortcut_error.clear();
    ctrl.conflicting_shortcut = None;
    ctrl.prefs.global_shortcut = None;
    shot(ctrl, "app-shortcut-unset")?;
    ctrl.startup_error = "Could not update the Windows startup entry.".into();
    ctrl.startup_warning = "Preferences could not be read, so Lightcal restored the defaults. The damaged file was kept as preferences.json.corrupt.".into();
    shot(ctrl, "app-warnings")?;

    // Accounts states (States.dc.html).
    reset(ctrl);
    ctrl.settings_tab = 1;
    ctrl.provider.sources.clear();
    ctrl.calendars_is_empty = true;
    shot(ctrl, "accounts-first-run")?;

    ctrl.connection_busy = true;
    ctrl.active_sign_in = crate::ActiveSignIn::Google;
    shot(ctrl, "accounts-signing-in")?;
    ctrl.connection_busy = false;

    ctrl.sign_in_cancelled = true;
    shot(ctrl, "accounts-sign-in-cancelled")?;
    ctrl.sign_in_cancelled = false;

    ctrl.connection_message = "Google sign-in is not configured in this build.".into();
    shot(ctrl, "accounts-connection-error")?;
    ctrl.connection_message.clear();

    ctrl.ics_dialog_open = true;
    ui.set_ics_source_url("https://calendar.example.com/private-8f3a/basic.ics".into());
    shot(ctrl, "accounts-ics")?;
    ctrl.connection_busy = true;
    shot(ctrl, "accounts-ics-checking")?;
    ctrl.connection_busy = false;
    ctrl.connection_message =
        "Use a direct HTTPS calendar address. Plain HTTP is not supported.".into();
    shot(ctrl, "accounts-ics-error")?;
    ui.set_ics_source_url("".into());
    ctrl.connection_message.clear();
    ctrl.ics_dialog_open = false;

    // Needs attention (expired Google) and offline (Outlook).
    reset(ctrl);
    ctrl.settings_tab = 1;
    ctrl.provider.sources = vec![source("graph:calendar", "Calendar", "#3B82C4")];
    ctrl.prefs.selected_calendar_ids.clear();
    ctrl.prefs
        .selected_calendar_ids
        .insert("graph:calendar".into());
    ctrl.google_needs_signin = true;
    ctrl.google_label = "alex@example.com".into();
    ctrl.microsoft_account = Ms::Unavailable {
        saved_credentials: true,
    };
    shot(ctrl, "accounts-attention-offline")?;
    ctrl.microsoft_account = Ms::NeedsSignIn;
    ctrl.google_needs_signin = false;
    ctrl.google_connected = true;
    ctrl.google_error = "Network unavailable; try refreshing again".into();
    shot(ctrl, "accounts-outlook-expired-google-error")?;

    // No selection, no calendars, add account over existing accounts.
    reset(ctrl);
    connect(ctrl);
    ctrl.settings_tab = 1;
    ctrl.prefs.selected_calendar_ids.clear();
    shot(ctrl, "accounts-no-selection")?;
    ctrl.add_account_open = true;
    shot(ctrl, "accounts-add")?;
    ctrl.add_account_open = false;
    ctrl.provider.sources.clear();
    ctrl.microsoft_account = Ms::Connected {
        label: Some("alex@work.example".into()),
    };
    shot(ctrl, "accounts-no-calendars")?;

    // Windows calendars: access denied, plus an ICS feed.
    reset(ctrl);
    ctrl.settings_tab = 1;
    ctrl.prefs.windows_calendars_enabled = true;
    ctrl.calendars_access_denied = true;
    ctrl.provider.status = ProviderStatus::AccessDenied;
    ctrl.provider.sources = vec![source("ics:fixture", "Team rota", "#10B981")];
    ctrl.prefs
        .selected_calendar_ids
        .insert("ics:fixture".into());
    shot(ctrl, "accounts-windows-denied")?;

    // About states.
    reset(ctrl);
    ctrl.settings_tab = 3;
    shot(ctrl, "about")?;
    ctrl.update_state = UpdateCheckState::Checking;
    shot(ctrl, "about-checking")?;
    ctrl.update_state = UpdateCheckState::UpToDate {
        current_version: lightcal::version::VERSION.into(),
    };
    shot(ctrl, "about-up-to-date")?;
    ctrl.settings_tab = 2;
    shot(ctrl, "app-up-to-date")?;
    ctrl.settings_tab = 3;
    ctrl.update_state = UpdateCheckState::UpdateAvailable {
        current_version: lightcal::version::VERSION.into(),
        latest_version: "9.9.9".into(),
        release_url: "https://github.com/andmetoo/lightcal/releases/tag/v9.9.9".into(),
    };
    shot(ctrl, "about-update-available")?;
    ctrl.settings_tab = 2;
    shot(ctrl, "app-update-available")?;
    ctrl.settings_tab = 3;
    ctrl.install_status = "Update verification failed: the signer does not match the configured certificate. The existing installation was not replaced.".into();
    shot(ctrl, "about-install-error")?;
    ctrl.install_status.clear();
    ctrl.update_state = UpdateCheckState::Offline;
    shot(ctrl, "about-offline")?;
    ctrl.update_state = UpdateCheckState::RateLimited;
    shot(ctrl, "about-error")?;

    // The end of the Calendar tab, below the fold, with Koi pond on: its label in every
    // language next to the switch.
    for locale in LocaleCode::ALL {
        reset(ctrl);
        ctrl.prefs.set_locale(locale);
        ctrl.prefs.show_koi = true;
        ctrl.settings_tab = 0;
        ctrl.prefs.theme = ThemePreference::Light;
        ctrl.sync_to_ui(ui);
        // Lay the tab out at settings size before scrolling it.
        helper.window.set_size(slint::PhysicalSize::new(448, 360));
        helper.window.draw_if_needed(|_| {});
        let scroll = |delta_y| {
            ui.window()
                .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                    position: slint::LogicalPosition::new(220.0, 220.0),
                    delta_x: 0.0,
                    delta_y,
                });
        };
        scroll(-1000.0);
        helper.render_to_png(
            &out_dir.join(format!(
                "settings-calendar-end-light-{}.png",
                locale.as_str()
            )),
            448,
            360,
        )?;
        scroll(1000.0);
    }
    ctrl.prefs.show_koi = false;

    // Languages (Languages.dc.html): every tab and About in each non-English language.
    for locale in LocaleCode::ALL.into_iter().filter(|&l| l != LocaleCode::En) {
        reset(ctrl);
        connect(ctrl);
        ctrl.prefs.set_locale(locale);
        ctrl.update_state = UpdateCheckState::UpToDate {
            current_version: lightcal::version::VERSION.into(),
        };
        for (tab, name) in [(0, "calendar"), (1, "accounts"), (2, "app"), (3, "about")] {
            ctrl.settings_tab = tab;
            for theme in themes {
                ctrl.prefs.theme = theme;
                ctrl.sync_to_ui(ui);
                helper.render_to_png(
                    &out_dir.join(format!(
                        "settings-{name}-{}-{}.png",
                        theme.as_str(),
                        locale.as_str()
                    )),
                    448,
                    360,
                )?;
            }
        }
        ctrl.settings_tab = 1;
        ctrl.provider.sources.clear();
        ctrl.google_connected = false;
        ctrl.prefs.theme = ThemePreference::Light;
        ctrl.sync_to_ui(ui);
        helper.render_to_png(
            &out_dir.join(format!(
                "settings-accounts-first-run-light-{}.png",
                locale.as_str()
            )),
            448,
            360,
        )?;
        ctrl.settings_tab = 2;
        ctrl.provider.sources = base_provider.sources.clone();
        ctrl.shortcut_error = "Shortcut 'Ctrl + Alt + K' is in use by another application.".into();
        ctrl.conflicting_shortcut = Some("Ctrl + Alt + K".into());
        ctrl.sync_to_ui(ui);
        helper.render_to_png(
            &out_dir.join(format!(
                "settings-app-shortcut-conflict-light-{}.png",
                locale.as_str()
            )),
            448,
            360,
        )?;
    }

    // The open language menu in the Chinese UI, where the UI font is Microsoft YaHei UI
    // and the other scripts come from fallback.
    reset(ctrl);
    ctrl.settings_tab = 2;
    ctrl.prefs.set_locale(LocaleCode::ZhHans);
    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.language_dropdown_open = true;
    ctrl.sync_to_ui(ui);
    helper.render_to_png(
        &out_dir.join("settings-app-language-menu-light-zh-Hans.png"),
        448,
        360,
    )?;
    ctrl.language_dropdown_open = false;

    reset(ctrl);
    ctrl.provider = base_provider;
    ctrl.prefs.selected_calendar_ids = base_selection;
    ctrl.prefs.global_shortcut = None;
    ctrl.last_refreshed = None;
    Ok(())
}
