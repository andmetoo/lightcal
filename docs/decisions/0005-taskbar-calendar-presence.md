# ADR 0005: Preserve taskbar activation and show the calendar in previews

Date: 2026-10-30. Status: accepted by the owner in chat.

Removing the expanded agenda must preserve both tray and taskbar entry points. The owner reported that right-clicking the taskbar button caused it to disappear, preventing pinning, and that Explorer displayed an empty preview.

## Implementation

- Remove the global foreground hook that hid every compact window when Explorer acquired focus. A taskbar menu is a normal focus transfer, not a dismissal.
- Keep the native window alive and registered with the taskbar. Tray toggles, Escape, and Close minimize it; activation explicitly restores it. Do not force or programmatically change the owner's pins.
- Use the current native minimized state when toggling rather than relying on a controller flag that Windows taskbar actions can leave stale.
- Explicitly disable the tray library's default menu-on-left-click behavior. Left clicks only toggle the calendar; right clicks keep the standard context menu.
- Set a stable portable application ID before creating the window. Do not replace an installed MSIX application's Windows-assigned identity.
- Handle Windows DWM thumbnail and Peek requests with calendar snapshots, resized to the requested bounds and encoded as premultiplied 32-bit BGRA DIBs. Keep the last calendar available while settings are displayed. Invalidate DWM's cache after UI state changes. Release GDI bitmaps after DWM copies them and detach the subclass before releasing its state.
- The subclass is attached to this process's native window only. There is no Explorer injection or global mouse polling.
- A custom `LIGHTCAL_DATA_DIR` gets an isolated single-instance namespace so fixture launches cannot activate the owner's real instance.

## Validation

Unit tests verify aspect ratio, bounds, premultiplied color channels, and the actual Windows GDI bitmap's dimensions and bit depth. A manually sent DWM message does not reproduce Explorer's live thumbnail request and is not evidence of successful preview presentation. Record interactive Explorer checks separately from unit-test results.

Manual checks: open from tray; minimize and restore from the taskbar; open the taskbar context menu and verify pinning remains available; hover for a calendar preview; check Peek; change month/theme and repeat while minimized; open settings and confirm the retained calendar preview; close and restore; forward a second launch; explicitly Quit. Use fixture calendars and temporary data. Do not change real pinning, autostart, credentials, or registry state during automated tests.
