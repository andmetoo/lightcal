# Contributing to Lightcal

Lightcal is currently in planning. Start with [docs](docs/README.md) and discuss substantial scope or architecture changes in an issue before implementation.

## Workflow

1. Create a `feature/<short-description>` branch from the default branch, `main`.
2. Keep each PR focused on a reviewable user outcome. Write all documentation in English.
3. Run `python scripts/check_docs.py` with Python 3.9 or newer for documentation changes. Rust setup and checks will be documented when application code exists.
4. Open a PR with the behavior, validation evidence, and any unresolved limitations. Include screenshots for UI changes and performance measurements for background/runtime changes.
5. Wait for owner acceptance and required checks. Do not push directly to `master` or `main`, merge without permission, or publish releases without permission.

Application-changing PRs must update the version and changelog once the application exists. Documentation-only PRs do not need an app version bump. Platform-specific changes must state which platforms were actually tested.

## License and assets

By contributing, you agree to license your contributions under GPL-3.0-only. Retain third-party notices and identify the origin and license of imported material. Do not include credentials, private profile data, proprietary fonts, or assets without redistribution rights.

Follow the [community rules](CODE_OF_CONDUCT.md). Report security issues through [SECURITY.md](SECURITY.md).
