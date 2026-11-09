//! Opt-in checks against the running AppKit/Slint event loop, without image capture.
use crate::{hide_application_window, mac_event_rust, AppWindow};
use serde::Deserialize;
use slint::ComponentHandle;
use std::{cell::Cell, ffi::CStr, rc::Rc, time::Duration};

unsafe extern "C" {
    fn lc_smoke_reopen() -> i32;
    fn lc_smoke_settings() -> bool;
    fn lc_smoke_state() -> *mut std::ffi::c_char;
    fn lc_free(value: *mut std::ffi::c_char);
}

pub fn validate(args: &[String]) -> Result<(), &'static str> {
    if !args.iter().any(|arg| arg == "--synthetic-calendars")
        || !args.iter().any(|arg| arg == "--background")
    {
        return Err("Native smoke checks require --synthetic-calendars --background.");
    }
    let data = std::env::var_os("LIGHTCAL_DATA_DIR")
        .and_then(|path| std::fs::canonicalize(path).ok())
        .filter(|path| path.is_dir())
        .ok_or("Native smoke checks require an existing temporary LIGHTCAL_DATA_DIR.")?;
    let temporary = std::fs::canonicalize(std::env::temp_dir())
        .map_err(|_| "Could not resolve the temporary directory.")?;
    if data == temporary || !data.starts_with(temporary) {
        return Err("Native smoke data must be in a child of the temporary directory.");
    }
    Ok(())
}

#[derive(Deserialize, serde::Serialize)]
struct State {
    visible: bool,
    miniaturized: bool,
    anchored: bool,
    alpha: f64,
    snapshot: bool,
    reopens: u32,
    delegate: bool,
}

fn state() -> Result<State, &'static str> {
    // The bridge allocates JSON containing only native window state, never user data.
    let pointer = unsafe { lc_smoke_state() };
    if pointer.is_null() {
        return Err("No native state returned");
    }
    let result = serde_json::from_slice(unsafe { CStr::from_ptr(pointer) }.to_bytes());
    unsafe { lc_free(pointer) };
    result.map_err(|error| {
        eprintln!("Native state decode: {error}");
        "Invalid native state"
    })
}

pub struct Check {
    _timer: slint::Timer,
    passed: Rc<Cell<bool>>,
}

impl Check {
    pub fn passed(&self) -> bool {
        self.passed.get()
    }
}

fn send_reopen() -> Result<(), &'static str> {
    if unsafe { lc_smoke_reopen() } == 0 {
        Ok(())
    } else {
        Err("Could not send the self-targeted reopen AppleEvent")
    }
}

pub fn start(ui: &AppWindow) -> Check {
    let timer = slint::Timer::default();
    let passed = Rc::new(Cell::new(false));
    let outcome = Rc::clone(&passed);
    let weak = ui.as_weak();
    let mut step = 0;
    let mut ticks = 0;
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(400),
        move || {
            ticks += 1;
            let result = (|| -> Result<(), &'static str> {
                if ticks > 25 {
                    return Err("Native lifecycle check timed out");
                }
                let ui = weak.upgrade().ok_or("Window was destroyed")?;
                let current = state()?;
                println!(
                    "native-smoke {}",
                    serde_json::json!({"step":step,"state":current})
                );
                if !current.delegate {
                    return Err("The toolkit delegate has no reopen callback");
                }
                if step == 0 {
                    if current.visible {
                        return Err("Background launch unexpectedly opened a window");
                    }
                    send_reopen()?;
                    step = 1;
                    return Ok(());
                }
                let expected = match step {
                    1 => 1,
                    2 => 2,
                    3 => 2,
                    4 => 3,
                    _ => 4,
                };
                if current.reopens < expected || !current.visible {
                    return Ok(()); // AppleEvent and UI callback run on subsequent turns.
                }
                if !current.anchored
                    || current.miniaturized
                    || (current.alpha - 1.0).abs() > 0.01
                    || current.snapshot
                {
                    return Err("The restored window is not anchored and fully visible");
                }
                match step {
                    1 => send_reopen()?, // A visible Dock click must keep the window open.
                    2 => {
                        if !unsafe { lc_smoke_settings() } {
                            return Err("The native Settings shortcut was not handled");
                        }
                    }
                    3 => {
                        if !ui.get_settings_open() || ui.get_settings_tab() != 2 {
                            return Err("Settings did not open");
                        }
                        hide_application_window(&ui);
                        send_reopen()?;
                    }
                    4 => {
                        if !ui.get_settings_open() || ui.get_settings_tab() != 2 {
                            return Err("Dock reopen lost the selected Settings view");
                        }
                        ui.invoke_close_settings();
                        hide_application_window(&ui); // Interrupt an active crossfade.
                        let hidden = state()?;
                        if hidden.visible || hidden.snapshot || (hidden.alpha - 1.0).abs() > 0.01 {
                            return Err("Hiding left an animation snapshot or faded window");
                        }
                        send_reopen()?;
                    }
                    _ => {
                        if ui.get_settings_open() {
                            return Err("Calendar did not restore after leaving Settings");
                        }
                        outcome.set(true);
                        println!("native-smoke {}", serde_json::json!({"passed":true}));
                        mac_event_rust(6);
                    }
                }
                step += 1;
                Ok(())
            })();
            if let Err(error) = result {
                eprintln!("Native smoke check failed: {error}");
                let _ = slint::quit_event_loop();
            }
        },
    );
    Check {
        _timer: timer,
        passed,
    }
}
