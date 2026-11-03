# ADR 0016: Calendar motion and the Today underline

Date: 2026-11-03. Status: accepted by the owner in chat.

## Context

After hover follow ([ADR 0015](0015-calendar-hover-follow.md)) the owner chose
four more motions from a proposed list: the Copy → Copied crossfade, Settings
switches and pages, the agenda expanding, and the month change slide. Settings
motion was later removed (below). Colour
fades everywhere were not chosen. The owner also asked for a selected Today to
be marked with an underline instead of a ring.

## Decision

- **Motion switch.** A `Motion` global in `ui/theme.slint` holds the durations
  (fade 120 ms, move 160 ms). Rust turns it off when Windows **Animation
  effects** is off, at every sync and on every system setting change, and
  every transition below then applies at once. The header's Today slide
  ([validation](../validation.md)) now follows the same setting.
- **Copy → Copied.** Both labels share one slot as wide as the longer one and
  crossfade over 120 ms, so the actions beside them never shift.
- **Settings stays as on main.** Opening Settings, its pages and its switches
  behave exactly as before this decision; `ui/settings.slint` and the switch
  are unchanged. Fades of the Settings view, of each page and of switch colours
  were tried and removed at the owner's request: the window also changes size
  (352 × 432 to 448 × 360) and is re-anchored above the tray, and any motion
  around that jump made it feel less smooth than main. A smooth switch would
  need both views to share one window size, which would mean redesigning
  Settings.
- **Fading in on creation.** Slint does not animate the first value of a
  property, so an element created already "open" would never fade. The
  `FadeIn` control (`ui/controls.slint`) instead takes its opacity from the
  animation clock since it was created, and a timer pins it at full opacity
  when done, so it stops redrawing once settled. Event details use it, and fade
  only for the event just clicked; details shown with the calendar, such as on
  returning from Settings, appear at once.
- **Agenda expand.** The details panel fades in over 160 ms, while the row
  above squares its lower corners at the same pace. Row fills, including hover,
  change at once.
  Collapsing is instant. The panel cannot also slide or grow: it is created
  when it opens and takes its height from its own layout, so the agenda below
  still moves at once.
  Agenda rows are now updated in place when they are the same events, so a
  sync (for example after Copy) no longer rebuilds them or restarts their
  transitions; another day's events get new rows, so nothing animates between
  unrelated events.
- **Month change.** When the displayed month changes, the six day rows fade
  in, ease-out cubic over 180 ms; nothing moves. An 8 px sideways slide was
  tried first and removed at the owner's request, because it read as the
  calendar shaking. The weekday letters, shaded bands, koi and hover highlight
  stay as they are. A month change during a fade carries on from the opacity
  on screen, so fast paging never blanks the grid. It plays only for changes
  made while the popup is on screen with Animation effects on; Rust drives it
  with a timer that runs only during the fade, and hiding the popup or opening
  Settings settles it.
- **Selected Today.** The 1.5 px coral ring is replaced by a 16 × 2 px coral
  underline, 1 px below the Today tile in the 4 px gap between week rows. The
  keyboard focus ring is unchanged.

## Consequences

- Calendar snapshots with Today selected show the underline instead of the
  ring. Snapshots render with Animation effects off, so they show every
  transition settled.
- CPU while these transitions run is not measured. None of them runs a timer
  or redraws while idle.
- The design canvas still shows the ring and has no motion notes yet.
- The transitions need a manual look on Windows; headless tests cover when
  they play, not how they look.
