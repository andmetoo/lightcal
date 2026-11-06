//! Windows shell integration: single-instance synchronization, window work area,
//! dark mode registry queries, user startup registration, and global shortcuts.

use crate::platform::{normalize_shortcut, TaskbarEdge, WorkArea};
#[cfg(windows)]
use crate::platform::{AnswerCache, ShellTaskbar};
use std::env;
use std::fs;
use std::path::Path;

pub const RUN_KEY_VALUE_NAME: &str = "Lightcal";

/// Returns `true` if Windows is configured for Dark app mode (`AppsUseLightTheme == 0`),
/// defaulting to `true` (Obsidian Graphite) when unavailable.
pub fn system_prefers_dark_mode() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Registry::{
            RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD,
        };
        let subkey: Vec<u16> =
            "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\0"
                .encode_utf16()
                .collect();
        let value_name: Vec<u16> = "AppsUseLightTheme\0".encode_utf16().collect();
        let mut data: u32 = 0;
        let mut data_len: u32 = std::mem::size_of::<u32>() as u32;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                value_name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                &mut data as *mut u32 as *mut _,
                &mut data_len,
            )
        };
        if status == 0 {
            return data == 0;
        }
    }
    true
}

/// Queries the primary monitor work area and its taskbar edge on Windows, falling back to
/// the system work area and then to `1920×1040`, with a bottom taskbar.
pub fn primary_tray_area() -> (WorkArea, TaskbarEdge) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{POINT, RECT};
        use windows_sys::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTOPRIMARY};
        use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA};
        // SAFETY: MonitorFromPoint takes a plain point and always returns a monitor here.
        let monitor = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
        // SAFETY: `monitor` came from MonitorFromPoint.
        if let Some(found) = unsafe { monitor_tray_area(monitor) } {
            return found;
        }
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        };
        let ok =
            unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut rect as *mut _ as *mut _, 0) };
        if ok != 0 && rect.right > rect.left && rect.bottom > rect.top {
            return (rect_area(rect), TaskbarEdge::Bottom);
        }
    }
    (
        WorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
        },
        TaskbarEdge::Bottom,
    )
}

#[cfg(windows)]
fn rect_area(r: windows_sys::Win32::Foundation::RECT) -> WorkArea {
    WorkArea {
        x: r.left,
        y: r.top,
        width: r.right - r.left,
        height: r.bottom - r.top,
    }
}

/// The work area of `monitor` and the edge its taskbar is on; see `platform::tray_area`.
///
/// # Safety
///
/// `monitor` must be a monitor handle from the Win32 monitor APIs.
#[cfg(windows)]
unsafe fn monitor_tray_area(
    monitor: windows_sys::Win32::Graphics::Gdi::HMONITOR,
) -> Option<(WorkArea, TaskbarEdge)> {
    use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITORINFO};

    let mut mi: MONITORINFO = std::mem::zeroed();
    mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    if GetMonitorInfoW(monitor, &mut mi) == 0 {
        return None;
    }
    let work = rect_area(mi.rcWork);
    if work.width <= 0 || work.height <= 0 {
        return None;
    }
    let monitor_area = rect_area(mi.rcMonitor);
    // A stale report stands in for a busy shell only while it still matches the work area;
    // right after the taskbar moves, the strips are the better guess.
    let report = shell_taskbar(mi.rcMonitor)
        .filter(|(bar, fresh)| *fresh || bar.fits(monitor_area, work))
        .map(|(bar, _)| bar);
    Some(crate::platform::tray_area(monitor_area, work, report))
}

#[cfg(windows)]
thread_local! {
    /// The shell's answer per monitor rectangle, kept until `forget_shell_taskbar`, so
    /// placing the window does not ask Explorer again each time.
    static SHELL_TASKBARS: std::cell::RefCell<AnswerCache<WorkArea, ShellTaskbar>> =
        const { std::cell::RefCell::new(AnswerCache::new()) };
}

/// How long a missing or busy shell is not asked again, so a burst of placements (a view
/// switch places the window up to three times) waits on it at most once.
#[cfg(windows)]
const SHELL_RETRY_AFTER: std::time::Duration = std::time::Duration::from_secs(2);

/// Makes the remembered taskbar reports stale; call it on display and setting changes,
/// which include the taskbar moving or starting to auto-hide, and when Explorer restarts.
#[cfg(windows)]
pub fn forget_shell_taskbar() {
    SHELL_TASKBARS.with(|cache| cache.borrow_mut().clear());
}

/// Hides the application window (`SW_HIDE`) and makes the taskbar reports stale: display
/// and setting changes do not reliably reach a hidden window, so the next show asks again.
/// A null `hwnd` only forgets.
///
/// # Safety
///
/// `hwnd` must be null or a valid, live Win32 window handle owned by this process.
#[cfg(windows)]
pub unsafe fn hide_native_window(hwnd: windows_sys::Win32::Foundation::HWND) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
    forget_shell_taskbar();
    if !hwnd.is_null() {
        ShowWindow(hwnd, SW_HIDE);
    }
}

/// The shell's taskbar report for `monitor` and whether it is fresh, remembered until
/// `forget_shell_taskbar`. A missing or busy shell is not asked again for
/// `SHELL_RETRY_AFTER`; meanwhile the last report for `monitor`, if there is one, is given
/// as stale. Right after sign-in, before Explorer has answered once, there is none.
#[cfg(windows)]
fn shell_taskbar(monitor: windows_sys::Win32::Foundation::RECT) -> Option<(ShellTaskbar, bool)> {
    SHELL_TASKBARS.with(|cache| {
        cache.borrow_mut().get_or_ask(
            rect_area(monitor),
            std::time::Instant::now(),
            SHELL_RETRY_AFTER,
            || query_shell_taskbar(monitor),
        )
    })
}

/// Whether taskbars show on every display (Settings > Personalization > Taskbar), from
/// `MMTaskbarEnabled`; Windows shows them everywhere when the value is missing.
#[cfg(windows)]
fn taskbars_on_all_displays() -> bool {
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
    let subkey: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced\0"
        .encode_utf16()
        .collect();
    let value_name: Vec<u16> = "MMTaskbarEnabled\0".encode_utf16().collect();
    let mut data: u32 = 1;
    let mut data_len: u32 = std::mem::size_of::<u32>() as u32;
    // SAFETY: NUL-terminated names and a DWORD buffer of the size we pass.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            value_name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            &mut data as *mut u32 as *mut _,
            &mut data_len,
        )
    };
    status != 0 || data != 0
}

