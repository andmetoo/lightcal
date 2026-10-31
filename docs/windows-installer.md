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

Signing uses `scripts/sign-windows.ps1`: `signtool sign /sha1` with the `SIGN_CERT_SHA1` thumbprint from `.env.local`, and a Certum timestamp (`http://time.certum.pl`). Certum SimplySign Desktop must already be signed in, so the certificate is in the current-user store. The script does not launch SimplySign. Keys stay in that tool. Set `SIGNTOOL` only when `signtool.exe` is neither on `PATH` nor in the Windows SDK. The generated uninstaller is not separately signed by this workflow. `LIGHTCAL_UPDATE_CERT_SHA256` remains the public certificate pin required for automatic application updates.

## Sign a draft release from Windows

`signtool` can sign only on the maintainer's Windows PC, and only while Certum SimplySign Desktop is signed in. A stable tag does not publish a release. It opens a GitHub **draft** that holds the unsigned CI build. The maintainer downloads that draft, signs it locally, and uploads the signed files back to the same draft.

On that PC, from `main`, with [GitHub CLI](https://cli.github.com) signed in (`gh auth login`):

```powershell
just sign-draft vX.Y.Z
```

`just sign-draft vX.Y.Z` checks that `SIGN_CERT_SHA1` is in the current-user store. It does not check out the tag. It downloads `lightcal-unsigned.exe` from that draft, checks its SHA-256, and refuses a file that is not a Windows executable. It then signs `target/dist/lightcal.exe` with `signtool`, verifies Authenticode, writes the portable ZIP, builds the NSIS setup from that signed application, signs the setup, and uploads:

- `lightcal.exe` and `lightcal.exe.sha256` (the updater asset; this name is never used for the unsigned draft file)
- `lightcal-vX.Y.Z-windows-x64.zip` and its SHA-256 file
- `lightcal-X.Y.Z-windows-x64-setup.exe` and its SHA-256 file

The release stays a draft. A signing window that returns without a valid signature fails the command. Signing does not recompile: the certificate pin is the `LIGHTCAL_UPDATE_CERT_SHA256` repository variable from the tagged CI build. Changing the pin needs a new tag. `just publish-release vX.Y.Z` deletes the unsigned draft files and marks the release public. Run it only when that signed `lightcal.exe` should become the stable download. The updater ignores drafts and installs only a published asset named exactly `lightcal.exe`.

## CI, draft releases, and validation

The pull-request and manual Windows workflow installs the stable Rust toolchain, builds a locked release, and uploads an unsigned portable ZIP, `lightcal-unsigned.exe`, and an unsigned NSIS setup as job artifacts. It runs the isolated installer smoke test. The build job has read-only repository permissions and does not call SimpleSign, Authenticode signing, or `RequireSigned`.

A stable `vX.Y.Z` tag runs the same build once and opens an unsigned GitHub draft. The job fails unless the tagged commit is contained in `main`, and it fails again if that tag is already a public release, including an existing unsigned prerelease. Already published tags are left unchanged. The executable version and Windows ProductVersion match that tag. Untagged builds keep the Cargo workspace version. The draft files are `lightcal-unsigned.exe`, `lightcal-<tag>-windows-x64-unsigned.zip`, and `lightcal-<tag>-windows-x64-unsigned-setup.exe`, each with a SHA-256 file. The workflow never attaches an unsigned file named `lightcal.exe`. Unsigned review builds do not install updates in place. The macOS disk image is not attached. Only the draft job receives `contents: write`; pull-request and build jobs remain read-only. The workflow never reads signing keys or presents these review files as signed production binaries.

An empty `LIGHTCAL_MICROSOFT_CLIENT_ID` repository variable still builds; Outlook sign-in stays disabled until that public client id is set.

Run `just test-installer` after building release code. The test compiles the same NSIS script with `TEST_SETUP`: registry registration, shortcuts, application launch, and real-profile detection are omitted at compile time. It installs under a checked `target/installer-tests/<random>` path, verifies the executable hash, uninstalls, and checks that an unrelated sentinel file survives. No real account, app profile, autostart, shortcuts, or uninstall registry entries are modified. This validates payload installation and removal; real installed-app registration, signing, and SmartScreen behavior require separate owner-driven validation.

NSIS behavior is based on its [scripting reference](https://nsis.sourceforge.io/Docs/Chapter4.html) and [Modern UI 2 guide](https://nsis.sourceforge.io/Docs/Modern%20UI%202/Readme.html).
