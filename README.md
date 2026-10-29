# Lightcal

A lightweight calendar for Windows with compact tray and expanded taskbar views, with a future macOS version in mind.

**Status: planning.** There is no application, installer, or performance measurement yet.

## Planned experience

- Click the tray icon for a compact calendar.
- Launch or activate the pinned taskbar application for a larger calendar.
- Navigate months, return to today, and use the calendar with a keyboard.
- Run quietly in the background, without a browser runtime or an account.

Rust is the implementation language and Slint is the selected UI framework. Platform integration and performance will be verified in a small prototype. Its widgets are drawn by the toolkit; they are not Windows or AppKit controls.

## Start here

See the [documentation index](docs/README.md), [implementation plan](docs/roadmap.md), and [framework decision](docs/decisions/0001-ui-framework.md).

All repository documentation is written in English. Contributions go through pull requests; see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Lightcal is licensed under [GPL-3.0-only](LICENSE). Copyright (c) 2026 Lightcal contributors.