/// Asks the shell (Explorer) for the taskbar edge, the auto-hide state and the auto-hidden
/// taskbar on that edge of `monitor`. Returns `None` when Explorer is missing or does not
/// answer a probe within 200 ms, so placement does not wait on a busy shell.
#[cfg(windows)]
fn query_shell_taskbar(monitor: windows_sys::Win32::Foundation::RECT) -> Option<ShellTaskbar> {
    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::UI::Shell::{
        SHAppBarMessage, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_GETAUTOHIDEBAREX, ABM_GETSTATE,
        ABM_GETTASKBARPOS, ABS_AUTOHIDE, APPBARDATA,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, GetWindowRect, SendMessageTimeoutW, SMTO_ABORTIFHUNG, SMTO_BLOCK, WM_NULL,
    };

    let class: Vec<u16> = "Shell_TrayWnd\0".encode_utf16().collect();
    let appbar = || {
        // SAFETY: APPBARDATA is plain data; all-zero is a valid value.
        let mut data: APPBARDATA = unsafe { std::mem::zeroed() };
        data.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
        data
    };
    // SAFETY: plain Win32 calls on a NUL-terminated class name and on structs we own; the
    // APPBARDATA messages read `cbSize`, `uEdge` and `rc` and write only into the struct.
    unsafe {
        let tray = FindWindowW(class.as_ptr(), std::ptr::null());
        if tray.is_null() {
            return None;
        }
        if SendMessageTimeoutW(
            tray,
            WM_NULL,
            0,
            0,
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            200,
            std::ptr::null_mut(),
        ) == 0
        {
            return None;
        }
        let mut bar = appbar();
        if SHAppBarMessage(ABM_GETTASKBARPOS, &mut bar) == 0 {
            return None;
        }
        let edge = match bar.uEdge {
            ABE_LEFT => TaskbarEdge::Left,
            ABE_TOP => TaskbarEdge::Top,
            ABE_RIGHT => TaskbarEdge::Right,
            _ => TaskbarEdge::Bottom,
        };
        let auto_hide = SHAppBarMessage(ABM_GETSTATE, &mut appbar()) as u32 & ABS_AUTOHIDE != 0;
        let mut auto_hide_bar = None;
        if auto_hide {
            let mut query = appbar();
            query.uEdge = bar.uEdge;
            query.rc = monitor;
            let hidden = SHAppBarMessage(ABM_GETAUTOHIDEBAREX, &mut query) as HWND;
            let mut rect: RECT = std::mem::zeroed();
            // The bar slides away but keeps its size, so its window rectangle gives the
            // room it takes when shown.
            if !hidden.is_null() && GetWindowRect(hidden, &mut rect) != 0 {
                auto_hide_bar = Some(rect_area(rect));
            }
        }
        // The primary taskbar's monitor holds its inner edge: an auto-hidden bar slides off
        // its monitor, possibly onto the next one, but keeps that edge on screen.
        let r = bar.rc;
        let (x, y) = match edge {
            TaskbarEdge::Left => (r.right - 1, (r.top + r.bottom) / 2),
            TaskbarEdge::Right => (r.left, (r.top + r.bottom) / 2),
            TaskbarEdge::Top => ((r.left + r.right) / 2, r.bottom - 1),
            TaskbarEdge::Bottom => ((r.left + r.right) / 2, r.top),
        };
        let holds_primary =
            monitor.left <= x && x < monitor.right && monitor.top <= y && y < monitor.bottom;
        Some(ShellTaskbar {
            edge,
            auto_hide_bar,
            auto_hide,
            taskbar_here: holds_primary || auto_hide_bar.is_some() || taskbars_on_all_displays(),
        })
    }
}

/// Manages the user-scoped startup registration.
/// During automated tests or when `LIGHTCAL_DATA_DIR` is set, writes an isolated marker file
/// instead of touching `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
#[derive(Debug, Clone)]
pub struct StartupManager {
    isolated_marker: Option<std::path::PathBuf>,
}

impl StartupManager {
    pub fn from_env(data_dir: &Path) -> Self {
        if env::var_os("LIGHTCAL_DATA_DIR").is_some() || cfg!(test) {
            Self {
                isolated_marker: Some(data_dir.join("autostart_entry.txt")),
            }
        } else {
            Self {
                isolated_marker: None,
            }
        }
    }

    pub fn set_enabled(&self, enabled: bool, exe_path: &Path) -> Result<(), String> {
        let quoted = format!("\"{}\" --tray", exe_path.display());
        if let Some(marker) = &self.isolated_marker {
            if enabled {
                if let Some(parent) = marker.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                fs::write(marker, quoted).map_err(|e| e.to_string())?;
            } else if marker.exists() {
                fs::remove_file(marker).map_err(|e| e.to_string())?;
            }
            return Ok(());
        }

        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Registry::{
                RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY_CURRENT_USER,
                KEY_SET_VALUE, REG_SZ,
            };

            let subkey: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0"
                .encode_utf16()
                .collect();
            let value_name: Vec<u16> = format!("{RUN_KEY_VALUE_NAME}\0").encode_utf16().collect();
            let mut hkey = std::ptr::null_mut();
            let status = unsafe {
                RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    subkey.as_ptr(),
                    0,
                    KEY_SET_VALUE,
                    &mut hkey,
                )
            };
            if status != 0 {
                return Err("Failed to open Windows Run registry key".to_string());
            }

            let result = if enabled {
                let wide_val: Vec<u16> = quoted.encode_utf16().chain(std::iter::once(0)).collect();
                let set_status = unsafe {
                    RegSetValueExW(
                        hkey,
                        value_name.as_ptr(),
                        0,
                        REG_SZ,
                        wide_val.as_ptr() as *const _,
                        (wide_val.len() * std::mem::size_of::<u16>()) as u32,
                    )
                };
                if set_status == 0 {
                    Ok(())
                } else {
                    Err("Failed to set startup value in Windows Registry".to_string())
                }
            } else {
                let _ = unsafe { RegDeleteValueW(hkey, value_name.as_ptr()) };
                Ok(())
            };

            unsafe {
                RegCloseKey(hkey);
            }
            result
        }
        #[cfg(not(windows))]
        {
            let _ = quoted;
            Ok(())
        }
    }

    pub fn is_enabled(&self) -> bool {
        if let Some(marker) = &self.isolated_marker {
            return marker.exists();
        }
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Registry::{
                RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ,
            };
            let subkey: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0"
                .encode_utf16()
                .collect();
            let value_name: Vec<u16> = format!("{RUN_KEY_VALUE_NAME}\0").encode_utf16().collect();
            let mut data_len: u32 = 0;
            let status = unsafe {
                RegGetValueW(
                    HKEY_CURRENT_USER,
                    subkey.as_ptr(),
                    value_name.as_ptr(),
                    RRF_RT_REG_SZ,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut data_len,
                )
            };
            status == 0
        }
        #[cfg(not(windows))]
        false
    }
}

#[cfg(all(windows, not(test)))]
enum ShortcutCommand {
    Bind {
        binding: Option<String>,
        resp: std::sync::mpsc::Sender<Result<Option<String>, String>>,
    },
    SetHandler(Box<dyn Fn() + Send + 'static>),
    Shutdown,
}

/// Parses a normalized shortcut string (e.g. `Ctrl + Alt + C`) into Win32 modifiers and virtual key.
#[cfg(windows)]
pub fn parse_shortcut_to_vk(normalized: &str) -> (u32, u32) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, VK_F1, VK_SPACE,
    };

    let mut modifiers = MOD_NOREPEAT;
    let parts: Vec<&str> = normalized.split(" + ").collect();
    if parts.is_empty() {
        return (0, 0);
    }
    for &part in &parts[..parts.len() - 1] {
        match part {
            "Ctrl" => modifiers |= MOD_CONTROL,
            "Alt" => modifiers |= MOD_ALT,
            "Shift" => modifiers |= MOD_SHIFT,
            "Win" => modifiers |= MOD_WIN,
            _ => {}
        }
    }

    let key = parts.last().copied().unwrap_or("");
    let vk = match key {
        "Space" => VK_SPACE as u32,
        k if k.starts_with('F') && k.len() >= 2 => {
            if let Ok(num) = k[1..].parse::<u32>() {
                if (1..=12).contains(&num) {
                    (VK_F1 as u32) + (num - 1)
                } else {
                    0
                }
            } else {
                0
            }
        }
        k if k.len() == 1 => {
            let ch = k.chars().next().unwrap();
            ch.to_ascii_uppercase() as u32
        }
        _ => 0,
    };

    (modifiers, vk)
}

/// Manages global shortcut validation and active registration state.
pub struct ShortcutManager {
    active_binding: Option<String>,
    #[cfg(all(windows, not(test)))]
    sender: Option<std::sync::mpsc::Sender<ShortcutCommand>>,
    #[cfg(all(windows, not(test)))]
    thread_id: u32,
}

impl Default for ShortcutManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ShortcutManager {
    pub fn new() -> Self {
        #[cfg(all(windows, not(test)))]
        {
            let (ready_tx, ready_rx) = std::sync::mpsc::channel();
            let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                use windows_sys::Win32::Foundation::GetLastError;
                use windows_sys::Win32::System::Threading::GetCurrentThreadId;
                use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                    RegisterHotKey, UnregisterHotKey,
                };
                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    GetMessageW, PeekMessageW, MSG, PM_NOREMOVE, WM_HOTKEY, WM_USER,
                };

