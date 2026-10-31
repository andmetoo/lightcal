# ADR 0008: Read-only Google Calendar and platform-specific design

Date: 2026-10-31. Status: accepted scope from the owner's design request in chat.

The owner requested a final design pass in `penpot_local`, Windows first and
macOS second, a clearer application icon, and future Google Calendar access
through OAuth with read-only permissions. This supersedes the Google deferral
in [ADR 0002](0002-direct-calendars-and-updates.md). It does not authorize Gmail
mail access, event editing, telemetry, or a UI framework change.

## Product behavior

- Keep the single 352 × 432 compact calendar. The owner rejected wasted space
  in both the sidebar design and the first horizontal-tab design. Following an
  independent UX review, settings use a fixed 448 × 360 panel with Calendar,
  Accounts and App tabs. About is a separate panel. The shipped settings window
  is unchanged until implementation. Expanded views remain historical.
- Remove the redundant-looking Current month chip and underline. Today is the
  single action that returns to and selects today's date; the date beside it is
  plain text. Use unambiguous two-letter weekday labels in shading controls.
- Replace theme thumbnails with a segmented control, remove repetitive helper
  sentences, and place controls close to labels. Disconnected providers belong
  in Add account, rather than permanently occupying large empty account cards.
- Windows has a static application icon and a dynamic day-of-month tray badge.
  After the numeral-only experiment, the owner's actual-shell review restored
  a calendar page with a slim header and contrasting date (see ADR 0010).
  Fit all dates at one font size per native icon size. Read the Windows shell
  theme independently of the app theme for contrast. Appearance offers color
  or monochrome calendars. Remove weekday/time tray modes
  from the new design; do not suggest replacing the Windows system clock.
- macOS retains the shared calendar and content hierarchy, with left-side window
  controls, “Open at login,” macOS shortcut notation and a separate static Dock
  icon. The owner's later sizing review selects the same calendar badge in the
  menu bar with Color/Mono choices, superseding date/weekday text labels.
  Do not duplicate the system clock.
- Google connects in the system browser. Present the read-only access boundary
  before sign-in. Distinguish disconnected, browser sign-in, cancellation,
  access refusal, connected, expired authorization, transport failure, no
  calendars and no selection. Never equate an empty list with sign-in failure.
- First connection selects no calendars. Checkboxes select what appears;
  disconnect operates on the account and never deletes remote events.
- Reuse the account-selection model for Microsoft and retain private ICS as a
  distinct subscription flow. Private calendar addresses are not public links.

## Implementation boundary

The original refinement produces Penpot boards, an interactive review prototype, vector
icon sources, raster design exports and documentation. It does not implement
Google OAuth or change the shipped application or installer. Provider work must
still use the toolkit-independent core and isolated fixture tests.

The subsequent [calendar status icon decision](0010-windows-calendar-tray-icon.md)
also updates the native Windows renderer and macOS size specification. It does not imply completion of the
other runtime UI or provider work described by these mockups.

For implementation, request only the Google Calendar permissions needed to
enumerate calendars and read events. The initial target is
`calendar.calendarlist.readonly` plus `calendar.events.readonly`; reevaluate
against the actual API calls before registration. Never request Gmail, write
or calendar-management scopes. Desktop OAuth registration, PKCE/state, local
credential protection, revocation and Google's verification requirements remain
implementation/release work, not a claim made by the mockups.

## Evidence and remaining validation

The owner subsequently rejected violet throughout both themes. The accepted
[Gray + Teal palette](../../design/palettes/README.md) supersedes the earlier
indigo styling in both native controls and active Penpot specimens.

The final independent UI review keeps the compact UX structure and standardizes
exact Onest typography, explicit Noto Sans JP weights, 28 px form controls,
shared exclusive selectors and optically centered icons. Today's default state
does not display a permanent keyboard-focus ring. Localized shortcut-error
specimens cover all four languages. See [the final review evidence](../../design/README.md).

The owner subsequently chose Onest 5.3 for Windows. The design pins the Fontsource package
to 5.3.1 and prepares native Onest 2.001 font files with OFL attribution;
[font provenance](../../design/fonts/onest/README.md) distinguishes those package
and font versions from Penpot's unreported hosted binary revision. This is a
design-font decision; the shipped Slint UI remains a separate implementation.

For macOS the owner then requested a separate system font/SF Pro. Use SF Pro
Text below 20 px and SF Pro Display at larger design sizes. Native runtime work
should use the platform system font, without redistributing Apple's font files.
[The prepared migration](../../design/fonts/macos-system.md) records the
required Penpot font import; it is not yet an applied or reviewed SF Pro claim.

The owner screenshots inform small-size icon legibility and removal of
irrelevant Windows controls. Penpot exports are inspected at their actual
logical dimensions, with geometry and text-bound checks. Exported icon assets
are examined on light and dark surfaces. Real Windows DPI/shell rendering and
real macOS/AppKit behavior remain separate release gates; no resource use or
platform validation result is inferred from a design export.

Primary references reviewed on 2026-10-31:

- [Google Calendar scopes](https://developers.google.com/workspace/calendar/api/auth)
- [Windows notification area and icon sizes](https://learn.microsoft.com/en-us/windows/win32/shell/notification-area)
- [Apple menu bar guidance](https://developer.apple.com/design/human-interface-guidelines/the-menu-bar)
