//! Headless UI snapshot rendering pipeline for design system and visual regression validation.

use crate::{AppController, AppWindow};
use calendar_core::{CalendarOccurrence, CalendarState, LocalDate, Weekday};
use lightcal::platform::windows::{render_squircle_icon_rgba_64, ShortcutManager, StartupManager};
use lightcal::platform::ProviderSnapshot;
use lightcal::preferences::{
    LocaleCode, Preferences, PreferencesStore, ThemePreference, TrayIconMode,
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
    crate::apply_system_font(&ui);
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
        last_refreshed: None,
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
            assert_eq!(ui.get_calendar_sources().row_count() > 0, partial);
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
    let _ = std::fs::remove_dir_all(&temp_dir);
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

    // Languages (Languages.dc.html): every tab and About in ru, es and ja.
    for locale in [LocaleCode::Ru, LocaleCode::Es, LocaleCode::Ja] {
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

    reset(ctrl);
    ctrl.provider = base_provider;
    ctrl.prefs.selected_calendar_ids = base_selection;
    ctrl.prefs.global_shortcut = None;
    ctrl.last_refreshed = None;
    Ok(())
}
