# ADR 0010: Restore the calendar identity in the Windows tray

Date: 2026-10-31. Status: accepted from the owner's screenshot review in chat.
Supersedes [ADR 0009](0009-windows-date-numerals.md).

The owner found the numeral-only Windows icon visually anonymous beside native
tray icons. The previous calendar badge was easier to recognize despite its
smaller date. The rest of the owner's completed design must be preserved.

## Decision and implementation

- Restore a compact calendar page with rounded corners and a slim header.
  The colored version uses a coral header, white page and dark date in both
  shell themes. Monochrome uses graphite on a light shell and silver on a dark
  shell. No binder marks, inner date tile or shadow are needed at tray sizes.
- Retain bundled Onest Bold, one font size for all dates at each native size,
  raster-bound centering, and the independent Windows shell-theme lookup.
  Fit the date inside the page beneath the header. Use the full native canvas,
  with transparent rounded corners and no additional empty outer rows/columns.
  Render glyphs at native resolution instead of resizing a large icon.
- Apply this artwork to the live tray and window/taskbar date icon. Preserve
  the static undated application icon and legacy multiline render modes.
  The owner's subsequent sizing review also approves the same calendar badge
  in the macOS menu bar, replacing the earlier text-only label.
- Patch only the existing date specimens in Penpot, preserving their numeral
  layer IDs, board geometry, links and all unrelated owner edits. The isolated
  migration verifies unchanged names, geometry, paints and text properties on
  unrelated shapes. It does not rebuild boards or run palette migrations.

The native renderer is `crates/lightcal/src/platform/windows/icon.rs`.
For Penpot, load `refinement-tray-calendar-helper.js`, then run
`refinement-tray-calendar.js`. The helper only defines the artwork function in
memory. Apply the sizing/macOS followup with `refinement-status-size.js`.
For the existing stroke-based v3 specimens, `refinement-status-outline.js`
replaces only the owned date bodies with nested fills. Fresh specimens already
use this geometry. It avoids Penpot's centered-stroke PNG export overflowing
the nominal icon canvas, even when the API reports an inner stroke.
Fresh full design builds must load the helper after `refinement.js` and
before any builders that call `brand` with a date. The earlier numeral-only
migration is historical and must not be reapplied to the current design.

## Validation

Pure renderer tests cover all 31 dates at 16, 18, 20, 24, 32, 36, 40, 48 and 64 px,
page/header presence in both styles and shell themes, transparent rounded corners,
and date placement within the page. The high-coverage date's center stays within
one pixel horizontally and vertically of the page center; fractional coverage
can make its visible opposite margins differ by two pixels.

[Native raster evidence](../../design/review/windows-tray-calendar.png) shows
actual sizes and enlarged detail. An independent UI review accepted the
16/20/24 px specimens and both styles. PNG exports from the current Penpot file
are inspected separately. These are raster/design checks; live Explorer DPI,
theme switching and taskbar presentation still require a Windows shell check.
No release resource measurement is claimed. Tests do not alter real preferences,
registry values, startup state, credentials or system dates.

## Native size and macOS followup

The owner requested efficient use of both dimensions and a shared status icon
on macOS. Primary platform documentation was reviewed on 2026-10-31:

- [Microsoft icon sizes](https://learn.microsoft.com/en-us/windows/apps/design/iconography/app-icon-construction):
  system tray rasters are 16/20/24/32/40/48/64 px for 100/125/150/200/250/300/400%.
  This is the icon canvas, not the height of the taskbar or its clickable slot.
- [Notification area](https://learn.microsoft.com/en-us/windows/win32/shell/notification-area):
  provide high-DPI-aware small icons. The dynamic renderer generates the exact
  requested raster, rather than loading and resizing a single bitmap.
- [GetDpiForWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getdpiforwindow):
  size the raster using the destination Explorer taskbar window's DPI, falling
  back to system DPI when that window or its DPI cannot be obtained. Do not
  use the application's current monitor to infer the tray's scale.
- [Apple menu bar guidance](https://developer.apple.com/design/human-interface-guidelines/the-menu-bar):
  the documented menu bar height is 24 pt. Penpot now uses 24 pt specimens;
  the former 36 pt illustrations were oversized.
- [NSStatusItem.squareLength](https://developer.apple.com/documentation/appkit/nsstatusitem/squarelength):
  AppKit sizes the square button to the actual status bar thickness. The chosen
  artwork is 18 pt square, with 18/36 px representations at 1x/2x. Eighteen
  points is a design choice, not a hard size requirement stated by Apple.

At 100% the previous drawing occupied 14x14 inside a 16x16 raster. It now spans
16x16, with antialiased round corners. This increases both bounding dimensions
without expanding the system-owned slot. Font size remains stable through the
month and all 31 dates are checked inside the page. Header and outline widths
scale from the same 16 px geometry, including fractional-DPI sizes.

macOS uses the same calendar proportions and Color/Mono choices as Windows.
The default colored calendar is an ordinary non-template image. The illustrated
gray paper variants also contain multiple opaque tones and must be refreshed
for the current appearance. They are not a black/clear `NSImage.isTemplate`
mask; [Apple's template contract](https://developer.apple.com/documentation/appkit/nsimage/istemplate)
requires a separate alpha-mask rendering if that approach is implemented.
The macOS adapter remains a stub: the Penpot port and asset size specification
are complete, while actual AppKit appearance/selection, Retina and shell
behavior still require implementation and a real Mac check.
