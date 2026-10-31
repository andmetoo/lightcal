# macOS system typography

Owner decision on October 31, 2026: use the macOS system font, represented by
**SF Pro** in design mockups. Windows uses Onest. The app implementation should
resolve the platform system font; do not bundle Apple's font download with
the application or GPL repository.

## Penpot setup

The connected Penpot catalog currently lacks SF Pro. Its MCP API can apply
existing fonts but cannot upload custom fonts. The owner must add the design
faces through **Dashboard → Fonts → Add custom font → Upload**, in the same
team as the Lightcal file. Retain family names **SF Pro Text** and
**SF Pro Display**, with Regular 400, Medium 500, Semibold 600 and Bold 700.

Prepared unmodified OTF files are in ignored
`target/sf-pro-proof/upload/`. Only use them for the macOS mockups under Apple's
terms. The files are deliberately excluded from repository deliverables.

- Below 20 px: SF Pro Text.
- At 20 px and above: SF Pro Display.
- Preserve existing explicit weights, font sizes, control dimensions and links.
- Japanese strings retain Noto Sans JP in the current multilingual design.
- Option/Shift icons in the shortcut specimen are vectors, independent of fonts.

Once the families are available, save a permanent version and run:

```powershell
bun design/penpot/local-client.mjs run design/penpot/refinement.js
bun design/penpot/local-client.mjs run design/penpot/refinement-macos-typography.js 2:3
bun design/penpot/local-client.mjs run design/penpot/refinement-macos-typography.js 3:4
bun design/penpot/local-client.mjs report design/penpot/refinement-verify.js design/penpot/refinement-manifest.json
```

Re-export and review the macOS App, calendar, account/consent and shortcut-error
specimens before recording completion. Until that import and review, current
Penpot macOS text remains Onest; this document records a prepared migration.

## Official source

[Apple's fonts page](https://developer.apple.com/fonts/) links the official
[SF Pro design package](https://devimages-cdn.apple.com/design/resources/download/SF-Pro.dmg)
and includes the license. Prepared faces report version **22.0d5e4**.
Downloaded October 31, 2026, package SHA-256:

```text
968ab3b8b1f92c2d8af61eb0680f5c2224c4e78d59b986bf004502a7fc012918
```

`python design/penpot/prepare-sf-pro.py` downloads/verifies the package and
extracts the selected faces using 7-Zip on Windows. It never runs the Apple
installer, installs system fonts or changes application data.

[Penpot's official font instructions](https://help.penpot.app/user-guide/designing/text-typo/)
explain the team upload and grouping behavior.