                let thread_id = unsafe { GetCurrentThreadId() };
                let mut msg: MSG = unsafe { std::mem::zeroed() };
                unsafe {
                    PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
                }
                let _ = ready_tx.send(thread_id);

                let current_id = 1i32;
                let mut current_binding: Option<String> = None;
                let mut handler: Option<Box<dyn Fn() + Send + 'static>> = None;

                loop {
                    let mut msg: MSG = unsafe { std::mem::zeroed() };
                    let ret = unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
                    if ret <= 0 {
                        break;
                    }

                    if msg.message == WM_USER {
                        while let Ok(cmd) = cmd_rx.try_recv() {
                            match cmd {
                                ShortcutCommand::Bind { binding, resp } => {
                                    if current_binding.is_some() {
                                        unsafe {
                                            UnregisterHotKey(std::ptr::null_mut(), current_id);
                                        }
                                    }
                                    if let Some(target) = binding {
                                        let (mods, vk) = parse_shortcut_to_vk(&target);
                                        let ok = unsafe {
                                            RegisterHotKey(
                                                std::ptr::null_mut(),
                                                current_id,
                                                mods,
                                                vk,
                                            )
                                        };
                                        if ok != 0 {
                                            current_binding = Some(target.clone());
                                            let _ = resp.send(Ok(Some(target)));
                                        } else {
                                            let err_code = unsafe { GetLastError() };
                                            let err_msg = format!(
                                                "Shortcut '{target}' is in use by another application or reserved (error {err_code})."
                                            );
                                            if let Some(prev) = &current_binding {
                                                let (prev_mods, prev_vk) =
                                                    parse_shortcut_to_vk(prev);
                                                unsafe {
                                                    RegisterHotKey(
                                                        std::ptr::null_mut(),
                                                        current_id,
                                                        prev_mods,
                                                        prev_vk,
                                                    );
                                                }
                                            }
                                            let _ = resp.send(Err(err_msg));
                                        }
                                    } else {
                                        current_binding = None;
                                        let _ = resp.send(Ok(None));
                                    }
                                }
                                ShortcutCommand::SetHandler(cb) => {
                                    handler = Some(cb);
                                }
                                ShortcutCommand::Shutdown => {
                                    if current_binding.is_some() {
                                        unsafe {
                                            UnregisterHotKey(std::ptr::null_mut(), current_id);
                                        }
                                    }
                                    return;
                                }
                            }
                        }
                    } else if msg.message == WM_HOTKEY {
                        if let Some(ref cb) = handler {
                            cb();
                        }
                    }
                }
            });

            let thread_id = ready_rx.recv().unwrap_or(0);
            Self {
                active_binding: None,
                sender: Some(cmd_tx),
                thread_id,
            }
        }
        #[cfg(any(not(windows), test))]
        {
            Self {
                active_binding: None,
            }
        }
    }

    pub fn active(&self) -> Option<&str> {
        self.active_binding.as_deref()
    }

    pub fn listen_for_trigger<F>(&self, on_trigger: F)
    where
        F: Fn() + Send + 'static,
    {
        #[cfg(all(windows, not(test)))]
        if let Some(tx) = &self.sender {
            let _ = tx.send(ShortcutCommand::SetHandler(Box::new(on_trigger)));
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                    self.thread_id,
                    windows_sys::Win32::UI::WindowsAndMessaging::WM_USER,
                    0,
                    0,
                );
            }
        }
        #[cfg(any(not(windows), test))]
        {
            let _ = on_trigger;
        }
    }

    pub fn bind(&mut self, requested: Option<&str>) -> Result<Option<String>, String> {
        let normalized = match requested {
            None | Some("") => None,
            Some(raw) => {
                let norm = normalize_shortcut(raw)?;
                if norm.eq_ignore_ascii_case("Win + L")
                    || norm.eq_ignore_ascii_case("Ctrl + Alt + Del")
                {
                    return Err(format!(
                        "Shortcut '{norm}' is reserved by the operating system."
                    ));
                }
                Some(norm)
            }
        };

        #[cfg(all(windows, not(test)))]
        if let Some(tx) = &self.sender {
            let (resp_tx, resp_rx) = std::sync::mpsc::channel();
            let _ = tx.send(ShortcutCommand::Bind {
                binding: normalized.clone(),
                resp: resp_tx,
            });
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                    self.thread_id,
                    windows_sys::Win32::UI::WindowsAndMessaging::WM_USER,
                    0,
                    0,
                );
            }
            let res = resp_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .map_err(|e| format!("Shortcut service did not respond: {e}"))??;
            self.active_binding = res.clone();
            return Ok(res);
        }

        self.active_binding = normalized.clone();
        Ok(normalized)
    }
}

impl Drop for ShortcutManager {
    fn drop(&mut self) {
        #[cfg(all(windows, not(test)))]
        if let Some(tx) = &self.sender {
            let _ = tx.send(ShortcutCommand::Shutdown);
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                    self.thread_id,
                    windows_sys::Win32::UI::WindowsAndMessaging::WM_USER,
                    0,
                    0,
                );
            }
        }
    }
}

/// Outcome of attempting to acquire single-instance ownership.
pub enum SingleInstance {
    /// This process is the first/primary instance and holds the mutex and activation event.
    FirstInstance(SingleInstanceGuard),
    /// Another instance is already running. The existing instance has been signaled to activate.
    AlreadyRunning,
}

/// Holds ownership handles of the single-instance synchronization primitives.
///
/// Dropping it releases the single-instance lock at once: a new launch then starts
/// normally, and the activation listener stops, so it no longer takes activations meant
/// for the new instance. Quit drops it early while it waits for a local events write
/// (ADR 0017).
pub struct SingleInstanceGuard {
    #[cfg(windows)]
    _mutex: windows_sys::Win32::Foundation::HANDLE,
    #[cfg(windows)]
    event: windows_sys::Win32::Foundation::HANDLE,
    /// Manual-reset and unnamed; set on drop to end the activation listener.
    #[cfg(windows)]
    stop: windows_sys::Win32::Foundation::HANDLE,
    /// Set on drop. The stop event is the normal way to end the listener: it wakes the
    /// listener at once and costs nothing while it sleeps. This flag covers a missing stop
    /// event (CreateEventW failed): the listener then also wakes every
    /// `FALLBACK_POLL` to check it, and one that wakes for an activation after the drop
    /// passes it on to the new instance instead of taking it.
    #[cfg(windows)]
    released: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl SingleInstanceGuard {
    /// Returns `true` if a secondary launch requested activation, resetting the event.
    pub fn check_activation_requested(&self) -> bool {
        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::WAIT_OBJECT_0;
            use windows_sys::Win32::System::Threading::WaitForSingleObject;
            unsafe { WaitForSingleObject(self.event, 0) == WAIT_OBJECT_0 }
        }
        #[cfg(not(windows))]
        false
    }

