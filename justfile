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

# Recipes with pass-through arguments run as Bun scripts, so each argument
# reaches the command unchanged (no shell splitting or evaluation).

# Compact calendar; extra flags pass through to lightcal
[positional-arguments]
[script("bun")]
[extension(".ts")]
run *args:
    const { exitCode } = Bun.spawnSync(["cargo", "run", "-p", "lightcal", "--", "--compact", ...process.argv.slice(2)], { stdio: ["inherit", "inherit", "inherit"] });
    process.exit(exitCode ?? 1);

# Sample events only; never connects to real calendars
run-demo: (run "--synthetic-calendars")

# Read-only Windows calendar probe, excluded from ordinary tests
probe-calendars:
    cargo test -p lightcal real_windows_store_does_not_crash -- --ignored --nocapture

# Release step: build | sign | verify | certificate-info | package | sign-installer | release
[positional-arguments]
[script("bun")]
[extension(".ts")]
release step="release" *args:
    const { exitCode } = Bun.spawnSync(["bun", "run", "scripts/release.ts", ...process.argv.slice(2)], { stdio: ["inherit", "inherit", "inherit"] });
    process.exit(exitCode ?? 1);

# Copy build configuration from .env to GitHub Actions variables/secrets; --dry-run to preview
[positional-arguments]
[script("bun")]
[extension(".ts")]
github-env *args:
    const { exitCode } = Bun.spawnSync(["bun", "run", "scripts/github-env.ts", ...process.argv.slice(2)], { stdio: ["inherit", "inherit", "inherit"] });
    process.exit(exitCode ?? 1);

# Download CI build vX.Y.Z, sign it on Windows and replace its assets
sign-draft version: (release "sign-draft" version)

# Verify signatures, then publish or promote vX.Y.Z
publish-release version: (release "publish-release" version)

# NSIS setup; -SkipBuild packages the existing release EXE
[positional-arguments]
[script("bun")]
[extension(".ts")]
installer *args:
    const { exitCode } = Bun.spawnSync(["pwsh", "-NoLogo", "-NoProfile", "-File", "scripts/package-nsis.ps1", ...process.argv.slice(2)], { stdio: ["inherit", "inherit", "inherit"] });
    process.exit(exitCode ?? 1);

# Fetch the pinned portable NSIS compiler into target/tools
prepare-nsis:
    pwsh -NoLogo -NoProfile -File scripts/prepare-nsis.ps1

# Isolated installer and uninstaller test inside target/
test-installer:
    pwsh -NoLogo -NoProfile -File scripts/test-nsis.ps1
