# Outlook, ICS, and update setup

## Local build configuration

Copy `.env.example` in the repository root to `.env.local`. Git ignores `.env.local` and `.env`. Only public configuration belongs there:

```dotenv
LIGHTCAL_MICROSOFT_CLIENT_ID=
LIGHTCAL_UPDATE_CERT_SHA256=
```

Cargo's build script reads those two keys with precedence: process environment, `.env.local`, then `.env`, including explicitly empty values. The MSIX packaging script uses the same precedence for package identity settings. Both files are ignored by Git. Values may be unquoted or enclosed in single/double quotes. Shell expansion, multiline values, and inline comments are not supported. Rebuild after changing a value. Bun also loads local environment files when running the release script from the repository root.

The public Microsoft Client ID is build configuration only (`LIGHTCAL_MICROSOFT_CLIENT_ID`, `.env.local`, or `.env`); Settings never displays or edits it. Obsolete per-user Client ID preferences are ignored. A failed/cancelled connection preserves the previous account credentials. Existing OAuth credentials retain their own client association, so changing a build default does not refresh an old token using a different registration.

Do not put client secrets, refresh/access tokens, account passwords, or private ICS URLs in environment files. The app needs no Microsoft client secret. Windows protects refresh tokens and feed URLs using current-user DPAPI in the application data directory (`credentials.dpapi`). They cannot be carried to another user/computer as ordinary settings. `LIGHTCAL_DATA_DIR` redirects app-owned data during development.

## Register Microsoft Graph access

The account used for Edge or Microsoft Store publishing is not automatically an Entra app registration. Prefer the business account if it has access to an Entra workforce directory and permission to register applications. No paid service purchase is part of this setup; if the portal reports a tenant/permission restriction, inspect that specific restriction before proceeding.