    /// Listens for activation events asynchronously on a background thread sleeping
    /// on the Windows kernel event (0.0% CPU usage), dispatching `on_activate` when signaled.
    ///
    /// Returns the listener thread, which ends once the guard is dropped.
    pub fn listen_for_activation<F>(&self, on_activate: F) -> Option<std::thread::JoinHandle<()>>
    where
        F: Fn() + Send + 'static,
    {
        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::{
                CloseHandle, DuplicateHandle, DUPLICATE_SAME_ACCESS, WAIT_OBJECT_0, WAIT_TIMEOUT,
            };
            use windows_sys::Win32::System::Threading::{
                GetCurrentProcess, WaitForMultipleObjects, INFINITE,
            };

            let duplicate = |handle| {
                let mut dup = std::ptr::null_mut();
                let ok = unsafe {
                    DuplicateHandle(
                        GetCurrentProcess(),
                        handle,
                        GetCurrentProcess(),
                        &mut dup,
                        0,
                        0,
                        DUPLICATE_SAME_ACCESS,
                    )
                };
                (ok != 0 && !dup.is_null()).then_some(dup)
            };
            let dup_event = duplicate(self.event)?;
            // Stop comes first: when both are set, the wait reports the lower index and
            // leaves the activation event set for the new instance. Without a stop event
            // (it could not be created) the listener still hears activations, and polls
            // `released` to end.
            let raw: Vec<isize> = duplicate(self.stop)
                .into_iter()
                .chain([dup_event])
                .map(|h| h as isize)
                .collect();
            let released = self.released.clone();
            Some(std::thread::spawn(move || {
                use std::sync::atomic::Ordering;
                use windows_sys::Win32::System::Threading::SetEvent;
                let handles: Vec<windows_sys::Win32::Foundation::HANDLE> =
                    raw.iter().map(|&h| h as _).collect();
                let count = handles.len() as u32;
                let timeout = if count == 1 {
                    FALLBACK_POLL.as_millis() as u32
                } else {
                    INFINITE
                };
                loop {
                    let res =
                        unsafe { WaitForMultipleObjects(count, handles.as_ptr(), 0, timeout) };
                    if res == WAIT_TIMEOUT && !released.load(Ordering::Acquire) {
                        continue;
                    }
                    if res != WAIT_OBJECT_0 + count - 1 {
                        break;
                    }
                    if released.load(Ordering::Acquire) {
                        // Meant for the instance that now holds the lock.
                        unsafe { SetEvent(handles[count as usize - 1]) };
                        break;
                    }
                    on_activate();
                }
                for handle in handles {
                    unsafe {
                        CloseHandle(handle);
                    }
                }
            }))
        }
        #[cfg(not(windows))]
        {
            let _ = on_activate;
            None
        }
    }
}

/// How often a listener without a stop event checks whether its guard was dropped.
#[cfg(windows)]
const FALLBACK_POLL: std::time::Duration = std::time::Duration::from_millis(250);

#[cfg(windows)]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::SetEvent;
        self.released
            .store(true, std::sync::atomic::Ordering::Release);
        unsafe {
            if !self.stop.is_null() {
                SetEvent(self.stop);
                CloseHandle(self.stop);
            }
            if !self._mutex.is_null() {
                CloseHandle(self._mutex);
            }
            if !self.event.is_null() {
                CloseHandle(self.event);
            }
        }
    }
}

impl SingleInstance {
    /// Attempts to acquire single-instance ownership. If another instance exists, signals it
    /// to activate and returns `AlreadyRunning`.
    pub fn acquire(app_id: &str) -> Self {
        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
            use windows_sys::Win32::System::Threading::{CreateEventW, CreateMutexW, SetEvent};

            let mutex_name: Vec<u16> = format!("Local\\{app_id}_SingleInstance_Mutex\0")
                .encode_utf16()
                .collect();
            let event_name: Vec<u16> = format!("Local\\{app_id}_SingleInstance_Event\0")
                .encode_utf16()
                .collect();

            let mutex = unsafe {
                CreateMutexW(
                    std::ptr::null(),
                    1, // Initial owner
                    mutex_name.as_ptr(),
                )
            };

            let already_exists = unsafe { GetLastError() == ERROR_ALREADY_EXISTS };
            let event = unsafe {
                CreateEventW(
                    std::ptr::null(),
                    0, // Auto-reset event
                    0, // Initially nonsignaled
                    event_name.as_ptr(),
                )
            };

            if already_exists {
                if !event.is_null() {
                    unsafe {
                        SetEvent(event);
                        windows_sys::Win32::Foundation::CloseHandle(event);
                    }
                }
                if !mutex.is_null() {
                    unsafe {
                        windows_sys::Win32::Foundation::CloseHandle(mutex);
                    }
                }
                return SingleInstance::AlreadyRunning;
            }

            let stop = unsafe {
                CreateEventW(
                    std::ptr::null(),
                    1, // Manual reset: stays set once the guard is dropped
                    0,
                    std::ptr::null(),
                )
            };
            SingleInstance::FirstInstance(SingleInstanceGuard {
                _mutex: mutex,
                event,
                stop,
                released: Default::default(),
            })
        }
        #[cfg(not(windows))]
        {
            let _ = app_id;
            SingleInstance::FirstInstance(SingleInstanceGuard {})
        }
    }
}

/// A hidden, untitled top-level window that hears sign-out and shutdown (ADR 0017).
///
/// `WM_QUERYENDSESSION` is answered at once, so Lightcal never prompts or delays the
/// session. `WM_ENDSESSION` for an ending session runs `on_end` on the thread that made
/// the watch, before the window procedure returns and Windows may end the process.
/// Created on the UI thread, whose event loop dispatches its messages. A message-only
/// window would not work: Windows sends these messages only to top-level windows.
#[cfg(windows)]
pub struct SessionEndWatch {
    hwnd: windows_sys::Win32::Foundation::HWND,
    on_end: *mut Box<dyn Fn()>,
}

#[cfg(windows)]
impl SessionEndWatch {
    pub fn new(on_end: impl Fn() + 'static) -> Option<Self> {
        use std::sync::atomic::{AtomicBool, Ordering};
        use windows_sys::Win32::Foundation::{GetLastError, ERROR_CLASS_ALREADY_EXISTS};
        use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, RegisterClassW, SetWindowLongPtrW, GWLP_USERDATA, WNDCLASSW,
            WS_EX_TOOLWINDOW, WS_POPUP,
        };
        // Only success is remembered, so a failed registration is tried again.
        static REGISTERED: AtomicBool = AtomicBool::new(false);
        let class: Vec<u16> = "LightcalSessionEnd\0".encode_utf16().collect();
        let title = [0u16];
        // SAFETY: the class name and title outlive the calls; the class is registered
        // once per process.
        let hwnd = unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            if !REGISTERED.load(Ordering::Acquire) {
                let wc = WNDCLASSW {
                    lpfnWndProc: Some(session_end_proc),
                    hInstance: instance,
                    lpszClassName: class.as_ptr(),
                    ..std::mem::zeroed()
                };
                if RegisterClassW(&wc) == 0 {
                    let error = GetLastError();
                    if error != ERROR_CLASS_ALREADY_EXISTS {
                        eprintln!("Lightcal: sign-out and shutdown are not watched (RegisterClassW error {error})");
                        return None;
                    }
                }
                REGISTERED.store(true, Ordering::Release);
            }
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                class.as_ptr(),
                title.as_ptr(),
                WS_POPUP,
                0,
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            )
        };
        if hwnd.is_null() {
            // SAFETY: reads this thread's last error, set by CreateWindowExW.
            let error = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            eprintln!(
                "Lightcal: sign-out and shutdown are not watched (CreateWindowExW error {error})"
            );
            return None;
        }
        let on_end: *mut Box<dyn Fn()> = Box::into_raw(Box::new(Box::new(on_end)));
        // SAFETY: the pointer stays valid until Drop destroys the window first.
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, on_end as isize) };
        Some(Self { hwnd, on_end })
    }

    /// The watch window, for tests that send it session messages.
    #[cfg(test)]
    pub(crate) fn hwnd(&self) -> windows_sys::Win32::Foundation::HWND {
        self.hwnd
    }
}

#[cfg(windows)]
impl Drop for SessionEndWatch {
    fn drop(&mut self) {
        use windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow;
        // SAFETY: the window is destroyed before the callback it points to is freed.
        unsafe {
            DestroyWindow(self.hwnd);
            drop(Box::from_raw(self.on_end));
        }
    }
}

#[cfg(windows)]
unsafe extern "system" fn session_end_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    message: u32,
    wp: windows_sys::Win32::Foundation::WPARAM,
    lp: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, GetWindowLongPtrW, GWLP_USERDATA, WM_ENDSESSION, WM_QUERYENDSESSION,
    };
    match message {
        // Never prompt or delay the session.
        WM_QUERYENDSESSION => 1,
        WM_ENDSESSION => {
            if wp != 0 {
                let on_end = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Box<dyn Fn()>;
                if !on_end.is_null() {
                    // A panic must not unwind into Windows. The panic hook has already
                    // reported it (the release profile unwinds; it does not abort).
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        (*on_end)();
                    }));
                }
            }
            0
        }
        _ => DefWindowProcW(hwnd, message, wp, lp),
    }
}

