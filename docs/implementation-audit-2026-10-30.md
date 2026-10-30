# Implementation audit — October 30, 2026

Scope: local review and fixes on `feature/slint-platform-prototype`. Nothing was pushed, published, or merged. Penpot MCP was not exposed among the tools available during this review; repository design sources and headless Slint renders were used instead. The owner's two attached screenshots are visual evidence, not implementation instructions embedded in a document.

Follow-up scope change: the owner subsequently approved direct ICS, replaced the proposed Google option with Microsoft Graph, and requested signed automatic installation using SimpleSign Desktop. [ADR 0002](decisions/0002-direct-calendars-and-updates.md) and [provider/update setup](provider-setup.md) describe the resulting implementation and current live-validation gates. The Windows-only/manual-install assessment and artifact hashes below are historical evidence from the first audit pass, not the current provider scope or final rebuilt archive identity.

## Assessment

The prototype has a useful foundation: calendar arithmetic and selected/today/month state live in a separate Rust crate; preferences have recovery and temporary-directory tests; Slint runs without a web container; Windows tray, shortcut, startup, and single-instance adapters exist. However, passing unit tests did not establish feature completeness: several controls were connected to placeholders or mismatched values. `main.rs` combines substantial UI synchronization, controller behavior, and platform orchestration and would benefit from extraction after the defects are stabilized.

## Fixed during this review

| Defect | Change |
| --- | --- |
| Only date icon mode worked | Slint sent `weekday-date` / `time-date`, while Rust recognized different values. Accept the UI values and test mode selection through preferences save/reload. |
| Settings always showed 29 / THU / 22:01 | Mode labels and sidebar date now use calendar state and local time. The explanatory translation strings still contain illustrative examples. |
| Icon previews disagreed with real icons and had malformed caps | Remove the duplicate Slint icon drawing and use the actual Rust rasterizer for both palettes. Render all three modes in both themes for inspection. |
| Hour/timezone changes could miss a clock update | Compare absolute minute stamps instead of only the minute component and refresh preview labels each minute. Midnight still updates the calendar state. |
| Month navigation retained the original event query | Query the visible month on previous/next/Today and selection crossings; refresh/retry use the visible month. Ignore results belonging to an older request or a different displayed month. |
| Fixed six-row calendar could exceed the provider range | Extend the query through two weeks after the displayed month. |
| Event query failures could look like successful empty results | Preserve provider failure status and display the provider error. Do not replace it with an account-empty explanation. |
| All-day events could move to the following date | Remove the arbitrary 12-hour shift before local date conversion. Preserve sub-minute timed-event ends when deciding exclusive date boundaries. Real recurring/DST/all-day fixtures remain a release gate. |
| Update button never used the network | Add a Windows native HTTPS request on a worker thread, stable-release response evaluation, timeout handling, and opt-in startup/daily checking. No download/install. |
| Release-link validation accepted a misleading path prefix | Require the repository releases path boundary. |
| Signed release could package an unsigned executable | Require valid Authenticode verification after the signing step and before signed-release packaging; a missing signer fails. Correct checksums so a signed staged file does not overwrite the checksum for a different release binary. |
| Archive version was hard-coded | Derive it from the Cargo workspace version. |
| Default development recipes silently used sample calendars | Default recipes use the Windows provider; sample data has an explicit demo recipe. |
| CI had no downloadable release-build artifact | Add a Windows release-build workflow producing an unsigned ZIP for review, with license/notices/readme and executable checksum. No publishing. |

## Observed calendar result

The explicit read-only Windows probe returned **one Microsoft-account calendar and zero occurrences in the queried range**. This demonstrates store enumeration on this machine, not synchronization of the owner's expected accounts/events, recurring expansion, or provider compatibility on a clean Windows installation. No calendar data, accounts, registry startup entries, or real preference files were changed by the probe.

The first real connection selects no calendars by design. The user must check sources in Settings. A Windows account, Outlook account, and an AppointmentStore calendar are not interchangeable guarantees. Direct Google sign-in and remote ICS subscriptions remain outside the approved provider scope pending the owner's answer. Do not infer that adding an Outlook/Gmail account will necessarily populate this store.

