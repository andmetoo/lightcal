//! Headless interaction checks. No native shortcut, shell, or provider callbacks run here.
use super::*;
use i_slint_backend_testing::ElementHandle;
use lightcal::snapshot::HeadlessPlatform;
use slint::platform::{PointerEventButton, WindowEvent};
use slint::Model;
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
        #[cfg(windows)]
        tray_menu: crate::TrayMenuItems::new(LocaleCode::En),
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
    click(&ui, 220.0, 286.0);
    assert!(
        ui.get_legacy_windows_enabled(),
        "Fresh users can enable Windows calendars"
    );
    click(&ui, 220.0, 286.0);
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
    click(&ui, 372.0, 146.0);
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
    click(&ui, 395.0, 193.0);
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
    click(&ui, 220.0, 194.0);
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
    assert_eq!(shortcut_from_key("k", false, false, true, false), None);
    assert_eq!(shortcut_from_key("-", true, false, false, false), None);
    assert_eq!(
        ics_name_from_address("https://user@calendar.example.com:443/private/basic.ics"),
        "calendar.example.com"
    );
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

/// Opens the language menu by clicking the select that shows `current`.
fn open_language_menu(ui: &AppWindow, current: &str) {
    let select = ElementHandle::find_by_element_type_name(ui, "Select")
        .find(|element| element.accessible_value().as_deref() == Some(current))
        .unwrap_or_else(|| panic!("no select showing {current:?}"));
    let center = element_center(&select);
    click(ui, center.x, center.y);
}

#[test]
fn language_menu_columns_select_and_save_each_language() {
    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    slint::platform::set_platform(Box::new(HeadlessPlatform {
        window: window.clone(),
    }))
    .unwrap();
    let ui = AppWindow::new().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "lightcal-language-menu-{}",
        lightcal::calendars::random_key().unwrap()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let mut ctrl = controller(&dir);
    ctrl.settings_open = true;
    ctrl.settings_tab = 2;
    let ctrl = Rc::new(RefCell::new(ctrl));
    wire_language(&ui, &ctrl);
    ui.show().unwrap();
    ctrl.borrow().sync_to_ui(&ui);
    window.set_size(slint::PhysicalSize::new(448, 360));
    window.draw_if_needed(|_| {});

    // Latin scripts fill the left column, Cyrillic and CJK the right, top to bottom.
    open_language_menu(&ui, "English");
    assert!(ui.get_language_dropdown_open());
    let left = labelled_center(&ui, LocaleCode::PtBr.display_name());
    let right = labelled_center(&ui, LocaleCode::ZhHans.display_name());
    assert!(
        right.x > left.x,
        "Simplified Chinese is in the right column"
    );
    assert!(
        (right.y - left.y).abs() < 1.0,
        "both columns have five rows"
    );
    assert!(labelled_center(&ui, LocaleCode::En.display_name()).y < left.y);
    assert!(labelled_center(&ui, LocaleCode::Ru.display_name()).y < right.y);

    // Every language can be picked from the menu and is saved under its code.
    let mut current = LocaleCode::En;
    for locale in LocaleCode::ALL.into_iter().rev() {
        if !ui.get_language_dropdown_open() {
            open_language_menu(&ui, current.display_name());
        }
        click_label(&ui, locale.display_name());
        assert_eq!(ctrl.borrow().prefs.locale(), locale);
        assert!(!ui.get_language_dropdown_open());
        let saved = PreferencesStore::new(dir.clone()).load();
        assert_eq!(saved.preferences.language, locale.as_str());
        current = locale;
    }
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
