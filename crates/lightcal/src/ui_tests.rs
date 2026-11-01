//! Headless interaction checks. No native shortcut, shell, or provider callbacks run here.
use super::*;
use lightcal::snapshot::HeadlessPlatform;
use slint::platform::{PointerEventButton, WindowEvent};
use std::cell::Cell;

fn click(ui: &AppWindow, x: f32, y: f32) {
    let position = slint::LogicalPosition::new(x, y);
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position,
        button: PointerEventButton::Left,
    });
    ui.window().dispatch_event(WindowEvent::PointerReleased {
        position,
        button: PointerEventButton::Left,
    });
}

fn controller(dir: &std::path::Path) -> AppController {
    let prefs = Preferences {
        theme: ThemePreference::Light,
        ..Preferences::default()
    };
    let today = LocalDate::new(2026, 11, 1).unwrap();
    AppController {
        store: PreferencesStore::new(dir.to_path_buf()),
        cal_state: CalendarState::new(today, prefs.first_weekday, prefs.gray_weekdays()),
        prefs,
        provider: ProviderSnapshot {
            status: lightcal::platform::ProviderStatus::NoCalendars,
            sources: Vec::new(),
            occurrences: Vec::new(),
        },
        provider_request: 0,
        connection_busy: false,
        connection_message: String::new(),
        startup_warning: String::new(),
        microsoft_account: lightcal::calendars::account::MicrosoftAccountState::Disconnected,
        provider_loading: false,
        account_before_check: None,
        install_busy: false,
        install_status: String::new(),
        startup_mgr: StartupManager::from_env(dir),
        shortcut_mgr: ShortcutManager::new(),
        shortcut_error: String::new(),
        startup_error: String::new(),
        conflicting_shortcut: None,
        update_state: UpdateCheckState::Idle,
        settings_open: true,
        settings_tab: 1,
        #[cfg(windows)]
        tray_icon: None,
        language_dropdown_open: false,
        theme_dropdown_open: false,
        first_day_dropdown_open: false,
        update_channel_dropdown_open: false,
        calendars_access_denied: false,
        calendars_is_empty: true,
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
    }
}

