# ADR 0004: Compact calendar and account settings

Date: 2026-10-30. Status: accepted by the owner in chat.

The owner removed the expanded calendar view and requested clear Outlook connection status and actions. The public Microsoft application Client ID must not be shown as an editable user setting.

## Behavior

- The calendar has one compact layout (352 by 432 logical pixels); settings retain their own 780 by 620 layout. Expanded-view toggles, default-view preferences, and the nonfunctional New event button are removed. Old `default_view_mode` preferences are ignored without losing other preferences.
- The Microsoft Client ID comes from build configuration. The old editable preference is neither read nor serialized. Existing encrypted OAuth credentials retain the registration associated with their refresh token.
- Account status distinguishes checking, disconnected, browser sign-in, verified connection, expired or revoked authorization, and inability to verify the connection. Empty calendars do not imply a disconnected account. The connected label comes from the calendar owner's address when Graph supplies one.
- Sign-in, disconnect, and refresh actions follow account state and are disabled during an active connection or provider query. A failed or cancelled sign-in preserves the previous credentials. Network errors do not claim that saved credentials are valid or invalid.
- Outlook calendars are selected with checkboxes; the account is disconnected with the account-level action. Per-source removal is offered only for local ICS subscriptions. No action deletes remote calendars or events.

## Verification

Use temporary preferences and fixture credentials for tests. Cover account action eligibility, expired authorization versus transport failure, and migration of obsolete preferences. Render both themes for disconnected, connecting, connected, and reauthentication states. Live Outlook validation remains a separate owner-driven check; rendering fixtures must not access real account data.

Earlier expanded-view design exports remain historical design evidence and do not describe current application behavior.
