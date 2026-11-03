# ADR 0015: Calendar hover follow

Date: 2026-11-03. Status: accepted by the owner in chat.

## Context

The owner asked for motion in the calendar popup, starting with a hover
highlight that follows the pointer across the day grid. Before this change
each day tile painted its own hover fill, so hover jumped from day to day.
An owner prototype with about 400 ms of lag felt right. A design review
(UX and UI, both by review agents) found that a fixed-time glide would
flicker in the 13 px gaps between tiles and that the hover fill (#F0F0F0 /
#2A2A2A) was only one step from the selected fill (#E8E8E8 / #313131).

## Decision

- One rounded highlight, the size of a day tile (34 × 28 px, 34 × 30 px with
  Larger dates, 8 px radius), replaces the per-day hover fill. It sits above
  the shaded weekday bands and the koi pond and under the selected day,
  Today, numbers and dots, so it slides beneath them.
- A critically damped spring with stiffness 20 per second pulls it toward the
  day cell under the pointer: 95 % of a one-column step in about 237 ms,
  settled (within about 0.5 px) in about 332 ms. The owner chose 20 over 16.6
  (the prototype, about 400 ms) and 26 (about 255 ms).
- A new cell only changes the target. Position and velocity carry on, so the
  highlight bends toward the new cell from where it is drawn and never visits
  cells passed on the way. Velocity toward a new target is capped at
  stiffness × distance, which is exactly what keeps a critically damped
  spring from passing its target, so fast sweeps that stop or reverse do not
  overshoot. Each frame uses the exact spring solution for the measured frame
  time, capped at 32 ms, so frame length does not change the path.
- The whole cell, gaps included, is the hover and click target, so a sweep
  along a row is one continuous highlight. Positions are in cells and mapped
  through the grid's current geometry, so date size, week numbers and month
  changes always line up.
- Entering the day rows places the highlight on the cell and fades it in over
  100 ms. Leaving them, including into the weekday letters, fades it out in
  place over 120 ms; once invisible it starts fresh at the next cell.
- Hover is independent of the selection. Arrow keys fade it out until the
  pointer moves again.
- The selected day uses the palette's Selected fill, #E0E0E0 / #3A3A3A
  (`Theme.day-selected`), so hover and selected stay distinct. Other uses of
  `Theme.selected` are unchanged.
- With Windows **Animation effects** off, the highlight jumps to the cell and
  appears or disappears at once, and no timer runs.
- A Rust timer (`src/hover_follow.rs`) advances the spring once per 15.625 ms
  Windows timer tick and only while position or opacity is still changing.
  When settled it snaps onto the cell and stops. Hiding the popup or opening
  Settings clears the highlight and stops the timer.

## Consequences

- The sliding selection and focus ring from the same review are not part of
  this decision; the review asked for a stroke-only ring before they move.
- Calendar snapshots change once, for the stronger selected fill; snapshots
  never show hover.
- CPU while the highlight moves is not measured. Idle and hidden, no hover
  timer runs.
- The motion itself, at 100 % and 150 % scale, light and dark, needs a manual
  look on Windows.
