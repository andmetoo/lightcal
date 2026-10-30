# Windows installer

Windows releases use NSIS setup executables alongside the portable application. No Store account, package identity, or Windows certificate import is needed to build an installer.

## Build

Run `just build-installer`, or `pwsh -NoProfile -File scripts/package-nsis.ps1`. The script builds locked release code and writes `target/nsis/lightcal-<version>-windows-x64-setup.exe` and a SHA-256 file. `just package-installer` skips Cargo compilation and packages the current `target/release/lightcal.exe`.

The script uses `LIGHTCAL_NSIS_COMPILER`, `makensis.exe` on PATH, or an installed NSIS compiler. Otherwise it downloads the pinned official portable NSIS 3.13 archive into ignored `target/tools`, verifies its published SHA-256, and extracts it there. `just prepare-nsis` performs that step separately. It does not install NSIS or change Windows configuration. The archive and hash come from the [official download page](https://sourceforge.net/projects/nsis/files/NSIS%203/3.13/nsis-3.13.zip/download).

Only the application, license, attribution, readme, NSIS notice, and uninstaller are installed. Root `.env` files, tokens, signing keys, and private calendar URLs are never bundled.

## Installed behavior

- Install for the current user under `%LOCALAPPDATA%\Programs\Lightcal`, with no elevation request. The destination can be changed in Setup.
- Add the normal Windows installed-app entry and a Start menu shortcut. A desktop shortcut is optional. Startup remains an application setting; Setup does not enable it.
- Ask the user to exit a running Lightcal instance before installation or removal. Never force-terminate the user's application.
- Starting Lightcal on the finish page is optional and unchecked by default.
- Remove only installer-owned files and shortcuts. Preserve app preferences, DPAPI credentials, and unrelated files. The uninstaller never recursively deletes a user-selected directory.
- Remove the startup entry only when it points exactly to this installation. A portable copy's startup entry is retained.

The existing trusted updater continues to require a signed release asset named exactly `lightcal.exe`. A setup executable has a different name and cannot be mistaken for the replacement application. Release metadata shown by Windows Installed Apps is refreshed when Setup is run; direct application-only updates do not rewrite that registry entry.

## Signing

`just release` builds and signs `target/dist/lightcal.exe` with SimpleSign Desktop, verifies it, creates the portable ZIP and NSIS setup from that signed payload, then signs and verifies the setup. `just sign-installer` packages an already signed staged application and signs the resulting setup. The signed route fails if the application or setup cannot pass Authenticode validation; it never labels an unsigned review build as signed.

Set `SIMPLESIGN_PATH` in the shell to the exact signer executable. Keys and certificate-service credentials remain in the signer, outside this repository. The generated uninstaller is not separately signed by this workflow. `LIGHTCAL_UPDATE_CERT_SHA256` remains the public certificate pin required for automatic application updates.

## CI and validation

GitHub Actions uses three workflows:

| Workflow | Runs on | Work |
| --- | --- | --- |
| `docs.yml` | Every pull request and `main` push | `Documentation checks` on Ubuntu |
| `rust.yml` | Pull requests that change more than documentation, every `main` push | Formatting and Clippy for `x86_64-pc-windows-msvc` on Ubuntu, unit tests on Windows, macOS library check |
| `release.yml` | `main` pushes that change more than documentation, `vX.Y.Z` tags, manual runs, and pull requests that change packaging | Locked release build, unsigned portable ZIP, unsigned NSIS setup, isolated installer smoke test |

Cargo builds are cached with `Swatinem/rust-cache`. Only `main` pushes save caches; pull requests and tags restore them, so caches never come from unreviewed branches. Debug information is disabled for CI test builds to keep caches small.

The release build job has read-only repository permissions. It does not call SimpleSign, Authenticode signing, or `RequireSigned`, and it never reads signing keys. An empty `LIGHTCAL_MICROSOFT_CLIENT_ID` repository variable still builds; Outlook sign-in stays disabled until that public client id is set.

### Release from a tag

1. Update the workspace version in `Cargo.toml` through a reviewed pull request and merge it into `main`.
2. Tag that `main` commit with `v` plus the same version, for example `git tag v0.2.0 && git push origin v0.2.0`.
3. CI rejects a tag that does not match the Cargo version or does not point at a `main` commit. Otherwise it builds the unsigned artifacts and creates a **draft** GitHub release with generated notes. A draft is invisible to the update checker.
4. Run `just release` locally from the same tag. Upload the signed `lightcal.exe`, signed setup, portable ZIP, and their checksums to the draft, for example with `gh release upload <tag> <files> --clobber`, and delete the unsigned ZIP.
5. The owner reviews and publishes the draft. CI never publishes a release.

Run `just test-installer` after building release code. The test compiles the same NSIS script with `TEST_SETUP`: registry registration, shortcuts, application launch, and real-profile detection are omitted at compile time. It installs under a checked `target/installer-tests/<random>` path, verifies the executable hash, uninstalls, and checks that an unrelated sentinel file survives. No real account, app profile, autostart, shortcuts, or uninstall registry entries are modified. This validates payload installation and removal; real installed-app registration, signing, and SmartScreen behavior require separate owner-driven validation.

NSIS behavior is based on its [scripting reference](https://nsis.sourceforge.io/Docs/Chapter4.html) and [Modern UI 2 guide](https://nsis.sourceforge.io/Docs/Modern%20UI%202/Readme.html).
