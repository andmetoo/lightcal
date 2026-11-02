# ADR 0012: Date size setting

Date: 2026-11-02. Status: accepted by the owner in chat.
Amends the fixed 352 × 432 calendar in
[ADR 0004](0004-compact-view-and-account-settings.md) and
[ADR 0008](0008-google-calendar-and-platform-design.md).

## Context

A user asked for bigger day numbers in the calendar grid. The grid draws them
at 14 logical px in 34 × 28 tiles inside 32 px week rows. The columns are wide
enough for larger digits; the row height is the limit, because the number,
the event dots and Today's darker top band share each tile. Windows display
scaling already enlarges the whole popup, but Windows' separate text-size
accessibility setting is not expected to reach Slint apps, so there is no
other way to get bigger dates.

## Decision

- Settings › Calendar gets a **Date size** row under Theme with three options:
  **Standard** (14 px, the default), **Large** (16 px) and **Larger** (18 px).
  Each option shows a "28" sample at a matching size.
- Only the day numbers grow. Weekday letters, week numbers, the header and the
  agenda keep their sizes.
- Standard and Large keep the 352 × 432 popup. Larger uses 30 px tiles in
  34 px week rows, so the popup is 352 × 444; the agenda keeps its height.
- Event dots sit 2 px above the tile bottom (1 px on Today) and the number
  keeps a 3 px gap above them at every size.
- Sizes above 18 px would need wider tiles and a wider popup and are not
  offered.
- The metrics live in the Slint `Theme` global; Rust sizes and positions the
  window from `Theme.calendar-height()`. Windows display scaling multiplies
  everything, so Large at 150 % draws 24 physical px dates.
- The design is the **Date size** page of the design canvas, mirrored in
  [design/canvas](../../design/canvas/).

## Consequences

- The calendar window has two heights. Code that sizes, anchors or captures
  the window reads the current height instead of assuming 432.
- The Calendar settings tab no longer fits 448 × 360 without scrolling, so it
  scrolls like the App tab; the week-start menu closes when the tab scrolls.
- The preference is stored as `date_size` (`standard`, `large`, `larger`);
  unknown values load as Standard with the usual preferences warning.
