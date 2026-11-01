# Lightcal design

The current design is the November 1, 2026 refresh, recorded in
[ADR 0011](../docs/decisions/0011-windows-only-design-refresh.md). It targets
Windows only.

## Where it lives

The owner edits the design on the design canvas **Lightcal Refresh**. The
board sources are mirrored in [canvas/](canvas/) so the repository keeps a
reviewable history. They are `.dc.html` design components that need the
canvas runtime to render; they are not application code.

| Canvas page | Boards |
| --- | --- |
| Windows | Calendar (today, event details, dark empty day), settings Calendar / Accounts / App, About, taskbar and tray at 150 %, tray menu |
| States | First run, signing in, private ICS link, needs attention, shortcut conflict, update available |
| Icons | App icon, live tray date in Color and Mono, real-size specimens and two-digit dates |
| Languages | Russian App, Spanish Calendar and Japanese App settings |

Every board has a light/dark switch. Calendar boards are interactive: days,
month arrows, Today and agenda rows work. Settings tabs link to each other.

The Slint settings view (`crates/lightcal/ui/settings.slint`) follows the
Settings, States, About and Languages boards. Two additions are not on the
boards: a fourth provider row, **Windows calendars**, for the optional legacy
Windows provider, and on About a **Release notes** link plus a card with the
two automatic-update switches below the links (reached by scrolling).
Account notices map every controller state onto the board patterns: expired
Outlook or Google access and Windows calendar access denied use the amber strip
with one action; offline Outlook, Google query errors, ICS refresh errors and
provider-wide failures use the quiet line; connection errors use the red
message box; a cancelled sign-in is a quiet line with **Try again**.

## Principles

- One compact 352 × 432 calendar and one 448 × 360 settings window.
- [Gray + Teal](palettes/README.md) surfaces and actions; coral only for
  identity and Today.
- Segoe UI Variable from Windows (Text for UI, Display for large headings),
  Segoe UI on Windows 10, and Yu Gothic UI for Japanese. Nothing is bundled.
- Only solid fills, rounded rectangles, 1–2 px borders and one clip, so every
  board maps to Slint. Window corners and shadow come from DWM; Mica is
  optional.
- New icon masters are in [assets/icons/canvas](../assets/icons/canvas/).

## History

The October 31 Penpot design is archived as
[lightcalendar-2026-11-01.penpot](archive/lightcalendar-2026-11-01.penpot).
Import it into Penpot to view it. Its automation scripts, manifest and board
exports were removed with ADR 0011. Native application screenshots remain in
[review/](review/):

- [Native Gray + Teal App, dark](review/native-gray-teal-app-dark.png)
- [Native Gray + Teal App, light](review/native-gray-teal-app-light.png)
- [Native Gray + Teal Calendar settings, dark](review/native-gray-teal-calendar-dark.png)
- [Native Windows calendar tray icon, both styles and themes](review/windows-tray-calendar.png)
- [Historical numeral-only experiment](review/windows-date-numerals.png)

These show the app before the refresh is implemented. No performance or
accessibility audit is claimed for the new design.
