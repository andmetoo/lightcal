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

# Build and sign release binary via SimpleSign Desktop
just release

# Sign an existing release binary (or pass custom SimpleSign CLI flags)
just sign-release
```

`just run` and `just run-tray` use configured Microsoft Graph / private HTTPS ICS subscriptions. Windows calendars are an optional, off-by-default legacy switch. `just run-demo` explicitly uses sample events. On first connection, select calendars in Settings before expecting event dots. `just probe-calendars` runs the explicit legacy Windows diagnostic; normal tests use fixtures and temporary preferences.

The existing `just release` workflow builds, launches SimpleSign Desktop, verifies Authenticode, and packages only after verification succeeds. It fails if the signer is missing or the signature is invalid. Configure `SIMPLESIGN_PATH` for an installed signer; credentials and signing keys remain outside the repository. For manual signing, use `just build-release`, sign `target/dist/lightcal.exe`, then run `just verify-release` and `just package-release`. `just package-release` alone also permits unsigned review builds; it does not attest a signature.

GitHub Actions checks the Rust workspace and prepares an **unsigned** Windows portable archive for pull requests or a manual workflow run. It does not publish a release or access signing keys. No workflow has been run remotely for unpushed local changes.

Update checking uses GitHub's latest stable release endpoint. Manual checks run on a worker thread with a 15-second request/body deadline. The optional automatic check is off by default; when enabled it runs at startup and daily. Signed automatic installation is separately opt-in and requires a build-configured certificate SHA-256 pin, matching Authenticode trust/version, and a signed portable installation. No update is installed without that trust configuration.

Copy [.env.example](.env.example) to the ignored `.env.local` for public build settings. Microsoft sign-in can remain unconfigured until the owner creates an Entra Desktop app registration; the Client ID is configured at build time, never edited in Settings. See [provider and signing setup](docs/provider-setup.md) for exact registration steps, supported ICS features, SimpleSign fingerprint extraction, and remaining live-test gates. Google sign-in is deferred.

Microsoft Store MSIX preparation is also available: `just build-msix-preview` creates an unsigned review package; `just build-msix` requires the real Partner Center identity. Microsoft signs Store-distributed MSIX packages after certification and supplies their update channel. This does not sign portable GitHub EXE downloads. The MSIX workflow only builds/uploads review artifacts; nothing submits or publishes. See [Store distribution setup](docs/store-distribution.md) for identity values, the first manual publication requirement, costs, and remaining installed-package validation.

See the [implementation audit](docs/implementation-audit-2026-10-30.md) for verified fixes, provider limitations, and remaining release gates.

## Start here

See the [documentation index](docs/README.md), [implementation plan](docs/roadmap.md), and [framework decision](docs/decisions/0001-ui-framework.md).

All repository documentation is written in English. Contributions go through pull requests; see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Lightcal is licensed under [GPL-3.0-only](LICENSE). Copyright (c) 2026 Lightcal contributors.
