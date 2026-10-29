# Product plan

## Promise

Lightcal gives Windows users an immediately available, readable month calendar from the tray or a pinned taskbar application, with minimal background resource use.

## Approved choices

- Windows first; Rust implementation; no web container.
- Two entry points: compact tray popup and larger taskbar calendar.
- Taskbar means a normal user-pinned application icon. It does not mean replacing the system clock or embedding a custom toolbar in Explorer.
- GPL-3.0. This repository uses the precise identifier `GPL-3.0-only`.
- Future macOS support must remain architecturally possible.
- All documentation is in English. Changes are reviewed through PRs; protected branches require owner approval.
- Penpot through MCP is the design tool. Editable design sources and review evidence belong in `design/`.
- Slint is accepted. The additional owner-required features and acceptance criteria are tracked in [Required features](features.md), including Windows calendar selection, event dots, startup, shortcut, update checking, and About links.

## Proposed first release

Target Windows 11 x64 on a Microsoft-supported release at release time. ARM64 and older Windows compatibility require separate validation. macOS is an early feasibility target, not a promised first-release download.

The first release includes a Gregorian month grid, selected date, today highlight, weekday/date display, previous/next month, current-month emphasis, return to today, configurable first weekday and gray weekdays, light/dark/system theme, optional ISO week numbers, keyboard navigation, tray menu, and explicit Quit. It also includes opt-in startup, an optional global shortcut, manual update checking, selection of Windows-exposed calendars, event dots, and About links. The tray icon should display the day of month if legibility testing succeeds; its tooltip includes the full local date and weekday.

Ship English (`en`) as the default regardless of the OS language, with optional Russian (`ru`), Spanish (`es`), and Japanese (`ja`) selected in Settings. Persist the choice and apply it to application text, accessible labels, weekday/month names, and date formatting. Keep the system timezone and explicit first-weekday/gray-weekday preferences independent of the UI language. Store strings separately from logic; use English as the fallback for missing/invalid translations. Validate all four languages before release; right-to-left languages are outside this release scope.

## Two presentation modes

| Property | Tray popup | Taskbar calendar |
| --- | --- | --- |
| Current design size | 352 x 432 logical pixels | 520 x 600 logical pixels |
| Entry | Left-click tray icon | Launch shortcut or activate app button |
| Placement | Next to tray icon, clamped to visible monitor work area | Restore last valid position; otherwise place near taskbar on active monitor |
| Focus | Keyboard focus on open | Standard application focus |
| Dismissal | Escape or focus loss hides | Close hides to tray; focus loss leaves it open |
| Taskbar / Alt-Tab | No separate button or Alt-Tab entry | Normal taskbar button and Alt-Tab entry while visible |

Sizes are the current Penpot proposal, pending owner acceptance and DPI/localization checks. Settings use a separate 780 x 620 logical-pixel window. Both calendar views share date selection and navigation state. Only one calendar surface is visible at a time: opening one hides the other. Repeated tray clicks toggle the compact popup. Standard Windows taskbar activation/minimization behavior is preserved; custom taskbar click interception is outside scope.

A pinned shortcut launches or forwards activation to the running process and opens the larger view. Launching a second instance must never create a second tray icon. A normal launch opens the larger calendar; `--background` starts hidden for opt-in login startup. Quit exits the process. Autostart is off by default.

Windows controls pinning and tray overflow visibility; Lightcal must not promise that it can force either setting. Shortcut identity and reactivation behavior are prototype acceptance criteria.

## Outside the first release

Direct calendar-account sign-in, event editing, reminders, ICS import/subscriptions, holidays requiring downloads, Lightcal-operated cloud synchronization, telemetry, plugins, system clock replacement, shell injection, and automatic update installation. Read-only Windows calendar access and manual update checking are approved scope. Calendar availability must be established by the API probe before committing to release compatibility.

## Remaining product decisions

- Approve the Penpot designs and final sizes before polishing the application UI.
- Confirm proposed Windows support and first-release scope before implementation expands beyond the prototype.
- Decide release format, signing budget, and distribution identity before public binaries.
- Resolve Windows calendar API/package feasibility and record any release blocker; do not substitute an account provider without owner approval.