/// Whether `hwnd` is a live, unowned top-level window of this process titled "Lightcal".
#[cfg(windows)]
fn is_lightcal_hwnd(hwnd: windows_sys::Win32::Foundation::HWND) -> bool {
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindow, GetWindowTextW, GetWindowThreadProcessId, IsWindow, GW_OWNER,
    };
    // SAFETY: these calls only read window state. A destroyed handle fails IsWindow; one
    // that Windows has recycled for another window fails the process, owner or title check.
    unsafe {
        if IsWindow(hwnd) == 0 {
            return false;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid != GetCurrentProcessId() || !GetWindow(hwnd, GW_OWNER).is_null() {
            return false;
        }
        let mut title_buf = [0u16; 64];
        let len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 64);
        len > 0
            && title_buf[..len as usize]
                .iter()
                .copied()
                .eq("Lightcal".encode_utf16())
    }
}

/// Finds the main application HWND owned by this process. A handle is cached only when it
/// is the one match, and re-checked on each call, so the top-level windows are enumerated
/// only when it changes; with several matches (native tests) every call looks again.
#[cfg(windows)]
pub fn find_lightcal_hwnd() -> windows_sys::Win32::Foundation::HWND {
    use std::sync::atomic::{AtomicPtr, Ordering};
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::EnumWindows;

    static CACHED: AtomicPtr<core::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

    // The first match and how many there are.
    struct Found {
        hwnd: HWND,
        count: usize,
    }

    unsafe extern "system" fn find(hwnd: HWND, data: LPARAM) -> i32 {
        if is_lightcal_hwnd(hwnd) {
            let found = &mut *(data as *mut Found);
            if found.count == 0 {
                found.hwnd = hwnd;
            }
            found.count += 1;
        }
        1
    }

    let cached = CACHED.load(Ordering::Relaxed);
    if !cached.is_null() && is_lightcal_hwnd(cached) {
        return cached;
    }
    let mut found = Found {
        hwnd: std::ptr::null_mut(),
        count: 0,
    };
    unsafe {
        EnumWindows(Some(find), &mut found as *mut Found as LPARAM);
    }
    let unique = if found.count == 1 {
        found.hwnd
    } else {
        std::ptr::null_mut()
    };
    CACHED.store(unique, Ordering::Relaxed);
    found.hwnd
}

/// Reads the actual native state, including taskbar minimization outside Slint callbacks.
#[cfg(windows)]
pub fn main_window_is_open() -> Option<bool> {
    let hwnd = find_lightcal_hwnd();
    if hwnd.is_null() {
        return None;
    }
    use windows_sys::Win32::UI::WindowsAndMessaging::{IsIconic, IsWindowVisible};
    Some(unsafe { IsWindowVisible(hwnd) != 0 && IsIconic(hwnd) == 0 })
}

/// Display power, from the `GUID_CONSOLE_DISPLAY_STATE` notifications the window hooks
/// subscribe to. It starts as on; Windows sends the current state right after subscribing.
#[cfg(windows)]
static DISPLAY_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Records the console display turning off (false) or on or dimmed (true).
#[cfg(windows)]
pub fn set_display_on(on: bool) {
    DISPLAY_ON.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Whether a window is cloaked by DWM (another virtual desktop, a suspended app).
#[cfg(windows)]
unsafe fn is_cloaked(hwnd: windows_sys::Win32::Foundation::HWND) -> bool {
    use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
    let mut cloaked: u32 = 0;
    let read = DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED as u32,
        &mut cloaked as *mut u32 as *mut _,
        std::mem::size_of::<u32>() as u32,
    );
    read == 0 && cloaked != 0
}

/// The visible frame of a window: DWM's extended frame bounds, which leave out the invisible
/// resize borders, or the window rectangle when DWM has none.
#[cfg(windows)]
unsafe fn frame_rect(
    hwnd: windows_sys::Win32::Foundation::HWND,
) -> Option<windows_sys::Win32::Foundation::RECT> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;
    let mut rect: RECT = std::mem::zeroed();
    if DwmGetWindowAttribute(
        hwnd,
        DWMWA_EXTENDED_FRAME_BOUNDS as u32,
        &mut rect as *mut RECT as *mut _,
        std::mem::size_of::<RECT>() as u32,
    ) == 0
    {
        return Some(rect);
    }
    (GetWindowRect(hwnd, &mut rect) != 0).then_some(rect)
}

/// Whether someone can see part of the application window, which must already be on screen:
/// false while the display is off, the session is locked, the window is cloaked on another
/// virtual desktop, or other windows cover it completely. Coverage is checked at five
/// points across the window against the opaque windows above it in z-order; layered and
/// click-through windows (overlays, which may be translucent) never count as covering.
#[cfg(windows)]
pub fn main_window_is_seen() -> bool {
    use windows_sys::Win32::System::StationsAndDesktops::{
        CloseDesktop, OpenInputDesktop, DESKTOP_SWITCHDESKTOP,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindow, GetWindowLongW, IsIconic, IsWindowVisible, GWL_EXSTYLE, GW_HWNDPREV,
        WS_EX_LAYERED, WS_EX_TRANSPARENT,
    };
    if !DISPLAY_ON.load(std::sync::atomic::Ordering::Relaxed) {
        return false;
    }
    let hwnd = find_lightcal_hwnd();
    if hwnd.is_null() {
        return false;
    }
    // SAFETY: these calls only read window, DWM and desktop state; the input desktop handle
    // is closed before returning.
    unsafe {
        if is_cloaked(hwnd) {
            return false;
        }
        // A locked session (or a UAC prompt) switches input to a desktop we cannot open.
        let input = OpenInputDesktop(0, 0, DESKTOP_SWITCHDESKTOP);
        if input.is_null() {
            return false;
        }
        CloseDesktop(input);
        let Some(rect) = frame_rect(hwnd) else {
            return true;
        };
        let (w, h) = (rect.right - rect.left, rect.bottom - rect.top);
        let points = [(1, 1), (1, 3), (3, 1), (3, 3), (2, 2)]
            .map(|(fx, fy)| (rect.left + w * fx / 4, rect.top + h * fy / 4));
        let mut covered = [false; 5];
        let mut above = GetWindow(hwnd, GW_HWNDPREV);
        while !above.is_null() && covered.contains(&false) {
            let ex_style = GetWindowLongW(above, GWL_EXSTYLE) as u32;
            let opaque = IsWindowVisible(above) != 0
                && IsIconic(above) == 0
                && ex_style & (WS_EX_LAYERED | WS_EX_TRANSPARENT) == 0
                && !is_cloaked(above);
            if let Some(r) = opaque.then(|| frame_rect(above)).flatten() {
                for (point, hit) in points.iter().zip(covered.iter_mut()) {
                    *hit |= point.0 >= r.left
                        && point.0 < r.right
                        && point.1 >= r.top
                        && point.1 < r.bottom;
                }
            }
            above = GetWindow(above, GW_HWNDPREV);
        }
        covered.contains(&false)
    }
}

/// A WinEvent hook and the callback it reports to.
#[cfg(windows)]
type WindowChangeWatch = (
    windows_sys::Win32::UI::Accessibility::HWINEVENTHOOK,
    std::rc::Rc<dyn Fn()>,
);

thread_local! {
    /// The WinEvent hook that reports window changes elsewhere, and its callback.
    #[cfg(windows)]
    static WINDOW_CHANGES: std::cell::RefCell<Option<WindowChangeWatch>> = const { std::cell::RefCell::new(None) };
}

