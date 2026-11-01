# Product plan

## Promise

Lightcal gives Windows users an immediately available, readable month calendar from the tray or a pinned taskbar application, with minimal background resource use.

## Approved choices

- Windows first; Rust implementation; no web container.
- One compact calendar, opened from the tray or pinned taskbar app; a separate settings layout. The owner removed the expanded agenda view on October 30, 2026; see [ADR 0004](decisions/0004-compact-view-and-account-settings.md).
- Taskbar means a normal user-pinned application icon. It does not mean replacing the system clock or embedding a custom toolbar in Explorer.
- GPL-3.0. This repository uses the precise identifier `GPL-3.0-only`.
- Future macOS support must remain architecturally possible.
- All documentation is in English. Changes are reviewed through PRs; protected branches require owner approval.
- Penpot through MCP is the design tool. Editable design sources and review evidence belong in `design/`.
- Slint is accepted. The additional owner-required features and acceptance criteria are tracked in [Required features](features.md), including Windows calendar selection, event dots, startup, shortcut, update checking, and About links.

## Proposed first release

Target Windows 11 x64 on a Microsoft-supported release at release time. ARM64 and older Windows compatibility require separate validation. macOS is an early feasibility target, not a promised first-release download. Pull requests may attach an unsigned review disk image; that image is not signed, notarized, or evidence that the application launches.

The first release includes a Gregorian month grid, selected date, today highlight, weekday/date display, previous/next month, current-month emphasis, return to today, configurable first weekday and gray weekdays, light/dark/system theme, optional ISO week numbers, keyboard navigation, tray menu, and explicit Quit. It also includes opt-in startup, an optional global shortcut, manual update checking, selection of Windows-exposed calendars, event dots, and About links. The tray icon should display the day of month if legibility testing succeeds; its tooltip includes the full local date and weekday.

Ship English (`en`) as the default regardless of the OS language, with optional Russian (`ru`), Spanish (`es`), and Japanese (`ja`) selected in Settings. Persist the choice and apply it to application text, accessible labels, weekday/month names, and date formatting. Keep the system timezone and explicit first-weekday/gray-weekday preferences independent of the UI language. Store strings separately from logic; use English as the fallback for missing/invalid translations. Validate all four languages before release; right-to-left languages are outside this release scope.

## Tray and taskbar entry points

Both entry points open the same 352 by 432 logical-pixel compact calendar. Settings use a 448 by 360 layout. Removing the expanded agenda does not remove the normal taskbar button.

- Tray clicks and the global shortcut restore the calendar, or minimize it when the calendar is already open. Taskbar activation restores the existing native window.
- Escape and Close minimize the window while preserving its taskbar button. Focus loss does not hide or destroy the window, including when Explorer opens the taskbar context menu.
- A normal launch opens the calendar. `--background` starts quietly in the tray until the first activation. A second launch forwards activation to the running instance.
- Windows DWM receives a rendered calendar bitmap for thumbnail and Peek requests, including while minimized. Settings retain the last available calendar preview. Images preserve aspect ratio and fit DWM's requested bounds.
- Portable builds use a stable application ID. Installed MSIX builds retain the identity assigned by Windows. Only explicit Quit terminates the process.

See [ADR 0005](decisions/0005-taskbar-calendar-presence.md) for the regression and verification limits.

Windows controls pinning and tray overflow visibility; Lightcal must not promise that it can force either setting. Shortcut identity and reactivation behavior are prototype acceptance criteria.

## Outside the first release

Direct calendar-account sign-in, event editing, reminders, ICS import/subscriptions, holidays requiring downloads, Lightcal-operated cloud synchronization, telemetry, plugins, system clock replacement, shell injection, and automatic update installation. Read-only Windows calendar access and manual update checking are approved scope. Calendar availability must be established by the API probe before committing to release compatibility.

## Remaining product decisions

- Approve the Penpot designs and final sizes before polishing the application UI.
- Confirm proposed Windows support and first-release scope before implementation expands beyond the prototype.
- Decide release format, signing budget, and distribution identity before public binaries.
- Resolve Windows calendar API/package feasibility and record any release blocker; do not substitute an account provider without owner approval.
