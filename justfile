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

# Build release binary, invoke SimpleSign Desktop signing workflow, and package portable ZIP + SHA-256 via Bun
release:
    bun run scripts/release.ts release

# Build an unsigned MSIX with an explicit preview identity; never install or submit
build-msix-preview:
    pwsh -NoLogo -NoProfile -File scripts/package-msix.ps1 -Preview

# Build an unsigned Store submission package using Partner Center identity values
build-msix:
    pwsh -NoLogo -NoProfile -File scripts/package-msix.ps1
