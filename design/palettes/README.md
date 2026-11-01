# Lightcal Gray + Teal

Owner decision, October 31, 2026: remove violet/indigo from all active interface
themes and design specimens. The selected palette uses strictly neutral Gray
surfaces with restrained Teal functional accents. Coral retains the application
identity and Today semantics.

The foundation is [Radix Colors](https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale),
using its [Gray/Teal light](https://raw.githubusercontent.com/radix-ui/colors/main/src/light.ts)
and [dark](https://raw.githubusercontent.com/radix-ui/colors/main/src/dark.ts)
scales. `#00796B` is the darker primary adaptation, chosen for white small text.
Attribution and the full [MIT license](RADIX-LICENSE.txt) are preserved.

[gray-teal.json](gray-teal.json) records the roles and legacy-to-current migration
map. The old values are migration inputs, not active theme colors.

| Role | Light | Dark |
| --- | --- | --- |
| Canvas / group | `#FFFFFF` / `#F0F0F0` | `#111111` / `#222222` |
| Main / secondary text | `#202020` / `#646464` | `#EEEEEE` / `#B4B4B4` |
| Selected fill / border | `#E0E0E0` / `#838383` | `#3A3A3A` / `#7B7B7B` |
| Primary action / switch on | `#00796B`, white ink | `#00796B`, white ink |
| Focus / active-tab cue | `#00796B` | `#53B9AB` |

Selection uses a neutral filled surface and an inner border. An active tab also
gets a short 2 px Teal marker. Focus specimens use an outer 2 px Teal ring with
a gap, rather than reusing the persistent selected border.

Calculated sRGB contrast: white primary text 5.32:1; white hover text on
`#008573` 4.56:1; selected text 12.34:1 light and 9.80:1 dark. This is a targeted
palette check, not a complete accessibility audit. An independent UI review
passed the native light/dark App, dark Calendar, dropdown-color and compact
calendar specimens at 1×. Headless popup crops do not prove native popup placement.

The active Penpot file is migrated in stable top-level board batches using
`refinement-palette.js` (for example `0:16`, through `96:114`). The CLI supplies
this JSON, preserving board IDs, geometry, text and prototype links. Permanent
version `Before Gray Teal palette 2026-10-31` preserves the earlier stage.

Real account/calendar colors supplied by providers remain user data. Only
the application's own sample colors and missing-color fallbacks change.
