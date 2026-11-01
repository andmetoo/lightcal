# Lightcal Penpot automation

The current owner file is **lightcalendar**, ID
`c2d27051-3cd1-813e-8008-dd9b45707753`, through `penpot_local`.
See [the design specification](../README.md) for the final concept and review evidence.

## Current scripts

| Script | Responsibility |
| --- | --- |
| [refinement.js](refinement.js) | Palettes, vector primitives, compact calendar and page ownership |
| [refinement-dense.js](refinement-dense.js) | Final 448 × 360 Calendar / Accounts / App panel and About |
| [refinement-settings.js](refinement-settings.js) | Windows settings population |
| [refinement-windows.js](refinement-windows.js) | Windows calendar and shell specimens |
| [refinement-states.js](refinement-states.js) | Provider, permission, empty, offline, shortcut and update states |
| [refinement-macos.js](refinement-macos.js) | macOS calendar, settings, menu bar and Dock |
| [refinement-languages.js](refinement-languages.js) | Four-language layout examples |
| [refinement-icons.js](refinement-icons.js) | Icon specimens and transparent export boards |
| [refinement-handoff.js](refinement-handoff.js) | Tray menu, additional dates, shared controls and Overview |
| [refinement-prototype.js](refinement-prototype.js) | Navigation, contrast normalization and three presentation flows |
| [refinement-ui-polish.js](refinement-ui-polish.js) | Exact fonts, 28 px controls, shared selectors and icon optics on existing boards |
| [refinement-typography.js](refinement-typography.js) | Apply owner-selected Onest while retaining geometry, weights and prototype links |
| [vendor-onest.py](vendor-onest.py) | Verify pinned Fontsource/upstream archives and prepare native design fonts with attribution |
| [refinement-macos-typography.js](refinement-macos-typography.js) | Apply SF Pro to macOS-specific boards after team font import |
| [prepare-sf-pro.py](prepare-sf-pro.py) | Prepare Apple's design faces in ignored scratch storage without installation |
| [refinement-macos-key-symbols.js](refinement-macos-key-symbols.js) | Replace missing keyboard glyphs with vector symbols |
| [refinement-font-content.js](refinement-font-content.js) | Read-only text inventory for native glyph coverage checks |
| [refinement-date-numerals.js](refinement-date-numerals.js) | Replace dated calendar badges with transparent numerals while retaining board IDs and links |
| [refinement-palette.js](refinement-palette.js) | Gray + Teal migration of all active boards, preserving geometry and links |
| [refinement-verify.js](refinement-verify.js) | Read-only inventory, duplicate and containment audit |
| [refinement-cleanup.js](refinement-cleanup.js) | Snapshot-guarded removal of old pages and concept boards |
| [refinement-manifest.json](refinement-manifest.json) | Final audited page/board inventory |
| [local-client.mjs](local-client.mjs) | Small client for the configured MCP server; no stored credential |
| [render-icons.mjs](render-icons.mjs) | Reproducible SVG-to-PNG assets and icon review sheet |

`design.js`, `pages.js`, `verify.js` and `manifest.json` are historical generation
sources for the earlier 39-board design. They do not describe the current UI.

## Connection and execution

The preferred connection is the configured Penpot MCP tool. If the host has not
exposed that tool, `local-client.mjs` reads `PENPOT_MCP_URL`, or the owner's
`mcp_servers.penpot_local.url` from the local Codex configuration. It does not
print that URL. Never copy a credential into source, documentation or logs.

Open the file in Penpot and connect its MCP plugin before running scripts.
Read the MCP high-level overview and relevant API documentation before editing.
Calls execute in the plugin context; `storage.refinement` persists between calls.
Only mutate shapes on the active page. Builders await page activation.

Example, from the repository root:

```powershell
bun design/penpot/local-client.mjs run design/penpot/refinement.js
bun design/penpot/local-client.mjs run design/penpot/refinement-dense.js
bun design/penpot/local-client.mjs run design/penpot/refinement-settings.js
bun design/penpot/local-client.mjs run design/penpot/refinement-windows.js 0:1
bun design/penpot/local-client.mjs run design/penpot/refinement-windows.js 1:2
```

Continue with states in batches `0:8`, `8:16`, `16:22`, `22:26` for each of
`Windows` and `macOS` (fourth argument). Run macOS batches `0:3`, `3:7`, `7:10`,
`10:14`; language batches `0:2`, `2:4`; icons; and handoff batches `0:2`, `2:3`,
`3:4`. Wait for each call to finish. Small batches avoid blocking the browser's
plugin heartbeat. A long call can complete edits after its transport times out;
inspect existing boards before retrying.

Before removing historical pages, create and verify the permanent snapshot:

```powershell
bun design/penpot/local-client.mjs snapshot "Before refinement cleanup 2026-10-31"
bun design/penpot/local-client.mjs run design/penpot/refinement-cleanup.js
bun design/penpot/local-client.mjs run design/penpot/refinement-prototype.js
bun design/penpot/local-client.mjs report design/penpot/refinement-verify.js design/penpot/refinement-manifest.json
```

The local Penpot build's whole-file `File.export` fails with `No matching clause`;
restoration relies on the verified permanent Penpot version. Image exports work.
Do not claim that a local `.penpot` archive was produced.

Page/board plugin keys preserve regeneration identity after display-name cleanup.
Builders only overwrite their own revision's boards and do not alter unrelated
pages. The prototype is a design review flow: it does not perform real OAuth,
updates, remote account changes or keyboard registration.

For an existing pre-polish file, create a permanent version, load `refinement.js`
and `refinement-dense.js`, then run `refinement-ui-polish.js` in page batches
`0:1` through `6:7`. The operation preserves board IDs and navigation, uses exact
font-name matching, and is idempotent. New builds use the same final primitives
directly. Run cleanup, prototype and verification after rebuilding examples.

For the owner's Onest migration, save a permanent version, load `refinement.js`,
then run `refinement-typography.js` in page batches `0:1` through `6:7`. Do not
rebuild the controls just to change fonts. Exact Onest and Noto Sans JP families
must be available. The Japanese fallback and explicit weights are preserved.
Fontsource 5.3.1 and native font 2.001 provenance are in
[the font record](../fonts/onest/README.md); Penpot's hosted revision is unreported.
The owner's subsequent macOS decision uses SF Pro Text/Display. Font-changing
builders now require the correct platform family and never silently substitute
Onest for a missing SF Pro face. See [macOS setup](../fonts/macos-system.md).

To regenerate icon PNGs without adding an application dependency:

```powershell
New-Item -ItemType Directory -Force target/design-tools
bun add --cwd target/design-tools @resvg/resvg-js@2.6.2
bun design/penpot/render-icons.mjs
```
