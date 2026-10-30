# Lightcal Penpot Automation Scripts

This directory contains the reproducible Penpot Plugin API scripts that generate and verify the 7-page, 39-board **Lightcal** desktop UI design in Penpot (`Новый файл 1`, file ID `c2d27051-3cd1-813e-8008-dd9b45707753`).

## Files

- [`design.js`](./design.js) — Installs `storage.lc` with the **Alabaster & Ink (Light)** and **Obsidian Graphite (Dark)** token palettes, calendar month fixtures (`OCTOBER_2026`, `NOVEMBER_2026`), and reusable drawing primitives (`drawCalendar`, `drawTrayMenu`, `drawSettingsBoard`, `drawAppIcon`, `drawDropdown`, `drawToggle`, `drawWeekdayPills`, `drawFlatRow`).
- [`pages.js`](./pages.js) — Creates the 7 pages (`00 · Overview`, `01 · Calendar`, `02 · Settings`, `03 · States`, `04 · Languages`, `05 · Design system`, `06 · Shell, Dock & Icons`) and populates all 39 artboards.
- [`verify.js`](./verify.js) — Audits all 7 pages, verifying board counts, dimensions, and shape containment.
- [`manifest.json`](./manifest.json) — Machine-readable inventory of the generated pages, board IDs, and dimensions.
