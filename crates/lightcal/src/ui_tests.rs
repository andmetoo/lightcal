//! Headless interaction checks. No native shortcut, shell, or provider callbacks run here.
use super::*;
use i_slint_backend_testing::ElementHandle;
use lightcal::snapshot::HeadlessPlatform;
use slint::platform::{PointerEventButton, WindowEvent};
use slint::Model;
use std::cell::Cell;

fn test_instance(app_id: &str, directory: &std::path::Path) -> SingleInstance {
    #[cfg(target_os = "macos")]
    {
        SingleInstance::acquire_isolated(app_id, directory)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = directory;
        SingleInstance::acquire(app_id)
    }
}

#[cfg(target_os = "macos")]
#[test]
fn isolated_instance_locks_use_the_explicit_binary_test_directory() {
    let directory = std::env::temp_dir().join(format!(
        "lightcal-instance-isolation-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let first_dir = directory.join("first");
    let second_dir = directory.join("second");
    let app_id = "Lightcal.Test.Isolated";
    let SingleInstance::FirstInstance(first) = test_instance(app_id, &first_dir) else {
        panic!("first lock");
    };
    assert!(matches!(
        test_instance(app_id, &first_dir),
        SingleInstance::AlreadyRunning
    ));
    let SingleInstance::FirstInstance(second) = test_instance(app_id, &second_dir) else {
        panic!("isolated directory has its own lock");
    };
    for dir in [&first_dir, &second_dir] {
        let paths: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].to_string_lossy().starts_with("instance-"));
    }
    drop(first);
    assert!(matches!(
        test_instance(app_id, &first_dir),
        SingleInstance::FirstInstance(_)
    ));
    drop(second);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn clock_tick_updates_agenda_without_querying_providers_or_opening_hidden_windows() {
    let (ui, _, ctrl, dir) = settings_window(0, "minute-agenda");
    let mut c = ctrl.borrow_mut();
    c.settings_open = false;
    let today = c.cal_state.today;
    c.provider.sources = vec![calendar_core::CalendarSource {
        id: "google:work".into(),
        name: "Work".into(),
        account_label: "Google".into(),
        color_hex: "#008573".into(),
    }];
    c.prefs.selected_calendar_ids.insert("google:work".into());
    c.provider.occurrences = vec![calendar_core::CalendarOccurrence {
        source_id: "google:work".into(),
        event_id: "clock".into(),
        title: "Meeting".into(),
        start_date: today,
        end_date_exclusive: today.add_days(1),
        time_label: "15:00".into(),
        start_minutes: Some(calendar_core::local_minutes(today, 15 * 60)),
        end_minutes: Some(calendar_core::local_minutes(today, 16 * 60)),
        ..Default::default()
    }];
    let request = c.provider_request;
    for (minute, is_now, dimmed) in [
        (15 * 60 - 1, false, false),
        (15 * 60, true, false),
        (16 * 60, false, true),
    ] {
        c.agenda.now_override = Some(calendar_core::local_minutes(today, minute));
        refresh_clock_ui(&mut c, &ui, today);
        let event = ui.get_agenda_events().row_data(0).unwrap();
        assert_eq!(event.is_now, is_now);
        assert_eq!(event.dimmed, dimmed);
        if minute == 15 * 60 - 1 {
            assert_eq!(event.badge, "in 1 min");
        }
        assert_eq!(c.provider_request, request);
    }
    ui.hide().unwrap();
    refresh_clock_ui(&mut c, &ui, today.add_days(1));
    assert!(!ui.window().is_visible());
    assert_eq!(c.cal_state.today, today.add_days(1));
    assert_eq!(c.provider_request, request);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn minute_tick_aligns_to_the_next_wall_clock_boundary() {
    use chrono::TimeZone;
    let minute = chrono::Utc
        .with_ymd_and_hms(2026, 11, 9, 12, 30, 0)
        .unwrap();
    assert_eq!(
        next_minute_delay(&minute),
        std::time::Duration::from_secs(60)
    );
    assert_eq!(
        next_minute_delay(&(minute + chrono::Duration::milliseconds(59_250))),
        std::time::Duration::from_millis(750)
    );
    assert_eq!(
        next_minute_delay(&(minute + chrono::Duration::nanoseconds(59_999_500_000))),
        std::time::Duration::from_millis(1)
    );
}

#[test]
fn automatic_update_entry_points_never_request_interactive_checks_on_macos() {
    let calls = Cell::new(0);
    // The shared dispatcher is used at startup, by the daily timer and by the
    // automatic-check/download toggles. Manual Check for Updates bypasses it.
    for _ in 0..4 {
        dispatch_automatic_update_check(true, false, || calls.set(calls.get() + 1));
    }
    assert_eq!(calls.get(), if cfg!(target_os = "macos") { 0 } else { 4 });
    dispatch_automatic_update_check(false, false, || panic!("automatic checks disabled"));
    dispatch_automatic_update_check(true, true, || {
        panic!("synthetic app must not check updates")
    });
}

#[test]
fn removing_native_sources_disables_and_persists_only_the_native_connection() {
    let directory = std::env::temp_dir().join(format!(
        "lightcal-native-remove-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let mut ctrl = controller(&directory);
    ctrl.prefs
        .selected_calendar_ids
        .insert("google:work".into());
    ctrl.google_connected = true;
    for id in ["mac:fixture", "win:fixture"] {
        ctrl.prefs.windows_calendars_enabled = true;
        assert!(ctrl.disconnect_native_source(id));
        assert!(!ctrl.prefs.windows_calendars_enabled);
        assert!(!ctrl.store.load().preferences.windows_calendars_enabled);
        assert!(ctrl.google_connected);
        assert!(ctrl.prefs.selected_calendar_ids.contains("google:work"));
    }
    ctrl.prefs.windows_calendars_enabled = true;
    for id in ["google:work", "graph:work", "ics:feed", "macaroni"] {
        assert!(!ctrl.disconnect_native_source(id));
        assert!(ctrl.prefs.windows_calendars_enabled);
    }
    let _ = std::fs::remove_dir_all(directory);
}

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

/// Centre of the element with this accessible label, wherever the layout puts it.
fn labelled_center(ui: &AppWindow, label: &str) -> slint::LogicalPosition {
    let element = ElementHandle::find_by_accessible_label(ui, label)
        .next()
        .unwrap_or_else(|| panic!("no element labelled {label:?}"));
    element_center(&element)
}

/// Centre of the switch with this label; a settings row title shares the label.
fn switch_center(ui: &AppWindow, label: &str) -> slint::LogicalPosition {
    let element = ElementHandle::find_by_accessible_label(ui, label)
        .find(|e| e.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::Checkbox))
        .unwrap_or_else(|| panic!("no switch labelled {label:?}"));
    element_center(&element)
}

fn element_center(element: &ElementHandle) -> slint::LogicalPosition {
    let (position, size) = (element.absolute_position(), element.size());
    slint::LogicalPosition::new(
        position.x + size.width / 2.0,
        position.y + size.height / 2.0,
    )
}

fn click_label(ui: &AppWindow, label: &str) {
    let center = labelled_center(ui, label);
    click(ui, center.x, center.y);
}

/// A navigation step: a header control by accessible label, or a point in the grid.
enum Target {
    Label(&'static str),
    Point(f32, f32),
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
        remote: ProviderSnapshot {
            status: lightcal::platform::ProviderStatus::NoCalendars,
            sources: Vec::new(),
            occurrences: Vec::new(),
        },
        local: lightcal::local_events::LocalEvents::in_memory(Vec::new()),
        local_writer: None,
        editor: None,
        editor_panel: None,
        local_save_error: None,
        start_over: Default::default(),
        demo_shown: Default::default(),
        provider_request: 0,
        connection_busy: false,
        connection_message: Default::default(),
        startup_warning: String::new(),
        microsoft_account: lightcal::calendars::account::MicrosoftAccountState::Disconnected,
        provider_loading: false,
        account_before_check: None,
        install_busy: false,
        install_status: String::new(),
        startup_mgr: StartupManager::from_env(dir),
        shortcut_mgr: {
            #[cfg(target_os = "macos")]
            {
                ShortcutManager::isolated()
            }
            #[cfg(not(target_os = "macos"))]
            {
                ShortcutManager::new()
            }
        },
        shortcut_error: String::new(),
        startup_error: String::new(),
        conflicting_shortcut: None,
        update_state: UpdateCheckState::Idle,
        settings_open: true,
        settings_tab: 1,
        #[cfg(windows)]
        tray_icon: None,
        #[cfg(windows)]
        tray_menu: crate::TrayMenuItems::new(LocaleCode::En),
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
        held_microsoft_error: String::new(),
        google_connected: false,
        google_needs_signin: false,
        google_label: String::new(),
        google_error: String::new(),
        microsoft_error: String::new(),
        ics_feed_errors: Default::default(),
        agenda: Default::default(),
        last_refreshed: None,
        // Off screen unless a test says otherwise; never the process-wide native window.
        on_screen: || false,
        // Settled, like snapshots; motion tests turn it on.
        animation_effects: || false,
    }
}

#[test]
fn full_repaint_redraws_a_surface_windows_discarded() {
    use slint::platform::software_renderer::{
        MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
    };
    // winit's software surface reports a reused buffer, as it does after a taskbar restore.
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    slint::platform::set_platform(Box::new(HeadlessPlatform {
        window: window.clone(),
    }))
    .unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-full-repaint-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.settings_open = false;
    ctrl.prefs.theme = ThemePreference::Dark;
    ctrl.sync_to_ui(&ui);
    ui.show().unwrap();
    ui.window()
        .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: 1.5 });
    let (width, height) = (528, 648);
    window.set_size(slint::PhysicalSize::new(width, height));
    let mut pixels = vec![PremultipliedRgbaColor::default(); (width * height) as usize];
    let render = |pixels: &mut [PremultipliedRgbaColor]| {
        slint::platform::update_timers_and_animations();
        let mut region = None;
        window.draw_if_needed(|renderer| region = Some(renderer.render(pixels, width as usize)));
        region.map_or(0, |region| {
            let size = region.bounding_box_size();
            size.width * size.height
        })
    };
    assert_eq!(render(&mut pixels), width * height);

    // Windows dropped the pixels while minimized; nothing in the UI changed.
    let discarded = |pixel: &PremultipliedRgbaColor| {
        (pixel.red, pixel.green, pixel.blue, pixel.alpha) == (255, 0, 255, 255)
    };
    pixels.iter_mut().for_each(|pixel| {
        *pixel = PremultipliedRgbaColor {
            red: 255,
            green: 0,
            blue: 255,
            alpha: 255,
        }
    });
    ui.window().request_redraw();
    assert_eq!(
        render(&mut pixels),
        0,
        "A plain redraw presents nothing, which left the restored window blank"
    );

    request_full_repaint(&ui);
    assert_eq!(render(&mut pixels), width * height);
    assert!(
        !pixels.iter().any(discarded),
        "Every pixel of the restored window is painted again"
    );
    // The layer is never visible: the window looks the same as before the request.
    let before = pixels.clone();
    request_full_repaint(&ui);
    render(&mut pixels);
    assert!(pixels
        .iter()
        .zip(&before)
        .all(|(a, b)| (a.red, a.green, a.blue, a.alpha) == (b.red, b.green, b.blue, b.alpha)));
    let _ = std::fs::remove_dir_all(&dir);
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
    // Compare only the title: from 16 px to just before the previous-month arrow at its
    // leftmost, which is with Today showing. Font metrics differ by platform (Linux CI has
    // no Segoe UI), so measure the arrow instead of assuming its position.
    ui.set_today_hidden(false);
    let title_end = ElementHandle::find_by_accessible_label(&ui, "Previous month")
        .next()
        .expect("previous-month arrow")
        .absolute_position()
        .x;
    ctrl.borrow().sync_to_ui(&ui);
    let title_end_px = ((title_end - 2.0) * 1.5) as usize;
    assert!(title_end_px > 150, "room for the month title");
    let header_pixels = |pixels: &[PremultipliedRgbaColor]| -> Vec<u8> {
        (15..51)
            .flat_map(|y| (24..title_end_px).map(move |x| y * 528 + x))
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
    // Keyed by month and whether Today is folded: Today takes room from the title, so a
    // wide fallback font (Linux CI has no Segoe UI) can lay the same month out differently.
    let mut rendered_titles = std::collections::BTreeMap::from([(
        ("November 2026", ui.get_today_hidden()),
        previous_title.clone(),
    )]);
    use Target::{Label, Point};
    const PREV: Target = Label("Previous month");
    const NEXT: Target = Label("Next month");
    const TODAY: Target = Label("Today");
    for (step, (target, month, changed_title)) in [
        (NEXT, "December 2026", true),
        (NEXT, "January 2027", true),
        (PREV, "December 2026", true),
        (PREV, "November 2026", true),
        // On today Today is folded away and the arrows sit together.
        (PREV, "October 2026", true),
        (Point(187.0, 200.0), "October 2026", false),
        (NEXT, "November 2026", true),
        (Point(145.0, 212.0), "November 2026", false),
        (TODAY, "November 2026", false),
        (Point(187.0, 200.0), "November 2026", false),
        (NEXT, "December 2026", true),
        (NEXT, "January 2027", true),
        (PREV, "December 2026", true),
        (TODAY, "November 2026", true),
        (Point(145.0, 212.0), "November 2026", false),
        (PREV, "October 2026", true),
    ]
    .into_iter()
    .enumerate()
    {
        let position = match target {
            Label(label) => labelled_center(&ui, label),
            Point(x, y) => slint::LogicalPosition::new(x, y),
        };
        ui.window()
            .dispatch_event(WindowEvent::PointerMoved { position });
        click(&ui, position.x, position.y);
        if matches!(target, Point(..)) {
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
        let key = (month, ui.get_today_hidden());
        if let Some(expected) = rendered_titles.get(&key) {
            assert!(
                &title == expected,
                "The rendered heading matches {month} at step {step}"
            );
        } else {
            rendered_titles.insert(key, title.clone());
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
            microsoft_error: String::new(),
            ics_feed_errors: Default::default(),
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
        click(&ui, 380.0, 138.0);
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
    click(&ui, 380.0, 138.0);
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

fn scroll(ui: &AppWindow, delta_y: f32) {
    ui.window().dispatch_event(WindowEvent::PointerScrolled {
        position: slint::LogicalPosition::new(220.0, 220.0),
        delta_x: 0.0,
        delta_y,
    });
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
    let dir = std::env::temp_dir().join(format!(
        "lightcal-ui-recovery-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    ui.show().unwrap();

    // Fresh users reach Windows calendars from the provider list (Accounts empty state).
    let weak = ui.as_weak();
    ui.on_toggle_legacy_windows(move || {
        let ui = weak.upgrade().unwrap();
        ui.set_legacy_windows_enabled(!ui.get_legacy_windows_enabled());
    });
    renderer
        .render_to_png(&dir.join("windows-disabled.png"), 448, 360)
        .unwrap();
    // The On this PC card sits above the provider list, so the Windows row is found by
    // its label after scrolling it into view.
    ui.set_settings_strings(SettingsStrings {
        windows_title: "Windows calendars".into(),
        ..ui.get_settings_strings()
    });
    scroll(&ui, -400.0);
    let windows_row = labelled_center(&ui, "Windows calendars");
    click(&ui, windows_row.x, windows_row.y);
    assert!(
        ui.get_legacy_windows_enabled(),
        "Fresh users can enable Windows calendars"
    );
    click(&ui, windows_row.x, windows_row.y);
    assert!(
        !ui.get_legacy_windows_enabled(),
        "The same provider row toggles them again"
    );

    // Shortcut key caps keep a reachable clear button in every language.
    ui.set_settings_tab(2);
    ui.set_shortcut_keys(ModelRc::new(VecModel::from(vec![
        SharedString::from("Alt"),
        SharedString::from("Shift"),
        SharedString::from("C"),
    ])));
    ui.set_shortcut_can_clear(true);
    let cleared = Rc::new(Cell::new(false));
    let observed = cleared.clone();
    ui.on_clear_shortcut(move || observed.set(true));
    for locale in LocaleCode::ALL {
        let strings = catalog(locale);
        ui.set_label_shortcut_text(strings.label_shortcut.into());
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
        click(&ui, 405.0, 186.0);
        assert!(
            cleared.get(),
            "Clear remains reachable in {}",
            locale.as_str()
        );
    }

    // Recording a shortcut sends the key and modifiers to Rust.
    let recorded = Rc::new(RefCell::new(None));
    let observed = recorded.clone();
    ui.on_record_shortcut(move |text, ctrl, alt, shift, meta| {
        *observed.borrow_mut() = shortcut_from_key(text.as_str(), ctrl, alt, shift, meta);
    });
    click(&ui, 340.0, 186.0);
    renderer
        .render_to_png(&dir.join("shortcut-recording.png"), 448, 360)
        .unwrap();
    for (key, pressed) in [
        (slint::platform::Key::Control, true),
        (slint::platform::Key::Alt, true),
    ] {
        let text = SharedString::from(key);
        ui.window().dispatch_event(if pressed {
            WindowEvent::KeyPressed { text }
        } else {
            WindowEvent::KeyReleased { text }
        });
    }
    ui.window()
        .dispatch_event(WindowEvent::KeyPressed { text: "k".into() });
    assert_eq!(
        recorded.borrow().as_deref(),
        Some("Ctrl + Alt + K"),
        "The recorder captures Ctrl + Alt + K"
    );

    // Account cards: refresh, expired sign-in and calendar selection.
    let mut ctrl = controller(&dir);
    ctrl.settings_tab = 1;
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
    // The On this PC card (44 px and an 8 px gap, plus its border) sits above.
    click(&ui, 372.0, 200.0);
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
    click(&ui, 395.0, 247.0);
    assert!(
        reconnected.get(),
        "An expired Google account offers Sign in"
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
    click(&ui, 220.0, 248.0);
    assert!(
        selected.get(),
        "Windows permission denial must not remove a readable calendar's checkbox"
    );
    scroll(&ui, -1000.0);
    renderer
        .render_to_png(&dir.join("mixed-permission-recovery.png"), 448, 360)
        .unwrap();
    let privacy_opened = Rc::new(Cell::new(false));
    let observed = privacy_opened.clone();
    ui.on_open_privacy_settings(move || observed.set(true));
    refreshed.set(false);
    click(&ui, 366.0, 238.0);
    assert!(
        privacy_opened.get(),
        "Permission recovery remains reachable"
    );
    click(&ui, 372.0, 187.0);
    assert!(refreshed.get(), "Permission recovery still offers a retry");

    // About: links first, then both automatic update switches.
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
    let opened = Rc::new(RefCell::new(Vec::new()));
    let observed = opened.clone();
    ui.on_open_external_url(move |url| observed.borrow_mut().push(url.to_string()));
    renderer
        .render_to_png(&dir.join("about-links.png"), 448, 360)
        .unwrap();
    click(&ui, 220.0, 195.0);
    click(&ui, 220.0, 235.0);
    click(&ui, 220.0, 275.0);
    assert_eq!(
        *opened.borrow(),
        [
            "https://github.com/andmetoo/lightcal",
            "https://koona.app",
            "https://github.com/andmetoo/lightcal/blob/main/LICENSE",
        ],
        "About links are reachable without scrolling"
    );
    for locale in LocaleCode::ALL {
        ctrl.prefs.set_locale(locale);
        ctrl.prefs.auto_check_updates = true;
        ctrl.prefs.auto_install_updates = true;
        ctrl.sync_to_ui(&ui);
        scroll(&ui, -1000.0);
        renderer
            .render_to_png(
                &dir.join(format!("about-updates-{}.png", locale.as_str())),
                448,
                360,
            )
            .unwrap();
        click(&ui, 400.0, 250.0);
        assert!(
            !ui.get_auto_check_updates(),
            "Automatic checking can be disabled in {}",
            locale.as_str()
        );
        click(&ui, 400.0, 290.0);
        assert!(
            !ui.get_auto_install_updates(),
            "Automatic installation can be disabled in {}",
            locale.as_str()
        );
        scroll(&ui, 1000.0);
    }

    // Private ICS link: one field; a pending subscription cannot be sent twice.
    ctrl.prefs.set_locale(LocaleCode::En);
    ctrl.settings_tab = 1;
    ctrl.ics_dialog_open = true;
    ctrl.sync_to_ui(&ui);
    ui.set_ics_source_url("http://example.invalid/calendar.ics".into());
    let subscribed = Rc::new(Cell::new(false));
    let observed = subscribed.clone();
    ui.on_add_ics_source(move |_, _| observed.set(true));
    ui.set_connection_busy(true);
    renderer
        .render_to_png(&dir.join("ics-loading.png"), 448, 360)
        .unwrap();
    click(&ui, 392.0, 249.0);
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
    click(&ui, 392.0, 249.0);
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
    // Add event comes first (LocalCal board), so Next sits one 26 px line lower.
    click(&ui, 60.0, 364.0);
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

#[test]
fn past_agenda_rows_keep_their_calendar_bar_at_full_color() {
    use slint::platform::software_renderer::{
        MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
    };
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    slint::platform::set_platform(Box::new(HeadlessPlatform {
        window: window.clone(),
    }))
    .unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-past-bar-{}",
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
    ctrl.sync_to_ui(&ui);
    ui.show().unwrap();
    window.set_size(slint::PhysicalSize::new(352, 432));

    let rows = ui.get_agenda_events();
    let (past, now, upcoming) = (
        rows.row_data(0).unwrap(),
        rows.row_data(1).unwrap(),
        rows.row_data(2).unwrap(),
    );
    assert!(past.dimmed && !now.dimmed && !upcoming.dimmed);
    let (width, height) = (352, 432);
    let mut pixels = vec![PremultipliedRgbaColor::default(); width * height];
    slint::platform::update_timers_and_animations();
    window.draw_if_needed(|renderer| {
        renderer.render(&mut pixels, width);
    });
    let rgb = |x: usize, y: usize| {
        let pixel = pixels[y * width + x];
        [pixel.red, pixel.green, pixel.blue]
    };

    // Find a row's 3 × 16 px bar wherever the layout puts it: the vertical
    // runs, at least 12 px tall, of exactly that row's color. Day dots and the
    // all-day chip dot in the same colors are too short to qualify.
    let bars_of = |item: &AgendaEventItem| {
        let swatch = item.swatch.light;
        let swatch = [swatch.red(), swatch.green(), swatch.blue()];
        let mut bars: Vec<(usize, usize)> = Vec::new();
        for x in 0..width {
            let mut y = 0;
            while y < height {
                let mut end = y;
                while end < height && rgb(x, end) == swatch {
                    end += 1;
                }
                if end - y >= 12 && !bars.iter().any(|&(top, _)| top.abs_diff(y) <= 2) {
                    bars.push((y, end));
                }
                y = end.max(y + 1);
            }
        }
        bars.sort_unstable();
        (swatch, bars)
    };

    // The current and upcoming rows are never dimmed, so their bars set the
    // row pitch and, one pitch above the current row, where the past row is.
    let (_, now_bars) = bars_of(&now);
    let &[now_bar] = now_bars.as_slice() else {
        panic!("one bar in the current row's color: {now_bars:?}");
    };
    let (_, upcoming_bars) = bars_of(&upcoming);
    let upcoming_bar = *upcoming_bars
        .iter()
        .find(|&&(top, _)| top > now_bar.0)
        .unwrap_or_else(|| {
            panic!("the upcoming row's bar sits below the current one: {upcoming_bars:?}")
        });
    let pitch = upcoming_bar.0 - now_bar.0;
    let past_band = (now_bar.0 - pitch, now_bar.1 - pitch);

    let (swatch, past_bars) = bars_of(&past);
    assert!(
        past_bars.iter().any(|&(top, _)| top.abs_diff(past_band.0) <= 1),
        "the past row's bar is drawn in its full color {swatch:?} at y = {}, not faded: {past_bars:?}",
        past_band.0
    );

    // The past row's title is still dimmed: its darkest text pixel is much
    // lighter than the upcoming row's.
    let darkest = |(top, bottom): (usize, usize)| {
        (top..bottom)
            .flat_map(|y| (110..width - 16).map(move |x| (x, y)))
            .map(|(x, y)| rgb(x, y).iter().map(|&c| u32::from(c)).sum::<u32>() / 3)
            .min()
            .unwrap()
    };
    let (past_ink, upcoming_ink) = (darkest(past_band), darkest(upcoming_bar));
    assert!(
        past_ink > upcoming_ink + 40,
        "past text is dimmed: darkest {past_ink} vs {upcoming_ink}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn recorded_keys_become_registrable_shortcuts() {
    // The physical key wins over the produced text: Ctrl+Alt+M types "\r", Shift+2 types "@".
    assert_eq!(
        shortcut_from_press(Some("M"), "\r", true, true, false, false).as_deref(),
        Some("Ctrl + Alt + M")
    );
    assert_eq!(
        shortcut_from_press(Some("2"), "@", true, false, true, false).as_deref(),
        Some("Ctrl + Shift + 2")
    );
    assert_eq!(
        shortcut_from_press(Some("M"), "\r", false, false, true, false),
        None
    );
    // Backspace, Tab and Enter share codes with Ctrl+H/I/J/M and are never recorded as letters.
    for named in ["\u{8}", "\t", "\n", "\r"] {
        assert_eq!(
            shortcut_from_key(named, true, false, false, false),
            None,
            "{named:?}"
        );
    }
    assert_eq!(
        shortcut_from_key("k", true, true, false, false).as_deref(),
        Some("Ctrl + Alt + K")
    );
    // Ctrl+K may arrive as the ASCII control character 0x0B.
    assert_eq!(
        shortcut_from_key("\u{b}", true, false, true, false).as_deref(),
        Some("Ctrl + Shift + K")
    );
    let f5 = SharedString::from(slint::platform::Key::F5);
    assert_eq!(
        shortcut_from_key(f5.as_str(), false, true, false, false).as_deref(),
        Some("Alt + F5")
    );
    #[cfg(target_os = "macos")]
    {
        let recorded = shortcut_from_key(f5.as_str(), true, false, false, false).unwrap();
        assert_eq!(
            ShortcutManager::isolated()
                .bind(Some(&recorded))
                .unwrap()
                .as_deref(),
            Some("Ctrl + F5")
        );
    }
    assert_eq!(shortcut_from_key("k", false, false, true, false), None);
    assert_eq!(shortcut_from_key("-", true, false, false, false), None);
    assert_eq!(
        ics_name_from_address("https://user@calendar.example.com:443/private/basic.ics"),
        "calendar.example.com"
    );
}

#[test]
fn failed_startup_and_unconfigured_connections_use_the_selected_language() {
    use lightcal::i18n::{calendar_failure_text, startup_failure_text, CalendarFailure};
    let (ui, _, ctrl, dir) = settings_window(1, "translated-connection-failures");
    ACTIVE_CONTROLLER.with(|active| *active.borrow_mut() = Some(Rc::downgrade(&ctrl)));
    wire_startup_setting(&ui, &ctrl, |_, _, _| {
        Err("Check Lightcal in System Settings → General → Login Items".into())
    });
    wire_outlook_sign_in(&ui, &ctrl, true);
    wire_google_sign_in(&ui, &ctrl, true);
    ctrl.borrow_mut().prefs.microsoft_client_id.clear();
    ctrl.borrow_mut().prefs.google_client_id.clear();
    for locale in LocaleCode::ALL {
        ctrl.borrow_mut().prefs.set_locale(locale);
        ui.invoke_toggle_start_with_windows();
        assert!(!ctrl.borrow().prefs.start_with_windows);
        assert_eq!(
            ui.get_startup_status_text(),
            startup_failure_text(locale, cfg!(target_os = "macos"))
        );
        if locale != LocaleCode::En {
            assert!(!ui.get_startup_status_text().contains("Check Lightcal"));
        }
        ui.invoke_connect_microsoft();
        assert_eq!(
            ui.get_connection_message(),
            calendar_failure_text(locale, CalendarFailure::OutlookNotConfigured)
        );
        ui.invoke_choose_google_account();
        assert_eq!(
            ui.get_connection_message(),
            calendar_failure_text(locale, CalendarFailure::GoogleNotConfigured)
        );
    }
    // Translate an already visible failure without another sign-in or startup action.
    ctrl.borrow_mut().prefs.set_locale(LocaleCode::Fr);
    ctrl.borrow().sync_to_ui(&ui);
    assert_eq!(
        ui.get_connection_message(),
        "La connexion à Google n’est pas disponible dans cette version de Lightcal."
    );
    ACTIVE_CONTROLLER.with(|active| *active.borrow_mut() = None);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn rejected_ics_inputs_and_provider_card_failures_are_translated_at_presentation() {
    use lightcal::i18n::{calendar_failure_text, CalendarFailure};
    let (ui, _, ctrl, dir) = settings_window(1, "translated-calendar-failures");
    let mut credentials = lightcal::secure_store::Credentials::default();
    let name_error =
        lightcal::calendars::add_feed(&mut credentials, "", "https://example.test/basic.ics")
            .unwrap_err();
    lightcal::calendars::add_feed(&mut credentials, "Work", "https://example.test/basic.ics")
        .unwrap();
    let duplicate =
        lightcal::calendars::add_feed(&mut credentials, "Work", "https://example.test/basic.ics")
            .unwrap_err();
    let invalid_link =
        lightcal::calendars::add_feed(&mut credentials, "Work", "http://example.test/basic.ics")
            .unwrap_err();
    let oversized = "x".repeat(8 * 1024 * 1024 + 1);
    let size_error = lightcal::calendars::ics::parse(
        &oversized,
        &credentials.ics[0],
        ctrl.borrow().cal_state.today,
        rrule::Tz::UTC,
    )
    .unwrap_err();
    {
        let mut c = ctrl.borrow_mut();
        c.google_connected = true;
        c.google_error = "Google sign-in timed out; try connecting again".into();
        c.microsoft_account =
            lightcal::calendars::account::MicrosoftAccountState::Connected { label: None };
        c.microsoft_error = "Invalid Microsoft calendar response".into();
        c.provider.sources.push(calendar_core::CalendarSource {
            id: "ics:fixture".into(),
            name: "Holidays".into(),
            account_label: "ICS".into(),
            color_hex: "#008573".into(),
        });
        c.ics_feed_errors
            .insert("ics:fixture".into(), size_error.clone());
    }
    for locale in LocaleCode::ALL {
        ctrl.borrow_mut().prefs.set_locale(locale);
        for (raw, kind) in [
            (&name_error, CalendarFailure::CalendarName),
            (&duplicate, CalendarFailure::Duplicate),
            (&invalid_link, CalendarFailure::Https),
            (&size_error, CalendarFailure::CalendarTooLarge),
        ] {
            ctrl.borrow_mut().connection_message = raw.clone().into();
            ctrl.borrow().sync_to_ui(&ui);
            assert_eq!(
                ui.get_connection_message(),
                calendar_failure_text(locale, kind)
            );
        }
        let cards: Vec<_> = ui.get_account_cards().iter().collect();
        for (id, kind) in [
            ("google", CalendarFailure::SignInTimeout),
            ("outlook", CalendarFailure::InvalidResponse),
            ("ics:fixture", CalendarFailure::CalendarTooLarge),
        ] {
            let card = cards.iter().find(|c| c.id == id).unwrap();
            assert_eq!(card.notice, calendar_failure_text(locale, kind));
        }
        ctrl.borrow_mut().provider.status = lightcal::platform::ProviderStatus::ProviderUnavailable(
            "HTTPS request failed or timed out".into(),
        );
        ctrl.borrow().sync_to_ui(&ui);
        assert_eq!(
            ui.get_provider_error_text(),
            calendar_failure_text(locale, CalendarFailure::Network)
        );
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn mac_settings_close_is_a_compact_neutral_control_with_the_same_action() {
    let (ui, window, ctrl, dir) = settings_window(2, "mac-close-control");
    ui.set_macos(true);
    window.request_redraw();
    window.draw_if_needed(|_| {});
    let label = lightcal::i18n::settings_text(LocaleCode::En).close;
    let close = ElementHandle::find_by_accessible_label(&ui, label)
        .find(|element| {
            element.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::Button)
        })
        .unwrap();
    assert_eq!(close.size().width, 28.0);
    assert_eq!(close.size().height, 28.0);
    let center = element_center(&close);
    click(&ui, center.x, center.y);
    assert!(!ctrl.borrow().settings_open);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn korean_available_update_title_has_no_particle_hedge() {
    let (ui, _, ctrl, dir) = settings_window(3, "korean-update-title");
    let mut c = ctrl.borrow_mut();
    c.prefs.set_locale(LocaleCode::Ko);
    c.update_state = UpdateCheckState::UpdateAvailable {
        latest_version: "1.2.3".into(),
        release_url: "https://github.com/andmetoo/lightcal/releases/tag/v1.2.3".into(),
        current_version: "1.2.2".into(),
    };
    c.sync_to_ui(&ui);
    assert_eq!(
        ui.get_update_ready_title_text(),
        "새 버전이 있습니다: 1.2.3"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn agenda_actions_use_the_row_that_was_rendered() {
    let platform = HeadlessPlatform::new();
    slint::platform::set_platform(Box::new(platform)).unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-agenda-race-{}",
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
    ctrl.sync_to_ui(&ui);
    let walk = (0..ui.get_agenda_events().row_count())
        .map(|row| ui.get_agenda_events().row_data(row).unwrap())
        .find(|row| row.title == "Weekend walk")
        .expect("the sample day has the walk");

    // A background refresh removes an earlier event before the UI re-syncs.
    ctrl.provider
        .occurrences
        .retain(|event| event.title != "Coffee with Maya");
    ctrl.toggle_agenda_event(&walk.key);
    ctrl.sync_to_ui(&ui);
    let expanded: Vec<_> = (0..ui.get_agenda_events().row_count())
        .map(|row| ui.get_agenda_events().row_data(row).unwrap())
        .filter(|row| row.expanded)
        .map(|row| row.title.to_string())
        .collect();
    assert_eq!(expanded, ["Weekend walk"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn agenda_details_fade_only_after_a_click_and_copied_crossfades() {
    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    use std::time::Duration;
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(HeadlessPlatform {
        window: window.clone(),
    }))
    .unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-agenda-motion-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    lightcal::snapshot::use_test_time();
    let mut ctrl = controller(&dir);
    ctrl.animation_effects = || true;
    ctrl.settings_open = false;
    ctrl.provider = crate::snapshot_render::canvas_month();
    ctrl.prefs.selected_calendar_ids = ctrl
        .provider
        .sources
        .iter()
        .map(|source| source.id.clone())
        .collect();
    // Friday 13 November has one event, so its actions are on screen (as in the snapshots).
    ctrl.cal_state
        .select_date(LocalDate::new(2026, 11, 13).unwrap());
    ctrl.sync_to_ui(&ui);
    ui.show().unwrap();
    let size = (352, 432);
    window.set_size(slint::PhysicalSize::new(352, 432));
    ink_after(&window, size, Duration::from_millis(50));
    let review = (0..ui.get_agenda_events().row_count())
        .map(|row| ui.get_agenda_events().row_data(row).unwrap())
        .find(|row| row.title == "Design review")
        .expect("13 November has the design review");
    // Changes over time: the first frame differs from one 400 ms later, which is steady.
    let moves = || {
        let first = ink_after(&window, size, Duration::ZERO);
        let settled = ink_after(&window, size, Duration::from_millis(400));
        assert_eq!(ink_after(&window, size, Duration::from_millis(50)), settled);
        first != settled
    };

    // A click expands the review: its details fade in, even when a refresh syncs again
    // before the next frame.
    let faded = Rc::new(RefCell::new(Vec::new()));
    let log = faded.clone();
    ui.on_agenda_details_faded(move |key| log.borrow_mut().push(key));
    ctrl.toggle_agenda_event(&review.key);
    ctrl.sync_to_ui(&ui);
    ctrl.sync_to_ui(&ui);
    assert!(moves(), "details fade in after a click");
    assert_eq!(*faded.borrow(), std::slice::from_ref(&review.key));
    for key in faded.borrow().iter() {
        ctrl.agenda_details_faded(key);
    }
    // A refresh brings another event that day: the rows are rebuilt, and the open details
    // appear at once.
    let mut extra = ctrl
        .provider
        .occurrences
        .iter()
        .find(|event| event.title == "Design review")
        .unwrap()
        .clone();
    extra.event_id.push_str("-later");
    extra.title = "Design follow-up".to_string();
    ctrl.provider.occurrences.push(extra);
    ctrl.sync_to_ui(&ui);
    assert!(!moves(), "rebuilt rows do not fade the open details again");
    // Another day and back: the still-open details appear at once.
    let review_day = ctrl.cal_state.selected_date;
    ctrl.cal_state.select_date(review_day.add_days(1));
    ctrl.sync_to_ui(&ui);
    ink_after(&window, size, Duration::from_millis(50));
    ctrl.cal_state.select_date(review_day);
    ctrl.sync_to_ui(&ui);
    assert!(!moves(), "details shown again on their day do not fade");
    // Any later sync leaves them as they are.
    ctrl.sync_to_ui(&ui);
    assert!(!moves(), "a sync does not fade them again");
    // Settings and back rebuilds the calendar: the open details appear at once.
    ctrl.settings_open = true;
    ctrl.sync_to_ui(&ui);
    ink_after(&window, size, Duration::from_millis(50));
    ctrl.settings_open = false;
    ctrl.sync_to_ui(&ui);
    assert!(!moves(), "details shown with the view do not fade");
    // Copy crossfades the action's label to Copied.
    ctrl.copy_agenda_event(&review.key)
        .expect("the review can be copied");
    ctrl.sync_to_ui(&ui);
    assert!(moves(), "Copy crossfades to Copied");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn date_size_sets_calendar_height_from_theme_metrics() {
    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    slint::platform::set_platform(Box::new(HeadlessPlatform { window })).unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-date-size-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.settings_open = false;
    for (size, height) in [
        (DateSize::Standard, 432.0),
        (DateSize::Large, 432.0),
        (DateSize::Larger, 444.0),
    ] {
        ctrl.prefs.date_size = size;
        ctrl.sync_to_ui(&ui);
        assert_eq!(ui.get_date_size(), CalendarDateSize::from(size));
        assert_eq!(calendar_height(&ui), height);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn koi_pace_follows_what_the_user_can_see() {
    use lightcal::platform::WindowView;
    let seen = WindowView {
        on_screen: true,
        seen: true,
        animation_effects: true,
    };
    assert_eq!(koi_pace(true, false, seen), KoiPace::Swim);
    assert_eq!(koi_pace(false, false, seen), KoiPace::Off);
    assert_eq!(koi_pace(true, true, seen), KoiPace::Off);
    let hidden = WindowView {
        on_screen: false,
        seen: false,
        ..seen
    };
    assert_eq!(koi_pace(true, false, hidden), KoiPace::Off);
    let covered = WindowView {
        seen: false,
        ..seen
    };
    assert_eq!(koi_pace(true, false, covered), KoiPace::Watch);
    let still = WindowView {
        animation_effects: false,
        ..seen
    };
    assert_eq!(koi_pace(true, false, still), KoiPace::Watch);
}

#[test]
fn koi_clock_swims_watches_and_stops_with_the_window() {
    use lightcal::platform::WindowView;
    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    slint::platform::set_platform(Box::new(HeadlessPlatform { window })).unwrap();
    let ui = AppWindow::new().unwrap();
    ui.set_show_koi(true);
    let view = |on_screen, seen, animation_effects| WindowView {
        on_screen,
        seen,
        animation_effects,
    };
    let swimming = || KOI_TIMER.with(|t| t.running());
    let watching = || KOI_WATCHING.with(Cell::get);

    // Hidden: nothing runs.
    wake_koi_with(&ui, view(false, false, true));
    assert!(!swimming() && !watching());

    // Shown and seen: swims, and the first frame already has the current time.
    ui.set_koi_clock(-1.0);
    wake_koi_with(&ui, view(true, true, true));
    assert!(swimming() && !watching());
    assert_eq!(KOI_TIMER.with(|t| t.interval()), KOI_FRAME);
    assert!(ui.get_koi_clock() >= 0.0);

    // Covered: the koi hold still with no timer and wait for window changes.
    ui.set_koi_clock(-1.0);
    wake_koi_with(&ui, view(true, false, true));
    assert!(!swimming() && watching());
    assert_eq!(ui.get_koi_clock(), -1.0);

    // Uncovered again: swims, and stops watching.
    wake_koi_with(&ui, view(true, true, true));
    assert!(swimming() && !watching());

    // Animation effects off: still and watching.
    wake_koi_with(&ui, view(true, true, false));
    assert!(!swimming() && watching());

    // Settings open, or hidden: nothing runs.
    ui.set_settings_open(true);
    wake_koi_with(&ui, view(true, true, true));
    assert!(!swimming() && !watching());
    ui.set_settings_open(false);
    wake_koi_with(&ui, view(true, false, true));
    wake_koi_with(&ui, view(false, false, true));
    assert!(!swimming() && !watching());
}

#[test]
fn koi_clock_wraps_on_a_whole_number_of_every_loop_and_tail_beat() {
    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    slint::platform::set_platform(Box::new(HeadlessPlatform { window })).unwrap();
    let ui = AppWindow::new().unwrap();
    let school = ui.global::<KoiSchool>().get_fish();
    assert!(school.row_count() > 0);
    // Periods arrive as f32, so allow for their rounding.
    let whole = |value: f64| (value - value.round()).abs() < 1e-3;
    for fish in school.iter() {
        // Whole loop frequencies bring every fish back to the same pose after each lap.
        assert!(whole(fish.a.into()) && whole(fish.b.into()), "{fish:?}");
        for period in [fish.period, fish.beat] {
            assert!(
                whole(KOI_CLOCK_WRAP_SECS / f64::from(period)),
                "{KOI_CLOCK_WRAP_SECS} s is not a whole number of {period} s"
            );
        }
    }
}

#[test]
fn koi_pond_switch_is_off_by_default_and_saves_the_choice() {
    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    slint::platform::set_platform(Box::new(HeadlessPlatform {
        window: window.clone(),
    }))
    .unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-koi-pond-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let mut ctrl = controller(&dir);
    ctrl.settings_open = true;
    ctrl.settings_tab = 0;
    let ctrl = Rc::new(RefCell::new(ctrl));
    wire_koi_pond(&ui, &ctrl);
    ui.show().unwrap();
    ctrl.borrow().sync_to_ui(&ui);
    window.set_size(slint::PhysicalSize::new(448, 360));
    window.draw_if_needed(|_| {});
    // Koi pond is the last Calendar row, below the fold.
    scroll(&ui, -1000.0);
    window.draw_if_needed(|_| {});
    assert!(!ui.get_show_koi());

    let switch = switch_center(&ui, "Koi pond");
    click(&ui, switch.x, switch.y);
    assert!(ctrl.borrow().prefs.show_koi);
    assert!(ui.get_show_koi());
    let saved = PreferencesStore::new(dir.clone()).load();
    assert!(saved.preferences.show_koi);

    click(&ui, switch.x, switch.y);
    assert!(!ctrl.borrow().prefs.show_koi);
    assert!(!ui.get_show_koi());
    let _ = std::fs::remove_dir_all(&dir);
}

/// The select whose row is titled `label` ("Week starts on", "Language").
fn select_handle(ui: &AppWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(ui, label)
        .find(|e| e.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::Combobox))
        .unwrap_or_else(|| panic!("no select labelled {label:?}"))
}

/// Opens a select's menu with a click, as a user would.
fn open_select(ui: &AppWindow, label: &str) {
    let center = element_center(&select_handle(ui, label));
    click(ui, center.x, center.y);
}

/// Where the popup of the select labelled `label` sits in the 448 px window, with the menu
/// `menu_width` wide: GridSelect's own placement (4 px under the select, right-aligned with
/// it, 16 px from either window edge). Options report positions inside the popup, so a click
/// on one adds this origin; if the placement changed, those clicks would miss and fail.
fn popup_origin(ui: &AppWindow, label: &str, menu_width: f32) -> slint::LogicalPosition {
    let select = select_handle(ui, label);
    let (at, size) = (select.absolute_position(), select.size());
    let (window, margin) = (448.0_f32, 16.0_f32);
    let width = menu_width.min(window - 2.0 * margin);
    let right = (at.x + size.width).min(window - margin);
    slint::LogicalPosition::new((right - width).max(margin), at.y + size.height + 4.0)
}

/// Whether the select's menu is open: its popup holds the key focus.
fn menu_open(ui: &AppWindow, label: &str) -> bool {
    select_handle(ui, label).accessible_expanded() == Some(true)
}

/// A menu option; `None` when no open menu shows it.
fn menu_option(ui: &AppWindow, label: &str) -> Option<ElementHandle> {
    ElementHandle::find_by_accessible_label(ui, label)
        .find(|e| e.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::Button))
}

/// Clicks the menu option `label` in the popup placed at `origin`.
fn click_option(ui: &AppWindow, origin: slint::LogicalPosition, label: &str) {
    let option = menu_option(ui, label).unwrap_or_else(|| panic!("no menu option {label:?}"));
    let center = element_center(&option);
    click(ui, origin.x + center.x, origin.y + center.y);
}

/// A headless window showing Settings on `tab`, wired like the app but with no native
/// window to resize. Key focus starts on the window, as when the app shows it.
fn settings_window(
    tab: i32,
    name: &str,
) -> (
    AppWindow,
    Rc<slint::platform::software_renderer::MinimalSoftwareWindow>,
    Rc<RefCell<AppController>>,
    std::path::PathBuf,
) {
    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    slint::platform::set_platform(Box::new(HeadlessPlatform {
        window: window.clone(),
    }))
    .unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-{name}-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let mut ctrl = controller(&dir);
    ctrl.open_settings_tab(tab);
    let ctrl = Rc::new(RefCell::new(ctrl));
    wire_language(&ui, &ctrl);
    wire_first_weekday(&ui, &ctrl);
    wire_settings_navigation(&ui, &ctrl, |_, _| {});
    ui.show().unwrap();
    ctrl.borrow().sync_to_ui(&ui);
    window.set_size(slint::PhysicalSize::new(448, 360));
    window.draw_if_needed(|_| {});
    ui.invoke_focus_calendar_keys();
    (ui, window, ctrl, dir)
}

const WEEK: &str = "Week starts on";

#[test]
fn settings_menus_close_like_native_drop_downs() {
    let (ui, window, ctrl, dir) = settings_window(0, "settings-menus");

    // Escape closes the menu and leaves Settings open.
    open_select(&ui, WEEK);
    assert!(menu_open(&ui, WEEK));
    assert!(menu_option(&ui, "Tuesday").is_some());
    press_key(&ui, SharedString::from(slint::platform::Key::Escape));
    assert!(!menu_open(&ui, WEEK));
    assert!(menu_option(&ui, "Tuesday").is_none(), "the popup is gone");
    assert!(ctrl.borrow().settings_open, "Escape closed only the menu");

    // A click outside the menu closes it and reaches nothing else: the Event dots switch
    // under the click is not toggled. With the menu closed, the same click toggles it.
    let dot_toggles = Rc::new(Cell::new(0));
    let counter = Rc::clone(&dot_toggles);
    ui.on_toggle_show_event_dots(move || counter.set(counter.get() + 1));
    window.draw_if_needed(|_| {});
    let away = switch_center(&ui, "Event dots");
    open_select(&ui, WEEK);
    click(&ui, away.x, away.y);
    assert!(
        menu_option(&ui, "Tuesday").is_none(),
        "a click away closes the menu"
    );
    assert_eq!(dot_toggles.get(), 0, "the click away reached only the menu");
    window.draw_if_needed(|_| {});
    click(&ui, away.x, away.y);
    assert_eq!(
        dot_toggles.get(),
        1,
        "with the menu closed the switch works"
    );

    // While the menu is open the window is modal to it: a wheel turn outside reaches
    // nothing under it, so the tab cannot scroll away from the menu, and the menu stays.
    open_select(&ui, WEEK);
    ui.window().dispatch_event(WindowEvent::PointerScrolled {
        position: slint::LogicalPosition::new(away.x, away.y),
        delta_x: 0.0,
        delta_y: -40.0,
    });
    assert!(menu_open(&ui, WEEK));
    press_key(&ui, SharedString::from(slint::platform::Key::Escape));

    // A tab switch or closing Settings leaves no menu behind.
    open_select(&ui, WEEK);
    ui.invoke_open_settings(2);
    assert!(menu_option(&ui, "Tuesday").is_none());
    ui.invoke_open_settings(0);
    open_select(&ui, WEEK);
    ui.invoke_close_settings();
    window.draw_if_needed(|_| {});
    assert!(
        menu_option(&ui, "Tuesday").is_none(),
        "no menu over the calendar"
    );
    ui.invoke_open_settings(0);
    window.draw_if_needed(|_| {});
    assert!(menu_option(&ui, "Tuesday").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn week_start_menu_works_from_the_keyboard() {
    let (ui, window, ctrl, dir) = settings_window(0, "week-start-keys");
    let key = |k: slint::platform::Key| press_key(&ui, SharedString::from(k));
    use slint::platform::Key;

    // Click the select, then close the menu: the select keeps the focus.
    open_select(&ui, WEEK);
    key(Key::Escape);
    assert!(!menu_open(&ui, WEEK));

    // Enter opens the menu with the cursor on the current day (Monday). Down moves to
    // Tuesday, Right jumps a column to Friday, End goes to Sunday, Left back to Thursday.
    key(Key::Return);
    assert!(menu_open(&ui, WEEK), "Enter opens the menu");
    key(Key::DownArrow);
    key(Key::RightArrow);
    key(Key::End);
    key(Key::LeftArrow);
    // Enter chooses the day under the cursor and closes the menu.
    key(Key::Return);
    assert!(!menu_open(&ui, WEEK));
    assert_eq!(ctrl.borrow().prefs.first_weekday, Weekday::Thursday);

    // Down opens it too; Home and Space choose Monday.
    key(Key::DownArrow);
    assert!(menu_open(&ui, WEEK), "Down opens the menu");
    key(Key::Home);
    press_key(&ui, " ".into());
    assert_eq!(ctrl.borrow().prefs.first_weekday, Weekday::Monday);

    // Tab leaves an open menu without choosing anything.
    key(Key::Return);
    key(Key::DownArrow);
    key(Key::Tab);
    assert!(!menu_open(&ui, WEEK));
    assert_eq!(ctrl.borrow().prefs.first_weekday, Weekday::Monday);
    window.draw_if_needed(|_| {});
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn week_start_menu_offers_every_day_and_saves_the_choice() {
    let (ui, window, ctrl, dir) = settings_window(0, "week-start-menu");
    assert_eq!(
        select_handle(&ui, WEEK).accessible_value().as_deref(),
        Some("Monday")
    );

    // Three columns, top to bottom: Monday-Wednesday, Thursday-Saturday, Sunday. Option
    // positions are measured inside the popup; the popup's origin places them in the window.
    open_select(&ui, WEEK);
    let origin = popup_origin(&ui, WEEK, 400.0);
    let footer_top = ElementHandle::find_by_accessible_label(&ui, "About Lightcal")
        .next()
        .unwrap()
        .absolute_position()
        .y;
    window.draw_if_needed(|_| {});
    let at = |label: &str| menu_option(&ui, label).unwrap().absolute_position();
    let (monday, tuesday, thursday, sunday) =
        (at("Monday"), at("Tuesday"), at("Thursday"), at("Sunday"));
    assert!(thursday.x > monday.x && sunday.x > thursday.x);
    assert!((thursday.y - monday.y).abs() < 1.0 && (sunday.y - monday.y).abs() < 1.0);
    assert!(tuesday.y > monday.y && (tuesday.x - monday.x).abs() < 1.0);
    // Every option lies within the window's 16 px margins and above the footer.
    for day in Weekday::ALL {
        let option = menu_option(&ui, lightcal::i18n::weekday_full(LocaleCode::En, day)).unwrap();
        let (at, size) = (option.absolute_position(), option.size());
        let (left, right) = (origin.x + at.x, origin.x + at.x + size.width);
        assert!(
            left >= 16.0 && right <= 432.0,
            "{day:?} from {left} to {right}"
        );
        assert!(origin.y + at.y + size.height <= footer_top, "{day:?}");
    }

    // A sync while the menu is open keeps the same entries instead of rebuilding them.
    let entries = ui.get_first_weekday_entries();
    ctrl.borrow().sync_to_ui(&ui);
    assert!(entries == ui.get_first_weekday_entries());
    assert!(menu_open(&ui, WEEK), "a sync leaves the menu open");

    click_option(&ui, origin, "Saturday");
    {
        let c = ctrl.borrow();
        assert_eq!(c.prefs.first_weekday, Weekday::Saturday);
        assert_eq!(c.cal_state.first_weekday, Weekday::Saturday);
    }
    assert!(!menu_open(&ui, WEEK));
    assert_eq!(
        select_handle(&ui, WEEK).accessible_value().as_deref(),
        Some("Saturday")
    );
    let saved = PreferencesStore::new(dir.clone()).load();
    assert_eq!(saved.preferences.first_weekday, Weekday::Saturday);
    // The shaded-day chips start with Saturday too.
    let chips = ui.get_gray_weekday_chips();
    assert_eq!(
        chips.row_data(0).unwrap().index,
        Weekday::Saturday.monday_index() as i32
    );

    // An index outside Monday..Sunday is ignored and nothing is saved.
    for bad in [-1, 7, i32::MAX] {
        ui.invoke_select_first_weekday_option(bad);
        assert_eq!(ctrl.borrow().prefs.first_weekday, Weekday::Saturday);
    }
    let saved = PreferencesStore::new(dir.clone()).load();
    assert_eq!(saved.preferences.first_weekday, Weekday::Saturday);

    // A language change rewrites the labels in place; the select shows the new name.
    ctrl.borrow_mut().prefs.set_locale(LocaleCode::Ru);
    ctrl.borrow().sync_to_ui(&ui);
    assert!(entries == ui.get_first_weekday_entries());
    assert_eq!(
        entries.row_data(5).map(|e| e.label).as_deref(),
        Some("Суббота")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn language_menu_columns_select_and_save_each_language() {
    let (ui, window, ctrl, dir) = settings_window(2, "language-menu");

    // Latin scripts fill the left column, Cyrillic and CJK the right, top to bottom.
    open_select(&ui, "Language");
    window.draw_if_needed(|_| {});
    let at = |locale: LocaleCode| {
        menu_option(&ui, locale.display_name())
            .unwrap()
            .absolute_position()
    };
    let (left, right) = (at(LocaleCode::PtBr), at(LocaleCode::ZhHans));
    assert!(
        right.x > left.x,
        "Simplified Chinese is in the right column"
    );
    assert!(
        (right.y - left.y).abs() < 1.0,
        "both columns have five rows"
    );
    assert!(at(LocaleCode::En).y < left.y);
    assert!(at(LocaleCode::Ru).y < right.y);
    press_key(&ui, SharedString::from(slint::platform::Key::Escape));

    // Every language can be picked from the menu and is saved under its code. The row's
    // title is translated, so find the select by the current language's own label.
    for locale in LocaleCode::ALL.into_iter().rev() {
        let title = lightcal::i18n::catalog(ctrl.borrow().prefs.locale()).label_language;
        open_select(&ui, title);
        let origin = popup_origin(&ui, title, 328.0);
        click_option(&ui, origin, locale.display_name());
        assert_eq!(ctrl.borrow().prefs.locale(), locale);
        let title = lightcal::i18n::catalog(locale).label_language;
        assert!(!menu_open(&ui, title));
        assert_eq!(
            select_handle(&ui, title).accessible_value().as_deref(),
            Some(locale.display_name())
        );
        let saved = PreferencesStore::new(dir.clone()).load();
        assert_eq!(saved.preferences.language, locale.as_str());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn opening_a_menu_stops_shortcut_recording() {
    let (ui, window, _ctrl, dir) = settings_window(2, "menu-while-recording");
    let recording = || {
        ElementHandle::find_by_accessible_label(&ui, "Press a key combination…")
            .next()
            .is_some()
    };
    let field = menu_option(&ui, "Open shortcut").expect("the shortcut field");
    let center = element_center(&field);
    click(&ui, center.x, center.y);
    window.draw_if_needed(|_| {});
    assert!(recording());

    // The menu takes the key focus, so recording stops instead of catching keys meant
    // for the menu; Escape then closes the menu and leaves Settings open.
    open_select(&ui, "Language");
    window.draw_if_needed(|_| {});
    assert!(menu_open(&ui, "Language"));
    assert!(!recording(), "recording stopped when the field lost focus");
    press_key(&ui, SharedString::from(slint::platform::Key::Escape));
    assert!(!menu_open(&ui, "Language"));
    assert!(!recording());
    assert!(ui.get_settings_open());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn date_size_control_saves_the_choice_and_grows_the_calendar() {
    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    slint::platform::set_platform(Box::new(HeadlessPlatform {
        window: window.clone(),
    }))
    .unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-date-size-control-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let mut ctrl = controller(&dir);
    ctrl.settings_open = true;
    ctrl.settings_tab = 0;
    let ctrl = Rc::new(RefCell::new(ctrl));
    wire_date_size(&ui, &ctrl);
    ui.show().unwrap();
    ctrl.borrow().sync_to_ui(&ui);
    window.set_size(slint::PhysicalSize::new(448, 360));
    window.draw_if_needed(|_| {});

    click_label(&ui, "Larger");
    assert_eq!(ctrl.borrow().prefs.date_size, DateSize::Larger);
    assert_eq!(ui.get_date_size(), CalendarDateSize::Larger);
    let saved = PreferencesStore::new(dir.clone()).load();
    assert_eq!(saved.preferences.date_size, DateSize::Larger);

    ctrl.borrow_mut().settings_open = false;
    ctrl.borrow().sync_to_ui(&ui);
    assert_eq!(calendar_height(&ui), 444.0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn header_hide_button_hides_the_calendar_like_escape() {
    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    slint::platform::set_platform(Box::new(HeadlessPlatform {
        window: window.clone(),
    }))
    .unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-hide-button-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.settings_open = false;
    let hidden = Rc::new(Cell::new(0));
    let observed = hidden.clone();
    ui.on_dismiss_compact(move || observed.set(observed.get() + 1));
    ui.show().unwrap();
    ctrl.sync_to_ui(&ui);
    window.set_size(slint::PhysicalSize::new(352, 432));
    window.draw_if_needed(|_| {});

    click_label(&ui, "Hide to tray");
    assert_eq!(hidden.get(), 1);
    // Settings, next to it, does not hide the calendar.
    click_label(&ui, "Settings");
    assert_eq!(hidden.get(), 1);
    // The − button sits after Settings, at the end of the header.
    assert!(labelled_center(&ui, "Hide to tray").x > labelled_center(&ui, "Settings").x);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn today_folds_between_the_arrows_on_the_current_day() {
    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    slint::platform::set_platform(Box::new(HeadlessPlatform {
        window: window.clone(),
    }))
    .unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-today-fold-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.settings_open = false;
    ui.show().unwrap();
    window.set_size(slint::PhysicalSize::new(352, 432));
    let arrow_gap = |ui: &AppWindow| {
        let prev = ElementHandle::find_by_accessible_label(ui, "Previous month")
            .next()
            .unwrap();
        let next = ElementHandle::find_by_accessible_label(ui, "Next month")
            .next()
            .unwrap();
        next.absolute_position().x - (prev.absolute_position().x + prev.size().width)
    };
    let today = |ui: &AppWindow| {
        ElementHandle::find_by_accessible_label(ui, "Today")
            .next()
            .unwrap()
    };

    // On today: Today is folded to nothing and leaves the element tree.
    ctrl.sync_to_ui(&ui);
    window.draw_if_needed(|_| {});
    assert!(ui.get_today_hidden());
    let close = |a: f32, b: f32| (a - b).abs() < 0.01;
    assert!(
        close(arrow_gap(&ui), 4.0),
        "only the two 2 px layout gaps remain"
    );
    assert!(
        ElementHandle::find_by_accessible_label(&ui, "Today")
            .next()
            .is_none(),
        "a folded Today is hidden from clicks and screen readers"
    );

    // A month away: the arrows move apart and Today sits between them.
    ctrl.cal_state.navigate_months(1);
    ctrl.sync_to_ui(&ui);
    window.draw_if_needed(|_| {});
    assert!(!ui.get_today_hidden());
    let today_width = today(&ui).size().width;
    assert!(today_width > 0.0);
    assert!(close(arrow_gap(&ui), 4.0 + today_width));
    assert_eq!(today(&ui).computed_opacity(), 1.0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn header_animates_only_while_the_popup_is_on_screen() {
    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    slint::platform::set_platform(Box::new(HeadlessPlatform { window })).unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-header-motion-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.settings_open = false;
    ui.show().unwrap();

    // Off screen (hidden, taskbar-minimized, snapshots; the tray Today item syncs before
    // showing): the header settles. main_window_is_open's native states are covered by
    // shell.rs's native_window_state_follows_taskbar_minimize_restore_and_hide.
    ctrl.cal_state.navigate_months(1);
    ctrl.sync_to_ui(&ui);
    assert!(!ui.get_header_motion());

    // On screen: every change synced animates, whatever triggered it (arrows, wheel,
    // keys, the agenda's Next link), with Animation effects on.
    ctrl.on_screen = || true;
    ctrl.animation_effects = || true;
    ctrl.cal_state.return_to_today();
    ctrl.sync_to_ui(&ui);
    assert!(ui.get_header_motion());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Ink in the window after `wait` on the test clock (`use_test_time`): how far its pixels
/// are from white, summed. A fading panel shows what lies beneath it in its first frame,
/// so its ink differs from the settled frame.
fn ink_after(
    window: &slint::platform::software_renderer::MinimalSoftwareWindow,
    (width, height): (usize, usize),
    wait: std::time::Duration,
) -> u64 {
    use slint::platform::software_renderer::PremultipliedRgbaColor;
    lightcal::snapshot::advance_test_time(wait);
    slint::platform::update_timers_and_animations();
    window.request_redraw();
    let mut pixels = vec![PremultipliedRgbaColor::default(); width * height];
    window.draw_if_needed(|renderer| {
        renderer.render(&mut pixels, width);
    });
    pixels
        .iter()
        .map(|p| 3 * 255 - u64::from(p.red) - u64::from(p.green) - u64::from(p.blue))
        .sum()
}

#[test]
fn opening_settings_shows_the_page_at_once() {
    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    use std::time::Duration;
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(HeadlessPlatform {
        window: window.clone(),
    }))
    .unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-settings-open-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    lightcal::snapshot::use_test_time();
    let mut ctrl = controller(&dir);
    ctrl.animation_effects = || true;
    ctrl.settings_open = false;
    ctrl.sync_to_ui(&ui);
    ui.show().unwrap();
    window.set_size(slint::PhysicalSize::new(352, 432));
    ink_after(&window, (352, 432), Duration::from_millis(50));

    // Only the window changes, as before motion: no fade over the resize.
    ctrl.settings_open = true;
    ctrl.settings_tab = 2;
    ctrl.sync_to_ui(&ui);
    window.set_size(slint::PhysicalSize::new(448, 360));
    let size = (448, 360);
    let first = ink_after(&window, size, Duration::ZERO);
    let settled = ink_after(&window, size, Duration::from_millis(400));
    assert_eq!(first, settled, "Settings opens without a fade");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn month_changes_fade_only_on_screen_with_animation_effects() {
    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    slint::platform::set_platform(Box::new(HeadlessPlatform { window })).unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-month-fade-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.animation_effects = || true;
    ctrl.settings_open = false;
    ctrl.sync_to_ui(&ui);
    let settled = |ui: &AppWindow| ui.get_month_fade() == 1.0;

    // Off screen (hidden popup, tray Today, snapshots): settled.
    ctrl.cal_state.navigate_months(1);
    ctrl.sync_to_ui(&ui);
    assert!(settled(&ui));
    assert!(ui.global::<Motion>().get_enabled());

    // On screen: the new month fades in, in either direction; nothing moves.
    ctrl.on_screen = || true;
    ctrl.cal_state.navigate_months(1);
    ctrl.sync_to_ui(&ui);
    assert_eq!(ui.get_month_fade(), 0.0);
    // A change during a fade carries on from the opacity on screen, never blanking again.
    let fade = ui.get_month_fade();
    ctrl.cal_state.navigate_months(-1);
    ctrl.sync_to_ui(&ui);
    assert!(ui.get_month_fade() >= fade);
    // A sync in the same month leaves a fade alone; hiding settles it.
    let fade = ui.get_month_fade();
    ctrl.sync_to_ui(&ui);
    assert_eq!(ui.get_month_fade(), fade);
    month_fade::settle_month_fade(&ui);
    assert!(settled(&ui));
    ctrl.cal_state.navigate_months(-1);
    ctrl.sync_to_ui(&ui);
    assert_eq!(ui.get_month_fade(), 0.0);
    month_fade::settle_month_fade(&ui);

    // Animation effects off: no fade, no header motion, and every transition is instant.
    ctrl.animation_effects = || false;
    ctrl.cal_state.navigate_months(1);
    ctrl.sync_to_ui(&ui);
    assert!(settled(&ui));
    assert!(!ui.get_header_motion());
    assert!(!ui.global::<Motion>().get_enabled());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hover_follows_whole_day_cells_and_hides_for_arrow_keys() {
    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    slint::platform::set_platform(Box::new(HeadlessPlatform {
        window: window.clone(),
    }))
    .unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-hover-follow-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.settings_open = false;
    ctrl.sync_to_ui(&ui);
    let reported = Rc::new(RefCell::new(Vec::new()));
    let log = reported.clone();
    ui.on_hover_cell(move |index| log.borrow_mut().push(index));
    ui.show().unwrap();
    ui.window()
        .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: 1.5 });
    window.set_size(slint::PhysicalSize::new(528, 648));

    // The grid starts below the 44 px header and the 24 px weekday row, 12 px in.
    let lead = if ui.get_show_week_numbers() {
        24.0
    } else {
        0.0
    };
    let col_width = (352.0 - 24.0 - lead) / 7.0;
    let at = |col: f32, row: f32, dx: f32, dy: f32| {
        slint::LogicalPosition::new(12.0 + lead + col * col_width + dx, 68.0 + row * 32.0 + dy)
    };
    let move_to = |position| {
        ui.window()
            .dispatch_event(WindowEvent::PointerMoved { position })
    };

    move_to(at(2.0, 1.0, col_width / 2.0, 16.0));
    assert_eq!(ui.get_hover_cell_index(), 9);
    // Beside and above the 34 × 28 px tile is still the same cell, so a sweep along a row
    // never leaves the grid between tiles.
    move_to(at(2.0, 1.0, col_width - 1.0, 1.0));
    assert_eq!(ui.get_hover_cell_index(), 9);
    move_to(at(3.0, 1.0, 1.0, 16.0));
    assert_eq!(ui.get_hover_cell_index(), 10);
    move_to(at(3.0, 2.0, col_width / 2.0, 1.0));
    assert_eq!(ui.get_hover_cell_index(), 17);
    // The weekday letters are off the day rows.
    move_to(slint::LogicalPosition::new(
        12.0 + lead + 3.5 * col_width,
        56.0,
    ));
    assert_eq!(ui.get_hover_cell_index(), -1);
    assert_eq!(*reported.borrow(), [9, 10, 17, -1]);

    // Arrow keys hide the highlight; the next pointer move, even within the cell, brings
    // it back.
    move_to(at(4.0, 3.0, col_width / 2.0, 16.0));
    hover_follow::hide_hover_for_keys(&ui);
    assert_eq!(ui.get_hover_cell_index(), -1);
    move_to(at(4.0, 3.0, col_width / 2.0 + 1.0, 16.0));
    assert_eq!(ui.get_hover_cell_index(), 25);
    assert_eq!(*reported.borrow(), [9, 10, 17, -1, 25, 25]);

    // A sync rebuilds the cells model but not the day tiles, so leaving the window after
    // a click still reports leaving and the highlight cannot get stuck.
    ctrl.sync_to_ui(&ui);
    ui.window().dispatch_event(WindowEvent::PointerExited);
    assert_eq!(ui.get_hover_cell_index(), -1);
    assert_eq!(reported.borrow().last(), Some(&-1));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_app_reads_the_native_window_for_header_motion() {
    // main() wires the controller to the native probe; headless there is no native
    // "Lightcal" window in this process, so it reports off screen.
    assert!(!popup_on_screen());
}

#[cfg(windows)]
#[test]
fn display_change_refresh_follows_preferences_and_skips_a_busy_controller() {
    let dir = std::env::temp_dir().join(format!(
        "lightcal-icon-refresh-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let ctrl = Rc::new(RefCell::new(controller(&dir)));
    ACTIVE_CONTROLLER.with(|active| *active.borrow_mut() = Some(Rc::downgrade(&ctrl)));
    let synced = || SYNCED_ICONS.with(|synced| synced.get());

    sync_icons_if_idle();
    let first = synced().expect("an idle controller syncs the icons");
    assert_eq!(first.mode, ctrl.borrow().prefs.tray_icon_mode);
    assert_eq!(
        first.small_size,
        lightcal::platform::windows::icon::native_small_icon_size()
    );

    // A busy controller (a callback running a nested message loop) is skipped, not borrowed.
    {
        let mut busy = ctrl.borrow_mut();
        busy.prefs.tray_icon_mode = TrayIconMode::TimeAndDate;
        sync_icons_if_idle();
    }
    assert_eq!(synced(), Some(first));

    // The next refresh catches up with what changed meanwhile.
    sync_icons_if_idle();
    assert_eq!(synced().unwrap().mode, TrayIconMode::TimeAndDate);
    assert!(synced().unwrap().hour_min.is_some());

    ACTIVE_CONTROLLER.with(|active| *active.borrow_mut() = None);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn reanchor_runs_once_after_a_burst_of_changes() {
    slint::platform::set_platform(Box::new(HeadlessPlatform::new())).unwrap();
    lightcal::snapshot::use_test_time();
    let runs = Rc::new(Cell::new(0));
    let last = Rc::new(Cell::new(0));
    let step = REANCHOR_QUIET / 2;
    // Changes closer together than the quiet time keep pushing the move back, and only
    // the last change's move runs.
    for change in 1..=4 {
        let (counter, latest) = (runs.clone(), last.clone());
        after_display_quiet(move || {
            counter.set(counter.get() + 1);
            latest.set(change);
        });
        lightcal::snapshot::advance_test_time(step);
        slint::platform::update_timers_and_animations();
        assert_eq!(runs.get(), 0);
    }
    lightcal::snapshot::advance_test_time(REANCHOR_QUIET);
    slint::platform::update_timers_and_animations();
    assert_eq!((runs.get(), last.get()), (1, 4));
    lightcal::snapshot::advance_test_time(REANCHOR_QUIET * 3);
    slint::platform::update_timers_and_animations();
    assert_eq!(runs.get(), 1, "a single-shot move");
}

#[test]
fn local_events_join_the_calendar_without_changing_provider_status() {
    use lightcal::local_events::{samples, LocalEvents, SOURCE_ID};
    let dir = std::env::temp_dir().join(format!(
        "lightcal-local-view-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    let today = ctrl.cal_state.today;
    ctrl.local = LocalEvents::in_memory(samples(today));
    ctrl.set_remote_snapshot(ProviderSnapshot {
        status: lightcal::platform::ProviderStatus::NoCalendars,
        sources: Vec::new(),
        occurrences: Vec::new(),
    });
    assert_eq!(
        ctrl.provider.status,
        lightcal::platform::ProviderStatus::NoCalendars,
        "the local calendar never hides an empty provider"
    );
    assert_eq!(ctrl.provider.sources[0].id, SOURCE_ID);
    assert_eq!(ctrl.provider.sources[0].name, "On this PC");
    assert_eq!(ctrl.provider.occurrences.len(), 3);

    // A provider refresh or failure leaves local events alone.
    ctrl.set_remote_snapshot(ProviderSnapshot {
        status: lightcal::platform::ProviderStatus::ProviderUnavailable("offline".into()),
        sources: Vec::new(),
        occurrences: Vec::new(),
    });
    assert_eq!(ctrl.provider.occurrences.len(), 3);
    assert!(ctrl.remote.occurrences.is_empty());

    // The name follows the language.
    ctrl.prefs.set_locale(LocaleCode::De);
    ctrl.refresh_local_view();
    assert_eq!(ctrl.provider.sources[0].name, "Auf diesem PC");
    assert_eq!(
        ctrl.provider.status,
        lightcal::platform::ProviderStatus::ProviderUnavailable("offline".into())
    );

    // Local dots appear only while the local calendar is selected.
    let grid_with = |ctrl: &AppController| {
        ctrl.cal_state
            .build_grid(
                &ctrl.provider.occurrences,
                &ctrl.prefs.selected_calendar_ids,
            )
            .cells
            .iter()
            .map(|cell| cell.total_event_count)
            .sum::<usize>()
    };
    assert_eq!(grid_with(&ctrl), 0);
    ctrl.prefs.selected_calendar_ids.insert(SOURCE_ID.into());
    assert!(grid_with(&ctrl) > 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_recovered_local_load_is_saved_back_at_once() {
    use lightcal::local_events::{LocalEvents, Storage, Writer};
    let dir = std::env::temp_dir().join(format!(
        "lightcal-local-repair-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let storage = Storage::new(&dir);
    let event = lightcal::local_events::LocalEvent::validated(
        "local:repair".into(),
        "Restored",
        LocalDate::new(2026, 11, 12).unwrap(),
        lightcal::local_events::Timing::AllDay,
        None,
        None,
    )
    .unwrap();
    storage.save(std::slice::from_ref(&event), false).unwrap();
    storage.save(std::slice::from_ref(&event), false).unwrap();
    std::fs::remove_file(storage.file_path()).unwrap();

    let mut ctrl = controller(&dir);
    let writer = Writer::spawn(storage.clone(), |_| {}).unwrap();
    ctrl.local_writer = Some(writer.clone());
    ctrl.apply_local_events(LocalEvents::from_loaded(storage.load()));
    assert!(writer.wait_saved(std::time::Duration::from_secs(5)));
    assert!(storage.file_path().exists(), "the main file is repaired");
    assert!(storage.load().notices.is_empty());
    assert_eq!(ctrl.provider.occurrences.len(), 1);
    writer.shutdown();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_lock_timeout_marks_unavailable_only_while_the_lock_is_awaited() {
    use lightcal::local_events::{Access, LocalEvents, Unavailable};
    let dir = std::env::temp_dir().join(format!(
        "lightcal-local-timeout-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.local = LocalEvents::loading();
    assert!(
        !ctrl.local_lock_timeout(true),
        "lock held: a slow read stays loading"
    );
    assert!(ctrl.local.is_loading());
    assert!(ctrl.local_lock_timeout(false));
    assert!(!ctrl.local.is_loading());
    assert_eq!(
        ctrl.local.access(),
        &Access::Unavailable(Unavailable::Locked)
    );
    assert!(!ctrl.local_lock_timeout(false), "only once");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn nothing_shows_the_window_while_quit_waits_for_local_events() {
    slint::platform::set_platform(Box::new(HeadlessPlatform::new())).unwrap();
    let ui = AppWindow::new().unwrap();
    crate::QUITTING.with(|quitting| quitting.set(true));
    // The shortcut, a second launch and the tray all show the window through here.
    crate::show_application_window(&ui);
    let visible = ui.window().is_visible();
    crate::QUITTING.with(|quitting| quitting.set(false));
    assert!(!visible);
}

#[test]
fn demo_mode_shows_its_calendars_without_saving_them() {
    use lightcal::local_events::SOURCE_ID;
    let dir = std::env::temp_dir().join(format!(
        "lightcal-demo-shown-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.demo_shown.insert(SOURCE_ID.into());
    ctrl.prefs.selected_calendar_ids.insert(SOURCE_ID.into());
    ctrl.toggle_calendar("win-work".into());
    assert!(ctrl.prefs.selected_calendar_ids.contains(SOURCE_ID));
    let saved = |ctrl: &AppController| ctrl.store.load().preferences.selected_calendar_ids;
    assert!(
        saved(&ctrl).contains("win-work"),
        "the user's toggle is saved"
    );
    assert!(
        !saved(&ctrl).contains(SOURCE_ID),
        "demo's own choice is not"
    );

    // Hiding a calendar only demo mode showed matches the saved selection.
    ctrl.toggle_calendar(SOURCE_ID.into());
    assert!(!ctrl.prefs.selected_calendar_ids.contains(SOURCE_ID));
    assert!(!saved(&ctrl).contains(SOURCE_ID));
    // Showing it again is the user's choice and is saved.
    ctrl.toggle_calendar(SOURCE_ID.into());
    assert!(saved(&ctrl).contains(SOURCE_ID));
    let _ = std::fs::remove_dir_all(dir);
}

fn press_key(ui: &AppWindow, text: SharedString) {
    ui.window()
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    ui.window()
        .dispatch_event(WindowEvent::KeyReleased { text });
}

fn type_text(ui: &AppWindow, text: &str) {
    for ch in text.chars() {
        press_key(ui, ch.to_string().into());
    }
}

/// Holds Ctrl while `key` is pressed and released.
fn press_with_ctrl(ui: &AppWindow, key: SharedString) {
    let control = SharedString::from(slint::platform::Key::Control);
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: control.clone(),
    });
    press_key(ui, key);
    ui.window()
        .dispatch_event(WindowEvent::KeyReleased { text: control });
}

#[test]
fn the_local_event_editor_adds_edits_and_deletes_events() {
    use lightcal::local_events::SOURCE_ID;
    let platform = HeadlessPlatform::new();
    let window = platform.window.clone();
    slint::platform::set_platform(Box::new(platform)).unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-editor-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.settings_open = false;
    ctrl.refresh_local_view();
    let ctrl = Rc::new(RefCell::new(ctrl));
    calendar_view::connect_agenda_callbacks(&ui, &ctrl, |_| {});
    local_editor::connect_local_editor_callbacks(&ui, &ctrl, |_| {});
    ctrl.borrow().sync_to_ui(&ui);
    ui.show().unwrap();
    window.set_size(slint::PhysicalSize::new(352, 432));
    ui.invoke_focus_calendar_keys();
    let return_key = || SharedString::from(slint::platform::Key::Return);

    // N opens the editor on the selected day, with the title focused; Enter saves.
    press_key(&ui, "N".into());
    assert!(ui.get_editor_open(), "N opens the editor");
    assert!(!ui.get_editor_editing());
    type_text(&ui, "Dentist");
    assert_eq!(ui.get_editor_title(), "Dentist", "the title has the focus");
    press_key(&ui, return_key());
    assert!(!ui.get_editor_open(), "Enter in the title saves");
    {
        let c = ctrl.borrow();
        assert_eq!(c.local.events().len(), 1);
        assert!(
            c.prefs.selected_calendar_ids.contains(SOURCE_ID),
            "saving selects On this PC, so the event stays in view"
        );
    }
    let saved = ui.get_agenda_events().row_data(0).unwrap();
    assert_eq!(saved.title, "Dentist");
    assert_eq!(saved.time_label, "09:00–10:00");
    assert!(saved.is_local && saved.expanded && saved.can_edit && saved.can_delete);
    assert!(!saved.can_open);

    // An empty title and reversed times are both reported; nothing is saved.
    ui.invoke_edit_event(saved.key.clone());
    assert!(ui.get_editor_editing());
    assert_eq!(ui.get_editor_title(), "Dentist");
    ui.set_editor_title("".into());
    ui.set_editor_start("18:00".into());
    ui.set_editor_end("17:30".into());
    ui.invoke_save_editor();
    assert!(ui.get_editor_open());
    assert_eq!(ui.get_editor_title_error(), "Add a title.");
    assert_eq!(ui.get_editor_time_error(), "End must be after start.");

    // Ctrl+Enter in the multi-line note saves (ADR 0017): it reaches the editor before
    // the note can turn it into a line break.
    ui.set_editor_title("Dentist".into());
    ui.set_editor_start("09:00".into());
    ui.set_editor_end("10:00".into());
    // Tab moves through the fields in order: Location, then the note (below the fold).
    ui.invoke_cancel_editor();
    ui.invoke_edit_event(saved.key.clone());
    click_label(&ui, "Location");
    press_key(&ui, SharedString::from(slint::platform::Key::Tab));
    type_text(&ui, "Card");
    press_with_ctrl(&ui, return_key());
    assert!(!ui.get_editor_open(), "Ctrl+Enter in the note saves");
    assert_eq!(
        ctrl.borrow().local.events()[0].note.as_deref(),
        Some("Card"),
        "no line break was added"
    );

    // Escape cancels without saving.
    ui.invoke_focus_calendar_keys();
    press_key(&ui, "n".into());
    type_text(&ui, "Discarded");
    press_key(&ui, SharedString::from(slint::platform::Key::Escape));
    assert!(!ui.get_editor_open(), "Escape cancels");
    assert_eq!(ctrl.borrow().local.events().len(), 1);

    // Right-clicking a day selects it and opens the editor there.
    let position = slint::LogicalPosition::new(188.0, 148.0);
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position,
        button: PointerEventButton::Right,
    });
    ui.window().dispatch_event(WindowEvent::PointerReleased {
        position,
        button: PointerEventButton::Right,
    });
    assert!(ui.get_editor_open(), "right-click opens the editor");
    assert_eq!(
        ctrl.borrow().cal_state.selected_date,
        LocalDate::new(2026, 11, 12).unwrap(),
        "on the day that was clicked"
    );
    // A second right-click while the editor is open only moves the date: what was
    // typed stays.
    ui.set_editor_title("Half typed".into());
    let other = slint::LogicalPosition::new(230.0, 148.0);
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position: other,
        button: PointerEventButton::Right,
    });
    ui.window().dispatch_event(WindowEvent::PointerReleased {
        position: other,
        button: PointerEventButton::Right,
    });
    assert!(ui.get_editor_open());
    assert_eq!(ui.get_editor_title(), "Half typed");
    assert_eq!(
        ctrl.borrow().cal_state.selected_date,
        LocalDate::new(2026, 11, 13).unwrap()
    );
    ui.invoke_cancel_editor();

    // Delete asks first, in the row.
    {
        let mut c = ctrl.borrow_mut();
        let first = LocalDate::new(2026, 11, 1).unwrap();
        c.cal_state.select_date(first);
        c.sync_to_ui(&ui);
    }
    let row = ui.get_agenda_events().row_data(0).unwrap();
    ui.invoke_ask_delete_event(row.key.clone());
    assert!(ui.get_agenda_events().row_data(0).unwrap().confirming);
    ui.invoke_keep_event(row.key.clone());
    assert!(!ui.get_agenda_events().row_data(0).unwrap().confirming);
    ui.invoke_confirm_delete_event(row.key.clone());
    assert_eq!(
        ctrl.borrow().local.events().len(),
        1,
        "Delete needs its question"
    );
    ui.invoke_ask_delete_event(row.key.clone());
    ui.invoke_confirm_delete_event(row.key.clone());
    assert!(ctrl.borrow().local.events().is_empty());
    assert!(ui.get_agenda_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn unavailable_local_events_open_the_panel_instead_of_the_editor() {
    use lightcal::local_events::{Loaded, LocalEvents, Unavailable};
    let platform = HeadlessPlatform::new();
    slint::platform::set_platform(Box::new(platform)).unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-panel-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.settings_open = false;
    ctrl.local = LocalEvents::from_loaded(Loaded::unavailable(Unavailable::Damaged {
        file: dir.join("local-events.json"),
    }));
    let ctrl = Rc::new(RefCell::new(ctrl));
    local_editor::connect_local_editor_callbacks(&ui, &ctrl, |_| {});
    ctrl.borrow().sync_to_ui(&ui);
    assert_eq!(ui.get_strip_text(), "Events on this PC are unavailable.");
    assert!(ui.get_strip_error());
    ui.invoke_new_event(-1);
    assert!(!ui.get_editor_open());
    assert!(ui.get_panel_open());
    assert_eq!(ui.get_panel_title(), "Can\u{2019}t add events right now");
    assert!(ui.get_panel_accounts());
    ui.invoke_panel_back();
    assert!(!ui.get_panel_open());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_on_this_pc_card_shows_one_notice_at_a_time() {
    use lightcal::local_events::{Loaded, LocalEvents, Notice, Unavailable};
    let dir = std::env::temp_dir().join(format!(
        "lightcal-local-card-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);

    // Editable with no notice: name, "Local · No events", the selection.
    let card = ctrl.local_card();
    assert_eq!(card.name, "On this PC");
    assert_eq!(card.sub, "Local · No events");
    assert_eq!(card.notice, 0);
    assert!(!card.checked);

    // A failed save: amber with Try now.
    ctrl.local_save_error = Some("Access is denied. (os error 5)".into());
    let card = ctrl.local_card();
    assert_eq!(card.notice, 2);
    assert!(card.retry);
    assert!(card.text.contains("Access is denied. (os error 5)."));
    ctrl.local_save_error = None;

    // A damaged file: red, names the kept file, Show in folder and Start over, which asks.
    let kept = dir.join("local-events.json");
    ctrl.local = LocalEvents::from_loaded(Loaded::unavailable(Unavailable::Damaged {
        file: kept.clone(),
    }));
    let card = ctrl.local_card();
    assert_eq!(card.notice, 3);
    assert_eq!(card.sub, "Local · Unavailable");
    assert_eq!(card.head, "Events on this PC couldn\u{2019}t be read");
    assert!(card.file.ends_with("local-events.json"));
    assert!(card.folder && card.start_over && !card.confirm);
    assert_eq!(ctrl.local_card_file(), Some(kept));
    ctrl.start_over = local_editor::StartOver::Asking;
    let card = ctrl.local_card();
    assert!(card.confirm);
    assert_eq!(card.head, "Start over with no events?");
    ctrl.start_over = local_editor::StartOver::Idle;

    // The writer did not start: red, read only, never Start over.
    ctrl.local = LocalEvents::from_loaded(Loaded::unavailable(Unavailable::CannotSave {
        error: "the disk is full".into(),
    }));
    let card = ctrl.local_card();
    assert_eq!(card.notice, 3);
    assert_eq!(card.sub, "Local · Read only");
    assert!(!card.start_over && !card.folder);
    ctrl.start_over = local_editor::StartOver::Asking;
    assert!(
        !ctrl.local_card().confirm,
        "no Start over for a healthy file"
    );
    ctrl.start_over = local_editor::StartOver::Idle;

    // Locked: a quiet line with the spinner.
    ctrl.local = LocalEvents::from_loaded(Loaded::unavailable(Unavailable::Locked));
    let card = ctrl.local_card();
    assert_eq!((card.notice, card.spinner), (1, true));

    // A restored backup: amber, the damaged file, Show in folder and OK, which dismisses.
    let mut loaded = Loaded::unavailable(Unavailable::Locked);
    loaded.access = lightcal::local_events::Access::Editable;
    loaded.notices = vec![
        Notice::RecoveredUnfinishedSave,
        Notice::RestoredBackup {
            damaged: Some(dir.join("local-events.preserved-1.json")),
        },
    ];
    ctrl.local = LocalEvents::from_loaded(loaded);
    let card = ctrl.local_card();
    assert_eq!(card.notice, 2);
    assert!(card.ok && card.folder);
    assert!(card.file.ends_with("local-events.preserved-1.json"));
    ctrl.local.dismiss_notices();
    assert_eq!(ctrl.local_card().notice, 0);
}

#[test]
fn saving_an_event_in_demo_mode_keeps_on_this_pc_selected() {
    use lightcal::local_events::SOURCE_ID;
    let dir = std::env::temp_dir().join(format!(
        "lightcal-demo-save-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    // Demo mode showed On this PC that the saved preferences have off.
    ctrl.demo_shown.insert(SOURCE_ID.into());
    ctrl.prefs.selected_calendar_ids.insert(SOURCE_ID.into());
    ctrl.open_new_editor();
    let fields = local_editor::EditorFields {
        title: "Dentist".into(),
        ..local_editor::EditorFields::blank()
    };
    assert!(ctrl.save_editor(&fields));
    assert!(
        ctrl.demo_shown.is_empty(),
        "the save made it the user's choice"
    );
    let saved = ctrl.store.load().preferences.selected_calendar_ids;
    assert!(saved.contains(SOURCE_ID));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn start_over_hides_its_buttons_while_it_runs_and_reports_a_failure() {
    use lightcal::local_events::{Loaded, LocalEvents, Unavailable};
    let dir = std::env::temp_dir().join(format!(
        "lightcal-start-over-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.local = LocalEvents::from_loaded(Loaded::unavailable(Unavailable::Damaged {
        file: dir.join("local-events.json"),
    }));
    ctrl.start_over = local_editor::StartOver::Running;
    let card = ctrl.local_card();
    assert!(
        !card.start_over && !card.confirm,
        "no second Start over while one runs"
    );
    ctrl.start_over =
        local_editor::StartOver::Failed("Failed to keep the events file: in use".into());
    let card = ctrl.local_card();
    assert!(card.start_over, "it can be tried again");
    assert!(card
        .text
        .ends_with("Failed to keep the events file: in use"));
}

#[test]
fn quit_asks_first_only_after_a_failed_save() {
    use lightcal::local_events::{Storage, Writer};
    let dir = std::env::temp_dir().join(format!(
        "lightcal-quit-prompt-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    assert!(!ctrl.quit_needs_prompt(), "no writer, nothing to lose");
    // A file where the data directory should be makes every save fail.
    std::fs::create_dir_all(&dir).unwrap();
    let blocked = dir.join("blocked");
    std::fs::write(&blocked, "not a directory").unwrap();
    let writer = Writer::spawn_with_backoff(
        Storage::new(&blocked),
        std::time::Duration::from_secs(600),
        std::time::Duration::from_secs(600),
        |_| {},
    )
    .unwrap();
    ctrl.local_writer = Some(writer.clone());
    assert!(!ctrl.quit_needs_prompt(), "nothing written yet");
    assert!(writer.submit(Vec::new(), false));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !writer.last_write_failed() && std::time::Instant::now() < deadline {
        std::thread::yield_now();
    }
    assert!(
        ctrl.quit_needs_prompt(),
        "a failed save asks before quitting"
    );
    writer.shutdown();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_quit_prompt_keeps_lightcal_open_unless_told_otherwise() {
    slint::platform::set_platform(Box::new(HeadlessPlatform::new())).unwrap();
    let prompt = QuitPrompt::new().unwrap();
    prompt.set_quit_label("Quit anyway".into());
    prompt.set_keep_label("Keep Lightcal open".into());
    let kept = Rc::new(Cell::new(0));
    let quit = Rc::new(Cell::new(false));
    let observed = kept.clone();
    prompt.on_keep_open(move || observed.set(observed.get() + 1));
    let observed = quit.clone();
    prompt.on_quit_anyway(move || observed.set(true));
    prompt.show().unwrap();
    for key in [slint::platform::Key::Escape, slint::platform::Key::Return] {
        let text = SharedString::from(key);
        prompt
            .window()
            .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
        prompt
            .window()
            .dispatch_event(WindowEvent::KeyReleased { text });
    }
    assert_eq!(kept.get(), 2, "Escape and Enter keep Lightcal open");
    assert!(!quit.get());
    let button = ElementHandle::find_by_accessible_label(&prompt, "Quit anyway")
        .next()
        .expect("Quit anyway button");
    let position = element_center(&button);
    for event in [
        WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        },
        WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        },
    ] {
        prompt.window().dispatch_event(event);
    }
    assert!(quit.get(), "only Quit anyway quits");
}

#[test]
fn closing_the_quit_prompt_keeps_lightcal_open() {
    slint::platform::set_platform(Box::new(HeadlessPlatform::new())).unwrap();
    let prompt = QuitPrompt::new().unwrap();
    let quit = Rc::new(Cell::new(false));
    let observed = quit.clone();
    crate::open_quit_prompt(prompt.clone_strong(), Rc::new(move || observed.set(true)));
    assert!(prompt.window().is_visible());
    prompt.window().dispatch_event(WindowEvent::CloseRequested);
    assert!(!quit.get(), "the close button never quits");
    assert!(!prompt.window().is_visible());
    // The prompt is freed after the event, never inside its own callback.
    assert!(crate::QUIT_ASK.with(|slot| slot.borrow().is_some()));
    slint::platform::update_timers_and_animations();
    assert!(crate::QUIT_ASK.with(|slot| slot.borrow().is_none()));
}

#[test]
fn a_lock_file_failure_says_events_are_not_shown() {
    use lightcal::local_events::{Loaded, LocalEvents, Unavailable};
    let dir = std::env::temp_dir().join(format!(
        "lightcal-no-lock-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.local = LocalEvents::from_loaded(Loaded::unavailable(Unavailable::NoLock {
        error: "Access is denied. (os error 5)".into(),
    }));
    let card = ctrl.local_card();
    assert_eq!(card.notice, 3);
    assert_eq!(card.head, "Events on this PC couldn\u{2019}t be opened");
    assert!(card.text.contains("Your events are not shown"));
    assert!(
        !card.text.contains("read only"),
        "nothing is shown, so nothing is read only"
    );
    assert!(!card.start_over && !card.folder);
}

#[test]
fn the_panel_names_a_kept_file_only_when_one_is_at_fault() {
    use lightcal::local_events::{Loaded, LocalEvents, Unavailable};
    slint::platform::set_platform(Box::new(HeadlessPlatform::new())).unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-panel-why-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    for (reason, kept) in [
        (
            Unavailable::Damaged {
                file: dir.join("local-events.json"),
            },
            true,
        ),
        (Unavailable::Locked, false),
        (
            Unavailable::NoLock {
                error: "denied".into(),
            },
            false,
        ),
        (
            Unavailable::CannotSave {
                error: "no thread".into(),
            },
            false,
        ),
    ] {
        ctrl.local = LocalEvents::from_loaded(Loaded::unavailable(reason.clone()));
        ctrl.editor_panel = Some(local_editor::EditorPanel::Unavailable);
        ctrl.sync_to_ui(&ui);
        assert_eq!(
            ui.get_panel_message().contains("The file is kept"),
            kept,
            "{reason:?}"
        );
    }
    // Still loading: Accounts has nothing to explain yet, so the panel says so.
    ctrl.local = LocalEvents::loading();
    ctrl.sync_to_ui(&ui);
    assert_eq!(
        ui.get_panel_message(),
        "Events on this PC are still loading. Try again in a moment."
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn session_end_waits_at_most_a_second_for_the_last_write() {
    use lightcal::local_events::{LocalEvent, Storage, Timing, Writer};
    use std::time::{Duration, Instant};
    assert!(
        wait_for_writes_at_session_end(None),
        "no writer, nothing to wait for"
    );

    let dir = std::env::temp_dir().join(format!(
        "lightcal-session-end-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let event = LocalEvent::validated(
        "local:session-end".into(),
        "Before sign-out",
        LocalDate::new(2026, 11, 12).unwrap(),
        Timing::AllDay,
        None,
        None,
    )
    .unwrap();

    let storage = Storage::new(dir.join("ok"));
    let writer = Writer::spawn(storage.clone(), |_| {}).unwrap();
    assert!(writer.submit(vec![event.clone()], false));
    assert!(wait_for_writes_at_session_end(Some(&writer)));
    assert_eq!(storage.load().events.len(), 1);
    writer.shutdown();

    // A file where the data directory should be makes every save fail. The wait ends
    // with the failed attempt, and never runs past SESSION_END_WAIT.
    let blocked = dir.join("blocked");
    std::fs::write(&blocked, "not a directory").unwrap();
    let (sender, reports) = std::sync::mpsc::channel();
    let writer = Writer::spawn(Storage::new(&blocked), move |report| {
        let _ = sender.send(report);
    })
    .unwrap();
    assert!(writer.submit(vec![event], false));
    reports.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(writer.last_write_failed());
    let started = Instant::now();
    assert!(!wait_for_writes_at_session_end(Some(&writer)));
    assert!(started.elapsed() < SESSION_END_WAIT + Duration::from_millis(500));

    // Once the obstacle is gone, session end tries the failed write again at once
    // instead of leaving it to the 5 s backoff.
    std::fs::remove_file(&blocked).unwrap();
    assert!(wait_for_writes_at_session_end(Some(&writer)));
    assert_eq!(Storage::new(&blocked).load().events.len(), 1);
    writer.shutdown();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_quit_wait_frees_the_single_instance_lock_and_retries_at_once() {
    use lightcal::local_events::{Storage, Writer};
    use std::time::Duration;
    let dir = std::env::temp_dir().join(format!(
        "lightcal-quit-wait-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let app_id = format!(
        "Lightcal.Test.QuitWait.{}",
        lightcal::calendars::random_key().unwrap()
    );
    let SingleInstance::FirstInstance(guard) = test_instance(&app_id, &dir) else {
        panic!("the first instance holds the lock");
    };
    SINGLE_INSTANCE.with(|slot| *slot.borrow_mut() = Some(guard));

    // A failed save waiting out a backoff far longer than the test.
    let blocked = dir.join("blocked");
    std::fs::write(&blocked, "not a directory").unwrap();
    let writer = Writer::spawn_with_backoff(
        Storage::new(&blocked),
        Duration::from_secs(600),
        Duration::from_secs(600),
        |_| {},
    )
    .unwrap();
    assert!(writer.submit(Vec::new(), false));
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !writer.last_write_failed() && std::time::Instant::now() < deadline {
        std::thread::yield_now();
    }
    assert!(writer.last_write_failed());
    std::fs::remove_file(&blocked).unwrap();

    let freed_first = Cell::new(false);
    begin_quit_wait(&writer, QuitKind::User, || {
        // Still held: the shortcut goes before the lock. A failure is only logged.
        freed_first.set(SINGLE_INSTANCE.with(|slot| slot.borrow().is_some()));
        Err("not registered in tests".into())
    });
    assert!(freed_first.get(), "the shortcut is freed before the lock");
    let quitting = QUITTING.with(std::cell::Cell::get);
    QUITTING.with(|quitting| quitting.set(false));
    assert!(quitting);
    assert!(SINGLE_INSTANCE.with(|slot| slot.borrow().is_none()));
    #[cfg(windows)]
    assert!(
        matches!(
            test_instance(&app_id, &dir),
            SingleInstance::FirstInstance(_)
        ),
        "a relaunch starts normally"
    );
    // The retry runs at once instead of after the 10 min backoff.
    assert!(writer.wait_saved(Duration::from_secs(5)));
    writer.shutdown();
    let _ = std::fs::remove_dir_all(dir);
}

/// A writer whose save failed, waiting out a backoff far longer than the test.
fn failed_writer(dir: &std::path::Path) -> lightcal::local_events::Writer {
    use lightcal::local_events::{Storage, Writer};
    use std::time::Duration;
    std::fs::create_dir_all(dir).unwrap();
    let blocked = dir.join("blocked");
    std::fs::write(&blocked, "not a directory").unwrap();
    let writer = Writer::spawn_with_backoff(
        Storage::new(&blocked),
        Duration::from_secs(600),
        Duration::from_secs(600),
        |_| {},
    )
    .unwrap();
    assert!(writer.submit(Vec::new(), false));
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !writer.last_write_failed() && std::time::Instant::now() < deadline {
        std::thread::yield_now();
    }
    assert!(writer.last_write_failed());
    writer
}

#[test]
fn quit_hides_the_window_then_frees_the_lock_and_keeps_its_first_deadline() {
    slint::platform::set_platform(Box::new(HeadlessPlatform::new())).unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-quit-order-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let app_id = format!(
        "Lightcal.Test.QuitOrder.{}",
        lightcal::calendars::random_key().unwrap()
    );
    let SingleInstance::FirstInstance(guard) = test_instance(&app_id, &dir) else {
        panic!("the first instance holds the lock");
    };
    SINGLE_INSTANCE.with(|slot| *slot.borrow_mut() = Some(guard));
    let writer = failed_writer(&dir);
    let mut ctrl = controller(&dir);
    ctrl.local_writer = Some(writer.clone());
    let ctrl = Rc::new(RefCell::new(ctrl));
    ACTIVE_CONTROLLER.with(|active| *active.borrow_mut() = Some(Rc::downgrade(&ctrl)));
    let ui = AppWindow::new().unwrap();
    ui.show().unwrap();
    assert!(ui.window().is_visible());

    quit_after_writes(&ui.as_weak(), QuitKind::User);
    assert!(!ui.window().is_visible(), "the window goes at once");
    assert!(SINGLE_INSTANCE.with(|slot| slot.borrow().is_none()));
    assert!(QUIT_WAIT.with(|slot| slot.borrow().is_some()));

    // A second quit (say, the installer finishing meanwhile) leaves the wait alone.
    let SingleInstance::FirstInstance(again) = test_instance(&app_id, &dir) else {
        panic!("the lock was released");
    };
    SINGLE_INSTANCE.with(|slot| *slot.borrow_mut() = Some(again));
    quit_after_writes(&ui.as_weak(), QuitKind::User);
    assert!(SINGLE_INSTANCE.with(|slot| slot.borrow().is_some()));

    QUIT_WAIT.with(|slot| slot.borrow_mut().take());
    SINGLE_INSTANCE.with(|slot| slot.borrow_mut().take());
    QUITTING.with(|quitting| quitting.set(false));
    ACTIVE_CONTROLLER.with(|active| *active.borrow_mut() = None);
    writer.shutdown();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_update_quit_keeps_the_single_instance_lock() {
    let dir = std::env::temp_dir().join(format!(
        "lightcal-quit-update-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let app_id = format!(
        "Lightcal.Test.QuitUpdate.{}",
        lightcal::calendars::random_key().unwrap()
    );
    let SingleInstance::FirstInstance(guard) = test_instance(&app_id, &dir) else {
        panic!("the first instance holds the lock");
    };
    SINGLE_INSTANCE.with(|slot| *slot.borrow_mut() = Some(guard));
    let writer = failed_writer(&dir);
    let freed = Cell::new(false);
    begin_quit_wait(&writer, QuitKind::Update, || {
        freed.set(true);
        Ok(())
    });
    let quitting = QUITTING.with(std::cell::Cell::get);
    QUITTING.with(|quitting| quitting.set(false));
    assert!(quitting);
    assert!(!freed.get(), "the shortcut stays until the process exits");
    assert!(SINGLE_INSTANCE.with(|slot| slot.borrow().is_some()));
    #[cfg(windows)]
    assert!(
        matches!(test_instance(&app_id, &dir), SingleInstance::AlreadyRunning),
        "a launch during the update activates this process"
    );
    SINGLE_INSTANCE.with(|slot| slot.borrow_mut().take());
    writer.shutdown();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn settings_ignores_calendar_navigation_keys_and_closes_on_escape() {
    slint::platform::set_platform(Box::new(HeadlessPlatform::new())).unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-settings-keys-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let mut ctrl = controller(&dir);
    ctrl.settings_open = true;
    ctrl.settings_tab = 0;
    let ctrl = Rc::new(RefCell::new(ctrl));

    let moved = Rc::new(Cell::new(false));
    let moved_obs = moved.clone();
    ui.on_move_selection_by(move |_| moved_obs.set(true));
    let paged = Rc::new(Cell::new(false));
    let paged_prev = paged.clone();
    ui.on_prev_month(move || paged_prev.set(true));
    let paged_next = paged.clone();
    ui.on_next_month(move || paged_next.set(true));

    wire_settings_navigation(&ui, &ctrl, |_, _| {});

    ctrl.borrow().sync_to_ui(&ui);
    ui.show().unwrap();
    ui.invoke_focus_calendar_keys();

    for key in [
        slint::platform::Key::LeftArrow,
        slint::platform::Key::RightArrow,
        slint::platform::Key::UpArrow,
        slint::platform::Key::DownArrow,
        slint::platform::Key::PageUp,
        slint::platform::Key::PageDown,
    ] {
        let text = SharedString::from(key);
        ui.window()
            .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
        ui.window()
            .dispatch_event(WindowEvent::KeyReleased { text });
    }
    assert!(
        !moved.get() && !paged.get(),
        "Arrow and page keys do not navigate the hidden calendar while Settings is open"
    );

    let esc = SharedString::from(slint::platform::Key::Escape);
    ui.window()
        .dispatch_event(WindowEvent::KeyPressed { text: esc.clone() });
    ui.window()
        .dispatch_event(WindowEvent::KeyReleased { text: esc.clone() });
    ui.window()
        .dispatch_event(WindowEvent::KeyReleased { text: esc });
    assert!(!ui.get_settings_open(), "Escape closes Settings");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn account_cards_route_microsoft_google_and_per_feed_ics_errors_to_their_own_cards() {
    slint::platform::set_platform(Box::new(HeadlessPlatform::new())).unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-card-errors-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    let ctrl = Rc::new(RefCell::new(controller(&dir)));
    ACTIVE_CONTROLLER.with(|active| *active.borrow_mut() = Some(Rc::downgrade(&ctrl)));

    let mut ics_feed_errors = std::collections::BTreeMap::new();
    ics_feed_errors.insert(
        "ics:second".to_string(),
        "Second feed failed: HTTP 502".to_string(),
    );
    apply_loaded_provider(
        lightcal::calendars::CalendarLoad {
            snapshot: ProviderSnapshot {
                status: lightcal::platform::ProviderStatus::Available,
                sources: vec![
                    calendar_core::CalendarSource {
                        id: "graph:work".into(),
                        name: "Work".into(),
                        account_label: "alex@contoso.com".into(),
                        color_hex: "#0078D4".into(),
                    },
                    calendar_core::CalendarSource {
                        id: "google:primary".into(),
                        name: "Personal".into(),
                        account_label: "alex@example.com".into(),
                        color_hex: "#008573".into(),
                    },
                    calendar_core::CalendarSource {
                        id: "ics:first".into(),
                        name: "First ICS".into(),
                        account_label: "Private ICS".into(),
                        color_hex: "#6E56CF".into(),
                    },
                    calendar_core::CalendarSource {
                        id: "ics:second".into(),
                        name: "Second ICS".into(),
                        account_label: "Private ICS".into(),
                        color_hex: "#E5484D".into(),
                    },
                ],
                occurrences: Vec::new(),
            },
            microsoft: lightcal::calendars::account::MicrosoftAccountState::Connected {
                label: Some("alex@contoso.com".into()),
            },
            google_connected: true,
            google_needs_signin: false,
            google_configuration: None,
            google_label: Some("alex@example.com".into()),
            google_error: "Google calendar events failed: HTTP 503".into(),
            google_skipped: 2,
            microsoft_error: "Microsoft Graph events failed: HTTP 504".into(),
            ics_feed_errors,
        },
        ui.as_weak(),
    );

    let cards = ui.get_account_cards();
    let find_card = |id: &str| {
        (0..cards.row_count())
            .filter_map(|i| cards.row_data(i))
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("missing account card {id}"))
    };
    let outlook = find_card("outlook");
    assert_eq!(outlook.notice_kind, 1);
    assert!(outlook.notice.contains("HTTP 504"));

    let google = find_card("google");
    assert_eq!(google.notice_kind, 1);
    assert!(google.notice.contains("HTTP 503"));
    assert!(google.notice.contains("Google: 2 events could not be read"));

    let first_ics = find_card("ics:first");
    assert_eq!(first_ics.notice_kind, 0);
    assert_eq!(first_ics.notice, "");

    let second_ics = find_card("ics:second");
    assert_eq!(second_ics.notice_kind, 1);
    assert!(second_ics.notice.contains("HTTP 502"));

    // A hard Microsoft failure leaves the account Unavailable: its error is localized,
    // and only a network failure gets the offline text and glyph.
    let outlook_after = |error: &str| {
        let mut c = ctrl.borrow_mut();
        c.microsoft_account = lightcal::calendars::account::MicrosoftAccountState::Unavailable {
            saved_credentials: true,
        };
        c.microsoft_error = error.into();
        c.sync_to_ui(&ui);
        let cards = ui.get_account_cards();
        (0..cards.row_count())
            .filter_map(|i| cards.row_data(i))
            .find(|card| card.id == "outlook")
            .expect("outlook card")
    };
    let rejected = outlook_after("Authentication request failed (HTTP 400): invalid_client");
    assert_eq!(rejected.notice_kind, 1);
    assert!(rejected.notice.contains("HTTP 400"));
    assert!(!rejected.notice.contains("invalid_client"));
    assert!(!rejected.notice_offline);
    let unreachable = outlook_after("HTTPS request failed or timed out");
    assert_eq!(unreachable.notice_kind, 1);
    assert_ne!(unreachable.notice, "HTTPS request failed or timed out");
    assert!(unreachable.notice_offline);

    ACTIVE_CONTROLLER.with(|active| *active.borrow_mut() = None);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn macos_software_surface_has_opaque_corner_pixels_in_both_views_and_themes() {
    use slint::platform::software_renderer::PremultipliedRgbaColor;
    let platform = HeadlessPlatform::new();
    let window = platform.window.clone();
    slint::platform::set_platform(Box::new(platform)).unwrap();
    let ui = AppWindow::new().unwrap();
    ui.set_macos(true);
    let (width, height) = (448u32, 432u32);
    window.set_size(slint::PhysicalSize::new(width, height));
    ui.show().unwrap();
    for dark in [false, true] {
        for settings in [false, true] {
            ui.set_dark_mode(dark);
            ui.set_settings_open(settings);
            window.request_redraw();
            let mut pixels = vec![PremultipliedRgbaColor::default(); (width * height) as usize];
            window.draw_if_needed(|renderer| {
                renderer.render(&mut pixels, width as usize);
            });
            for index in [0, width - 1, width * (height - 1), width * height - 1] {
                let pixel = pixels[index as usize];
                assert_eq!(
                    pixel.alpha, 255,
                    "AppKit must clip an opaque software surface"
                );
                assert!(
                    pixel.red > 0 || pixel.green > 0 || pixel.blue > 0,
                    "Corner backing must match the theme instead of black"
                );
            }
        }
    }
}

#[test]
fn mac_accounts_offer_google_and_outlook_without_local_or_ics_creation() {
    let (ui, window, _, dir) = settings_window(1, "mac-direct-providers");
    ui.set_macos(true);
    window.request_redraw();
    window.draw_if_needed(|_| {});
    let strings = lightcal::i18n::settings_text(LocaleCode::En);
    let button = |label: &str| {
        ElementHandle::find_by_accessible_label(&ui, label).any(|element| {
            element.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::Button)
        })
    };
    assert!(button(strings.google_title));
    assert!(button(strings.outlook_title));
    assert!(!button(strings.ics_title));
    assert!(ElementHandle::find_by_accessible_label(&ui, "On this PC")
        .next()
        .is_none());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn native_calendar_permission_copy_and_read_only_footer_follow_the_platform_and_language() {
    let (ui, window, ctrl, dir) = settings_window(1, "native-calendar-copy");
    let locales = [
        LocaleCode::En,
        LocaleCode::Ru,
        LocaleCode::Es,
        LocaleCode::Ja,
        LocaleCode::De,
        LocaleCode::Fr,
        LocaleCode::PtBr,
        LocaleCode::Uk,
        LocaleCode::ZhHans,
        LocaleCode::Ko,
    ];
    let mut controller = ctrl.borrow_mut();
    controller.calendars_access_denied = true;
    controller.prefs.windows_calendars_enabled = true;
    controller.google_connected = true;
    controller
        .provider
        .sources
        .push(calendar_core::CalendarSource {
            id: "google:fixture".into(),
            name: "Work".into(),
            account_label: "Google".into(),
            color_hex: "#008573".into(),
        });
    for macos in [true, false] {
        ui.set_macos(macos);
        for locale in locales {
            controller.prefs.set_locale(locale);
            controller.sync_settings_view(&ui);
            window.request_redraw();
            window.draw_if_needed(|_| {});
            let strings = lightcal::i18n::settings_text(locale);
            let cards: Vec<_> = ui.get_account_cards().iter().collect();
            let native = cards.iter().find(|card| card.id == "windows").unwrap();
            assert_eq!(native.notice_kind, 2);
            assert_eq!(native.action_label, strings.open_privacy);
            assert!(
                cards.iter().any(|card| card.id == "google"
                    && card.notice_kind == 0
                    && card.sources.row_count() == 1),
                "System calendar permission must not block direct Google calendars"
            );
            if macos {
                assert_eq!(native.title, lightcal::i18n::mac_calendars_title(locale));
                assert_eq!(ui.get_settings_strings().windows_title, native.title);
                assert_eq!(native.initial, "M");
                assert_eq!(native.detail, lightcal::i18n::mac_labels(locale)[5]);
                assert_eq!(
                    native.notice,
                    lightcal::i18n::mac_calendar_access_denied(locale)
                );
                assert!(!native.notice.contains("Windows"));
                assert_eq!(
                    ui.get_settings_strings().read_only_note,
                    strings.read_only_note
                );
            } else {
                assert_eq!(native.initial, "W");
                assert_eq!(native.title, strings.windows_title);
                assert_eq!(native.notice, strings.access_denied);
                assert_eq!(
                    ui.get_settings_strings().read_only_note,
                    lightcal::i18n::local_settings_text(locale).accounts_read_only
                );
            }
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}
