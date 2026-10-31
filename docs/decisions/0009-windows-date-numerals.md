# ADR 0009: Numeral-only Windows date icon

Date: 2026-10-31. Status: accepted from the owner's screenshot review in chat.

The calendar cap used vertical space without adding information to a live day
number. The owner requested just the numerals, with more even alignment.

## Decision and implementation

- Render the live date on a transparent canvas, without a page, cap or outline.
- Use bundled Onest Bold and center the visible glyph bounds instead of the
  text advance width. Fit all 31 dates at one font size per native icon size;
  verify the rounded raster metrics so wide dates such as 30 keep their padding.
- Preserve coral and monochrome choices as numeral ink colors. Monochrome uses
  dark ink on a light shell and light ink on a dark shell. Read
  `SystemUsesLightTheme` separately from the application's chosen theme.
  Light-shell colored text uses `#CF2835` (calculated contrast 4.69:1 on
  the `#F2F2F5` specimen); dark-shell colored text uses `#FF8A90`.
- Apply this to the native tray and live window/taskbar date icon. The static
  application identity remains an undated calendar mark.
- Update existing Penpot date specimens without rebuilding settings or links.
  Legacy weekday/time render paths retain their existing structure.

The renderer is in `crates/lightcal/src/platform/windows/icon.rs`; the live
window icon uses the shell theme in `main.rs`. Registry access is read-only.
The standalone native raster comparison is
[included in the review evidence](../../design/review/windows-date-numerals.png).

## Validation

Pure renderer tests cover transparent backgrounds and every date from 1 through
31 at 16, 20, 24, 32, 48 and 64 px. Visible bounds retain a transparent edge and
their opposite margins differ by at most one pixel. These tests do not change
the real date, registry, credentials, startup or preference profile.

Native raster exports and Penpot samples are inspected on light and dark
surfaces. Explorer's actual DPI/theme changes, taskbar resampling and live shell
presentation remain manual checks; a raster proof is not evidence of those
platform behaviors. No release performance measurement is claimed.
