# Architecture proposal

Status: the Rust/Slint prototype is implemented. The sections below retain the planning baseline; [ADR 0002](decisions/0002-direct-calendars-and-updates.md) supersedes its Windows-only provider and manual-install restrictions. Direct providers live in `crates/lightcal/src/calendars`, bounded HTTPS in `net.rs`, Windows credential encryption in `secure_store.rs`, and the signed portable updater in `installer.rs`. Calendar arithmetic stays in `calendar-core`; Windows timezone-name conversion is behind a platform adapter. Read [setup](provider-setup.md) for configuration and remaining live validation.

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
design/             # Penpot source, page manifest, and review evidence
```

Start with these two crates. Split platform crates only when dependencies or ownership justify it. Keep OS calls and toolkit types out of `calendar-core`. The core accepts an explicit current date and first-weekday preference, allowing deterministic tests without changing the real system clock.

## State and lifecycle

Application state contains displayed month, selected date, today, preferences, and presentation (initially background, compact calendar, settings, or minimized). Both tray and taskbar restore the same compact calendar. Closing or dismissing minimizes the native window; loss of focus preserves it. Quit releases tray/menu objects and exits. Windows DWM thumbnail and Peek requests use Slint calendar pixels via a subclass attached only to the process-owned window.

Windows-specific work includes stable application identity and shortcut association, visible-window styles for taskbar/Alt-Tab behavior, per-monitor DPI awareness, tray rectangle lookup, focus and activation, and Explorer restart recovery. A single-instance mechanism must be per-user, accept only bounded activation messages, and reject arbitrary commands or file paths. Its exact implementation is selected in the prototype.

Route tray callbacks into the existing event loop. Create platform UI objects on the threads required by their APIs; macOS UI/menu-bar work belongs on the main thread. Never block the UI thread waiting for another process or performing disk writes.

## Time and power

Use local system time for today. Schedule the next local date boundary rather than repainting every second. Recompute after resume, timezone/date changes, and before opening the calendar; days are not always 24 hours. Reconcile on display and taskbar changes. When hidden, do not render or poll for mouse state.

Use logical coordinates for layout and convert physical tray/monitor coordinates explicitly. Clamp to monitor work area. If Windows cannot supply the tray rectangle, use the invocation point or active-monitor work area as a documented fallback. Reposition after DPI or monitor changes; never restore a window off screen.

## Preferences and diagnostics

Store a small versioned JSON preferences file in the OS per-user application configuration directory. Final folder/bundle identifiers are decided before the first installer. No SQLite is needed for these preferences.

Write via a temporary file and platform-appropriate atomic replacement, retaining a last-known-good copy. On malformed or unsupported newer preferences, start with safe defaults, inform the user, and preserve the original instead of overwriting it. Log write failures without freezing or closing the calendar.

`LIGHTCAL_DATA_DIR` redirects all application-owned writable data in development and tests. Autostart, registry/shortcut changes, and real profile writes are disabled in automated tests unless explicitly exercised in an isolated profile. Logs are opt-in, bounded, and contain no personal calendar data. No telemetry is permitted. Explicit update checks may contact GitHub; About links open only on user action.

## Localization

Use bundled message catalogs for `en`, `ru`, `es`, and `ja`, with English as the initial and fallback language. Keep locale identifiers in preferences and message keys in code; localize in the presentation layer, including tray/menu strings and accessibility announcements. Use the selected locale for month/weekday names, date patterns, and event-count plurals. Never translate provider/user event data. A language change rebuilds presentation strings without re-querying or altering the calendar domain state. Use installed Japanese-capable font fallback and test glyph coverage; no remote font requests. Preserve system timezone and user-chosen weekday settings across language changes.

## Calendar provider, shortcut, and update checking

Follow [Required features](features.md). Keep provider enumeration and bounded occurrence queries behind a Windows adapter. The provider feasibility probe precedes final installer/package selection. Windows appointment-store access does not imply access to every Outlook calendar. Use read-only APIs and explicit user connection; store selected opaque calendar IDs separately from labels. Query the visible date range, cancel obsolete requests when navigating, and never block calendar opening on provider work. Cache only bounded data in memory initially; keep stale/loading/error distinct from a confirmed absence of events. Do not persist event titles or credentials without another design decision.

Represent event occurrences with source ID, event ID, calendar color, timezone-aware boundaries, and all-day semantics. Map them into displayed local dates in the domain layer, deduplicating per source/event/occurrence. Changes to source selection invalidate dot counts. Tests use a synthetic provider, never the user's live calendars.

Store gray-weekday selection as a weekday set; retain a separate first-weekday preference. A platform shortcut adapter registers only an explicitly configured combination and posts activation into the existing GUI loop. Startup registration and shortcut binding need rollback/error behavior; preferences must not claim registration succeeded when it failed.

The update checker is a separate bounded HTTPS request to GitHub release metadata, invoked manually. Compare stable semantic versions, exclude drafts/prereleases, preserve OS TLS validation, and handle timeout/rate-limit/offline states. Accept release-page links only from the expected repository on github.com; a response must never become an arbitrary executable or shell command. No background polling, binary installation, or restart is introduced. Document GitHub's receipt of normal connection metadata in the release privacy statement.

## Future macOS port

Reuse date logic, preference schema, and the selected view where practical. Replace tray semantics with a menu-bar status item and review Dock activation separately. Windows taskbar behavior is not a portable abstraction. Signing and notarization are a later distribution gate.

The `macOS application build` check runs the portable tests on `macos-latest`, then `scripts/package-dmg.sh` release-builds `lightcal` and packages an unsigned disk image. The workflow uploads that image as `lightcal-macos-<arch>-unsigned` and keeps it for 14 days. Preferences for that binary use `~/Library/Application Support/Lightcal`. Allowed links open with the system `open` command, and the calendar privacy action targets the macOS Privacy pane. Login-item copy on that binary does not mention Windows.

The review bundle uses the provisional identifier `app.koona.lightcal`. Its Info.plist names macOS 11 as a minimum so Launch Services can read the bundle. Neither value is a frozen release identity or a tested support matrix. The script does not sign, notarize, or staple the image, and the workflow does not publish a GitHub release. Gatekeeper blocks the application until it is opened explicitly. A green job is not a launch, menu-bar, Dock, accessibility, or calendar-provider result.
