# Contributing to Lightcal

Lightcal is a small Rust and Slint calendar for Windows. Start with the [documentation index](docs/README.md) and [required features](docs/features.md). Discuss substantial scope or architecture changes in an issue before implementing them.

## Set up

Install Rust (the repository pins the stable channel in `rust-toolchain.toml`), [just](https://github.com/casey/just), Python 3.9 or newer, and [Bun](https://bun.sh) for the release-script tests.

```powershell
just check
```

`just check` runs the documentation check, the release-script tests, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --locked`. Windows-only code sits behind `cfg`, so the same gate runs on Linux and macOS.

Tests that write preferences must use a temporary `LIGHTCAL_DATA_DIR`. Never touch real autostart, registry, or profile data from an automated test.

Without a display, render the UI to PNG files with no network access:

```powershell
cargo run -p lightcal --locked -- --render-snapshots target/snapshots
```

`just run-demo` opens the app with sample events only. Copy `.env.example` to the ignored `.env` only when you need your own OAuth client ids; never commit it.

## Workflow

1. Create a `feature/<short-description>` branch from `main`.
2. Keep each PR focused on one reviewable user outcome. Write all documentation in English.
3. Run `just check` before opening the PR.
4. Describe the behavior, the validation evidence, and any unresolved limitations in the PR. Include screenshots for UI changes and release-build measurements for background or runtime changes. State which platforms you actually tested.
5. Wait for owner acceptance and required checks. Do not push directly to `main`, merge without permission, or publish releases without permission.

App-changing PRs update the workspace version in `Cargo.toml` and the user-visible notes when applicable. Documentation-only PRs do not need a version bump.

## Scope

Calendar access is read-only. Telemetry, other event providers, and automatic update installation need an owner decision first; see [required features](docs/features.md).

## License and assets

By contributing, you agree to license your contributions under GPL-3.0-only. Retain third-party notices and identify the origin and license of imported material in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). Do not include credentials, signing material, private profile data, proprietary fonts, or assets without redistribution rights.

The UI design lives on the owner's design canvas; [design/README.md](design/README.md) explains how the mirrored sources in `design/canvas/` relate to it.

Follow the [community rules](CODE_OF_CONDUCT.md). Report security issues through [SECURITY.md](SECURITY.md).
