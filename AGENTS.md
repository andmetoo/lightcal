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
