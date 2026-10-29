# Documentation

Planning baseline: October 29, 2026.

| Document | Purpose |
| --- | --- |
| [Taskbar presence decision](decisions/0005-taskbar-calendar-presence.md) | Persistent taskbar activation and calendar thumbnails |
| [Compact view and account settings decision](decisions/0004-compact-view-and-account-settings.md) | Compact-only calendar and verified account controls |
| [Product](product.md) | Scope, approved choices, and window behavior |
| [Required features](features.md) | Owner-required settings, Windows calendars, event dots, updates, and About |
| [Direct providers and updates decision](decisions/0002-direct-calendars-and-updates.md) | Owner-approved Microsoft Graph, private ICS, and signed automatic installation |
| [Provider and update setup](provider-setup.md) | Microsoft registration, root local configuration, ICS, and SimpleSign certificate setup |
| [MSIX Store decision](decisions/0003-msix-store-channel.md) | Preparation of a Microsoft-managed signing and update channel |
| [Microsoft Store distribution](store-distribution.md) | Free Store signing, package identity, build recipes, CI, and first-publication requirements |
| [UI framework decision](decisions/0001-ui-framework.md) | Accepted Slint choice, alternatives, evidence, and validation gate |
| [Architecture](architecture.md) | Rust boundaries and platform integration |
| [Roadmap](roadmap.md) | Ordered milestones and acceptance criteria |
| [Validation](validation.md) | Performance targets and functional checks |
| [Implementation audit](implementation-audit-2026-10-30.md) | October 30 review, fixes, observed Windows provider result, and remaining release gates |
| [Open-source readiness](open-source.md) | Contribution, repository settings, licensing, and release gates |
| [Calendar design concepts](../design/README.md) | Penpot redesign, page structure, source, and review evidence |

These documents describe planned behavior unless explicitly marked as implemented or measured. The Cargo workspace (`crates/calendar-core` and `crates/lightcal`), Penpot design specification (`design/`), and `just` + `bun` release/signing scripts (`justfile`, `scripts/release.ts`) are implemented.