#[test]
fn month_navigation_continues_after_today() {
    use slint::platform::software_renderer::{
        MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
    };
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    let platform = HeadlessPlatform {
        window: window.clone(),
    };
    slint::platform::set_platform(Box::new(platform)).unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-month-navigation-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.settings_open = false;
    let ctrl = Rc::new(RefCell::new(ctrl));
    let weak = ui.as_weak();
    let observed = ctrl.clone();
    ui.on_prev_month(move || {
        let mut ctrl = observed.borrow_mut();
        ctrl.cal_state.prev_month();
        ctrl.sync_to_ui(&weak.upgrade().unwrap());
    });
    let weak = ui.as_weak();
    let observed = ctrl.clone();
    ui.on_next_month(move || {
        let mut ctrl = observed.borrow_mut();
        ctrl.cal_state.next_month();
        ctrl.sync_to_ui(&weak.upgrade().unwrap());
    });
    let weak = ui.as_weak();
    let observed = ctrl.clone();
    ui.on_return_to_today(move || {
        let mut ctrl = observed.borrow_mut();
        ctrl.cal_state.return_to_today();
        ctrl.sync_to_ui(&weak.upgrade().unwrap());
    });
    let weak = ui.as_weak();
    let observed = ctrl.clone();
    ui.on_select_cell(move |idx| {
        let mut ctrl = observed.borrow_mut();
        let grid = ctrl.cal_state.build_grid(
            &ctrl.provider.occurrences,
            &ctrl.prefs.selected_calendar_ids,
        );
        if let Some(cell) = grid.cells.get(idx as usize) {
            ctrl.cal_state.select_date(cell.date);
        }
        ctrl.sync_to_ui(&weak.upgrade().unwrap());
    });
    ctrl.borrow().sync_to_ui(&ui);
    ui.show().unwrap();
    ui.window()
        .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: 1.5 });
    window.set_size(slint::PhysicalSize::new(528, 648));
    let mut pixels = vec![PremultipliedRgbaColor::default(); 528 * 648];
    let header_pixels = |pixels: &[PremultipliedRgbaColor]| -> Vec<u8> {
        (15..51)
            .flat_map(|y| (24..270).map(move |x| y * 528 + x))
            .flat_map(|index| {
                let pixel = pixels[index];
                [pixel.red, pixel.green, pixel.blue, pixel.alpha]
            })
            .collect()
    };
    slint::platform::update_timers_and_animations();
    window.draw_if_needed(|renderer| {
        renderer.render(&mut pixels, 528);
    });
    let mut previous_title = header_pixels(&pixels);
    let mut rendered_titles =
        std::collections::BTreeMap::from([("November 2026", previous_title.clone())]);
    for (step, (x, y, month, changed_title)) in [
        (285.0, 22.0, "December 2026", true),
        (285.0, 22.0, "January 2027", true),
        (200.0, 22.0, "December 2026", true),
        (200.0, 22.0, "November 2026", true),
        (200.0, 22.0, "October 2026", true),
        (187.0, 200.0, "October 2026", false),
        (285.0, 22.0, "November 2026", true),
        (145.0, 212.0, "November 2026", false),
        (243.0, 22.0, "November 2026", false),
        (187.0, 200.0, "November 2026", false),
        (285.0, 22.0, "December 2026", true),
        (285.0, 22.0, "January 2027", true),
        (200.0, 22.0, "December 2026", true),
        (243.0, 22.0, "November 2026", true),
        (145.0, 212.0, "November 2026", false),
        (200.0, 22.0, "October 2026", true),
    ]
    .into_iter()
    .enumerate()
    {
        ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: slint::LogicalPosition::new(x, y),
        });
        click(&ui, x, y);
        if y > 100.0 {
            assert_ne!(
                ctrl.borrow().cal_state.selected_date,
                ctrl.borrow().cal_state.today,
                "Date selection was delivered at step {step}"
            );
        }
        assert_eq!(ui.get_month_title(), month, "navigation step {step}");
        slint::platform::update_timers_and_animations();
        window.draw_if_needed(|renderer| {
            renderer.render(&mut pixels, 528);
        });
        let title = header_pixels(&pixels);
        if changed_title {
            assert!(
                title != previous_title,
                "The month title repaints at step {step}"
            );
        }
        if let Some(expected) = rendered_titles.get(month) {
            assert!(
                &title == expected,
                "The rendered heading matches {month} at step {step}"
            );
        } else {
            rendered_titles.insert(month, title.clone());
        }
        previous_title = title;
    }
}

#[test]
fn expired_google_account_restores_saved_reconnect_configuration() {
    slint::platform::set_platform(Box::new(HeadlessPlatform::new())).unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-google-reconnect-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.prefs.google_client_id.clear();
    ctrl.prefs.google_client_secret.clear();
    let ctrl = Rc::new(RefCell::new(ctrl));
    ACTIVE_CONTROLLER.with(|active| *active.borrow_mut() = Some(Rc::downgrade(&ctrl)));
    apply_loaded_provider(
        lightcal::calendars::CalendarLoad {
            snapshot: ProviderSnapshot {
                status: lightcal::platform::ProviderStatus::NoCalendars,
                sources: Vec::new(),
                occurrences: Vec::new(),
            },
            microsoft: lightcal::calendars::account::MicrosoftAccountState::Disconnected,
            google_connected: false,
            google_needs_signin: true,
            google_configuration: Some(lightcal::calendars::GoogleConfiguration {
                client_id: "1234567890-fixture.apps.googleusercontent.com".into(),
                client_secret: "fixture-desktop-secret".into(),
            }),
            google_label: None,
            google_error: String::new(),
            google_skipped: 0,
            ics_error: String::new(),
        },
        ui.as_weak(),
    );
    assert!(ui.get_google_needs_signin());
    let ctrl = ctrl.borrow();
    assert!(lightcal::calendars::google::valid_client_id(
        &ctrl.prefs.google_client_id
    ));
    assert!(lightcal::calendars::google::valid_client_secret(
        &ctrl.prefs.google_client_secret
    ));
    let preferences = serde_json::to_string(&ctrl.prefs).unwrap();
    assert!(!preferences.contains("fixture-desktop-secret"));
    ACTIVE_CONTROLLER.with(|active| *active.borrow_mut() = None);
}

