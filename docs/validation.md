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

Current documentation check: `python scripts/check_docs.py` (Python 3.9 or newer). It checks local file targets and whitespace, not external URLs, heading anchors, or YAML semantics. Rust build/test commands will be added with the Cargo workspace. This planning baseline does not claim application tests have passed.
