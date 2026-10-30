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

CI pull-request checks never build a release or publish. Releases are built only by the release workflow, which runs on a `v*.*.*` tag or a manual dispatch. Before building, a tag run verifies that the tagged commit is on `main` and that the tag matches the workspace version in `Cargo.toml`; the workflow then builds the locked release binary once, uploads an unsigned portable ZIP plus an unsigned NSIS setup as artifacts, and runs the isolated installer smoke test. Only a tag run publishes those files to a GitHub release, and it is the only job with write permission. A manual dispatch performs the same build and smoke test without publishing. Neither path calls SimpleSign, Authenticode signing, or `RequireSigned`, and neither reads signing keys. An empty `LIGHTCAL_MICROSOFT_CLIENT_ID` repository variable still builds; Outlook sign-in stays disabled until that public client id is set.

Run `just test-installer` after building release code. The test compiles the same NSIS script with `TEST_SETUP`: registry registration, shortcuts, application launch, and real-profile detection are omitted at compile time. It installs under a checked `target/installer-tests/<random>` path, verifies the executable hash, uninstalls, and checks that an unrelated sentinel file survives. No real account, app profile, autostart, shortcuts, or uninstall registry entries are modified. This validates payload installation and removal; real installed-app registration, signing, and SmartScreen behavior require separate owner-driven validation.

NSIS behavior is based on its [scripting reference](https://nsis.sourceforge.io/Docs/Chapter4.html) and [Modern UI 2 guide](https://nsis.sourceforge.io/Docs/Modern%20UI%202/Readme.html).
