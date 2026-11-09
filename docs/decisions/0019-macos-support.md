# ADR 0019: macOS menu bar calendar

Date: 2026-11-09. Status: accepted by the owner in chat; implementation and validation in progress.
Supersedes the Windows-only restriction in [ADR 0011](0011-windows-only-design-refresh.md).

## Decision

Support Windows and macOS 14 or newer, on Intel and Apple Silicon. Keep Rust,
Slint and the shared calendar design. Calendar arithmetic stays independent of
platform APIs. A small Objective-C bridge provides AppKit, EventKit,
ServiceManagement, clipboard and shortcut services through Rust adapters; Slint
owns the single main event loop. This does not change the UI framework.

On macOS the Dock and menu bar both open the calendar. The menu bar always has
the live date icon, with optional adjacent weekday text (Off, Short, Full) and
a separate month toggle. Short weekday and month are the defaults. The owner
replaced the earlier year option with a localized abbreviated month; saved year
toggle values migrate to the month toggle. Menu bar text uses 14 pt medium weight. Text follows
the application's selected language. The running Dock icon shows today's date in
a colored calendar tile and refreshes when the date changes. The calendar header
omits the Windows minimize control on macOS. Closing a window leaves the app resident.

The owner subsequently approved direct read-only Outlook and Google connections
on macOS, alongside EventKit. The existing browser, loopback, PKCE/state and
read-only scopes are shared with Windows. macOS refresh tokens and client
configuration live in an app-scoped Keychain item; disconnect removes only local
credentials. Keychain I/O runs on provider workers, never the UI thread. Tests
and isolated data directories cannot access the real Keychain.

Lightcal's local-event store/editor and private ICS connections remain absent on
macOS. Native accounts and event creation/editing belong in Apple Calendar;
direct accounts connect through Lightcal. EventKit permission is requested only
by an explicit connection action. Apple's Full Access permission is needed for
EventKit reads, while direct Google/Outlook connections do not depend on it.
Lightcal never writes remote events. No calendars start selected. Windows
features are retained.

Distribution uses a universal app and DMG, Developer ID signing, notarization,
and Sparkle-signed updates. Tag workflows prepare draft release assets; the owner
publishes explicitly. Automatic checking and downloads are opt-in, with Sparkle
presenting installation/restart choices. Telemetry remains excluded.

## Window placement and Settings motion

Owner follow-up on 2026-11-09: keep the popup anchored below its menu bar item,
including first launch, Dock activation, switching views and display changes.
Placement tracks the exact Slint-owned AppKit view and is repeated after pending
layout changes. Resizing never activates or opens a hidden window. If the status
item is unavailable, use the top-right of the window's current display, not its center.

Pointer-driven Settings navigation uses the existing 160 ms Motion move duration
and a strong ease-out opacity crossfade. A temporary native snapshot of the old
view fades out while the newly sized, anchored view fades in; window dimensions
are not animated and Slint does not repaint for every opacity frame. Rapid changes
retarget one transition. Hiding cancels it and releases the snapshot. Reduce Motion,
Escape and global shortcut activation apply immediately. Windows Settings behavior
from ADR 0016 is unchanged.

## Integration gate and evidence

The [ADR 0001](0001-ui-framework.md) prototype gate applies to this new platform.
The native bridge passes Xcode clang syntax checks on the development Mac, and
the Rust/Slint integration compiles. These facts do not establish the desktop,
accessibility or performance gates. Full desktop checks on both architectures,
signed installation and older-to-newer updates remain release prerequisites.

All resource targets remain unmeasured for macOS. Record release-build CPU,
memory, activation latency, hardware, OS version and package size separately from
Windows; do not equate macOS memory counters with Windows private working set.

See [macOS distribution](../macos.md) for configuration and manual validation.

## Native menu and Dock activation follow-up

The status item's right-click menu uses native contextual-menu tracking, rather
than assigning a menu and recursively clicking the status button. Commands are
grouped with separators; weekday choices are a checked submenu, Show month is a
checked toggle, and Calendars opens the Accounts page rather than requesting
EventKit permission. Update checking is disabled in review builds without Sparkle.
Settings and Quit have the standard Command-comma and Command-Q equivalents.
The main Lightcal application menu also offers Settings (Command-comma) and
Calendars. Their labels follow the selected language; the toolkit's existing
About, Services, Hide and Quit commands remain in place.

Dock activation is handled by AppKit's optional application-delegate reopen
callback. The integration adds only that missing selector to the existing
Slint/winit delegate, preserving its identity and lifecycle implementations.
Dock clicks restore the current view, including Settings and minimized windows,
and never toggle a visible window off. Native regression tests exercise the menu
states and a fixture delegate without opening the real app or accessing user data.
An optional, isolated native smoke build additionally exercises actual reopen
AppleEvent dispatch through the Slint/AppKit event loop, the Settings menu key
equivalent, restoration and crossfade cancellation without screen capture.
