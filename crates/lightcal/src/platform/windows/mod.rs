//! Windows shell integration: tray icon, dynamic day-of-month rendering,
//! single-instance activation, startup registry adapter, global shortcut binding,
//! and read-only WinRT calendar provider.

pub mod clipboard;

/// `%WINDIR%\Fonts` (falling back to `%SystemRoot%\Fonts`), where Windows keeps system fonts.
pub fn system_fonts_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("WINDIR")
        .or_else(|| std::env::var_os("SystemRoot"))
        .map(|windows| std::path::PathBuf::from(windows).join("Fonts"))
}

/// Opens File Explorer with `file` selected (Show in folder). Only ever called on a
/// user's click; nothing happens on other platforms.
pub fn reveal_in_explorer(file: &std::path::Path) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Explorer parses `/select,"path"` itself, so the path is passed raw and quoted.
        let _ = std::process::Command::new("explorer.exe")
            .raw_arg(format!("/select,\"{}\"", file.display()))
            .spawn();
    }
    #[cfg(not(windows))]
    let _ = file;
}

pub mod icon;
pub mod provider;
pub mod shell;
#[cfg(windows)]
pub mod taskbar;
pub mod timezone;

/// An off-screen, initially hidden top-level window for native tests, destroyed on drop.
#[cfg(all(test, windows))]
pub(crate) mod test_window {
    use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};

    pub struct TestWindow(pub HWND);

    impl TestWindow {
        pub fn new(title: &str) -> Self {
            let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
            let title: Vec<u16> = title.encode_utf16().chain([0]).collect();
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
            Self(hwnd)
        }
    }

    impl Drop for TestWindow {
        fn drop(&mut self) {
            unsafe {
                DestroyWindow(self.0);
            }
        }
    }
}

pub use icon::{
    encode_ico_from_pngs, generate_multi_size_ico, render_squircle_icon_generic,
    render_squircle_icon_rgba_64, render_tray_icon_rgba, render_tray_icon_variant_rgba,
};
pub use provider::{
    load_provider_snapshot_async, query_calendar_provider, query_windows_calendar_store,
};
#[cfg(windows)]
pub use shell::{
    animation_effects_enabled, apply_window_chrome, find_lightcal_hwnd, forget_shell_taskbar,
    hide_native_window, main_window_is_open, main_window_is_seen, position_window_by_tray,
    set_display_on, show_and_restore_window, stop_watching_window_changes, watch_window_changes,
};
pub use shell::{
    bring_to_foreground, mica_requested, primary_tray_area, system_prefers_dark_mode,
    ShortcutManager, SingleInstance, SingleInstanceGuard, StartupManager, WindowChrome,
    WindowChromeOutcome, MICA_ENV_VAR, RUN_KEY_VALUE_NAME,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences::TrayIconMode;
    use calendar_core::Weekday;

    #[test]
    fn reexported_tray_icon_renders_valid_rgba() {
        let dark = render_tray_icon_rgba(29, true);
        assert_eq!(dark.len(), 32 * 32 * 4);
    }

    #[test]
    fn reexported_variant_renders_valid_rgba() {
        let rgba = render_tray_icon_variant_rgba(
            TrayIconMode::WeekdayAndDate,
            false,
            29,
            Weekday::Thursday,
            (22, 1),
            true,
        );
        assert_eq!(rgba.len(), 32 * 32 * 4);
    }
}
