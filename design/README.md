# Lightcal desktop design

The current design is the October 31, 2026 refinement in the owner's connected
`penpot_local` file **lightcalendar** (`c2d27051-3cd1-813e-8008-dd9b45707753`).
Open **Overview** for the review entry point.

## Current concept

The current palette is [Gray + Teal](palettes/README.md): neutral surfaces and
selection fills, Teal actions/focus markers, and Coral identity/Today. The
owner rejected violet throughout both themes. Native UI and all active Penpot
specimens use the updated palette.

- One 352 × 432 calendar, with larger dates, distinct today/selection states,
  event dots and a compact agenda. Today is the only return-to-today action.
- A fixed **448 × 360** settings panel with **Calendar / Accounts / App** tabs.
  Theme is a segmented control. Rows are 40 px; repeated descriptions and large
  theme previews are removed. About is a separate panel.
- Accounts shows connected accounts and selected calendars. Google, Microsoft
  and private ICS are choices in Add account. Consent, cancellation, revoked
  access, offline data and empty selection have separate states.
- Windows uses a static application icon and a live date badge in the tray.
  Following the owner's screenshot review, the live date uses only numerals
  on transparency: no paper, cap or outline. Color/monochrome controls select
  the ink. The native renderer centers visible glyph bounds and uses one font
  size for all dates at each shell size; shell contrast follows Windows rather
  than the application's selected theme.
  macOS uses platform window controls, login/shortcut terminology, a monochrome
  menu-bar label and a separate Dock icon. Neither replaces the system clock.
- English, Russian, Spanish and Japanese examples retain user event titles.

Typography: Windows uses the owner's selected Onest (Fontsource 5.3.1).
The owner subsequently selected the system font/SF Pro for macOS. That
[migration is prepared](fonts/macos-system.md) and requires the SF Pro faces
to be added to the Penpot team; current macOS exports still show Onest.

The owner rejected both the old sidebar layout and an initial 620 × 520 tabbed
layout as too spacious. An independent UX review led to the three-tab concept;
the two primary screens were reviewed again before migration.

## Working file

| Page | Boards | Purpose |
| --- | ---: | --- |
| Windows settings | 8 | Three tabs and About, light/dark |
| Windows calendar | 12 | Compact dates, empty days, taskbar/tray and menus |
| Connections & states | 52 | Windows/macOS connection, shortcut and update states |
| macOS | 18 | Compact calendar, settings, menu bar and Dock |
| Icons & components | 11 | Icon sizes, day variants, shared controls and export boards |
| Languages | 12 | Calendar, App settings and shortcut conflicts in four languages |
| Overview | 1 | Presentation and prototype entry point |

Original pages and intermediate layouts are preserved in Penpot's permanent
version **Before refinement cleanup 2026-10-31**. They were removed from the
working page list after the version was verified. Restoring that version restores
the whole earlier file, including the original pages.

## Sources and evidence

- [Automation and verification](penpot/README.md)
- [Machine-readable inventory and geometry report](penpot/refinement-manifest.json)
- [Refined vector and PNG icon sources](../assets/icons/refined/README.md)
- [Owner scope and implementation boundary](../docs/decisions/0008-google-calendar-and-platform-design.md)
- [Windows Calendar settings](review/windows-calendar-settings.png)
- [Windows Accounts](review/windows-accounts.png)
- [Dark macOS App settings](review/macos-app-dark.png)
- [Japanese App settings](review/japanese-app.png)
- [Final compact calendar](review/windows-calendar.png)
- [Review overview](review/overview.png)
- [Final icon family](review/icon-family.png)
- [Japanese shortcut error](review/japanese-shortcut-error.png)
- [Russian shortcut error](review/russian-shortcut-error.png)
- [macOS shortcut error with vector key symbols](review/macos-shortcut-error.png)
- [Native Windows date: calendar cap versus numeral-only](review/windows-date-numerals.png)
- [Native Gray + Teal App, dark](review/native-gray-teal-app-dark.png)
- [Native Gray + Teal App, light](review/native-gray-teal-app-light.png)
- [Native Gray + Teal Calendar settings, dark](review/native-gray-teal-calendar-dark.png)

The final geometry audit covers 7 pages and 114 top-level boards, with no duplicate
board names or out-of-bounds descendants. The navigation prototype has 553 links
and no unresolved destinations. Pixel review remains separate from geometry.
Today text uses white on `#D73840` (calculated contrast 4.64:1); white primary
button text uses `#00796B` (5.32:1). This is not a complete accessibility audit.

## Final UI review

An independent UI reviewer inspected the final 1× Windows, dark macOS, calendar
and Russian/Japanese shortcut-error exports after the UX structure was settled.
The final review passed without clipping or collisions in those specimens.

- Use the owner's selected **Onest** family for Latin, Cyrillic and date
  numerals, with explicit 400/500/600/700 weights. CJK strings use exact
  Noto Sans JP. See [pinned fonts and attribution](fonts/onest/README.md) for
  Fontsource 5.3.1, native version 2.001 and the Penpot revision limitation.
  This review covers the Onest stage. The owner's later macOS SF Pro selection
  requires a separate font import and visual review as recorded above.
- Form fields and ordinary buttons are 28 px high, centered in 40 px rows.
  Theme, tray palette and menu-bar labels share the same segmented control.
  Weekdays remain independent toggles.
- Today's selected tile keeps the coral fill. A keyboard-focus ring is shown
  only in a focus state. Shared controls include hover, pressed, focus and
  disabled specimens.
- The master icon grid is centered below the cap. The 32 px drawing receives
  a 1 px optical correction; the 16/24 px geometry is preserved. The subsequent
  Gray + Teal pass updates neutral ink colors in those PNGs.

Permanent versions **Before final UI polish 2026-10-31** and
**Lightcal final UI 2026-10-31** preserve the reviewed stages.

The subsequent owner-selected Onest pass preserves the compact geometry and
prototype destinations. **Before Onest typography 2026-10-31** preserves the
previous Inter version.

These are design deliverables. Google OAuth, runtime UI alignment, real Windows
DPI/shell checks and real macOS integration remain implementation/release work.
No application performance measurement is claimed. The subsequent native
[Windows date-icon followup](../docs/decisions/0009-windows-date-numerals.md)
implements the numeral-only renderer and reads the shell theme. It does not
change credentials, autostart, registry values or release packaging.
