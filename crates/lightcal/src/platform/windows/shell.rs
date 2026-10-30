//! Windows shell integration: single-instance synchronization, window work area,
//! dark mode registry queries, user startup registration, and global shortcuts.

use crate::platform::{normalize_shortcut, WorkArea};
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

/// Queries the primary monitor work area on Windows, falling back to `1920×1040`.
pub fn primary_work_area() -> WorkArea {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::RECT;
        use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA};
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        };
        let ok =
            unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut rect as *mut _ as *mut _, 0) };
        if ok != 0 && rect.right > rect.left && rect.bottom > rect.top {
            return WorkArea {
                x: rect.left,
                y: rect.top,
                width: rect.right - rect.left,
                height: rect.bottom - rect.top,
            };
        }
    }
    WorkArea {
        x: 0,
        y: 0,
        width: 1920,
        height: 1040,
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
            let res = resp_rx.recv().map_err(|e| e.to_string())??;
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
pub struct SingleInstanceGuard {
    #[cfg(windows)]
    _mutex: windows_sys::Win32::Foundation::HANDLE,
    #[cfg(windows)]
    event: windows_sys::Win32::Foundation::HANDLE,
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
    pub fn listen_for_activation<F>(&self, on_activate: F)
    where
        F: Fn() + Send + 'static,
    {
        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::{
                CloseHandle, DuplicateHandle, DUPLICATE_SAME_ACCESS, WAIT_OBJECT_0,
            };
            use windows_sys::Win32::System::Threading::{
                GetCurrentProcess, WaitForSingleObject, INFINITE,
            };

            let mut dup_event = std::ptr::null_mut();
            let ok = unsafe {
                DuplicateHandle(
                    GetCurrentProcess(),
                    self.event,
                    GetCurrentProcess(),
                    &mut dup_event,
                    0,
                    0,
                    DUPLICATE_SAME_ACCESS,
                )
            };
            if ok != 0 && !dup_event.is_null() {
                let raw_handle = dup_event as isize;
                std::thread::spawn(move || {
                    let event_handle = raw_handle as windows_sys::Win32::Foundation::HANDLE;
                    loop {
                        let res = unsafe { WaitForSingleObject(event_handle, INFINITE) };
                        if res == WAIT_OBJECT_0 {
                            on_activate();
                        } else {
                            break;
                        }
                    }
                    unsafe {
                        CloseHandle(event_handle);
                    }
                });
            }
        }
        #[cfg(not(windows))]
        {
            let _ = on_activate;
        }
    }
}

#[cfg(windows)]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        unsafe {
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

            SingleInstance::FirstInstance(SingleInstanceGuard {
                _mutex: mutex,
                event,
            })
        }
        #[cfg(not(windows))]
        {
            let _ = app_id;
            SingleInstance::FirstInstance(SingleInstanceGuard {})
        }
    }
}

/// Finds the main application HWND owned by this process.
#[cfg(windows)]
pub fn find_lightcal_hwnd() -> windows_sys::Win32::Foundation::HWND {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindow, GetWindowTextW, GetWindowThreadProcessId, GW_OWNER,
    };

    unsafe extern "system" fn find(hwnd: HWND, data: LPARAM) -> i32 {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == GetCurrentProcessId() && GetWindow(hwnd, GW_OWNER).is_null() {
            let mut title_buf = [0u16; 64];
            let len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 64);
            if len > 0 {
                let title = String::from_utf16_lossy(&title_buf[..len as usize]);
                if title == "Lightcal" {
                    *(data as *mut HWND) = hwnd;
                    return 0; // stop enumeration
                }
            }
        }
        1
    }

    let mut hwnd: HWND = std::ptr::null_mut();
    unsafe {
        EnumWindows(Some(find), &mut hwnd as *mut HWND as LPARAM);
    }
    hwnd
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

/// Atomically positions and sizes the window above the system tray / taskbar on the current monitor.
///
/// # Safety
///
/// `hwnd` must be a valid, live Win32 window handle owned by this process.
#[cfg(windows)]
pub unsafe fn position_window_above_tray(
    hwnd: windows_sys::Win32::Foundation::HWND,
    logical_w: f32,
    logical_h: f32,
) {
    if hwnd.is_null() {
        return;
    }
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
    };
    use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOP, SWP_NOACTIVATE, SWP_NOZORDER,
    };

    let dpi = GetDpiForWindow(hwnd);
    let scale = if dpi > 0 { (dpi as f32) / 96.0 } else { 1.0 };

    let phys_w = (logical_w * scale).round() as i32;
    let phys_h = (logical_h * scale).round() as i32;
    let margin = (12.0 * scale).round() as i32;

    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
    let mut mi: MONITORINFO = std::mem::zeroed();
    mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;

    let (work_x, work_y, work_w, work_h) = if GetMonitorInfoW(monitor, &mut mi) != 0 {
        (
            mi.rcWork.left,
            mi.rcWork.top,
            mi.rcWork.right - mi.rcWork.left,
            mi.rcWork.bottom - mi.rcWork.top,
        )
    } else {
        let wa = primary_work_area();
        (wa.x, wa.y, wa.width, wa.height)
    };

    let (target_x, target_y) = WorkArea {
        x: work_x,
        y: work_y,
        width: work_w,
        height: work_h,
    }
    .anchor_above_tray(phys_w, phys_h, margin);

    SetWindowPos(
        hwnd,
        HWND_TOP,
        target_x,
        target_y,
        phys_w,
        phys_h,
        SWP_NOACTIVATE | SWP_NOZORDER,
    );
}

/// Restores a minimized window and positions it accurately above the tray.
///
/// # Safety
///
/// `hwnd` must be a valid, live Win32 window handle owned by this process.
#[cfg(windows)]
pub unsafe fn show_and_restore_window(
    hwnd: windows_sys::Win32::Foundation::HWND,
    logical_w: f32,
    logical_h: f32,
) {
    if hwnd.is_null() {
        return;
    }
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
    };

    if IsIconic(hwnd) != 0 {
        ShowWindow(hwnd, SW_RESTORE);
    } else {
        ShowWindow(hwnd, SW_SHOW);
    }

    position_window_above_tray(hwnd, logical_w, logical_h);

    BringWindowToTop(hwnd);
    SetForegroundWindow(hwnd);
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
    fn native_window_state_follows_taskbar_minimize_restore_and_hide() {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        struct TestWindow(windows_sys::Win32::Foundation::HWND);
        impl Drop for TestWindow {
            fn drop(&mut self) {
                unsafe {
                    DestroyWindow(self.0);
                }
            }
        }
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        let title: Vec<u16> = "Lightcal\0".encode_utf16().collect();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW,
                -10000,
                -10000,
                352,
                432,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        assert!(!hwnd.is_null());
        let _window = TestWindow(hwnd);
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
    }
}