/// Calls `on_change` when another app's window comes to the foreground, finishes moving or
/// resizing, or is minimized or restored: the changes that can uncover this window. The
/// callback runs on this thread from its message loop. Does nothing if already watching.
#[cfg(windows)]
pub fn watch_window_changes(on_change: impl Fn() + 'static) {
    use windows_sys::Win32::UI::Accessibility::SetWinEventHook;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND, WINEVENT_OUTOFCONTEXT,
        WINEVENT_SKIPOWNPROCESS,
    };
    unsafe extern "system" fn changed(
        _hook: windows_sys::Win32::UI::Accessibility::HWINEVENTHOOK,
        _event: u32,
        _hwnd: windows_sys::Win32::Foundation::HWND,
        _object: i32,
        _child: i32,
        _thread: u32,
        _time: u32,
    ) {
        // Cloned out first, so the callback may stop the watch.
        let callback = WINDOW_CHANGES.with(|slot| slot.borrow().as_ref().map(|(_, f)| f.clone()));
        if let Some(callback) = callback {
            callback();
        }
    }
    WINDOW_CHANGES.with(|slot| {
        if slot.borrow().is_some() {
            return;
        }
        // SAFETY: an out-of-context hook with a plain function; it is removed with
        // UnhookWinEvent in `stop_watching_window_changes`.
        let hook = unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_MINIMIZEEND,
                std::ptr::null_mut(),
                Some(changed),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        };
        if !hook.is_null() {
            *slot.borrow_mut() = Some((hook, std::rc::Rc::new(on_change)));
        }
    });
}

/// Removes the hook set by `watch_window_changes`, if any.
#[cfg(windows)]
pub fn stop_watching_window_changes() {
    use windows_sys::Win32::UI::Accessibility::UnhookWinEvent;
    if let Some((hook, _)) = WINDOW_CHANGES.with(|slot| slot.borrow_mut().take()) {
        // SAFETY: `hook` came from SetWinEventHook and is unhooked once.
        unsafe { UnhookWinEvent(hook) };
    }
}

/// Windows Settings › Accessibility › Visual effects › Animation effects. Reads as on
/// when the query fails, which matches the Windows default.
#[cfg(windows)]
pub fn animation_effects_enabled() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPI_GETCLIENTAREAANIMATION,
    };
    let mut enabled: windows_sys::core::BOOL = 1;
    // SAFETY: SPI_GETCLIENTAREAANIMATION writes one BOOL to the pointer we pass.
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            &mut enabled as *mut _ as *mut _,
            0,
        )
    };
    ok == 0 || enabled != 0
}

/// Atomically positions and sizes the window next to the system tray on the current monitor,
/// in the work-area corner by the tray for whichever edge the taskbar is on. One SetWindowPos
/// makes the change show as a single step, and a window already there is left alone.
/// Returns whether the window landed exactly there:
/// false when Windows clamped it (for example to size limits the window still reports for
/// its previous content) or there is no window.
///
/// # Safety
///
/// `hwnd` must be a valid, live Win32 window handle owned by this process.
#[cfg(windows)]
pub unsafe fn position_window_by_tray(
    hwnd: windows_sys::Win32::Foundation::HWND,
    logical_w: f32,
    logical_h: f32,
) -> bool {
    if hwnd.is_null() {
        return false;
    }
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTOPRIMARY};
    use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowRect, SetWindowPos, HWND_TOP, SWP_NOACTIVATE, SWP_NOZORDER,
    };

    let dpi = GetDpiForWindow(hwnd);
    let scale = if dpi > 0 { (dpi as f32) / 96.0 } else { 1.0 };

    let phys_w = (logical_w * scale).round() as i32;
    let phys_h = (logical_h * scale).round() as i32;
    let margin = (12.0 * scale).round() as i32;

    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
    let (work_area, edge) = monitor_tray_area(monitor).unwrap_or_else(primary_tray_area);
    let (target_x, target_y) = work_area.anchor_by_tray(edge, phys_w, phys_h, margin);
    let target = (target_x, target_y, target_x + phys_w, target_y + phys_h);
    let mut placed: RECT = std::mem::zeroed();
    if GetWindowRect(hwnd, &mut placed) != 0
        && (placed.left, placed.top, placed.right, placed.bottom) == target
    {
        return true;
    }

    SetWindowPos(
        hwnd,
        HWND_TOP,
        target_x,
        target_y,
        phys_w,
        phys_h,
        SWP_NOACTIVATE | SWP_NOZORDER,
    );
    GetWindowRect(hwnd, &mut placed) != 0
        && (placed.left, placed.top, placed.right, placed.bottom) == target
}

/// Restores a minimized window and positions it accurately next to the tray. Returns whether
/// it landed exactly there; see `position_window_by_tray`.
///
/// # Safety
///
/// `hwnd` must be a valid, live Win32 window handle owned by this process.
#[cfg(windows)]
pub unsafe fn show_and_restore_window(
    hwnd: windows_sys::Win32::Foundation::HWND,
    logical_w: f32,
    logical_h: f32,
) -> bool {
    if hwnd.is_null() {
        return false;
    }
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
    };

    if IsIconic(hwnd) != 0 {
        ShowWindow(hwnd, SW_RESTORE);
    } else {
        ShowWindow(hwnd, SW_SHOW);
    }

    let placed = position_window_by_tray(hwnd, logical_w, logical_h);

    BringWindowToTop(hwnd);
    SetForegroundWindow(hwnd);
    placed
}

/// Environment variable that opts into the Windows 11 Mica backdrop prototype (ADR 0011).
pub const MICA_ENV_VAR: &str = "LIGHTCAL_MICA";

/// Returns `true` only when the Mica prototype was explicitly requested with `LIGHTCAL_MICA=1`.
pub fn mica_requested() -> bool {
    mica_flag_enabled(env::var_os(MICA_ENV_VAR).as_deref())
}

fn mica_flag_enabled(value: Option<&std::ffi::OsStr>) -> bool {
    value.is_some_and(|v| v == "1")
}

/// DWM chrome requested for the Lightcal window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowChrome {
    /// Follows the resolved app theme; used for the immersive dark-mode frame attribute.
    pub dark_mode: bool,
    /// Requests the Windows 11 main-window (Mica) system backdrop.
    pub mica: bool,
}

/// Which DWM requests the system accepted. Older Windows builds reject the Windows 11
/// attributes; callers treat a `false` field as "not available" rather than an error.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowChromeOutcome {
    /// Windows 11 accepted the round-corner preference; DWM then also draws the system shadow.
    pub rounded_corners: bool,
    /// Windows 11 22H2+ accepted the Mica backdrop request (only when Mica was requested).
    pub backdrop: bool,
}

/// Applies the DWM window chrome from ADR 0011: Windows 11 rounded corners and the system
/// shadow on the frameless Slint window, plus the optional Mica backdrop prototype.
///
/// Every call is idempotent and cheap, so it may be repeated after show, resize and theme
/// changes. Failures (Windows 10, older Windows 11 builds) are ignored: the window keeps
/// the square, shadowless frameless look it had before.
///
/// # Safety
///
/// `hwnd` must be a valid, live Win32 window handle owned by this process.
#[cfg(windows)]
pub unsafe fn apply_window_chrome(
    hwnd: windows_sys::Win32::Foundation::HWND,
    chrome: WindowChrome,
) -> WindowChromeOutcome {
    use windows_sys::Win32::Graphics::Dwm::{
        DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMSBT_AUTO, DWMSBT_MAINWINDOW,
        DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE,
        DWMWCP_ROUND,
    };
    use windows_sys::Win32::UI::Controls::MARGINS;

    let mut outcome = WindowChromeOutcome::default();
    if hwnd.is_null() {
        return outcome;
    }

    let set_i32 = |attribute: i32, value: i32| -> bool {
        DwmSetWindowAttribute(
            hwnd,
            attribute as u32,
            &value as *const i32 as *const _,
            std::mem::size_of::<i32>() as u32,
        ) >= 0
    };

    // Windows 11 (build 22000+) only; Windows 10 returns E_INVALIDARG and keeps square corners.
    outcome.rounded_corners = set_i32(DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND);

    // Measured on Windows 11: once DWM rounds this frameless winit window it also draws the
    // system shadow, so no frame extension is needed. Mica only paints in the DWM frame, so
    // the prototype turns the whole client area into frame ("sheet of glass") for the
    // backdrop to show through transparent Slint pixels; otherwise the frame stays at zero.
    let inset = if chrome.mica { -1 } else { 0 };
    let margins = MARGINS {
        cxLeftWidth: inset,
        cxRightWidth: inset,
        cyTopHeight: inset,
        cyBottomHeight: inset,
    };
    let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

    // Keep the frame colors in step with the app theme (supported from Windows 10 20H1).
    let _ = set_i32(DWMWA_USE_IMMERSIVE_DARK_MODE, i32::from(chrome.dark_mode));

    // Windows 11 22H2+; earlier builds reject the attribute.
    outcome.backdrop = set_i32(
        DWMWA_SYSTEMBACKDROP_TYPE,
        if chrome.mica {
            DWMSBT_MAINWINDOW
        } else {
            DWMSBT_AUTO
        },
    ) && chrome.mica;

    outcome
}

