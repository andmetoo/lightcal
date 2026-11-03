# ADR 0011: Windows-only design refresh on a design canvas

Date: 2026-11-01. Status: accepted by the owner in chat.
Supersedes the macOS parts of [ADR 0008](0008-google-calendar-and-platform-design.md),
the tray artwork in [ADR 0010](0010-windows-calendar-tray-icon.md), and the
Penpot design-tool choice in [Product](../product.md).

## Context

The owner found the October 31 Penpot design dated and asked for a modern
refresh. Independent UI and UX reviews identified the main problems: a
two-row header, a repeated date subtitle, heavy rounded weekend columns, one
weight for every number, a starved agenda, and settings tabs with two
selection cues. Penpot's font rendering was not good enough for review, and
most of its 114 boards repeated the same screen in another theme, platform or
language. Designing and maintaining a separate macOS look was judged too
expensive for a one-person project.

## Decision

- **Windows only.** Lightcal targets Windows. All macOS code, designs, the
  disk-image script, its CI job and `just` recipe, and macOS passages in the
  documentation are removed.
- **Design source.** The design now lives on the owner's design canvas
  **Lightcal Refresh**. Its board sources are mirrored in
  [design/canvas](../../design/canvas/). Each board has a light/dark switch, so
  one board per distinct screen or state replaces the Penpot duplicates. The
  final Penpot file is archived in
  [design/archive](../../design/archive/) for history; its automation scripts
  and exports are removed.
- **Calendar.** One 44 px header row (month, previous, Today, next, settings);
  Today appears only away from today. Six fixed week rows. Weekend columns are a
  flat, barely tinted band. Today is a coral tile with a darker top band; the
  selected day is a neutral fill; selected today adds a coral ring (replaced by an underline in ADR 0016). Event dots
  stay at most three. The agenda shows all-day chips, time ranges, past events
  dimmed, **Now** and **in 3 h** on today, and an empty day links to the next
  event.
- **Event details.** Clicking an agenda row expands it in place with the time
  range and duration, location, calendar, **Copy** and **Open**. Open goes to
  the event in the provider's web calendar (Google or Outlook) in the default
  browser, only on user action. It is hidden for private ICS events. Events
  remain read-only.
- **Settings.** Fluent-style selector bar with one teal marker, cards with
  40 px rows, a one-track shaded-days picker, a key-cap shortcut field and a
  Close-only title bar. Account rows show "Updated … ago"; the Read-only pill is
  replaced by one line under the account list. The provider choices are the
  Accounts empty state. Problems appear in the account card: expired access in
  amber with one action, offline as a quiet line.
- **Icons.** One shape everywhere: a coral `#D73840` rounded tile with a darker
  band (`#A92A31` light, `#B02E35` dark). The app icon puts a 2×2 grid inside,
  one square solid white for today. The live tray date is the same tile with a
  bold white numeral. Mono is an outline in the taskbar's ink and follows the
  Windows shell theme. Vector masters are in
  [assets/icons/canvas](../../assets/icons/canvas/).
  *Implementation:* `platform/windows/icon.rs` renders every tray raster
  (16/20/24/32/40/48/64 px) at its exact size. The tile fills the raster with
  a 6/32 corner radius and a 6/32 band snapped to whole pixels; Mono uses a
  2/32 outline (at least 1 px) in `#1A1A1A` on a light shell and white on a
  dark one. The numeral is Segoe UI Variable Bold (Text optical size) from
  `%WINDIR%\Fonts`, falling back to Segoe UI Bold and then built-in stroke
  digits; only the needed outlines stay in memory. Digits are about 0.4 of the
  raster tall, with one size for all 31 dates at each raster (ADR 0010; the
  canvas's 19/18 split is not used), sit on a whole-pixel baseline centred
  below the band, and shrink only if the widest date would cross the margin.
  The application icon and `crates/lightcal/assets/lightcal.ico` come from the
  same renderer (`--export-app-icon`); `--export-icon-review <dir>` writes
  review sheets. The window/taskbar icon stays the dated tile (ADR 0010).
- **Typography.** Segoe UI Variable replaces Onest: Text for UI copy and the
  tray numeral, Display for large headings, with Segoe UI as the Windows 10
  fallback and Yu Gothic UI supplied by Windows for Japanese. It matches the
  taskbar clock and menus, is tuned for small sizes, and ships with Windows,
  so the app no longer bundles a font. Manrope (too wide, no Japanese),
  Satoshi (no Cyrillic, non-OFL license) and Geist were considered.
- **Window chrome.** DWM rounded corners and shadow. A Windows 11 Mica backdrop
  is approved as optional, behind a prototype.

## Consequences

- The Slint UI, the native tray renderer, the application icon and the
  installer icon must be updated to match. The new tray numerals need pixel
  checks at 100/125/150/200 % on real light and dark taskbars before release.
- Slint must render Segoe UI Variable from the system: confirm whether it
  selects the Text and Display optical sizes or needs the static Segoe UI.
  Remove the bundled Onest files and their notice once the UI no longer uses
  them; Segoe UI is never redistributed.
- Mica needs a Slint/DWM prototype that keeps idle CPU and memory within the
  [validation](../validation.md) targets. The platform side exists behind
  `LIGHTCAL_MICA=1` and is off by default; the backdrop stays hidden until the
  Slint root is made transparent. First app-side numbers and their limits are
  in [validation](../validation.md).
- DWM rounds the window with an 8 px radius on Windows 11. The Slint surfaces
  currently draw their own 12 px (calendar) and 10 px (settings) corners and a
  1 px border inside it, so the UI should match the DWM radius or fill the
  window edge to edge.
- The darker coral band has not had a contrast measurement; white numerals sit
  on `#D73840` (4.64:1), not on the band.
- Event "Open" links must be built from provider data, opened only on user
  action, and never fetched in the background.
- AGENTS.md, the product plan and the roadmap no longer mention a macOS port.
