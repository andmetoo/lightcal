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
  ogon and a dark showa. Each follows its own slow closed loop (48, 64 and
  80 s per lap), points along the loop's tangent and beats its tail.
- The pond sits above the shaded weekday bands and below the weekday letters,
  dates, Today, selection and event dots. It is drawn at 50 % opacity, and the
  koi have a faint outline: dark in the light theme, light in the dark theme,
  where the showa body is also lightened.
- The fish are vector paths in `ui/koi.slint` with no image assets. Their
  pose comes from a pond clock in seconds that Rust advances about 30 times a
  second and wraps every 960 s, a whole number of every loop and tail beat, so
  the wrap is seamless and the f32 clock stays precise however long Lightcal
  runs. Each fish lives in its own small box so the software renderer
  repaints only the boxes that move.
- The clock runs only while the pond is shown, the popup is on screen and
  Windows **Animation effects** is on. Its timer stops itself when the popup
  hides or minimizes, and showing or restoring the popup starts it again. With
  Animation effects off the koi stay still.
- Labels: Koi pond, Пруд с карпами кои, Estanque koi, 鯉の池.

## Consequences

- While the popup is open with the pond on, Lightcal repaints the koi about
  30 times a second. [Validation](../validation.md) records 0 CPU with the pond on and the popup
  hidden; the cost while the popup is open is not measured yet.
- Headless snapshots pin the pond clock, so `calendar-koi-*.png` are
  reproducible.
- The design canvas has no Koi pond board yet.
