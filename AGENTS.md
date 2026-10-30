# Repository instructions

- Read `docs/README.md` and the relevant plan/decision before implementation.
- Keep all repository documentation in English.
- Use feature branches and PRs. Never push directly to `main` or `master`. Do not merge or publish without explicit owner permission.
- The app must use Rust and Slint without a web container. The owner accepted Slint; follow the integration/performance prototype gate in ADR 0001 and obtain owner approval before changing frameworks.
- Keep calendar logic independent of toolkit and platform APIs. Windows ships first; preserve a macOS port path.
- Measure resource claims in release builds and label unmeasured targets honestly.
- Use temporary application data for tests. Never alter real autostart/registry/profile state during automated tests without an isolated, explicit test setup.
- Follow `docs/features.md`: read-only Windows calendar access, manual GitHub update checking, optional startup/shortcut, and About links are approved scope. Direct account sign-in, other event providers, telemetry, and automatic update installation still require an owner decision.
- Preserve GPL-3.0-only licensing and third-party attribution. Never commit credentials or signing material.
- Check documentation with `python scripts/check_docs.py`; add documented Rust checks with the first Cargo workspace.

## Cloud agents

Cloud agents run on Linux. The image provides Rust stable with rustfmt and clippy, `just`, Bun, and `python` on `PATH`, plus the system libraries used by Slint and native-tls. `just` recipes invoke `python`; on Ubuntu that command comes from `python-is-python3`.

`just check` is the Linux gate. It runs documentation checks, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --locked`. Windows-only code stays behind `cfg` and does not fail this gate. Use a temporary `LIGHTCAL_DATA_DIR` when a check writes preferences.

Headless UI proof, with no Microsoft Graph traffic: `cargo run -p lightcal --locked -- --render-snapshots <directory>`. `just run-demo` needs a display and uses sample events only. NSIS, Authenticode, and SimpleSign recipes are Windows-only.

Penpot uses `.cursor/mcp.json`. Set `PENPOT_MCP_USER_TOKEN` to the MCP key from Penpot account → Integrations → MCP Server. Open the Lightcal file and choose File → MCP Server → Connect before asking an agent to read or change that design. Do not commit the key.
