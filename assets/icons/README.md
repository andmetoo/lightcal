# Lightcal icon artwork

Original vector artwork, created for Lightcal on 2026-10-29 under GPL-3.0-only. No fonts, stock icons, external images, or raster effects are embedded.

| Source | Purpose |
| --- | --- |
| [lightcal.svg](lightcal.svg) | Color app icon: blue tile, white calendar, two date strokes and one event dot |
| [lightcal-mark.svg](lightcal-mark.svg) | White foreground on a genuinely transparent canvas; cutouts remain transparent |
| [lightcal-background.svg](lightcal-background.svg) | Full-bleed square blue background without rounded masking, for future platform composition |
| [lightcal-tray.svg](lightcal-tray.svg) | Simplified monochrome tray/menu-bar mark using `currentColor` |

The color artwork and separated layers use a 1024 x 1024 viewBox. They scale without raster upsampling. The tray source uses a 24 x 24 grid with heavier, simplified strokes. Choose explicit foreground colors at export time. The static tray mark is a branding/fallback asset; dynamic day-of-month rendering remains a separate app feature.

## Visual validation

Imported as editable vectors into Lunacy through MCP. The icon board (`EiyyynnQSFKE-EFUIoP8JQ`) shows a large app preview, color sizes 16/24/32/48/64, and monochrome sizes 16/24/32 on light/dark surfaces. Its exported PNG preview was visually inspected. This checks artwork, not actual Windows shell rendering or macOS packaging.

Reproduce using [design/icon_design.py](../../design/icon_design.py). It imports these SVG files through Lunacy MCP; use an empty area at y=2100. SVG import coordinates are absolute even with a parent frame. A new run creates new layers. `--board-id` resumes imports into the specified board; do not rerun it blindly after an uncertain timeout. Inspect existing layers first.

## Platform packaging

- Windows: generate a multi-image `.ico` from the vectors when integrating app resources. Check the actual shell result at small sizes and high DPI; additional optical adjustments may be appropriate.
- macOS: reuse the unmasked background and transparent foreground as separate layers. Adapt them in the supported Apple toolchain; do not reuse the pre-rounded Windows tile as an Apple background layer. The SVGs are preparation for a port, not a finished `.icon`/`.icns` package.
- Keep the original vectors authoritative. Generate each raster size from source, rather than enlarging a small PNG. Preserve simple geometry and clear contrast when creating platform appearances.

Primary platform guidance reviewed on 2026-10-29: [Microsoft icon resources](https://learn.microsoft.com/en-us/windows/win32/menurc/about-icons) and [Apple app icon design and vector layers](https://developer.apple.com/design/human-interface-guidelines/app-icons/).
