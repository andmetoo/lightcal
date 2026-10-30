# Open-source readiness

## Repository baseline

The owner renamed the default branch from `master` to `main` on 2026-10-29. Use `main` as the base for feature branches and pull requests. The local branch and remote HEAD have been updated to match.

Included now: GPLv3 license text, README, English plans, contributing and community rules, security-reporting guidance, issue forms, PR template, documentation CI, Dependabot configuration (`.github/dependabot.yml`, grouped weekly Cargo and GitHub Actions updates), and a tag-triggered release workflow. Only the release workflow publishes, only from a `v*.*.*` tag on `main`, and its binaries are unsigned; signing stays a local SimpleSign step. One known Dependabot limitation: the transitive `glib` 0.18 line enters `Cargo.lock` only through the Linux tray backend (`libappindicator`/`gtk`) and is never compiled into the Windows binary; its parents pin that line, so no resolvable fixed version exists yet. It is ignored in `.github/dependabot.yml` with that rationale, and the matching GitHub alert should be dismissed as not used until a parent update resolves it.

## Maintainer settings to apply on GitHub

These are required settings/checks, not settings already applied by this change:

- Protect the actual default branch with a ruleset: require a PR, block force pushes/deletion, and require passing `Documentation checks` once the workflow has run.
- Require owner review and resolution of review conversations; do not allow automation to merge. For external contributors, require a maintainer approval. GitHub does not let an author approve their own PR: if the owner authors a PR, explicit owner acceptance and a manual merge remain the policy, with any ruleset bypass controlled by the owner.
- Enable private vulnerability reporting before public binaries. Until then, the security policy must not imply that a private channel already exists.
- Enable appropriate dependency/security alerts when Cargo manifests exist. Consider secret scanning and push protection where available.
- Audit public history and future contributions for secrets, private paths, unlicensed assets, and generated personal data. The local planning inspection is not a complete remote history/security audit.

Protection and security settings must be verified on GitHub; documentation does not apply them automatically. Publishing a feature branch or opening a PR does not authorize merging or releasing.

## Contributions and licensing

All project code and documentation use GPL-3.0-only unless a file explicitly carries a compatible third-party license. Preserve attribution. Contributors retain copyright and contribute under the project license; no CLA is introduced.

Use original icons and layouts. Record third-party fonts, icons, snippets, and dependency licenses when introduced. Avoid bundling proprietary system fonts; use system fonts at runtime where supported.

### Third-party notices and bundled assets

- **Inter Font**: `crates/lightcal/assets/fonts/Inter.ttf` is licensed under the SIL Open Font License, Version 1.1 (Copyright 2016-2024 The Inter Project Authors, Rasmus Andersson). License text is preserved in `crates/lightcal/assets/fonts/OFL.txt`. Used for high-quality dynamic tray icon and application icon rasterization.

Commit `Cargo.lock` for reproducible application dependencies. Pin the Rust toolchain at implementation setup and pin CI actions to reviewed commit SHAs. Add dependency advisories/license checks when dependencies exist; do not claim a clean audit for an empty manifest set.

## Before distributing binaries

1. Freeze app/shortcut/bundle identifiers and document supported OS versions and architectures.
2. Review all dependency licenses and produce third-party notices from the locked dependency set.
3. Provide corresponding source and necessary build instructions with each GPL binary release. Preserve exact source revisions and any modifications to bundled dependencies.
4. Decide signing/installer approach, key custody, and recovery with the owner. Never commit signing keys, certificates, tokens, or passwords.
5. Publish a privacy statement matching actual behavior: manual update checks contact GitHub; About links open GitHub/koona.app in the browser on user action; read-only calendar access uses the Windows provider. No telemetry is planned. Document exactly which permissions and connection metadata are involved.
6. Build from a reviewed source revision, test on a clean machine, attach checksums and validation evidence, and obtain owner permission to publish.

Automatic update installation, funding infrastructure, and additional contributor bureaucracy are deferred until needed. Manual update checking is required scope.
