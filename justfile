set windows-shell := ["pwsh.exe", "-NoLogo", "-NoProfile", "-Command"]

default:
    @just --list

# Linux gate: docs, script tests, format, Clippy, tests
check:
    bun scripts/check-docs.ts
    bun test scripts
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked

# Compact calendar; extra flags pass through to lightcal
run *args:
    cargo run -p lightcal -- --compact {{args}}

# Sample events only; never connects to real calendars
run-demo: (run "--synthetic-calendars")

# Read-only Windows calendar probe, excluded from ordinary tests
probe-calendars:
    cargo test -p lightcal real_windows_store_does_not_crash -- --ignored --nocapture

# Release step: build | sign | verify | certificate-info | package | sign-installer | release
release step="release" *args:
    bun run scripts/release.ts {{step}} {{args}}

# Download CI build vX.Y.Z, sign it on Windows and replace its assets
sign-draft version: (release "sign-draft" version)

# Verify signatures, then publish or promote vX.Y.Z
publish-release version: (release "publish-release" version)

# NSIS setup; -SkipBuild packages the existing release EXE
installer *args:
    pwsh -NoLogo -NoProfile -File scripts/package-nsis.ps1 {{args}}

# Fetch the pinned portable NSIS compiler into target/tools
prepare-nsis:
    pwsh -NoLogo -NoProfile -File scripts/prepare-nsis.ps1

# Isolated installer and uninstaller test inside target/
test-installer:
    pwsh -NoLogo -NoProfile -File scripts/test-nsis.ps1
