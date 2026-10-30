# Lightcal

A lightweight calendar for Windows with a compact calendar accessible from the tray or taskbar, with a future macOS version in mind.

**Status: prototype & workspace implemented.** The `calendar-core` and `lightcal` (Slint) Cargo workspace, multi-page Penpot design system, and `just` + `bun` release/signing pipeline for SimpleSign Desktop are implemented on the feature branch.

## Features & Architecture

- Open the compact calendar (`352×432`) from the tray or pinned app; settings use a separate `780×620` layout. The expanded agenda view has been removed.
- Configurable dynamic tray icon modes: **Date**, **Weekday + Date**, or **Time + Date**. These change Lightcal's icon; they do not replace Windows' system clock. Pinned shortcut artwork can remain static even while the running window and tray icons update.
- Navigate months, return to today (`Today` pill with `#E5484D` coral dot), and navigate the 42-day grid with keyboard shortcuts.
- Run quietly in the background without a browser runtime or mandatory account.
- English by default, with bundled Russian (`ru`), Spanish (`es`), and Japanese (`ja`) catalogs.
- Read-only Microsoft Graph / ICS event dots (capped at 3 per day), configurable gray weekdays, isolated startup/shortcut management, GitHub update checking, and optional signed portable-update installation.

## Build, Check & Sign Release (`just` + `bun`)

```powershell
# Run documentation, formatting, clippy, and unit test gates
just check

# Build optimized release binary (target/dist/lightcal.exe) + SHA-256
just build-release

# Build an unsigned NSIS installer for local review (target/nsis/)
just build-installer

# Build and sign both application and NSIS setup via SimpleSign Desktop
just release

# Sign an existing release binary (or pass custom SimpleSign CLI flags)
just sign-release
```

`just run` and `just run-tray` use configured Microsoft Graph / private HTTPS ICS subscriptions. Windows calendars are an optional, off-by-default legacy switch. `just run-demo` explicitly uses sample events. On first connection, select calendars in Settings before expecting event dots. `just probe-calendars` runs the explicit legacy Windows diagnostic; normal tests use fixtures and temporary preferences.

`just release` builds and signs the application with SimpleSign Desktop, verifies Authenticode, packages the portable ZIP, builds an NSIS installer containing that signed application, then signs and verifies the installer. It stops on a missing signer or invalid signature. Set `SIMPLESIGN_PATH` for an installed signer; credentials and signing keys stay outside the repository. For manual signing, run `just build-release`, sign `target/dist/lightcal.exe`, then `just verify-release` and `just sign-installer`. The default installer build permits unsigned local-review packages and never claims they are signed.

GitHub Actions checks the Rust workspace on pull requests. Every push to `main` builds an **unsigned** Windows NSIS installer and portable archive, and a `vX.Y.Z` tag on `main` turns that build into a draft release without recompiling. CI skips executable signing, never publishes, and never accesses signing keys. See [Windows installer setup](docs/windows-installer.md#ci-and-validation).

Update checking uses GitHub's latest stable release endpoint. Manual checks run on a worker thread with a 15-second request/body deadline. The optional automatic check is off by default; when enabled it runs at startup and daily. Signed automatic installation is separately opt-in and requires a build-configured certificate SHA-256 pin, matching Authenticode trust/version, and a signed portable installation. No update is installed without that trust configuration.

Copy [.env.example](.env.example) to the ignored `.env.local` for public build settings. Microsoft sign-in can remain unconfigured until the owner creates an Entra Desktop app registration; the Client ID is configured at build time, never edited in Settings. See [provider and signing setup](docs/provider-setup.md) for exact registration steps, supported ICS features, SimpleSign fingerprint extraction, and remaining live-test gates. Google sign-in is deferred.

Windows distribution uses an NSIS `setup.exe`: a per-user installation, Start menu shortcut, optional desktop shortcut, and an uninstaller that retains calendar settings and credentials. Use `just build-installer` to build or `just package-installer` to package the existing release binary. The build locates NSIS or downloads a checksum-verified portable compiler into `target/tools`; no compiler installation is required. See [Windows installer setup](docs/windows-installer.md) for signing, CI artifacts, and isolated installation tests.

See the [implementation audit](docs/implementation-audit-2026-10-30.md) for verified fixes, provider limitations, and remaining release gates.

## Start here

See the [documentation index](docs/README.md), [implementation plan](docs/roadmap.md), and [framework decision](docs/decisions/0001-ui-framework.md).

All repository documentation is written in English. Contributions go through pull requests; see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Lightcal is licensed under [GPL-3.0-only](LICENSE). Copyright (c) 2026 Lightcal contributors.
