# ADR 0014: Ten interface languages

Date: 2026-11-03. Status: accepted by the owner in chat.

## Context

Lightcal shipped in English, Russian, Spanish and Japanese. The owner asked
for more languages and chose six that cover large groups of Windows users and
need nothing the app cannot already do: German, French, Brazilian Portuguese,
Ukrainian, Simplified Chinese and Korean. Right-to-left scripts (Arabic,
Hebrew, Persian) still need a mirrored layout and stay out of scope. Thai and
Devanagari need shaping and line breaking that have not been tested with
Slint here, so they stay out too.

## Decision

- Settings offers ten languages, stored under these preference codes: `en`,
  `de`, `es`, `fr`, `pt-BR`, `ru`, `uk`, `ja`, `ko` and `zh-Hans`. A fresh
  installation still starts in English, and an unknown saved code still falls
  back to English. `pt`, `zh` and `zh-CN` load as `pt-BR` and `zh-Hans`.
- Each language is listed by its own name. The language menu shows the ten in
  two columns of five, filled top to bottom: Latin scripts on the left,
  Cyrillic and CJK on the right. The menu is right-aligned under the select
  and never scrolls.
- Ukrainian shares the Russian plural rule (one, few, many) and uses genitive
  month names in dates. Chinese and Korean put the year first in the calendar
  header, like Japanese.
- Simplified Chinese uses Microsoft YaHei UI, which ships with Windows. Slint's
  fallback otherwise takes CJK glyphs from a Japanese face that lacks many
  simplified characters. The 简体中文 entry in the language menu uses the same
  font in every language. Nothing is bundled.
- The new catalogs were written from the English source and have not been
  reviewed by native speakers. A native review of each language is a release
  task.

## Consequences

- The `LocaleCode` enum and every catalog match cover ten languages; `LocaleCode::ALL`
  drives the language menu, the per-language tests and the snapshots.
- `--render-snapshots` writes every settings tab in each non-English language,
  a calendar today and event-detail view per new language, and
  `settings-app-language-menu-*.png` with the menu open, including
  `settings-app-language-menu-light-zh-Hans.png` in the Chinese UI.
- The [Languages board](../../design/canvas/Languages.dc.html) adds the open
  menu, German Calendar and Chinese App boards.
