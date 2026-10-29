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
- Lunacy is available for UI design. Editable design files and preview exports belong in `design/` when created.

## Proposed first release

Target Windows 11 x64 on a Microsoft-supported release at release time. ARM64 and older Windows compatibility require separate validation. macOS is an early feasibility target, not a promised first-release download.

The first release includes a Gregorian month grid, selected date, today highlight, previous/next month, return to today, configurable first weekday, light/dark/system theme, optional ISO week numbers, keyboard navigation, tray menu, and explicit Quit. The tray icon should display the day of month if legibility testing succeeds; its tooltip includes the full local date.

Start with English application text; use system locale for month/day names where supported. Keep strings separate from logic so additional translations are possible. Localization and right-to-left coverage must not be claimed before testing.

## Two presentation modes

| Property | Tray popup | Taskbar calendar |
| --- | --- | --- |
| Initial design size | About 320 x 360 logical pixels | About 440 x 480 logical pixels |
| Entry | Left-click tray icon | Launch shortcut or activate app button |
| Placement | Next to tray icon, clamped to visible monitor work area | Restore last valid position; otherwise place near taskbar on active monitor |
| Focus | Keyboard focus on open | Standard application focus |
| Dismissal | Escape or focus loss hides | Close hides to tray; focus loss leaves it open |
| Taskbar / Alt-Tab | No separate button or Alt-Tab entry | Normal taskbar button and Alt-Tab entry while visible |

Sizes are starting points for Lunacy, not fixed pixel requirements. Both views share date selection and navigation state. Only one calendar surface is visible at a time: opening one hides the other. Repeated tray clicks toggle the compact popup. Standard Windows taskbar activation/minimization behavior is preserved; custom taskbar click interception is outside scope.

A pinned shortcut launches or forwards activation to the running process and opens the larger view. Launching a second instance must never create a second tray icon. A normal launch opens the larger calendar; `--background` starts hidden for opt-in login startup. Quit exits the process. Autostart is off by default.

Windows controls pinning and tray overflow visibility; Lightcal must not promise that it can force either setting. Shortcut identity and reactivation behavior are prototype acceptance criteria.

## Outside the first release

Calendar accounts, event editing, reminders, ICS import, holidays requiring downloads, cloud synchronization, telemetry, accounts, plugins, system clock replacement, and shell injection. Agenda/event providers can be proposed after the lightweight calendar is useful on its own.

## Remaining product decisions

- Approve the Lunacy designs and final sizes before polishing the application UI.
- Confirm proposed Windows support and first-release scope before implementation expands beyond the prototype.
- Decide release format, signing budget, and distribution identity before public binaries.
- Decide whether to add event providers after the initial calendar release.