#[test]
fn about_offers_manual_updates_without_running_an_installer() {
    let platform = HeadlessPlatform::new();
    let renderer = HeadlessPlatform {
        window: platform.window.clone(),
    };
    slint::platform::set_platform(Box::new(platform)).unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-manual-update-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.settings_tab = 3;
    ctrl.update_state = UpdateCheckState::UpdateAvailable {
        current_version: "0.2.2".into(),
        latest_version: "9.9.9".into(),
        release_url: "https://github.com/andmetoo/lightcal/releases/tag/v9.9.9".into(),
    };
    let installed = Rc::new(Cell::new(false));
    let observed = installed.clone();
    ui.on_install_update(move || observed.set(true));
    ui.show().unwrap();
    for locale in LocaleCode::ALL {
        ctrl.prefs.set_locale(locale);
        ctrl.sync_to_ui(&ui);
        renderer
            .render_to_png(
                &dir.join(format!("manual-update-{}.png", locale.as_str())),
                448,
                360,
            )
            .unwrap();
        installed.set(false);
        click(&ui, 340.0, 164.0);
        assert!(
            installed.get(),
            "Manual Download/Install is reachable in {}",
            locale.as_str()
        );
    }
    ctrl.install_busy = true;
    ctrl.sync_to_ui(&ui);
    renderer
        .render_to_png(&dir.join("install-busy.png"), 448, 360)
        .unwrap();
    installed.set(false);
    click(&ui, 340.0, 164.0);
    assert!(
        !installed.get(),
        "A running update cannot be submitted twice"
    );
    ctrl.install_busy = false;
    ctrl.install_status = "Update verification failed: the signer does not match the configured certificate. The existing installation was not replaced.".into();
    ctrl.sync_to_ui(&ui);
    renderer
        .render_to_png(&dir.join("install-error.png"), 448, 360)
        .unwrap();
    println!("Manual-update snapshots: {}", dir.display());
}

