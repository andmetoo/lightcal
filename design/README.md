# Lightcal calendar concepts

Created in Lunacy 14.1 on 2026-10-29 using its MCP integration only.

## Deliverables

The active Lunacy document contains four editable boards: two calendar boards and two settings boards.

| Board | Frame sizes | Lunacy layer ID |
| --- | --- | --- |
| Light | Compact 320 x 360; expanded 440 x 480 | `dY96Ijk-QwmotazSGNUrSg` |
| Dark | Compact 320 x 360; expanded 440 x 480 | `mGamTq11SoOtGV0zvOshTg` |
| Preferences / Light | Appearance, General, Calendars, About: each 440 x 480 | `cwFodYGARcW_l2o52NPv6A` |
| Preferences / Dark | Appearance, General, Calendars, About: each 440 x 480 | `CUNrZu_hRsmc5sUQCGf4-A` |

All elements are editable text and vector layers, not flattened screenshots. Native document saving is not exposed by the available MCP tools, so no saved `.free` project file is claimed. The reproducible source is [lunacy_design.py](lunacy_design.py). Rendered board previews were inspected through MCP and displayed in the design review chat; no preview image files are stored here yet.

## Design choices

- Segoe UI system typography, quiet surfaces, blue accent, and restrained rounded corners.
- Six calendar rows keep the popup height stable across months. Monday-first examples show October 2026.
- Today (October 29) has an outlined cell; the selected date (October 30) has a filled cell. Both states remain distinguishable without depending only on hue.
- Compact mode emphasizes the grid, with Today, week number, and settings access in its footer.
- Expanded mode increases spacing and text size, adds today's full date, and shows the selected date explicitly in its footer.
- Month navigation, Today, and selected-date behavior are shared between modes. No agenda or account interface is implied.
- Today's weekday appears in both views. The current month has a soft accent title and a `Current month` label, distinct from today and selection.
- Saturday/Sunday columns are gray in these examples; Appearance supports arbitrary gray-weekday selection. Sample event dots use illustrative Work/Personal calendar colors.
- General shows opt-in Windows startup and an unassigned optional global shortcut. Calendars demonstrates checked sources. About includes GitHub, koona.app, GPL licensing, and a manual update-check button.

These are first visual concepts for owner review, not implemented application screenshots. Calendar names/events are synthetic; no live account data was accessed. Hover/focus states, settings navigation, permission/error/empty states, tray menu, high-contrast treatment, scaling, long localized text, and accessibility still require the checks in [the roadmap](../docs/roadmap.md). Feature acceptance criteria are in [Required features](../docs/features.md).

## Reproduction and inspection

Requires Python 3.9+ and installed Lunacy with its MCP endpoint available. No third-party Python packages, external images, or downloaded icons are used. The connector starts `C:\Program Files\Lunacy\Lunacy.exe -mcp` and calls its stdio MCP tools. Use `--exe` to override the installation path.

```powershell
python design/lunacy_design.py probe
python design/lunacy_design.py area
python design/lunacy_design.py create
```

Use a blank document when reproducing: creation adds new boards at x=0 and x=1000 and does not replace existing layers. The command prints the new board IDs; use those IDs with `inspect`, `layers`, or `export`. Export returns MCP image content on stdout; it does not save files. Do not commit captured base64 image payloads.

Inspect the lower area with `area --y 820`, then use `preferences` to create the settings boards below the calendars. The `revise <light-board-id> <dark-board-id>` command updates the existing calendar concepts while retaining matching layer IDs; use it only for these concept boards, as it replaces their content from the source. Repeating a creation command adds another set of boards.

```powershell
python design/lunacy_design.py inspect <board-id>
python design/lunacy_design.py export <board-id>
```

The source is GPL-3.0-only under the repository license. Icons are original vector strokes. Segoe UI is referenced as an installed system font, not bundled.
