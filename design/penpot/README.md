# Penpot redesign

Design date: 2026-10-29. Target: native Rust/Slint Windows application. All design changes use Penpot MCP.

Connected local file: `app`, file ID `c2d27051-3cd1-813e-8008-dd81ca9d88c1`. Page IDs and counts are recorded in [manifest.json](manifest.json).

## Organization

The design is organized on six separate pages, in this order:

| Page | Contents |
| --- | --- |
| 00 · Overview | One presentation board showing both modes/themes |
| 01 · Calendar | 10 boards: canonical windows, current/other month, and tray menus |
| 02 · Settings | 8 boards: General, Appearance, Calendars, About; light/dark paired by row |
| 03 · States | 12 boards: language menu, shortcut conflict, provider unavailable/empty, update checking/available |
| 04 · Languages | 4 compact examples: English, Russian, Spanish, Japanese |
| 05 · Design system | One board: palettes, controls, toggle states, keyboard focus, error treatment |

Status: completed in the local Penpot file on 2026-10-29. The former 35-board mixed canvas was replaced with 36 organized boards, including an added English localization reference. The old canvas and temporary page were removed after verification. A saved version preserves the pre-cleanup file; a final saved version is named `Lightcal redesign — organized into six pages`.

## Layout and behavior

- Tray calendar: 352 x 432 logical pixels. Taskbar calendar: 520 x 600. Settings: 780 x 620.
- The current month uses a small accent underline and a labeled badge. Both disappear in the November example. Today is outlined; selection is filled. Event dots are separate from both states.
- Gray weekday backgrounds remain readable and enabled. Examples shade Saturday/Sunday; Appearance permits choosing any days.
- General includes English as default, the other three language choices, startup off, and a global shortcut initially unassigned.
- Calendar selection is read-only and lists synthetic Windows provider data. Empty/access-failure screens preserve use of the date grid.
- About includes GitHub/koona.app links and manual update-check states. Update availability is illustrative, not a real release check.
- Inter is used in Penpot, with Noto Sans JP for Japanese. The native implementation should use appropriate installed UI fonts and must recheck metrics; the prototype is not evidence of Windows system font fallback.
- Examples show event counts, not an agenda/event editor. No direct account sign-in, telemetry, or automatic installation is introduced.

## Source and reproduction

- [design.js](design.js) defines editable native Penpot boards, text, paths, controls, palettes, and helpers in `storage.lightcal`. It does not build all screens on load.
- [pages.js](pages.js) defines `storage.lightcal.buildNext()`. Run one phase per MCP call to keep the plugin responsive. Never use one long loop to rebuild and wire the entire design.
- Upload the owner's [PNG reference](../../assets/icons/better/png/icon-128.png) via `penpot.uploadMediaData` and assign the returned image to `storage.lightcal.image` before running the page builder.
- Use a fresh connected file for reproduction, or inspect an existing file before rebuilding. Page creation is not an export of a `.penpot` archive.
- `wireCurrentPage()` binds representative navigation only within the current page, skips self-navigation, and guards root ancestry against cycles. The Calendar page has 14 transitions and Settings has 26. Cross-page jumps and arbitrary date selection are not simulated. Use the editor page list to inspect the separate scenarios.
- Prototype flows named `Taskbar calendar` and `Settings` start on their respective pages. About links open the requested public URLs on click. No prototype performs live OS actions.
- [verify.js](verify.js) checks current-page text overflow and containment without altering the design. Keep calls short; do not loop through an unbounded parent chain or perform one long global wiring operation.

## Verification limits

The rebuilt overview, dark Appearance, and Japanese calendar were exported from local Penpot and visually inspected; General/Appearance were also reviewed in the initial draft. All six pages passed text-overflow and board-containment checks with zero findings. Penpot's file validation reported zero errors. The final page count/order and board counts were verified after deleting the old canvas.

Core text/background contrast pairs range from 5.18:1 to 16.27:1 in light mode and 5.40:1 to 14.79:1 in dark mode. These checks cover the selected palette pairs, not all interactive/system contrast cases.

Real Windows DPI, accessibility, event APIs, fonts, startup, shortcut registration, and update networking are not implemented or verified by mockups. Translated examples are draft design copy, not complete reviewed translation catalogs. Prototype links were structurally checked; a complete manual viewer walkthrough is not claimed.