#[test]
fn settings_recovery_controls_remain_reachable() {
    let platform = HeadlessPlatform::new();
    let renderer = HeadlessPlatform {
        window: platform.window.clone(),
    };
    slint::platform::set_platform(Box::new(platform)).unwrap();
    let ui = AppWindow::new().unwrap();
    ui.set_settings_open(true);
    ui.set_settings_tab(1);
    ui.set_legacy_windows_label("Windows calendars".into());
    ui.set_calendars_empty_title_text("No accounts connected".into());
    ui.set_calendars_empty_desc_text("Add an account to show events.".into());
    let dir = std::env::temp_dir().join(format!(
        "lightcal-ui-recovery-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    ui.show().unwrap();

    let weak = ui.as_weak();
    ui.on_toggle_legacy_windows(move || {
        let ui = weak.upgrade().unwrap();
        ui.set_legacy_windows_enabled(!ui.get_legacy_windows_enabled());
    });
    renderer
        .render_to_png(&dir.join("windows-disabled.png"), 448, 360)
        .unwrap();
    click(&ui, 402.0, 158.0);
    assert!(
        ui.get_legacy_windows_enabled(),
        "Fresh users can enable Windows calendars"
    );
    renderer
        .render_to_png(&dir.join("windows-enabled.png"), 448, 360)
        .unwrap();
    click(&ui, 402.0, 158.0);
    assert!(
        !ui.get_legacy_windows_enabled(),
        "The same switch can disable them again"
    );

    ui.set_settings_tab(2);
    ui.set_label_shortcut_text("Global shortcut".into());
    ui.set_global_shortcut_display("Alt + Shift + C".into());
    ui.set_shortcut_change_label("Change".into());
    ui.set_shortcut_clear_label("Clear".into());
    ui.set_shortcut_can_clear(true);
    let cleared = Rc::new(Cell::new(false));
    let observed = cleared.clone();
    ui.on_clear_shortcut(move || observed.set(true));
    renderer
        .render_to_png(&dir.join("shortcut-clear.png"), 448, 360)
        .unwrap();
    click(&ui, 388.0, 196.0);
    assert!(cleared.get(), "The shortcut clear action is reachable");
    for locale in LocaleCode::ALL {
        let strings = catalog(locale);
        ui.set_label_shortcut_text(strings.label_shortcut.into());
        ui.set_shortcut_change_label(
            match locale {
                LocaleCode::Ru => "Изменить",
                LocaleCode::Es => "Cambiar",
                LocaleCode::Ja => "変更",
                LocaleCode::En => "Change",
            }
            .into(),
        );
        ui.set_shortcut_clear_label(strings.shortcut_clear.into());
        ui.set_label_tray_palette_text(strings.label_tray_palette.into());
        ui.set_palette_colored_text(strings.palette_colored.into());
        ui.set_palette_monochrome_text(strings.palette_monochrome.into());
        renderer
            .render_to_png(
                &dir.join(format!("shortcut-clear-{}.png", locale.as_str())),
                448,
                360,
            )
            .unwrap();
        cleared.set(false);
        click(&ui, 388.0, 196.0);
        assert!(
            cleared.get(),
            "Clear remains reachable in {}",
            locale.as_str()
        );
    }

    let mut ctrl = controller(&dir);
    ctrl.google_connected = true;
    ctrl.google_error = "Network unavailable; try refreshing again".into();
    ctrl.sync_to_ui(&ui);
    assert!(ui.get_google_connected());
    assert!(!ui.get_google_needs_signin());
    let refreshed = Rc::new(Cell::new(false));
    let observed = refreshed.clone();
    ui.on_refresh_calendars(move || observed.set(true));
    renderer
        .render_to_png(&dir.join("google-retry.png"), 448, 360)
        .unwrap();
    click(&ui, 272.0, 224.0);
    assert!(
        refreshed.get(),
        "A failed Google query still offers manual refresh"
    );

    ctrl.google_connected = false;
    ctrl.google_needs_signin = true;
    ctrl.google_error.clear();
    ctrl.sync_to_ui(&ui);
    assert!(!ui.get_google_connected());
    assert!(ui.get_google_needs_signin());
    let reconnected = Rc::new(Cell::new(false));
    let observed = reconnected.clone();
    ui.on_choose_google_account(move || observed.set(true));
    renderer
        .render_to_png(&dir.join("google-expired.png"), 448, 360)
        .unwrap();
    click(&ui, 364.0, 224.0);
    assert!(
        reconnected.get(),
        "An expired Google account offers Reconnect"
    );

    ctrl.google_connected = true;
    ctrl.google_needs_signin = false;
    ctrl.calendars_access_denied = true;
    ctrl.provider.sources.push(calendar_core::CalendarSource {
        id: "google:fixture".into(),
        name: "Readable calendar".into(),
        account_label: "Google Calendar".into(),
        color_hex: "#008573".into(),
    });
    ctrl.sync_to_ui(&ui);
    let selected = Rc::new(Cell::new(false));
    let observed = selected.clone();
    ui.on_toggle_calendar_source(move |id| observed.set(id == "google:fixture"));
    renderer
        .render_to_png(&dir.join("mixed-permission-denied.png"), 448, 360)
        .unwrap();
    click(&ui, 36.0, 194.0);
    assert!(
        selected.get(),
        "Windows permission denial must not remove a readable calendar's checkbox"
    );
    ui.window().dispatch_event(WindowEvent::PointerScrolled {
        position: slint::LogicalPosition::new(220.0, 220.0),
        delta_x: 0.0,
        delta_y: -1000.0,
    });
    renderer
        .render_to_png(&dir.join("mixed-permission-recovery.png"), 448, 360)
        .unwrap();
    let privacy_opened = Rc::new(Cell::new(false));
    let observed = privacy_opened.clone();
    ui.on_open_privacy_settings(move || observed.set(true));
    let retried = Rc::new(Cell::new(false));
    let observed = retried.clone();
    ui.on_retry_calendar_sync(move || observed.set(true));
    click(&ui, 100.0, 236.0);
    assert!(
        privacy_opened.get(),
        "Permission recovery remains reachable"
    );
    click(&ui, 220.0, 236.0);
    assert!(retried.get(), "Permission recovery still offers Retry Sync");

    ctrl.settings_tab = 3;
    ctrl.prefs.auto_check_updates = true;
    ctrl.prefs.auto_install_updates = true;
    ctrl.sync_to_ui(&ui);
    let weak = ui.as_weak();
    ui.on_toggle_auto_check_updates(move || {
        let ui = weak.upgrade().unwrap();
        ui.set_auto_check_updates(!ui.get_auto_check_updates());
    });
    let weak = ui.as_weak();
    ui.on_toggle_auto_install_updates(move || {
        let ui = weak.upgrade().unwrap();
        ui.set_auto_install_updates(!ui.get_auto_install_updates());
    });
    for locale in LocaleCode::ALL {
        ctrl.prefs.set_locale(locale);
        ctrl.sync_to_ui(&ui);
        renderer
            .render_to_png(
                &dir.join(format!("about-updates-{}.png", locale.as_str())),
                448,
                360,
            )
            .unwrap();
        click(&ui, 402.0, 224.0);
        assert!(
            !ui.get_auto_check_updates(),
            "Automatic checking can be disabled in {}",
            locale.as_str()
        );
        click(&ui, 402.0, 264.0);
        assert!(
            !ui.get_auto_install_updates(),
            "Automatic installation can be disabled in {}",
            locale.as_str()
        );
    }
    ui.window().dispatch_event(WindowEvent::PointerScrolled {
        position: slint::LogicalPosition::new(220.0, 220.0),
        delta_x: 0.0,
        delta_y: -1000.0,
    });
    renderer
        .render_to_png(&dir.join("about-links.png"), 448, 360)
        .unwrap();
    let opened = Rc::new(RefCell::new(Vec::new()));
    let observed = opened.clone();
    ui.on_open_external_url(move |url| observed.borrow_mut().push(url.to_string()));
    click(&ui, 220.0, 212.0);
    click(&ui, 220.0, 250.0);
    click(&ui, 220.0, 288.0);
    assert_eq!(
        *opened.borrow(),
        [
            "https://github.com/andmetoo/lightcal",
            "https://koona.app",
            "https://github.com/andmetoo/lightcal/blob/main/LICENSE",
        ],
        "About links remain reachable below the restored update preferences"
    );

    ctrl.settings_tab = 1;
    ctrl.sync_to_ui(&ui);
    ui.set_ics_dialog_open(true);
    ui.set_ics_dialog_title("Add a calendar link".into());
    ui.set_ics_privacy_text("Anyone with the address can read this calendar.".into());
    ui.set_ics_source_name("Fixture calendar".into());
    ui.set_ics_source_url("http://example.invalid/calendar.ics".into());
    ui.set_subscribe_label("Subscribe".into());
    ui.set_cancel_sign_in_label("Cancel".into());
    ui.set_ics_loading_text("Checking calendar…".into());
    let subscribed = Rc::new(Cell::new(false));
    let observed = subscribed.clone();
    ui.on_add_ics_source(move |_, _| observed.set(true));
    ui.set_connection_busy(true);
    renderer
        .render_to_png(&dir.join("ics-loading.png"), 448, 360)
        .unwrap();
    click(&ui, 362.0, 274.0);
    assert!(
        !subscribed.get(),
        "A pending subscription cannot be submitted twice"
    );
    ui.set_connection_busy(false);
    ui.set_connection_message(
        "Use a direct HTTPS calendar address. Plain HTTP is not supported.".into(),
    );
    renderer
        .render_to_png(&dir.join("ics-error.png"), 448, 360)
        .unwrap();
    click(&ui, 362.0, 274.0);
    assert!(
        subscribed.get(),
        "A failed subscription can be corrected and retried"
    );
    println!("Recovery-control snapshots: {}", dir.display());
}

#[test]
fn agenda_rows_expand_in_place_and_the_next_link_selects_its_day() {
    let platform = HeadlessPlatform::new();
    let window = platform.window.clone();
    slint::platform::set_platform(Box::new(platform)).unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-agenda-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.settings_open = false;
    ctrl.provider = crate::snapshot_render::canvas_month();
    ctrl.prefs.selected_calendar_ids = ctrl
        .provider
        .sources
        .iter()
        .map(|source| source.id.clone())
        .collect();
    let today = ctrl.cal_state.today;
    ctrl.agenda.now_override = Some(calendar_core::local_minutes(today, 14 * 60 + 5));
    let ctrl = Rc::new(RefCell::new(ctrl));
    calendar_view::connect_agenda_callbacks(&ui, &ctrl, |_| {});
    let weak = ui.as_weak();
    let observed = ctrl.clone();
    ui.on_select_cell(move |idx| {
        let mut ctrl = observed.borrow_mut();
        let grid = ctrl.cal_state.build_grid(
            &ctrl.provider.occurrences,
            &ctrl.prefs.selected_calendar_ids,
        );
        if let Some(cell) = grid.cells.get(idx as usize) {
            ctrl.cal_state.select_date(cell.date);
        }
        ctrl.sync_to_ui(&weak.upgrade().unwrap());
    });
    let weak = ui.as_weak();
    let observed = ctrl.clone();
    ui.on_prev_month(move || {
        let mut ctrl = observed.borrow_mut();
        ctrl.cal_state.prev_month();
        ctrl.sync_to_ui(&weak.upgrade().unwrap());
    });
    ctrl.borrow().sync_to_ui(&ui);
    ui.show().unwrap();
    window.set_size(slint::PhysicalSize::new(352, 432));

    // Today: one all-day chip, then three timed rows; the walk is in progress.
    assert!(ui.get_today_hidden());
    assert_eq!(ui.get_agenda_chip_rows().row_count(), 1);
    let rows = ui.get_agenda_events();
    assert_eq!(rows.row_count(), 3);
    let first = rows.row_data(0).unwrap();
    assert!(first.dimmed);
    assert_eq!(first.time_label, "10:00–11:00");
    assert!(rows.row_data(1).unwrap().is_now);
    assert_eq!(rows.row_data(2).unwrap().badge, "in 3 h 55 min");

    // The first timed row sits under the chip line (list top 301 + 28).
    click(&ui, 180.0, 344.0);
    let opened = ui.get_agenda_events().row_data(0).unwrap();
    assert!(opened.expanded, "Clicking a row expands it");
    assert_eq!(opened.detail_time, "10:00–11:00 · 1 h");
    assert_eq!(opened.location, "Café Lumen");
    assert_eq!(opened.provider_line, "Personal · Google Calendar");
    assert!(opened.can_open);
    click(&ui, 180.0, 344.0);
    assert!(
        !ui.get_agenda_events().row_data(0).unwrap().expanded,
        "Clicking again collapses it"
    );

    // Thursday 12 November is empty and links to Friday's design review.
    click(&ui, 188.0, 148.0);
    assert_eq!(
        ctrl.borrow().cal_state.selected_date,
        LocalDate::new(2026, 11, 12).unwrap()
    );
    assert!(ui.get_agenda_empty());
    assert_eq!(
        ui.get_agenda_next_text(),
        "Next: Fri 13 Nov · Design review →"
    );
    assert!(!ui.get_today_hidden());
    click(&ui, 60.0, 338.0);
    assert_eq!(
        ctrl.borrow().cal_state.selected_date,
        LocalDate::new(2026, 11, 13).unwrap()
    );

    // One wheel notch over the grid moves one month.
    ui.window().dispatch_event(WindowEvent::PointerScrolled {
        position: slint::LogicalPosition::new(180.0, 150.0),
        delta_x: 0.0,
        delta_y: 60.0,
    });
    assert_eq!(ui.get_month_title(), "October 2026");
    let _ = std::fs::remove_dir_all(&dir);
}