The live provider probe is now explicitly ignored in ordinary tests. Run `just probe-calendars` when a read-only live check is intended.

## Validation and remaining work

Final validation on Windows with Rust 1.99.1:

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo test --workspace --locked`: 29 tests passed; two explicit live probes excluded by default.
- Both live probes were also run explicitly: Windows returned one calendar / zero events; the native HTTPS GitHub check completed with `NoStableRelease` (no latest stable release visible to the unauthenticated endpoint).
- `python scripts/check_docs.py`: 19 Markdown files, zero errors before this final evidence update; rerun after editing documentation.
- Headless Slint rendering completed, and representative light/dark appearance screens were visually inspected.
- `bun run scripts/release.ts build` and `package`: passed. Unsigned EXE payload: 10,339,328 bytes (9.86 MiB), not an idle-memory measurement.
- `bun run scripts/release.ts verify`: intentionally failed with exit 1 / `No signature found`, confirming rejection of this unsigned review build. Successful signing remains untested.

Local portable archive: `target/dist/lightcal-v0.1.0-windows-x64.zip`. EXE SHA-256: `1c1c75c58e3402d70e48af55a5e417afd26a10956f5119b18a4531b0049903b7`; archive SHA-256: `e411834fa16b30c2be4c7eebca19c39fbf36d7ecadc50c8262cba59bf87c65d1`.

Visual outputs are under `target/audit-snapshots/`, including `19-appearance-{light,dark}-{date_only,weekday_and_date,time_and_date}.png`. These are synthetic UI evidence, not screenshots of Explorer or proof of native shell behavior.

Remaining release gates:

1. Confirm the desired real calendar source and validate populated event data, recurrence, timezones/DST, all-day events, account removal, permission changes, and stale/offline behavior. Re-test the tray-menu Today action and midnight month crossing on the real desktop; both now trigger provider refresh.
2. Test running-window versus pinned-shortcut icons in Explorer. Runtime icons render the current date; the embedded EXE icon remains static. Three icon modes are unrelated to Windows calendar access. System-clock replacement and arbitrary text beside the tray icon are not implemented.
3. Real desktop checks: mixed DPI/multiple monitors, Explorer restart, tray overflow, focus handling, keyboard-only Settings, Narrator/high contrast, long translations, and Japanese font fallback. The shortcut picker still cycles presets instead of capturing arbitrary keys; startup/persistence errors need better user feedback.
4. Review provider loading/stale state presentation, refresh coalescing/cancellation, and calendar selection cleanup. The asynchronous request guard prevents stale month results from replacing newer ones, but does not cancel the older worker.
5. Verify GitHub success/no-release/rate-limit/timeout/offline behavior end to end on an unrestricted network. Unit fixtures cover response evaluation; transport failures are currently reported generically as offline. The Preview update-channel UI has no separate prerelease implementation.
6. Sign using the owner's installed tool/certificate and verify the resulting executable and archive. No signing credentials were accessed and no genuine signed artifact is claimed. Review complete dependency attribution and matching source availability before GPL distribution.
7. Run the new Actions workflow after the owner authorizes pushing. Remote CI cannot validate unpushed changes. GitHub release publication remains an owner action.
8. Complete ADR 0001's release-build CPU/private bytes/working set/activation measurements and actual shell acceptance matrix. Earlier prose about platform support is not measurement evidence. macOS launch/menu-bar validation remains open; no framework change was made.
9. Verify the advertised Rust 1.80 minimum against the locked dependency graph. This review built with Rust 1.99.1, so it does not establish that minimum-version compatibility.

Primary references: [AppointmentManager.RequestStoreAsync](https://learn.microsoft.com/en-us/uwp/api/windows.applicationmodel.appointments.appointmentmanager.requeststoreasync), [native Windows HTTP client bindings](https://microsoft.github.io/windows-docs-rs/doc/windows/Web/Http/struct.HttpClient.html).
