# Lightcal icon artwork

The current icon design is in [canvas/](canvas/): the coral app tile and the
live tray date in Color and Mono, from
[ADR 0011](../../docs/decisions/0011-windows-only-design-refresh.md). They are
original vector masters; runtime and installer assets have not been replaced
yet. The earlier Penpot `refined/` set was removed. The supplied artwork below
remains preserved for provenance.

The owner-selected artwork is in `better/`. Earlier icon candidates and their generator have been removed at the owner's request. The owner supplied these assets; retain their provenance and confirm redistribution rights before a public binary release.

| Source | Purpose |
| --- | --- |
| [better/lightcal-icon.svg](better/lightcal-icon.svg) | Owner-supplied 1024-viewBox vector master |
| [better/png/icon-512.png](better/png/icon-512.png) | Clean raster rendition used in earlier designs |
| [better/png/icon-1024.png](better/png/icon-1024.png) | Largest supplied raster rendition |
| [better/lightcal.ico](better/lightcal.ico) | Windows ICO containing 16/32/48/256 px entries |
| [better/lightcal-tray-monochrome.svg](better/lightcal-tray-monochrome.svg) | Supplied tray candidate; despite its name, it currently uses multiple colors |

PNG files from 16 through 1024 px include alpha transparency. The SVG and PNG artwork differ: the SVG includes glow, rims, a header indicator, and extra event dots. They have not been silently normalized. The supplied PNG rendition is the visual reference for the new UI; preserve the SVG for future vector work.

## Visual validation

The supplied PNG dimensions/alpha and ICO structure were checked. Small raster variants were inspected on light/dark surfaces. This does not verify actual Windows shell rendering.

The application design is maintained on the design canvas. See [design notes](../../design/README.md).

## Platform packaging

- Windows: validate the supplied multi-image ICO in the actual shell. Fine details need optical simplification at small sizes.
- Tray: prepare a genuinely monochrome version with theme-appropriate foreground color before integration. Dynamic day-of-month rendering remains a separate app feature.
- Never upscale a small PNG to make a larger icon; render from the vector masters.

Primary platform guidance reviewed on 2026-10-29: [Microsoft icon resources](https://learn.microsoft.com/en-us/windows/win32/menurc/about-icons).
