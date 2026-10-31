# Lightcal icon artwork

The latest design pass is in [refined/](refined/README.md): original vector
masters, optical 16/24/32 px drawings and transparent PNG exports. The existing
runtime and packaging assets have not been replaced by this design-only change.
The supplied artwork below remains preserved for provenance.

The owner-selected artwork is in `better/`. Earlier icon candidates and their generator have been removed at the owner's request. The owner supplied these assets; retain their provenance and confirm redistribution rights before a public binary release.

| Source | Purpose |
| --- | --- |
| [better/lightcal-icon.svg](better/lightcal-icon.svg) | Owner-supplied 1024-viewBox vector master |
| [better/png/icon-512.png](better/png/icon-512.png) | Clean raster rendition used in the Penpot UI |
| [better/png/icon-1024.png](better/png/icon-1024.png) | Largest supplied raster rendition |
| [better/lightcal.ico](better/lightcal.ico) | Windows ICO containing 16/32/48/256 px entries |
| [better/lightcal-tray-monochrome.svg](better/lightcal-tray-monochrome.svg) | Supplied tray candidate; despite its name, it currently uses multiple colors |

PNG files from 16 through 1024 px include alpha transparency. The SVG and PNG artwork differ: the SVG includes glow, rims, a header indicator, and extra event dots. They have not been silently normalized. The supplied PNG rendition is the visual reference for the new UI; preserve the SVG for future vector work.

## Visual validation

The supplied PNG dimensions/alpha and ICO structure were checked. Small raster variants were inspected on light/dark surfaces. This does not verify actual Windows shell rendering or macOS packaging.

The new application design is maintained in Penpot. See [design notes](../../design/README.md).

## Platform packaging

- Windows: validate the supplied multi-image ICO in the actual shell. Fine details need optical simplification at small sizes.
- Tray: prepare a genuinely monochrome version with theme-appropriate foreground color before integration. Dynamic day-of-month rendering remains a separate app feature.
- macOS: separate the master into unmasked background/foreground layers and adapt in the supported Apple toolchain. The current files are not a finished `.icon`/`.icns` package.
- Reconcile the vector/raster differences before creating release assets; never upscale a small PNG to make the macOS icon.

Primary platform guidance reviewed on 2026-10-29: [Microsoft icon resources](https://learn.microsoft.com/en-us/windows/win32/menurc/about-icons) and [Apple app icon design and vector layers](https://developer.apple.com/design/human-interface-guidelines/app-icons/).
