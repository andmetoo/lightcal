# Implementation roadmap

Each milestone ends with a reviewable PR and owner acceptance. Estimates are planning time boxes, not delivery promises. Do not start application implementation in the planning PR.

## 0. Agree on the plan and public repository foundation

Deliver this documentation, GPL license, contribution/security policies, issue forms, PR template, and documentation checks.

Acceptance:

1. Documentation links and whitespace checks pass from a fresh checkout.
2. The accepted Slint decision is explicit about custom drawing and unmeasured performance.
3. Owner reviews scope and repository settings in [open-source readiness](open-source.md).
4. Changes remain on a feature branch and enter the default branch only through a PR approved by the owner.

## 1. Prove the platform and framework (2-3 days)

Build the smallest useful Windows application: tray icon, month view, larger taskbar window, explicit Quit. Resolve exact crate versions from current stable releases, pin the Rust toolchain, commit `Cargo.lock`, and record licenses. Establish Rust CI for format, Clippy, tests, and Windows release compilation. Add a macOS compile lane and arrange a real launch probe.

Acceptance:

1. Tray click opens and toggles a compact calendar; Escape and focus loss hide it.
2. Taskbar shortcut opens a larger calendar. Relaunch forwards activation to one process without duplicate tray icons. Hide and restart preserve a valid lifecycle.
3. Normal close hides to tray; Quit exits. Tray recovery after Explorer restart works or a blocking issue is recorded before full UI implementation.
4. Mixed-DPI positioning, first keyboard navigation, and Narrator discoverability are demonstrated.
5. Release-build metrics are recorded using [validation](validation.md); pass targets or obtain an explicit adjustment backed by measurements.
6. A macOS launch/menu-bar probe is recorded; unavailable hardware is an open gate, never a claimed pass.
7. A separate Windows calendar probe demonstrates read-only enumeration and occurrence queries for the intended packaging model, or records a blocker for owner review before package/distribution decisions.

Gate: record passing integration/performance evidence in ADR 0001 before building the full UI; bring blockers to the owner if the accepted decision needs revision.

## 2. Approve the visual design (about 1 day)

Use Lunacy for editable compact and expanded views. Commit portable design sources and preview exports under `design/`; document asset licenses.

Acceptance:

1. Both views have light/dark designs, readable selected/today/weekend states, keyboard focus, month navigation, optional week numbers, and tray menu.
2. Review at 100%, 150%, and 200% scaling; include long localized month/day labels and Windows high-contrast behavior.
3. Expanded space improves readability and navigation. Do not invent empty agenda content to fill the window.
4. Owner accepts the previews and interaction behavior before visual implementation.
5. Apply [the required features](features.md): weekday/date text, selectable gray weekdays, current-month label, event dots, and Settings/Calendars/About designs including the two requested links.
6. General includes a language selector: English by default, Russian, Spanish, and Japanese. Check layout with long Spanish/Russian strings and Japanese glyphs; do not assume the English layout proves localized usability.

## 3. Complete the offline calendar (2-4 days)

Implement the approved UI, month/year transitions, leap years, configurable first weekday and gray weekdays, weekday/date display, current-month emphasis, optional ISO week numbers, themes, date rollover, and keyboard navigation. Validate dot layout with explicitly synthetic event data until the provider milestone.

Acceptance:

1. Today, selected date, and displayed month have distinct behavior; returning to today restores the current local date.
2. Arrow keys move selection; Page Up/Down navigate months; Tab reaches controls; Escape dismisses the compact view. Accessible labels describe dates and selection.
3. Deterministic tests cover leap days, year boundaries, ISO week-year boundaries, and 4/5/6-row month grids.
4. Suspend/resume and date/timezone changes refresh today without a continuous render loop.
5. Shared selection remains consistent when switching entry points; no clipped controls at supported scaling.

## 4. Preferences and reliable background behavior (1-2 days)

Implement theme/weekday/week-number settings, optional autostart/global shortcut, safe persistence, tray recovery, and About with GitHub/koona.app links. Complete read-only Windows calendar selection, real event dots, and manual update checking before release.

Acceptance:

1. Settings survive process restart, and corrupt/newer files remain recoverable without silent overwrite.
2. Autostart is off initially; enabling/disabling it affects only Lightcal's entry and is verified in an isolated user profile.
3. Missing monitor, changed scaling, Explorer restart, and failed settings writes leave the app usable.
4. Performance targets still hold after features are enabled.
5. Shortcut capture/change/clear handles conflicts and preserves the last working binding on failure. About opens the correct external links only on user action.
6. Calendar selection persists by ID; unselected calendars contribute no dots. Access denial, missing/deleted calendars, recurrence, all-day/multi-day events, timezone conversion, and stale provider data are tested with controlled fixtures and a real-provider smoke check.
7. Manual update checking correctly handles newer/current versions, no stable release, offline, timeout, rate-limit, and invalid responses; it never installs or restarts the app.
8. Ship complete English/Russian/Spanish/Japanese UI catalogs. A fresh non-English OS still starts in English; language changes apply immediately and survive restart. Verify date names/formatting, pluralized counts, errors, menus, shortcuts, glyph coverage, and accessibility in all four languages. Preserve calendar/date/weekday settings across language changes and fall back safely on an invalid saved locale or missing message.

## 5. Prepare the first Windows release (2-3 days, excluding certificate lead time)

Choose portable archive and/or installer, stable identifiers, versioning, and signing with the owner. Start signing research early; purchase and publication require permission. Automatic updates are not part of the initial scope.

Acceptance:

1. A clean standard-user machine can install/run/uninstall without unexplained elevation; retained or removed preferences match documented behavior.
2. Package includes license, notices, checksums, and an exact source tag/archive with build instructions for GPL compliance.
3. Record signature/SmartScreen status honestly; an unsigned preview is clearly labeled if the owner approves distributing one.
4. Complete functional, accessibility, and performance evidence is attached to the release candidate.
5. Owner approves the release before any public binary upload or tag publication.

## 6. macOS product port (after the Windows baseline)

Complete menu-bar activation, appearance, preferences location, accessibility, packaging, signing, and notarization on macOS. Re-run Windows checks throughout. Select macOS versions/architectures from actual testing and publish a separate support matrix.
