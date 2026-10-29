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

`LIGHTCAL_DATA_DIR` redirects all application-owned writable data in development and tests. Autostart, registry/shortcut changes, and real profile writes are disabled in automated tests unless explicitly exercised in an isolated profile. Logs are opt-in, bounded, and contain no personal calendar data. No telemetry is permitted. Explicit update checks may contact GitHub; About links open only on user action.

## Calendar provider, shortcut, and update checking

Follow [Required features](features.md). Keep provider enumeration and bounded occurrence queries behind a Windows adapter. The provider feasibility probe precedes final installer/package selection. Windows appointment-store access does not imply access to every Outlook calendar. Use read-only APIs and explicit user connection; store selected opaque calendar IDs separately from labels. Query the visible date range, cancel obsolete requests when navigating, and never block calendar opening on provider work. Cache only bounded data in memory initially; keep stale/loading/error distinct from a confirmed absence of events. Do not persist event titles or credentials without another design decision.

Represent event occurrences with source ID, event ID, calendar color, timezone-aware boundaries, and all-day semantics. Map them into displayed local dates in the domain layer, deduplicating per source/event/occurrence. Changes to source selection invalidate dot counts. Tests use a synthetic provider, never the user's live calendars.

Store gray-weekday selection as a weekday set; retain a separate first-weekday preference. A platform shortcut adapter registers only an explicitly configured combination and posts activation into the existing GUI loop. Startup registration and shortcut binding need rollback/error behavior; preferences must not claim registration succeeded when it failed.

The update checker is a separate bounded HTTPS request to GitHub release metadata, invoked manually. Compare stable semantic versions, exclude drafts/prereleases, preserve OS TLS validation, and handle timeout/rate-limit/offline states. Accept release-page links only from the expected repository on github.com; a response must never become an arbitrary executable or shell command. No background polling, binary installation, or restart is introduced. Document GitHub's receipt of normal connection metadata in the release privacy statement.

## Future macOS port

Reuse date logic, preference schema, and the selected view where practical. Replace tray semantics with a menu-bar status item and review Dock activation separately. Windows taskbar behavior is not a portable abstraction. Build and launch on macOS runners/hardware; successful Windows compilation or cross-compilation alone is insufficient. Signing/notarization is a later distribution gate.
