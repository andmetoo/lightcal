# Validation and performance protocol

All numbers below are **proposed targets, not measured results**. Record results during the platform prototype and revise only through a documented decision.

## Resource targets

| Metric | Initial target |
| --- | --- |
| Hidden idle CPU | At most 0.1% average on the reference machine over 5 minutes |
| Hidden private working set | At most 30 MiB after warmup |
| Visible private working set | At most 50 MiB for either calendar mode |
| First launch to usable calendar | p95 at most 500 ms for 20 fresh-process starts under documented warm filesystem-cache conditions |
| Resident tray activation to first usable frame | p95 at most 100 ms across 50 activations |
| Packaged application payload | At most 20 MiB uncompressed, excluding debug symbols |

Choose and document a reference Windows 11 x64 machine with SSD before collecting numbers. Report CPU model, RAM, Windows build, graphics adapter, Rust/dependency versions, renderer, and power mode. Keep cold-cache starts as a separately labeled experiment. Include binaries and any required redistributable payloads when reporting size.

Use optimized builds without a debugger, repeat after a 60-second warmup, and record raw samples plus p50/p95 in the prototype PR. Define CPU as CPU-time delta divided by wall time and logical CPU count; report absolute CPU seconds too. Record private bytes alongside private working set so paging cannot hide allocation growth. Use Windows process counters and, where needed, WPR/WPA; record tool versions and capture method. Screen recordings or instrumented event-to-frame timestamps can validate activation latency; state measurement overhead.

Hidden mode must not repaint continuously, run a busy timer, or poll the tray. Check repeated open/hide cycles for memory/handle growth. Do not compare renderer results from different build profiles or machines as if they were equivalent.

## Functional and accessibility matrix

- Entry points: first launch, pinned launch when resident, app-button activation, tray toggle, right-click menu, Quit, crash/restart.
- Shell: Explorer restart, tray overflow, auto-hidden taskbar, monitor disconnect, restored window position, multi-monitor placement.
- Display: 100/125/150/200% scaling, mixed DPI, light/dark/system theme, high contrast, long labels.
- Input: mouse, keyboard-only use, focus loss, focus transfer between modes, and menu interaction without accidental dismissal.
- Accessibility: Narrator names, role/selection announcements, focus visibility, contrast, and no color-only selected/today distinction.
- Dates: leap years including century rules, December/January, first-weekday changes, ISO week-year boundaries, local midnight, timezone changes, and suspend/resume.
- Persistence: missing settings, malformed settings, unsupported schema, permission failure, and restart after interrupted replacement.
- Features: weekday display, gray-weekday subsets, current-month label/year matching, optional shortcut registration/conflicts, startup, and About GitHub/koona.app links.
- Events: checked-calendar filtering, zero/multiple sources, permission/provider failures, loading/stale counts, all-day/recurring/multi-day occurrences, DST and exclusive date boundaries. Use synthetic data for automated tests; keep live-provider checks explicit and read-only.
- Updates: stable-version comparison, no release, offline, timeout, rate limiting, invalid metadata/link rejection, and no implicit download/install/restart.
- Languages: English default on every OS locale; switch/restart persistence for English/Russian/Spanish/Japanese; missing-message fallback; localized dates/plurals/errors/accessibility; long strings and Japanese font fallback; no changes to source event text or user date/weekday preferences.

## Automation boundaries

Pure Rust unit tests cover date arithmetic and state transitions. Integration tests use a temporary `LIGHTCAL_DATA_DIR`. OS shell and accessibility behavior need real desktop smoke checks; a headless successful build is insufficient. Add regression tests for concrete defects, not snapshots duplicating the implementation.

Current checks: `python scripts/check_docs.py` (Python 3.9 or newer), `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --locked`. The documentation check validates local file targets and whitespace, not external URLs, heading anchors, or YAML semantics.

## Review regression checks

- Window geometry reuses the shared work-area clamp. Tests include the 780 by 620 settings layout at 200% DPI on a 1920 by 1040 work area, oversized windows in both axes, and negative monitor origins. An oversized layout is anchored safely rather than causing invalid clamp bounds; this does not claim that its entire content fits a smaller screen.
- A process-owned offscreen native Windows window tests visibility, taskbar-style minimization, restoration, and hidden state. Tray and shortcut toggles read native state rather than a controller visibility flag. Dismissal minimizes the existing window and preserves its HWND for Explorer and DWM.
- Fixture snapshots preserve `AccessDenied` through provider aggregation and retain other providers' sources and events. Headless settings renders cover denied-only and mixed readable/denied sources in both themes; permission-recovery controls and readable calendar selection must coexist.
- The macOS CI job builds the `lightcal` binary with `cargo build -p lightcal --locked` and runs `calendar-core` plus `lightcal` library tests. A successful build is not a launch, menu-bar, Dock, or calendar-provider result.
- `pwsh -NoProfile -File scripts/test-nsis.ps1` exercises the installer and uninstaller with all real-profile operations compiled out, verifying the payload hash and preservation of unrelated files.
