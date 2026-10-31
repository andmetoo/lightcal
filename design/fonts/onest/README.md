# Onest design font

The owner selected **Onest 5.3** for Windows on October 31, 2026. This means the Fontsource
5.3.x package line, pinned here to **@fontsource/onest 5.3.1**. It contains font
version **2.001**, Google Fonts revision **v11**; these version numbers describe
different distributions of the same family.

Native TrueType files come from the author's
[Onest 2.001 release](https://github.com/simpals/onest/releases/tag/2.001).
Regular, Medium, SemiBold and Bold cover the design's 400/500/600/700 weights.
The design does not add a browser container or a Fontsource runtime dependency.
The shipped Rust/Slint application's font is unchanged by this design update.

Copyright 2021 The Onest Project Authors (https://github.com/simpals/onest).
These font files are licensed under **SIL Open Font License 1.1**, included in
[OFL.txt](OFL.txt); they retain that license alongside the GPL application.

## Provenance and verification

[source.json](source.json) pins the official npm archive's SHA-512 integrity,
the author's release ZIP SHA-256, the source commit and each local file's SHA-256.
No downloaded package scripts are executed. Recreate the files from the repo root:

```powershell
python design/penpot/vendor-onest.py
```

FontTools verification on October 31, 2026 found version 2.001 in both sources.
At each of 400, 500, 600 and 700, all 223 shared Latin and 104 shared Cyrillic
glyph advances match between the native TTF and Fontsource WOFF subsets. This
checks character coverage and widths, not byte identity or rasterization.

Penpot uses exact built-in family **Onest**, ID `gfont-onest`. The plugin API
does not expose that service's binary revision, so the design does not claim
that Penpot's hosted binary is byte-identical to the pinned native files.
Japanese strings use Noto Sans JP; Latin text and numerals use Onest.
The subsequent [macOS system-font decision](../macos-system.md) replaces Onest
in macOS-specific mockups once SF Pro is imported into the Penpot team.
