# ADR 0002: Microsoft Graph, ICS, and signed automatic updates

Date: 2026-10-30. Status: accepted by the owner in chat.

Update, 2026-10-31: [ADR 0008](0008-google-calendar-and-platform-design.md) supersedes
the Google deferral below with planned read-only Google Calendar OAuth. The
Google provider is not implemented by the design change.

The owner approved private ICS subscriptions, direct Microsoft Graph calendar access for Outlook/Microsoft 365, and automatic update installation. Google integration is deferred by the owner's latest instruction. This supersedes the earlier Windows-only provider restriction and manual-install-only release scope. Windows calendar access is retained as an optional legacy source, not the default.

Microsoft authentication uses the system browser, a loopback redirect, PKCE S256, random state, and delegated Calendars.Read plus offline_access scopes. Public desktop client configuration belongs to the owner's Microsoft Entra app registration. No client secret is used. Refresh tokens and private ICS URLs must be protected with Windows DPAPI rather than ordinary preference JSON. Never log tokens, authorization codes, feed URLs, or event bodies. Disconnect removes local credentials; no remote calendar/account modification is permitted.

The owner's callback-path follow-up selects `http://localhost/lightcalendar/oauth/microsoft` in Entra, with an ephemeral port inserted at runtime. The `lighcalendar` spelling was a typo and is not accepted. Root/other paths are rejected, including aliases. The Windows listener requests exclusive address use before binding. This remains a loopback authorization-code flow with state/PKCE; it does not register a custom Windows URI scheme or treat the path as protection against code interception.

ICS is read-only over HTTPS with bounded response size/timeouts, recurring instances, timezone conversion, and exclusive all-day ends. Unsupported timezone/rule data must produce a visible error instead of a fabricated successful sync. Source selection remains explicit. No embedded browser, telemetry, or event-writing scope is approved.

The owner uses SimpleSign Desktop. Automatic installation accepts only official GitHub release assets, validates Authenticode against a build-configured SHA-256 signer-certificate pin, verifies the signed file version, stages a same-volume replacement with rollback, and restarts after the main process exits. It does not elevate or bypass Windows trust checks. Missing trust configuration disables installation with an actionable error. Signing keys remain outside the repository.

The owner has no Entra registration yet and explicitly accepted leaving the Client ID as an optional root-level build setting. `.env.example` documents the public configuration values; the ignored `.env` file and process environment variables supply them, and the Settings field can override the Client ID. [Setup instructions](../provider-setup.md) cover registration and the SimpleSign certificate fingerprint. No registration, credential, or certificate is fabricated.

Tests use temporary credentials/preferences and synthetic calendar/HTTP fixtures. Live Microsoft consent, certificate signing, and replacement of the owner's running installation require separate real-world validation and never run against real user state in automated tests. Changes remain local until the owner authorizes pushing.
