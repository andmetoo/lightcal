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
  pose comes from a pond clock in seconds that Rust advances 32 times a second
  (two ticks of the default Windows timer, so frames stay evenly spaced) and wraps every 960 s, a whole number of every loop and tail beat, so
  the wrap is seamless and the f32 clock stays precise however long Lightcal
  runs. Each fish lives in its own small box so the software renderer
  repaints only the boxes that move.
- The koi swim while the pond is shown, the popup can be seen and Windows
  **Animation effects** is on. "Seen" means on screen, not cloaked on another
  virtual desktop, not under the lock screen, and not completely covered:
  five points across the window are hit-tested, so a popup shown without
  activation, or partly behind another app, still swims.
- When the popup is on screen but unseen, or Animation effects is off, the koi
  hold still and the timer checks once a second for that to change. When the
  popup hides or minimizes, the pond closes or settings open, the timer stops
  entirely. Showing, restoring or activating the popup, or an Animation effects
  change, re-checks at once, and swimming always resumes from the current pond
  time.
- A test reads the school from Slint and checks that the 960 s wrap is a whole
  number of every loop period and tail beat.
- Labels: Koi pond, Пруд с карпами кои, Estanque koi, 鯉の池.

## Consequences

- While the popup is open with the pond on, Lightcal repaints the koi about
  32 times a second, and checks once a second while it is open but unseen.
  [Validation](../validation.md) records 0 CPU with the pond on and the popup
  hidden; the cost while the popup is open is not measured yet.
- Headless snapshots pin the pond clock, so `calendar-koi-*.png` are
  reproducible.
- The design canvas has no Koi pond board yet.
