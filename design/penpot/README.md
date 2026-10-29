# Penpot redesign

Design date: 2026-10-29. Target: native Rust/Slint Windows application. All design changes use Penpot MCP.

Connected file: `app`, file ID `2bd3d7f1-914f-8188-8008-dd6ebe65c25b`.

## Organization

The owner requested separate pages rather than one large canvas. The target structure is:

| Page | Contents |
| --- | --- |
| 00 · Overview | Presentation copies of the two calendar modes in both themes |
| 01 · Calendar | Canonical calendar windows, current/other month, and tray menus |
| 02 · Settings | General, Appearance, Calendars, and About; light/dark paired by row |
| 03 · States | Language menu, shortcut conflict, provider unavailable/empty, update checking/available |
| 04 · Languages | English, Russian, Spanish, Japanese compact calendar examples |
| 05 · Design system | Paired palettes, controls, toggle states, keyboard focus, error treatment |

Status: the initial 35-screen redesign was created and several boards were visually inspected. Reorganization into these pages is in progress following owner feedback. The plugin must be responsive before page organization and prototype checks can be completed; do not infer completion from the source files alone.

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
- Wire prototypes in short batches, skip self-navigation, and guard root ancestry against cycles. Check page compatibility before adding cross-page navigation; a design mockup need not pretend to execute live OS actions.

## Verification limits

The initial calendar overview, General, and Appearance were exported from Penpot and visually inspected. Core text/background contrast pairs range from 5.18:1 to 16.27:1 in light mode and 5.40:1 to 14.79:1 in dark mode. These checks cover the selected palette pairs, not all interactive/system contrast cases.

Final text-bound, containment, prototype, and page-organization checks remain pending until reorganization is complete. Real Windows DPI, accessibility, event APIs, fonts, startup, shortcut registration, and update networking are not implemented or verified by mockups. Translated examples are draft design copy, not complete reviewed translation catalogs.
