# Documentation

Documentation baseline: October 29, 2026; updated for the public repository on November 1, 2026. The application is implemented (release 1.0.2).

| Document | Purpose |
| --- | --- |
| [Windows-only design refresh](decisions/0011-windows-only-design-refresh.md) | November 1 redesign, new icons, event details, design canvas, macOS dropped |
| [Gray + Teal palette](../design/palettes/README.md) | Owner-selected neutral surfaces, Teal actions, and removal of violet |
| [Calendar status icon](decisions/0010-windows-calendar-tray-icon.md) | Calendar-page tray icon and native raster checks; artwork superseded by ADR 0011 |
| [Windows date numerals](decisions/0009-windows-date-numerals.md) | Historical numeral-only experiment, superseded by ADR 0010 |
| [Google Calendar and platform design](decisions/0008-google-calendar-and-platform-design.md) | Read-only Google scope; its design is superseded by ADR 0011 |
| [Windows installer](windows-installer.md) | NSIS setup, signing, CI, and isolated installer tests |
| [NSIS distribution decision](decisions/0006-nsis-windows-distribution.md) | Owner-approved Windows installer replacing the retired package channel |
| [Release signing decision](decisions/0007-draft-release-signing.md) | Cloud build, local signing for any release status, and explicit publication |
| [Taskbar presence decision](decisions/0005-taskbar-calendar-presence.md) | Persistent taskbar activation and calendar thumbnails |
| [Compact view and account settings decision](decisions/0004-compact-view-and-account-settings.md) | Compact-only calendar and verified account controls |
| [Product](product.md) | Scope, approved choices, and window behavior |
| [Required features](features.md) | Owner-required settings, Windows calendars, event dots, updates, and About |
| [Direct providers and updates decision](decisions/0002-direct-calendars-and-updates.md) | Owner-approved Microsoft Graph, private ICS, and signed automatic installation |
| [Provider and update setup](provider-setup.md) | Microsoft registration, root local configuration, ICS, and SimplySign certificate setup |
| [UI framework decision](decisions/0001-ui-framework.md) | Accepted Slint choice, alternatives, evidence, and validation gate |
| [Architecture](architecture.md) | Rust boundaries and platform integration |
| [Roadmap](roadmap.md) | Ordered milestones and acceptance criteria |
| [Validation](validation.md) | Performance targets and functional checks |
| [Implementation audit](implementation-audit-2026-10-30.md) | October 30 review, fixes, observed Windows provider result, and remaining release gates |
| [Open-source readiness](open-source.md) | Contribution, repository settings, licensing, and release gates |
| [Calendar design](../design/README.md) | Design canvas pages, mirrored sources, archived Penpot file |

Decision records, the roadmap, and the validation targets keep their original planning wording. Treat their numbers as proposed targets unless a document marks them as measured. The Cargo workspace (`crates/calendar-core` and `crates/lightcal`), design sources (`design/`), and `just` + `bun` release/signing scripts (`justfile`, `scripts/release.ts`) are implemented.
