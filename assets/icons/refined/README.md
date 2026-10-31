# Refined Lightcal icon sources

Created for the owner's October 31, 2026 Penpot refinement. These original vector
assets use the repository's GPL-3.0-only license. They preserve the calendar
silhouette of the owner's preferred small icon: paper body, coral cap, clear tiles.

- `lightcal.svg`: 1024 px vector master with a 128-unit viewBox and safe padding.
- `lightcal-16.svg`, `lightcal-24.svg`, `lightcal-32.svg`: pixel-specific drawings.
  Use these at their intended sizes rather than shrinking the large master.
- `macos-menu-template.svg`: single-color template for an optional calendar-symbol
  menu-bar item. Let macOS supply the foreground color and selected appearance.

The app icon stays static. The Windows tray badge uses the current day numeral
inside the same silhouette; Penpot contains separate specimens at 16/20/24/32 px.
The monochrome tray option changes the cap to graphite while retaining the paper
body. It is distinct from the macOS transparent template.

The default macOS status item is text, `Sat 31`, with a `31` alternative. There is
no colored miniature Dock icon in the menu bar and no replacement system clock.
The Dock uses the full application artwork with platform-appropriate safe space.

The final UI review centers the master grid at y51/86, and the 32 px grid at
y13/22. The crisp 16/24 px drawings are preserved. Rebuild the transparent PNGs
with [the renderer](../../../design/penpot/render-icons.mjs); dependencies stay
in ignored `target/design-tools`. See [the icon sheet](../../../design/review/icon-family.png).

These are design deliverables. Existing runtime, ICO, ICNS, signing and packaging
assets have not been replaced. Native shell QA remains required before release.
