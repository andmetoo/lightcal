set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

# List available recipes
default:
    @just --list

# Run documentation, formatting, clippy, and unit tests
check: check-docs fmt-check clippy test

# Check Markdown links and trailing whitespace
check-docs:
    python scripts/check_docs.py

# Check Rust formatting
fmt-check:
    cargo fmt --all -- --check

# Run Clippy across the workspace with warnings denied
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Run all workspace unit tests
test:
    cargo test --workspace

# Run the compact calendar
run:
    cargo run -p lightcal -- --compact

# Run Lightcal in compact tray popup mode
run-tray:
    cargo run -p lightcal -- --compact

# Demo fixture only; never connects to real calendars
run-demo:
    cargo run -p lightcal -- --compact --synthetic-calendars

# Explicit read-only Windows calendar probe (excluded from ordinary tests)
probe-calendars:
    cargo test -p lightcal real_windows_store_does_not_crash -- --ignored --nocapture

# Build optimized release binary and stage to target/dist/lightcal.exe with SHA-256 via Bun
build-release:
    bun run scripts/release.ts build

# Sign target/dist/lightcal.exe via SimpleSign Desktop and update SHA-256 via Bun
sign-release:
    bun run scripts/release.ts sign

# Verify Authenticode signature on target/dist/lightcal.exe and refresh SHA-256 checksum
verify-release:
    bun run scripts/release.ts verify

# Print the public SHA-256 signing certificate fingerprint after signature verification
certificate-info:
    bun run scripts/release.ts certificate-info

# Package portable archive with license/notices/readme and SHA-256 (permits unsigned review builds)
package-release:
    bun run scripts/release.ts package

# Build, sign, and verify the app and NSIS setup; also package the portable ZIP
release:
    bun run scripts/release.ts release

# Build an unsigned NSIS review installer
build-installer:
    pwsh -NoLogo -NoProfile -File scripts/package-nsis.ps1

# Package the existing release EXE without rebuilding
package-installer:
    pwsh -NoLogo -NoProfile -File scripts/package-nsis.ps1 -SkipBuild

# Verify a signed app, package it, sign the setup, and verify the setup
sign-installer:
    bun run scripts/release.ts sign-installer

# Fetch the pinned portable compiler without a system installation
prepare-nsis:
    pwsh -NoLogo -NoProfile -File scripts/prepare-nsis.ps1

# Exercise a compile-time isolated installer/uninstaller in target/ only
test-installer:
    pwsh -NoLogo -NoProfile -File scripts/test-nsis.ps1

# Build an unsigned macOS review disk image; macOS only, no signing
package-dmg:
    ./scripts/package-dmg.sh
