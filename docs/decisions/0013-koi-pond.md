# ADR 0013: Koi pond background

Date: 2026-11-03. Status: accepted by the owner in chat.

## Context

The owner asked for an optional decoration: koi swimming on the calendar
background. Lightcal is a light tray app with an idle CPU target
([validation](../validation.md)), so a decoration must not run while the
popup is hidden and must not make dates harder to read.

## Decision

- Settings › Calendar gets a **Koi pond** switch below Event dots. It is off
  by default and stored as `show_koi`; files without the key load it as off.
- When on, three koi swim behind the day grid: a white and red kohaku, a gold
  ogon and a dark showa. Each follows its own slow closed loop (46, 64 and
  80 s per lap), points along the loop's tangent and beats its tail.
- The pond sits above the shaded weekday bands and below the weekday letters,
  dates, Today, selection and event dots. It is drawn at 50 % opacity, and the
  white koi has a faint outline in the light theme.
- The fish are vector paths in `ui/koi.slint`, computed from Slint's
  `animation-tick()`. There are no image assets. Each fish lives in its own
  small box so the software renderer repaints only the boxes that move.
- The pond is only built while the calendar view shows it. A hidden or
  minimized popup is not rendered, so the animation does not run.
- Labels: Koi pond, Пруд с карпами кои, Estanque koi, 鯉の池.

## Consequences

- While the popup is open with the pond on, Lightcal repaints every frame.
  [Validation](../validation.md) records 0 CPU with the pond on and the popup
  hidden; the cost while the popup is open is not measured yet.
- Headless snapshots that include koi show whatever pose the clock gives at
  render time, so `calendar-koi-*.png` are not pixel-stable.
- The design canvas has no Koi pond board yet.
