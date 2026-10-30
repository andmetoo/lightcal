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
    ui.show().expect("Failed to show headless AppWindow");

    let mut ctrl = AppController {
        store,
        prefs,
        cal_state,
        provider,
        provider_request: 0,
        connection_busy: false,
        connection_message: String::new(),
        microsoft_account: lightcal::calendars::account::MicrosoftAccountState::Disconnected,
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
        tray_icon: None,
        language_dropdown_open: false,
        theme_dropdown_open: false,
        first_day_dropdown_open: false,
        update_channel_dropdown_open: false,
        calendars_access_denied: false,
        calendars_is_empty: false,
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

    // 5. Settings — General (Dark & Light, 780×620)
    ctrl.prefs.set_locale(LocaleCode::En);
    ctrl.prefs.theme = ThemePreference::Dark;
    ctrl.settings_open = true;
    ctrl.settings_tab = 0;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("05-settings-general-dark.png"), 780, 620)?;

    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("05-settings-general-light.png"), 780, 620)?;

    // 6. Settings — Appearance (Dark & Light, 780×620)
    ctrl.prefs.theme = ThemePreference::Dark;
    ctrl.settings_tab = 1;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("06-settings-appearance-dark.png"), 780, 620)?;

    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("06-settings-appearance-light.png"), 780, 620)?;

    // 7. Settings — Calendars (Dark & Light, 780×620)
    ctrl.prefs.theme = ThemePreference::Dark;
    ctrl.settings_tab = 2;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("07-settings-calendars-dark.png"), 780, 620)?;

    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("07-settings-calendars-light.png"), 780, 620)?;

    // 8. Settings — About (Dark & Light, 780×620)
    ctrl.prefs.theme = ThemePreference::Dark;
    ctrl.settings_tab = 3;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("08-settings-about-dark.png"), 780, 620)?;

    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("08-settings-about-light.png"), 780, 620)?;

    // 9. Settings — General in Russian (Light, 780×620)
    ctrl.prefs.set_locale(LocaleCode::Ru);
    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.settings_tab = 0;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("09-settings-general-ru-light.png"), 780, 620)?;

    // State 1: Languages Dropdown Open (Light & Dark)
    ctrl.prefs.set_locale(LocaleCode::En);
    ctrl.settings_tab = 0;
    ctrl.language_dropdown_open = true;
    ctrl.prefs.theme = ThemePreference::Dark;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("state-lang-open-dark.png"), 780, 620)?;

    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("state-lang-open-light.png"), 780, 620)?;
    ctrl.language_dropdown_open = false;

    // State 2: Shortcut Conflict (Light & Dark)
    ctrl.shortcut_error =
        "Shortcut unavailable: Alt + Space is already registered by Raycast.".to_string();
    ctrl.conflicting_shortcut = Some("Alt + Space".to_string());
    ctrl.prefs.theme = ThemePreference::Dark;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("state-shortcut-conflict-dark.png"), 780, 620)?;

    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("state-shortcut-conflict-light.png"), 780, 620)?;
    ctrl.shortcut_error.clear();
    ctrl.conflicting_shortcut = None;

    // State 3: Calendars Access Denied (Light & Dark)
    ctrl.settings_tab = 2;
    ctrl.calendars_access_denied = true;
    ctrl.prefs.theme = ThemePreference::Dark;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(
        &out_dir.join("state-calendars-access-denied-dark.png"),
        780,
        620,
    )?;

    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(
        &out_dir.join("state-calendars-access-denied-light.png"),
        780,
        620,
    )?;
    ctrl.calendars_access_denied = false;

    // State 4: Calendars Empty (Light & Dark)
    ctrl.calendars_is_empty = true;
    ctrl.prefs.theme = ThemePreference::Dark;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("state-calendars-empty-dark.png"), 780, 620)?;

    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("state-calendars-empty-light.png"), 780, 620)?;
    ctrl.calendars_is_empty = false;

    // State 5: About Checking Updates (Light & Dark)
    ctrl.settings_tab = 3;
    ctrl.update_state = UpdateCheckState::Checking;
    ctrl.prefs.theme = ThemePreference::Dark;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("state-about-checking-dark.png"), 780, 620)?;

    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("state-about-checking-light.png"), 780, 620)?;

    // State 6: About Update Available (Light & Dark)
    ctrl.update_state = UpdateCheckState::UpdateAvailable {
        current_version: "0.4.2".into(),
        latest_version: "0.5.0".into(),
        release_url: "https://github.com/andmetoo/lightcal/releases/tag/v0.5.0".into(),
    };
    ctrl.prefs.theme = ThemePreference::Dark;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(
        &out_dir.join("state-about-update-available-dark.png"),
        780,
        620,
    )?;

    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(
        &out_dir.join("state-about-update-available-light.png"),
        780,
        620,
    )?;
    ctrl.install_status = "Automatic installation is unavailable: build with LIGHTCAL_UPDATE_CERT_SHA256 set to the SHA-256 fingerprint of your SimpleSign signing certificate".into();
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(
        &out_dir.join("state-about-install-status-light.png"),
        780,
        620,
    )?;
    ctrl.install_status.clear();
    ctrl.update_state = UpdateCheckState::Idle;

    // Localized Japanese Settings (Dark & Light)
    ctrl.prefs.set_locale(LocaleCode::Ja);
    ctrl.settings_tab = 0;
    ctrl.prefs.theme = ThemePreference::Dark;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("settings-general-ja-dark.png"), 780, 620)?;

    ctrl.settings_tab = 1;
    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("settings-appearance-ja-light.png"), 780, 620)?;

    // Localized Spanish Settings (Light)
    ctrl.prefs.set_locale(LocaleCode::Es);
    ctrl.settings_tab = 0;
    ctrl.prefs.theme = ThemePreference::Light;
    ctrl.sync_to_ui(&ui);
    helper.render_to_png(&out_dir.join("settings-general-es-light.png"), 780, 620)?;

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
            color_hex: "#5E6AD2".to_string(),
            start_date: nov(2),
            end_date_exclusive: nov(3),
            is_all_day: false,
            title: "Sprint planning".to_string(),
            time_label: "10:00".to_string(),
        },
        CalendarOccurrence {
            source_id: "win-work".to_string(),
            event_id: "nov-4".to_string(),
            color_hex: "#5E6AD2".to_string(),
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
            color_hex: "#5E6AD2".to_string(),
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
            color_hex: "#5E6AD2".to_string(),
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
            color_hex: "#5E6AD2".to_string(),
            start_date: nov(18),
            end_date_exclusive: nov(19),
            is_all_day: false,
            title: "Backend sync".to_string(),
            time_label: "15:00".to_string(),
        },
        CalendarOccurrence {
            source_id: "win-work".to_string(),
            event_id: "nov-20".to_string(),
            color_hex: "#5E6AD2".to_string(),
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
            color_hex: "#5E6AD2".to_string(),
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
    ctrl.settings_tab = 1;
    ctrl.cal_state
        .update_today(LocalDate::new(2026, 10, 30).unwrap());
    for theme in [ThemePreference::Light, ThemePreference::Dark] {
        ctrl.prefs.theme = theme;
        for mode in [
            TrayIconMode::DateOnly,
            TrayIconMode::WeekdayAndDate,
            TrayIconMode::TimeAndDate,
        ] {
            ctrl.prefs.tray_icon_mode = mode;
            ctrl.sync_to_ui(&ui);
            helper.render_to_png(
                &out_dir.join(format!(
                    "19-appearance-{}-{}.png",
                    theme.as_str(),
                    mode.as_str()
                )),
                780,
                620,
            )?;
        }
    }

    ctrl.settings_open = true;
    ctrl.settings_tab = 2;
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
                780,
                620,
            )?;
        }
    }
    // Permission recovery must coexist with readable calendars from other providers.
    ctrl.settings_open = true;
    ctrl.settings_tab = 2;
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
                780,
                620,
            )?;
        }
    }
    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(())
}
