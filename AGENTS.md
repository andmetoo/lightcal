# Repository instructions

- Read `docs/README.md` and the relevant plan/decision before implementation.
- Keep all repository documentation in English.
- Use feature branches and PRs. Never push directly to `main` or `master`. Do not merge or publish without explicit owner permission.
- The app must use Rust and Slint without a web container. The owner accepted Slint; follow the integration/performance prototype gate in ADR 0001 and obtain owner approval before changing frameworks.
- Keep calendar logic independent of toolkit and platform APIs. Lightcal targets Windows and macOS 14+ (Intel and Apple Silicon); follow [ADR 0019](docs/decisions/0019-macos-support.md). macOS supports EventKit and direct read-only Outlook/Google connections, with Keychain credential storage; it does not expose the Lightcal local-event store or private ICS connections.
- Measure resource claims in release builds and label unmeasured targets honestly.
- Use temporary application data for tests. Never alter real autostart/registry/profile state during automated tests without an isolated, explicit test setup.
- Follow `docs/features.md`: read-only calendar connections (Outlook, Google Calendar, private ICS, and Windows calendars), local single-day events (`On this PC`), manual and signed automatic GitHub updates, optional startup/shortcut, and About links are approved scope (see ADRs [0002](docs/decisions/0002-direct-calendars-and-updates.md), [0008](docs/decisions/0008-google-calendar-and-platform-design.md), and [0017](docs/decisions/0017-local-events.md)). Additional event providers, remote event editing, and telemetry still require an owner decision.
- Preserve GPL-3.0-only licensing and third-party attribution. Never commit credentials or signing material.
- Check documentation with `bun scripts/check-docs.ts`. Write new scripts for Bun (TypeScript); use Python only when Bun cannot do the job, and PowerShell only for Windows-only tooling.

## Cloud agents

Cloud agents run on Linux. The image provides Rust stable with rustfmt and clippy, `just` and Bun on `PATH`, plus the system libraries used by Slint and native-tls.

`just check` is the Linux gate. It runs `bun scripts/check-docs.ts`, `bun test scripts`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --locked`. Windows-only code stays behind `cfg` and does not fail this gate. Use a temporary `LIGHTCAL_DATA_DIR` when a check writes preferences.

Headless UI proof, with no Microsoft Graph traffic: `cargo run -p lightcal --locked -- --render-snapshots <directory>`. `just run-demo` needs a display and uses sample events only. `just installer` builds a native macOS DMG or Windows NSIS setup on its respective host. The `release` recipe and Windows signing tools remain Windows-only; macOS distribution follows `docs/macos.md`.

The design lives on the owner's design canvas; [design/README.md](design/README.md) describes it and the mirrored board sources in `design/canvas/`. The old Penpot file is archived in `design/archive/` for history only.