1. Sign in at [Microsoft Entra admin center](https://entra.microsoft.com/). Select the intended directory if the account belongs to several.
2. Open **Entra ID → App registrations → New registration**. Use a recognizable name, for example `Lightcal Desktop`.
3. Choose **Any Entra ID tenant + personal Microsoft accounts**, or the equivalent label **Accounts in any organizational directory and personal Microsoft accounts**. This matches the app's `common` authority and permits Outlook.com as well as work/school accounts, subject to their administrator policies.
4. Register the application and copy **Application (client) ID** from Overview. This is a public GUID, not a secret.
5. Under **Authentication → Add a platform**, choose **Mobile and desktop applications**. Register the system-browser redirect URI **`http://localhost/lightcalendar/oauth/microsoft`**. Use the native/public desktop platform, not Web or SPA. The app listens only on `127.0.0.1` with a temporary port and sends `http://localhost:<port>/lightcalendar/oauth/microsoft`. Microsoft's localhost native redirect matching ignores the port, but the path must match exactly. `lightcalendar` includes the `t`; the earlier `lighcalendar` spelling was a typo. Remove the old root-only redirect if it is no longer needed. Windows socket exclusivity is set before binding to prevent a competing reuse bind; the exact callback path, per-attempt state, and PKCE S256 are all required. The path is routing, not a secret or a replacement for PKCE.
6. Under **API permissions → Add a permission → Microsoft Graph → Delegated permissions**, add **`Calendars.Read`**. Do not add `Calendars.ReadWrite`, application-level calendar permissions, mail access, or a client secret. The app also requests `offline_access` during user consent so it can refresh its read-only access. Remove the default `User.Read` permission if it is not needed by another use of the registration.
7. If the tenant requires admin consent, have its administrator approve the delegated permission under the organization's policy. Otherwise the user consents in the normal browser flow. Do not turn on implicit token grants or password/device-code flows for this app.
8. Set `LIGHTCAL_MICROSOFT_CLIENT_ID` in build configuration and rebuild. Click **Connect Outlook**, choose the account in the browser, and inspect/accept the requested read-only calendar permission yourself.
9. Return to Lightcal, verify the **Outlook connected** status and account label, and check the calendars to display. The first connection selects none. Events come from Graph's `calendarView`, including server-expanded recurring instances and exceptions, with pagination. Only selected calendars' events are requested.

If sign-in fails, retain the error code shown by Microsoft; never share the callback URL, authorization code, or token. `AADSTS50011` usually indicates a redirect/platform mismatch. An organizational account may lack an Exchange Online mailbox or be restricted by policy even though sign-in works. Current implementation connects one Microsoft account at a time; reconnecting successfully replaces it. Disconnect removes local tokens but does not delete remote calendars or revoke the user's entire Microsoft account.

Google sign-in is deferred. This implementation does not use Google OAuth, CalDAV, Windows account settings, or a web container as a substitute for Graph.

## Add a private ICS subscription

In Settings → Calendars, enter a calendar name and the provider's direct HTTPS ICS address, then choose **Add ICS**. The address field is masked. The app validates the download and parses it before saving the protected subscription; check the resulting source to display its events. The URL is cleared from the input after successful connection. Remove deletes the local subscription and its selection.

Use a direct UTF-8 `VCALENDAR` response with HTTP 200. Embedded username/password credentials, plain HTTP, fragments, redirects, and non-calendar responses are rejected. A private feed URL grants access to that feed: use the provider's revoke/regenerate operation if it is exposed elsewhere.

Supported: folded lines, escaped text, DATE and DATE-TIME values, UTC/floating/IANA timezones, exclusive `DTEND`, `DURATION`, RRULE expansion, RDATE, EXDATE, cancelled instances, and moved `RECURRENCE-ID` overrides. Unsupported custom VTIMEZONE names, EXRULE, RDATE periods, and `RANGE=THISANDFUTURE` overrides report errors. The parser enforces download and recurrence limits instead of silently showing truncated data. Network/parse errors are shown in Calendars; a successful fixture test does not establish compatibility with every provider's export.

Sources refresh when selected, on month navigation and manually, plus every 15 minutes while the app runs. No push notification subscription or remote event-writing operation is used. Windows AppointmentStore remains an optional, off-by-default legacy switch.

## Enable signed update installation with SimplySign

Update checking and installation have separate settings. Both automatic settings are off by default. Enabling automatic installation also enables startup/daily checking. A manual **Install and restart** action is available when a stable update is found. Publication is not automatic.

1. Sign in to Certum SimplySign Desktop. Put the code-signing certificate's SHA-1 thumbprint in `SIGN_CERT_SHA1` in `.env.local`. Build with `just build-release`, then sign `target/dist/lightcal.exe` with `just sign-release`. That runs `signtool`. Keep signing keys in SimplySign.
2. Run `just certificate-info`. It first requires a valid Authenticode signature, then prints the public certificate's **SHA-256** fingerprint. A Windows certificate UI may label a SHA-1 value as a thumbprint: that is not the 64-hex-character value required here.
3. Put the SHA-256 value in `LIGHTCAL_UPDATE_CERT_SHA256` in `.env.local`. Rebuild so the certificate pin is embedded, and sign the final rebuilt executable again. Rebuilding invalidates a previous signature because it produces different bytes.
4. Run `just verify-release` and `just package-release`, or `just release` when SimplySign is signed in. `signtool` returning is not proof of signing; verification must pass after it returns.
5. When the owner authorizes release publication, tag a newer stable version that is contained in `main`. CI opens an unsigned draft; it does not publish. On the Windows PC where Certum SimplySign Desktop is signed in, run `just sign-draft vX.Y.Z`, then `just publish-release vX.Y.Z`. The updater installs the published asset named exactly `lightcal.exe`, not an installer or ZIP. CI artifacts and the unsigned draft files are review builds and must not be presented as signed releases. A build from a stable `vX.Y.Z` tag reports that version in About and in ProductVersion. Without `LIGHTCAL_UPDATE_CERT_SHA256`, **Install and restart** is replaced by **Download**, which opens the newer GitHub release instead of failing inside the app. See [Sign a draft release from Windows](windows-installer.md#sign-a-draft-release-from-windows).
6. Validate an update between two disposable, signed portable installations before enabling it for real users. Check wrong signer, modified payload, downgrade, locked/unwritable directory, failed restart, and recovery. Automated tests exercise temporary file replacement/rollback, never the owner's real installed executable.

Installation requires a valid Authenticode chain, exact pinned certificate SHA-256, a signed ProductVersion matching the newer release tag, and a trusted existing installation. It downloads to a bounded staging directory, starts a helper, waits for a readiness handshake and the main process to exit, then replaces the executable on the installation volume and restarts. An immediate restart failure restores the previous file; a previous EXE is retained for manual recovery. Result/error text is recorded beside the staged plan under the app data `updates` directory. This is Windows x64 portable-update support, not an MSIX/Store or elevated Program Files installer. Certificate renewal/rotation needs a reviewed transition release; no key rotation mechanism is implemented yet.

## Sources and validation limits

- [Register an application](https://learn.microsoft.com/en-us/entra/identity-platform/quickstart-register-app)
- [Microsoft desktop authorization code flow and PKCE](https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow)
- [Redirect URI matching](https://learn.microsoft.com/en-us/entra/identity-platform/reply-url)
- [Graph calendarView, recurring instances, and timezone behavior](https://learn.microsoft.com/en-us/graph/api/calendar-list-calendarview?view=graph-rest-1.0)
- [iCalendar RFC 5545](https://www.rfc-editor.org/rfc/rfc5545)

Live delegated sign-in, populated Outlook events, private ICS compatibility, and actual signed upgrade/restart require separately configured and verified credentials/certificates. A syntactically valid Client ID or certificate fingerprint alone does not establish successful authentication or signing trust. No credentials were invented or committed, and no release was pushed/published during implementation.

Local verification on 2026-10-30 used Rust 1.99.1: workspace tests, Clippy with warnings denied, formatting, Markdown checks, headless light/dark settings rendering, and optimized portable EXE/ZIP builds. Tests include recurrence exceptions/DST, callback state validation and loopback cancellation, unsigned-payload rejection, changed-download rejection, and isolated Windows atomic file replacement/rollback. ProductVersion/FileVersion are `0.1.0`; the unsigned optimized payload is approximately 13.46 MiB. That is executable file size, not process memory or idle CPU evidence. No live account, signing identity, or installed executable was modified by those tests.
