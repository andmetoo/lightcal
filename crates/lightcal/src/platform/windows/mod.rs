//! Windows shell integration: tray icon, dynamic day-of-month rendering,
//! single-instance activation, startup registry adapter, global shortcut binding,
//! and read-only WinRT calendar provider.

pub mod icon;
pub mod provider;
pub mod shell;
#[cfg(windows)]
pub mod taskbar;
pub mod timezone;

pub use icon::{
    encode_ico_from_pngs, generate_multi_size_ico, render_native_tray_icon,
    render_squircle_icon_generic, render_squircle_icon_rgba_64, render_tray_icon_rgba,
    render_tray_icon_variant_rgba,
};
pub use provider::{
    load_provider_snapshot_async, query_calendar_provider, query_windows_calendar_store,
};
pub use shell::{
    bring_to_foreground, find_lightcal_hwnd, position_window_above_tray, primary_work_area,
    show_and_restore_window, system_prefers_dark_mode, ShortcutManager, SingleInstance,
    SingleInstanceGuard, StartupManager, RUN_KEY_VALUE_NAME,
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
