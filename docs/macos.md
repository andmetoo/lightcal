# macOS build and distribution

Lightcal supports macOS 14+ on Apple Silicon and Intel alongside Windows.
The scope is recorded in [ADR 0019](decisions/0019-macos-support.md).

## Development

Install Xcode command-line tools, Rust stable and Bun. Run `cargo run -p lightcal
--locked` from a terminal for UI development. Use a temporary `LIGHTCAL_DATA_DIR`
for tests; it disables real login-item and shortcut registration. Calendar access
must be tested explicitly from an application bundle, because privacy consent is
associated with application identity. Do not use a raw Cargo invocation as proof
that the signed application's permission flow works.
Binary tests also supply explicit temporary instance-lock directories and isolated
shortcut managers, so their safety does not depend on a library's `cfg(test)` or
an inherited environment override. Temporary startup-manager paths cannot register
login items.

For an unsigned universal review bundle:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
bun scripts/package-macos.ts
```

Run `just installer` to build the universal app plus a local unsigned disk image
at `target/macos/lightcal-1.2.2-macos-universal.dmg` and its SHA-256 file. This uses
Apple's native packaging tools and does not require PowerShell or NSIS. The image
contains `Lightcal.app` and an Applications link for drag-and-drop installation.
`just installer --help` lists macOS options; Windows-only flags such as
`-SkipBuild` are rejected before building. Signed distribution still requires the
release configuration below; `just installer --release` prepares those artifacts
without publishing them.

The bundle is written to `target/macos/Lightcal.app`. Unsigned review builds have
no updater framework and are not public release artifacts. Preferences are stored
in `~/Library/Application Support/Lightcal`; `LIGHTCAL_DATA_DIR` overrides it.
The Mac app does not load or create `local-events.json` or Windows credentials.
It supports read-only direct Outlook and Google connections as well as EventKit.
OAuth credentials are stored in the macOS Keychain, never ordinary JSON files.
Direct sign-in is disabled under `LIGHTCAL_DATA_DIR` and in automated tests;
launch the installed app normally for an explicit live sign-in. See
[provider setup](provider-setup.md#macos-direct-accounts) for client configuration.
The isolated sample app cannot be used as proof of live authentication.
All-day EventKit events cross the native bridge as civil start/end dates, with
an exclusive end date; UTC-midnight source dates and floating default-zone dates
are preserved across display zones and DST. Timed events still use local instants.
An explicit event timezone wins when it and the display timezone both see midnight
on different dates. The open agenda refreshes at minute boundaries without querying
providers, updating countdowns, Now badges and past-event styling. Recorded F1–F12
bindings use their native Carbon keycodes.
Connection, sign-in and ICS diagnostics are translated at presentation time,
including cached failures and partial-read counts after a language change. Raw
provider diagnostics remain available internally; unknown diagnostics use a
localized fallback rather than English text. HTTP status codes are retained.
Open-at-login failures use localized macOS Login Items guidance. Both native
calendar headings follow the app language, and Settings uses the compact neutral
close control on macOS rather than the Windows caption button.
Removing a `mac:` source turns off the native connection, just as `win:` does on
Windows, without changing direct-account credentials.
Packaging explicitly forwards the macOS 14 deployment target to Cargo and verifies
both executable slices with `vtool`, separately from the bundle's minimum-OS plist.
After compiling, packaging recreates the application bundle before assembling it;
review builds cannot retain an older release's Sparkle framework or code signature.

For placement diagnostics, set `LIGHTCAL_DEBUG_PLACEMENT=1` explicitly. It prints
at most 32 geometry records per process and includes no calendar or account data.
AppKit can expose an off-screen placeholder for a menu item; the app rejects it
and uses a top-right fallback until a valid menu item or menu-bar click is known.
`bun test scripts` runs the native geometry regression on macOS and skips it on
other platforms.

For a nonvisual lifecycle check on a logged-in development Mac:

```sh
cargo build -p lightcal --locked --features native-smoke
bun scripts/smoke-macos.ts
```

This opt-in build sends reopen AppleEvents to its own running Slint/AppKit event
loop. It checks hidden startup, repeated reopening, Command-comma, Settings restoration, anchored
placement and cancellation of an interrupted Settings crossfade using native state,
without screen capture or computer vision. The runner supplies synthetic events and
disposable data, enforces a timeout, and removes its data afterward. The app refuses
the mode without synthetic/background flags and an existing child of the system
temporary directory. Normal builds exclude the native test entry points. This
does not replace manual Dock, multi-display, permission or accessibility checks.

## Tag workflow configuration

The macOS workflow checks pull requests on the macOS 15 runner; the deployment
target remains macOS 14. GitHub retired its macOS 14 runner on November 2, 2026
([runner retirement](https://github.com/actions/runner-images/issues/13518)).
Stable `vX.Y.Z` tags reachable from main
also build universal release assets in the protected `macos-release` environment.
Configure environment protection for owner approval before exposing signing keys.

Environment variables:

| Variable | Value |
| --- | --- |
| `APPLE_SIGNING_IDENTITY` | Developer ID Application signing identity |
| `SPARKLE_PUBLIC_KEY` | Sparkle Ed25519 public key |
| `LIGHTCAL_MICROSOFT_CLIENT_ID` | Existing Microsoft public desktop client ID |
| `LIGHTCAL_GOOGLE_CLIENT_ID` | Existing Google Desktop OAuth client ID |
| `SPARKLE_SHA256` | Independently verified SHA-256 for the pinned Sparkle 2.8.1 distribution archive |

Environment secrets:

| Secret | Value |
| --- | --- |
| `APPLE_CERTIFICATE_P12` | Base64-encoded Developer ID certificate and private key export |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the P12 export |
| `APPLE_ID` | Apple developer account used for notarization |
| `APPLE_TEAM_ID` | Apple developer team identifier |
| `APPLE_APP_PASSWORD` | App-specific password for notarization |
| `LIGHTCAL_GOOGLE_CLIENT_SECRET` | Google Desktop OAuth client secret (non-confidential desktop configuration) |
| `SPARKLE_PRIVATE_KEY` | Sparkle private signing key in `sign_update --ed-key-file` format |

The workflow imports credentials into a temporary keychain, signs nested Sparkle
components and the application with hardened runtime, notarizes and staples the
app and DMG, signs the update ZIP, and attaches assets to a draft release.
Cleanup runs even after a failure. Missing configuration fails the release; it
never substitutes unsigned release assets or overwrites a public release.
ZIP and DMG notarization both pass the ephemeral signing keychain explicitly.

Sparkle alone schedules automatic macOS update checks. The shared startup/daily
Rust checks and automatic-check/download toggles never call its interactive
update command; Check for Updates remains an explicit user action. UI refreshes
write Sparkle preferences only when their values change, preserving its cycle.
See [Sparkle's programmatic setup guidance](https://sparkle-project.org/documentation/programmatic-setup/).

Sparkle reads the macOS appcast from the latest stable GitHub release's
`lightcal-macos-appcast.xml` asset. Every future stable release must include this
asset and its referenced signed ZIP, including releases prepared by Windows jobs.
Keep signing private keys outside the repository. Publication is a separate owner
action after both platform assets are ready.

## Required desktop acceptance

Use synthetic events and temporary data for automated tests. Real permission and
calendar checks are deliberate manual, read-only checks.

- Verify Dock activation, second launch, menu bar toggle, right-click menu, Escape,
  outside-click dismissal, Settings focus, Quit and login startup.
- Exercise all ten languages, weekday modes and month, color/mono artwork, Retina,
  multiple displays, Spaces, full-screen apps, Reduce Motion and VoiceOver.
- Connect Google and Outlook in the browser, choose calendars, restart, refresh tokens and disconnect each provider independently. Verify local events remain absent.
- Connect EventKit calendars from the signed bundle; test denial, later grant, revocation,
  empty stores, deselection, recurring and all-day events, DST, wake, timezone
  changes and obsolete navigation results. The native account uses a calendar icon
  and localized macOS Full Access guidance; Open settings targets the Calendars
  privacy page. The Accounts footer describes read-only access without offering
  local event editing. System permission denial must not block direct Google or
  Outlook calendars.
- Install a notarized build from a quarantined DMG on both architectures. Update
  an older signed installation with valid and invalid update signatures; verify
  cancellation, offline behavior and unchanged preferences.
- Record release-build hidden CPU and memory, visible memory, activation latency,
  package size and repeated-open growth. All macOS resource targets are currently
  unmeasured, and public release is not yet validated.
