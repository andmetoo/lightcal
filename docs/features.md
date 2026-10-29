# Required features

Owner requirements recorded on 2026-10-29. These extend the original offline-calendar scope; they are planned, not implemented. The owner selected calendars already connected in Windows as the event source, subject to an API feasibility probe.

| Feature | Expected behavior and acceptance |
| --- | --- |
| Weekday display | Show weekday headings and today's weekday/date in both calendar modes and the tray tooltip. Include the weekday in the selected-date description. An optional weekday abbreviation in the tray icon is subject to small-icon legibility testing; it is not a promise of arbitrary text beside the icon. |
| Configurable gray weekdays | Let the user choose any subset of Monday-Sunday, including none/all. Default to Saturday and Sunday. Apply a subtle neutral background to those columns in both themes; the dates remain enabled and readable. Keep this separate from the first-weekday setting. |
| Start with Windows | Off by default; opt-in per-user login startup opens hidden in the tray. Persist the setting, report registration failures, and remove only Lightcal's startup entry when disabled. |
| Check for updates | Provide a manual check in Settings/About. Compare the installed version with the latest stable GitHub release, show the result, and offer the verified release page in the browser. Handle offline, timeout, rate-limit, malformed-response, and no-release states. Check does not download, install, or restart the app. Optional scheduled checking remains off unless separately enabled. |
| Optional global shortcut | Unset by default. Allow capture/change/clear of a global key combination to toggle the compact calendar. Report conflicts without stealing another application's shortcut, preserve the previous working binding on registration failure, and release it on Quit. |
| Calendar selection | List only calendars actually exposed by the approved Windows provider, with display name and a local source/account label where available. Persist checked calendar IDs; show events only from checked calendars. First connection selects none until the user chooses. Support no calendars, no selection, access denied, unavailable provider, stale data, and calendar removal. Read-only; no event/account modification. |
| Current-month emphasis | When displayed year/month equals local today, underline the month title with an accent and show a soft `Current month` badge. Remove both when browsing another month/year. Today returns to the current date/month. This is independent of the today outline and selected-day fill; recompute after local midnight/timezone changes. |
| Event dots | For dates containing events from selected calendars, show up to three small dots below the day number, using calendar colors when available. More events retain three dots; an accessible label/tooltip gives the exact count. No selected-calendar events means no dots. Count all-day, recurring, timezone-converted, and multi-day occurrences correctly; use exclusive end boundaries. Preserve clear today/selection states. |
| About links | About includes app version, GPL license, [GitHub repository](https://github.com/andmetoo/lightcal), and [koona.app](https://koona.app). Open links only on user action; no embedded browser, remote fonts, automatic website fetches, or analytics. |
| Languages | Default to English (`en`) on a fresh installation, even on a non-English OS. Settings offers English, Русский (`ru`), Español (`es`), and 日本語 (`ja`). Persist the selection, update UI/date/accessible text immediately without restarting, and use English if a translation or saved language is unavailable. Do not translate event titles, user calendar names, proper names, URLs, or keyboard combinations. |

## Windows calendar feasibility gate

Use a small disposable Rust probe before choosing package/installer format. Demonstrate calendar enumeration and read-only occurrence queries in the intended distribution model, on supported Windows builds with representative configured accounts. Test missing data, revoked access, and the difference between Windows appointment-store calendars and calendars shown only inside Outlook or another application.

Record package-identity/capability requirements and whether a usable store exists on a clean Windows installation. OS account configuration alone is not evidence that a calendar is exposed to this API. Do not silently replace this provider with direct Google/Microsoft sign-in or ICS subscriptions; bring any blocker to the owner for a scope decision.

Relevant primary sources, reviewed on 2026-10-29:

- [AppointmentManager.RequestStoreAsync](https://learn.microsoft.com/en-us/uwp/api/windows.applicationmodel.appointments.appointmentmanager.requeststoreasync)
- [Windows app capability declarations](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/app-capability-declarations)
- [AppointmentStore Rust bindings](https://microsoft.github.io/windows-docs-rs/doc/windows/ApplicationModel/Appointments/struct.AppointmentStore.html)

These identify an API family to investigate; they do not establish compatibility with this app or distribution model.

## UI and release implications

Settings cover Appearance, General, Calendars, and About. General contains startup, shortcut, and language controls. Appearance includes weekday visibility, gray-weekday selection, theme, and week numbering. Calendars shows permission/provider state and checked sources. About contains update checking and the requested links.

All four languages are first-release requirements. English is the source catalog; use stable message keys, complete translated sentences and locale-aware pluralization/date formatting instead of concatenated fragments. Japanese requires working system font fallback without bundled proprietary fonts or downloaded font assets. Layout, focus, screen-reader text, event counts, update/error messages, and tray/menu labels must be checked in each language. Language changes must preserve selected dates, calendar sources, shortcut, timezone, and weekday preferences.

The current redesign is in Penpot through MCP: calendars, settings, provider/shortcut/update states, localized examples, and a control guide. Earlier Lunacy concepts are historical. Provider names/events are illustrative, not an assertion that live Windows calendars were accessed. See [the design notes](../design/README.md) for actual completion and validation status.

The complete first release includes these features. The earliest prototype may use synthetic events to validate layout, but cannot claim the calendar integration works or silently drop it from the release.
