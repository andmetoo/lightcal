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

`just release` builds and signs `target/dist/lightcal.exe` with `signtool`, verifies it, creates the portable ZIP and NSIS setup from that signed payload, then signs and verifies the setup. `just sign-installer` packages an already signed staged application and signs the resulting setup. The signed route fails if the application or setup cannot pass Authenticode validation; it never labels an unsigned review build as signed.

Signing uses `scripts/sign-windows.ps1`: `signtool sign /sha1` with the `SIGN_CERT_SHA1` thumbprint from `.env`, and a Certum timestamp (`http://time.certum.pl`). Certum SimplySign Desktop must already be signed in, so the certificate is in the current-user store. The script does not launch SimplySign. Keys stay in that tool. Set `SIGNTOOL` only when `signtool.exe` is neither on `PATH` nor in the Windows SDK. The generated uninstaller is not separately signed by this workflow. `LIGHTCAL_UPDATE_CERT_SHA256` remains the public certificate pin required for automatic application updates.

## Sign a release from Windows

`signtool` can sign only on the maintainer's Windows PC, while Certum SimplySign Desktop is signed in. CI builds the unsigned application and NSIS setup. A release may be a draft, a prerelease, or public: neither its title nor its visibility determines whether it can be signed. If the release does not exist, CI creates a draft.

On that PC, from `main`, with [GitHub CLI](https://cli.github.com) signed in (`gh auth login`):

```powershell
just sign-draft vX.Y.Z
```

The command retains its old name for compatibility. It checks that `SIGN_CERT_SHA1` is in the current-user store. It downloads `lightcal.exe` and its SHA-256 from that release; older `lightcal-unsigned.exe` assets are also supported. If no application/checksum pair is attached, it downloads the Windows Actions artifact for the same tag and commit, including runs where the build succeeded but the release-upload job failed. It does not check out or recreate the tag. It checks SHA-256, Windows executable format, and ProductVersion before signing. It signs `target/dist/lightcal.exe`, verifies Authenticode, writes the portable ZIP, rebuilds NSIS from that signed application, signs the setup, and replaces these release assets:

- `lightcal.exe` and `lightcal.exe.sha256` (the updater asset)
- `lightcal-vX.Y.Z-windows-x64.zip` and its SHA-256 file
- `lightcal-X.Y.Z-windows-x64-setup.exe` and its SHA-256 file

These are also the filenames CI attaches before signing. An existing release retains its draft/prerelease/public status. If it does not exist, signing creates a draft only after the local files are signed and verified. A signing window that returns without a valid signature fails the command. Signing does not recompile the application: the certificate pin is the `LIGHTCAL_UPDATE_CERT_SHA256` repository variable from the tagged CI build. Changing the pin needs a new tag. NSIS must be rebuilt locally because signing the cloud setup alone would leave its embedded application unsigned.

`just publish-release vX.Y.Z` runs on Windows. It downloads the attached files into a fresh temporary staging directory under `target`, verifies all SHA-256 checksums, Authenticode on the application and setup, the application ProductVersion, and that the ZIP contains exactly the verified application. Ordinary filenames are not proof of signing. It then deletes legacy unsigned assets and explicitly publishes/promotes the release. Run it only when that signed `lightcal.exe` should become the stable download. The updater ignores drafts and prereleases and requires a valid signature and configured certificate pin before installing the published `lightcal.exe`.

### Recover an interrupted release

Do not delete or recreate a tag to change release visibility. Edit the GitHub release status instead. If the Windows build succeeded but attaching the release failed, run `just sign-draft vX.Y.Z` with the current scripts: it recovers the matching Actions artifact automatically. Artifacts expire after 14 days. A missing or moved tag cannot reuse a build from a different commit.

To rebuild an existing tag using the current workflow without moving that tag:

```powershell
gh workflow run windows-build.yml --ref main -f release_tag=vX.Y.Z
```

This checks out the tagged application source, builds it with the requested version, and attaches missing assets to the existing release. Existing assets are retained to avoid overwriting a signed build. Update your local `main` before using these commands. A failed old Actions run continues to use its old workflow when rerun; use the manual command above to apply the current workflow.

## CI, releases, and validation

The pull-request and manual Windows workflow installs the stable Rust toolchain, builds a locked release, and uploads an unsigned portable ZIP, `lightcal-unsigned.exe`, and an unsigned NSIS setup as job artifacts. It runs the isolated installer smoke test. The build job has read-only repository permissions and does not call SimpleSign, Authenticode signing, or `RequireSigned`.

A stable `vX.Y.Z` tag runs the same build once. The tagged commit must be contained in `main`. An existing release is accepted regardless of its title or draft/prerelease/public status. If none exists, CI creates a draft with the title `Lightcal vX.Y.Z`. It attaches `lightcal.exe`, `lightcal-vX.Y.Z-windows-x64.zip`, and `lightcal-X.Y.Z-windows-x64-setup.exe`, each with SHA-256. Existing assets are never overwritten by CI, and existing release metadata is preserved. The upload job uses the artifact without a second repository checkout. The executable version and Windows ProductVersion match the tag. Untagged builds keep the Cargo workspace version. Unsigned review builds do not install updates in place. Only the release-upload job receives `contents: write`; pull-request and build jobs remain read-only. CI never reads signing keys. New draft notes state that its files are unsigned; an existing release's notes are owner-managed until explicit publication/promotion.

An empty `LIGHTCAL_MICROSOFT_CLIENT_ID` repository variable still builds; Outlook sign-in stays disabled until that public client id is set.

Run `just test-installer` after building release code. The test compiles the same NSIS script with `TEST_SETUP`: registry registration, shortcuts, application launch, and real-profile detection are omitted at compile time. It installs under a checked `target/installer-tests/<random>` path, verifies the executable hash, uninstalls, and checks that an unrelated sentinel file survives. No real account, app profile, autostart, shortcuts, or uninstall registry entries are modified. This validates payload installation and removal; real installed-app registration, signing, and SmartScreen behavior require separate owner-driven validation.

NSIS behavior is based on its [scripting reference](https://nsis.sourceforge.io/Docs/Chapter4.html) and [Modern UI 2 guide](https://nsis.sourceforge.io/Docs/Modern%20UI%202/Readme.html).