/// Brings this application's active window to the foreground on Windows.
pub fn bring_to_foreground() {
    #[cfg(windows)]
    {
        let hwnd = find_lightcal_hwnd();
        if !hwnd.is_null() {
            unsafe {
                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    BringWindowToTop, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE,
                };
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                }
                BringWindowToTop(hwnd);
                SetForegroundWindow(hwnd);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(windows)]
    #[test]
    fn primary_tray_area_matches_the_live_taskbar() {
        use windows_sys::Win32::Foundation::POINT;
        use windows_sys::Win32::Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
        };
        let mut mi: MONITORINFO = unsafe { std::mem::zeroed() };
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        let monitor = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
        assert_ne!(unsafe { GetMonitorInfoW(monitor, &mut mi) }, 0);
        let monitor_area = rect_area(mi.rcMonitor);
        let (work, edge) = unsafe { monitor_tray_area(monitor) }.expect("primary monitor");
        assert!(work.width > 0 && work.height > 0);
        let inside = monitor_area.x <= work.x
            && monitor_area.y <= work.y
            && work.x + work.width <= monitor_area.x + monitor_area.width
            && work.y + work.height <= monitor_area.y + monitor_area.height;
        assert!(inside, "{work:?} inside {monitor_area:?}");
        // Sessions without Explorer (some CI services) have no taskbar to compare against.
        let Some(bar) = query_shell_taskbar(mi.rcMonitor) else {
            return;
        };
        // A taskbar that keeps room (a strip, or an auto-hidden bar found) is on the
        // shell's edge; one hidden in tablet posture or not reported may keep none.
        if work != monitor_area {
            assert_eq!(
                edge, bar.edge,
                "the primary monitor has the primary taskbar"
            );
        }
        // The answer is remembered until it is forgotten.
        assert_eq!(shell_taskbar(mi.rcMonitor), shell_taskbar(mi.rcMonitor));
        assert!(SHELL_TASKBARS.with(|cache| cache.borrow().has_fresh()));
        // Hiding the window makes it stale, so the next show asks again.
        unsafe { hide_native_window(std::ptr::null_mut()) };
        assert!(!SHELL_TASKBARS.with(|cache| cache.borrow().has_fresh()));
    }

    #[cfg(windows)]
    #[test]
    fn native_window_state_follows_taskbar_minimize_restore_and_hide() {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        let window = crate::platform::windows::test_window::TestWindow::new("Lightcal");
        let hwnd = window.0;
        assert_eq!(main_window_is_open(), Some(false));
        unsafe {
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        assert_eq!(main_window_is_open(), Some(true));
        // Windows taskbar actions bypass the controller: the next toggle must still restore.
        unsafe {
            ShowWindow(hwnd, SW_MINIMIZE);
        }
        assert_eq!(main_window_is_open(), Some(false));
        assert_eq!(find_lightcal_hwnd(), hwnd);
        unsafe {
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        assert_eq!(main_window_is_open(), Some(true));
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
        assert_eq!(main_window_is_open(), Some(false));
    }

    #[cfg(windows)]
    #[test]
    fn placing_the_window_is_one_change_and_reports_a_clamp() {
        use std::cell::Cell;
        use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
        use windows_sys::Win32::UI::Shell::{
            DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        const COUNT_ID: usize = 0x4c43_5043;
        const CLAMP_ID: usize = 0x4c43_434c;

        /// Counts the native changes of size or position the user could see.
        unsafe extern "system" fn count(
            hwnd: HWND,
            message: u32,
            wp: WPARAM,
            lp: LPARAM,
            _id: usize,
            data: usize,
        ) -> LRESULT {
            if message == WM_WINDOWPOSCHANGED {
                let pos = &*(lp as *const WINDOWPOS);
                if pos.flags & SWP_NOSIZE == 0 || pos.flags & SWP_NOMOVE == 0 {
                    let changes = &*(data as *const Cell<u32>);
                    changes.set(changes.get() + 1);
                }
            }
            DefSubclassProc(hwnd, message, wp, lp)
        }
        /// Size limits the window still reports, as Slint can for the previous view.
        unsafe extern "system" fn clamp(
            hwnd: HWND,
            message: u32,
            wp: WPARAM,
            lp: LPARAM,
            _id: usize,
            _data: usize,
        ) -> LRESULT {
            let result = DefSubclassProc(hwnd, message, wp, lp);
            if message == WM_GETMINMAXINFO {
                let info = &mut *(lp as *mut MINMAXINFO);
                info.ptMaxTrackSize.x = 200;
                info.ptMaxTrackSize.y = 200;
            }
            result
        }

        let window =
            crate::platform::windows::test_window::TestWindow::new("Lightcal placement test");
        let hwnd = window.0;
        let changes = Box::new(Cell::new(0u32));
        let data = &*changes as *const Cell<u32> as usize;
        assert_ne!(
            unsafe { SetWindowSubclass(hwnd, Some(count), COUNT_ID, data) },
            0
        );
        // Settings, then the calendar: each a single change, exactly where asked.
        for (width, height) in [(448.0, 360.0), (352.0, 432.0)] {
            let before = changes.get();
            assert!(unsafe { position_window_by_tray(hwnd, width, height) });
            assert_eq!(changes.get() - before, 1, "{width} × {height} in one step");
        }
        assert_ne!(
            unsafe { SetWindowSubclass(hwnd, Some(clamp), CLAMP_ID, 0) },
            0
        );
        assert!(!unsafe { position_window_by_tray(hwnd, 448.0, 360.0) });
        assert!(!unsafe { position_window_by_tray(std::ptr::null_mut(), 448.0, 360.0) });
        unsafe {
            RemoveWindowSubclass(hwnd, Some(clamp), CLAMP_ID);
            RemoveWindowSubclass(hwnd, Some(count), COUNT_ID);
        }
    }

    #[test]
    fn mica_prototype_needs_an_explicit_one() {
        use std::ffi::OsStr;
        assert!(!mica_flag_enabled(None));
        assert!(!mica_flag_enabled(Some(OsStr::new(""))));
        assert!(!mica_flag_enabled(Some(OsStr::new("0"))));
        assert!(!mica_flag_enabled(Some(OsStr::new("true"))));
        assert!(mica_flag_enabled(Some(OsStr::new("1"))));
    }

    #[cfg(windows)]
    #[test]
    fn window_chrome_is_best_effort_and_reports_what_dwm_accepted() {
        use windows_sys::Win32::Graphics::Dwm::{
            DwmGetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
        };

        let chrome = WindowChrome {
            dark_mode: true,
            mica: false,
        };
        assert_eq!(
            unsafe { apply_window_chrome(std::ptr::null_mut(), chrome) },
            WindowChromeOutcome::default()
        );

        let window = crate::platform::windows::test_window::TestWindow::new("Lightcal chrome test");
        let hwnd = window.0;
        let outcome = unsafe { apply_window_chrome(hwnd, chrome) };
        // Without the prototype flag no backdrop is ever reported.
        assert!(!outcome.backdrop);
        if outcome.rounded_corners {
            // Windows 11: DWM stores the preference. Windows 10 rejects it, which is fine.
            let mut corner = 0i32;
            let hr = unsafe {
                DwmGetWindowAttribute(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                    &mut corner as *mut i32 as *mut _,
                    std::mem::size_of::<i32>() as u32,
                )
            };
            assert!(hr >= 0);
            assert_eq!(corner, DWMWCP_ROUND);
        }
        // Repeating the call (show, resize, theme change) must be harmless.
        assert_eq!(unsafe { apply_window_chrome(hwnd, chrome) }, outcome);
    }

    #[test]
    fn isolated_startup_manager_writes_and_removes_marker_without_touching_registry() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = env::temp_dir().join(format!("lightcal-startup-test-{stamp}"));
        let mgr = StartupManager {
            isolated_marker: Some(dir.join("autostart_entry.txt")),
        };
        let fake_exe = Path::new(r"C:\Program Files\Lightcal\lightcal.exe");
        mgr.set_enabled(true, fake_exe).unwrap();
        assert!(mgr.is_enabled());
        mgr.set_enabled(false, fake_exe).unwrap();
        assert!(!mgr.is_enabled());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn shortcut_manager_rejects_reserved_os_hotkeys() {
        let mut mgr = ShortcutManager::new();
        assert!(mgr.bind(Some("Win + L")).is_err());
        assert_eq!(
            mgr.bind(Some("Ctrl + Alt + C")).unwrap().as_deref(),
            Some("Ctrl + Alt + C")
        );
        assert_eq!(mgr.bind(None).unwrap(), None);
    }

    #[test]
    #[cfg(windows)]
    fn parse_shortcut_to_vk_matches_modifiers_and_vk() {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, VK_F2, VK_SPACE,
        };

        let (mods1, vk1) = parse_shortcut_to_vk("Ctrl + Alt + C");
        assert_eq!(mods1, MOD_NOREPEAT | MOD_CONTROL | MOD_ALT);
        assert_eq!(vk1, b'C' as u32);

        let (mods2, vk2) = parse_shortcut_to_vk("Win + Shift + Space");
        assert_eq!(mods2, MOD_NOREPEAT | MOD_WIN | MOD_SHIFT);
        assert_eq!(vk2, VK_SPACE as u32);

        let (mods3, vk3) = parse_shortcut_to_vk("Alt + F2");
        assert_eq!(mods3, MOD_NOREPEAT | MOD_ALT);
        assert_eq!(vk3, VK_F2 as u32);
    }

    #[test]
    #[cfg(windows)]
    fn single_instance_detects_duplicate_and_signals_event() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let app_id = format!("Lightcal_Test_{stamp}");

        let first = SingleInstance::acquire(&app_id);
        let guard = match first {
            SingleInstance::FirstInstance(g) => g,
            SingleInstance::AlreadyRunning => panic!("First instance should have acquired"),
        };

        assert!(!guard.check_activation_requested());

        let second = SingleInstance::acquire(&app_id);
        match second {
            SingleInstance::AlreadyRunning => {}
            SingleInstance::FirstInstance(_) => {
                panic!("Second instance should have detected duplicate")
            }
        }

        assert!(guard.check_activation_requested());
        assert!(!guard.check_activation_requested()); // Auto-reset

        // Test async listening
        let activated = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let act_clone = activated.clone();
        guard.listen_for_activation(move || {
            act_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        let third = SingleInstance::acquire(&app_id);
        match third {
            SingleInstance::AlreadyRunning => {}
            SingleInstance::FirstInstance(_) => {
                panic!("Third instance should have detected duplicate")
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(activated.load(std::sync::atomic::Ordering::SeqCst));

        // Dropping the guard (Quit while a write is pending) lets a new launch start at
        // once, and the old listener leaves its activations alone.
        let late = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let late_clone = late.clone();
        let listener = guard
            .listen_for_activation(move || {
                late_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            })
            .unwrap();
        drop(guard);
        listener.join().unwrap();
        let relaunch = match SingleInstance::acquire(&app_id) {
            SingleInstance::FirstInstance(g) => g,
            SingleInstance::AlreadyRunning => panic!("A relaunch should start normally"),
        };
        assert!(matches!(
            SingleInstance::acquire(&app_id),
            SingleInstance::AlreadyRunning
        ));
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(!late.load(std::sync::atomic::Ordering::SeqCst));
        assert!(relaunch.check_activation_requested());
    }

    #[test]
    #[cfg(windows)]
    fn the_activation_listener_works_without_a_stop_event() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let app_id = format!("Lightcal_Test_NoStop_{stamp}");
        let mut guard = match SingleInstance::acquire(&app_id) {
            SingleInstance::FirstInstance(g) => g,
            SingleInstance::AlreadyRunning => panic!("First instance should have acquired"),
        };
        // As if CreateEventW had failed for the stop event.
        unsafe { windows_sys::Win32::Foundation::CloseHandle(guard.stop) };
        guard.stop = std::ptr::null_mut();
        let activated = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let observed = activated.clone();
        let listener = guard
            .listen_for_activation(move || {
                observed.store(true, std::sync::atomic::Ordering::SeqCst);
            })
            .unwrap();
        assert!(matches!(
            SingleInstance::acquire(&app_id),
            SingleInstance::AlreadyRunning
        ));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !activated.load(std::sync::atomic::Ordering::SeqCst)
            && std::time::Instant::now() < deadline
        {
            std::thread::yield_now();
        }
        assert!(activated.load(std::sync::atomic::Ordering::SeqCst));

        // Released early (Quit while a write is pending). The activation that follows at
        // once wakes the old listener, which is blocked on the event well inside its poll
        // interval: it passes the activation on and ends.
        activated.store(false, std::sync::atomic::Ordering::SeqCst);
        drop(guard);
        let relaunch = match SingleInstance::acquire(&app_id) {
            SingleInstance::FirstInstance(g) => g,
            SingleInstance::AlreadyRunning => panic!("A relaunch should start normally"),
        };
        assert!(matches!(
            SingleInstance::acquire(&app_id),
            SingleInstance::AlreadyRunning
        ));
        // The relaunch looks only once the old listener has ended, so the old one alone
        // could have taken the activation.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !listener.is_finished() {
            assert!(
                std::time::Instant::now() < deadline,
                "the old listener ends after the release"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(!activated.load(std::sync::atomic::Ordering::SeqCst));
        assert!(
            relaunch.check_activation_requested(),
            "the activation went on to the relaunch"
        );
    }

    #[test]
    #[cfg(windows)]
    fn session_end_watch_never_delays_and_runs_only_when_the_session_ends() {
        use std::cell::Cell;
        use std::rc::Rc;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SendMessageW, WM_ENDSESSION, WM_QUERYENDSESSION,
        };
        let ended = Rc::new(Cell::new(0));
        let counter = ended.clone();
        let watch = SessionEndWatch::new(move || counter.set(counter.get() + 1)).unwrap();
        let hwnd = watch.hwnd();
        assert_eq!(unsafe { SendMessageW(hwnd, WM_QUERYENDSESSION, 0, 0) }, 1);
        assert_eq!(ended.get(), 0);
        // A cancelled end (wParam FALSE) does nothing.
        unsafe { SendMessageW(hwnd, WM_ENDSESSION, 0, 0) };
        assert_eq!(ended.get(), 0);
        unsafe { SendMessageW(hwnd, WM_ENDSESSION, 1, 0) };
        assert_eq!(ended.get(), 1);
        // A second watch registers the same class without trouble.
        let second = SessionEndWatch::new(|| {}).unwrap();
        drop(second);
        drop(watch);
        assert_eq!(Rc::strong_count(&ended), 1, "the callback is freed");
    }
}
