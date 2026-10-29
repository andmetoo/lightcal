# Architecture proposal

Status: planned; no application code exists yet.

## Boundaries

Use one process and one desktop GUI event loop. Avoid a server, browser runtime, database, or async runtime unless an actual feature needs one.

Proposed Cargo workspace:

```text
crates/
  calendar-core/     # Pure calendar calculations and navigation state
  lightcal/          # Application lifecycle, UI, preferences, platform adapters
    ui/             # Slint views, selected in ADR 0001
    src/platform/
      windows/      # Tray, window styles, shell identity, system events
      macos/        # Menu bar and activation behavior, added during probe
design/             # Lunacy source files and exported previews, when created
```

Start with these two crates. Split platform crates only when dependencies or ownership justify it. Keep OS calls and toolkit types out of `calendar-core`. The core accepts an explicit current date and first-weekday preference, allowing deterministic tests without changing the real system clock.

## State and lifecycle

Application state contains displayed month, selected date, today, preferences, and presentation (`Hidden`, `Compact`, or `Expanded`). UI callbacks dispatch typed Rust actions; the state drives both views. Closing a surface changes presentation, while Quit releases tray/menu objects and exits.

Windows-specific work includes stable application identity and shortcut association, visible-window styles for taskbar/Alt-Tab behavior, per-monitor DPI awareness, tray rectangle lookup, focus and activation, and Explorer restart recovery. A single-instance mechanism must be per-user, accept only bounded activation messages, and reject arbitrary commands or file paths. Its exact implementation is selected in the prototype.

Route tray callbacks into the existing event loop. Create platform UI objects on the threads required by their APIs; macOS UI/menu-bar work belongs on the main thread. Never block the UI thread waiting for another process or performing disk writes.

## Time and power

Use local system time for today. Schedule the next local date boundary rather than repainting every second. Recompute after resume, timezone/date changes, and before opening the calendar; days are not always 24 hours. Reconcile on display and taskbar changes. When hidden, do not render or poll for mouse state.

Use logical coordinates for layout and convert physical tray/monitor coordinates explicitly. Clamp to monitor work area. If Windows cannot supply the tray rectangle, use the invocation point or active-monitor work area as a documented fallback. Reposition after DPI or monitor changes; never restore a window off screen.

## Preferences and diagnostics

Store a small versioned JSON preferences file in the OS per-user application configuration directory. Final folder/bundle identifiers are decided before the first installer. No SQLite is needed for these preferences.

Write via a temporary file and platform-appropriate atomic replacement, retaining a last-known-good copy. On malformed or unsupported newer preferences, start with safe defaults, inform the user, and preserve the original instead of overwriting it. Log write failures without freezing or closing the calendar.

`LIGHTCAL_DATA_DIR` redirects all application-owned writable data in development and tests. Autostart, registry/shortcut changes, and real profile writes are disabled in automated tests unless explicitly exercised in an isolated profile. Logs are opt-in, bounded, and contain no personal calendar data. Initial runtime behavior requires no outbound requests or telemetry.

## Future macOS port

Reuse date logic, preference schema, and the selected view where practical. Replace tray semantics with a menu-bar status item and review Dock activation separately. Windows taskbar behavior is not a portable abstraction. Build and launch on macOS runners/hardware; successful Windows compilation or cross-compilation alone is insufficient. Signing/notarization is a later distribution gate.
