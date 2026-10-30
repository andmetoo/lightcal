# Lightcal — 2026 Desktop Visual & UX Specification

Lightcal is a native Rust + Slint desktop calendar for Windows (and future macOS) crafted in the visual tradition of **Linear Desktop**, **Raycast**, and **Notion Calendar (Cron)**.

## Core Design Principles

1. **Atmospheric Tonal Layering (Zero Plastic Navy, Zero GNOME-1 Gray Boxes)**
   - **Light Mode ("Alabaster & Ink")**: Pure paper canvas (`#FFFFFF`) paired with warm stone secondary panels (`#F7F7F8`), whisper-soft weekend column bands (`#F8F8FA`), and `1px` `#ECECEF` structural hairlines.
   - **Dark Mode ("Obsidian Graphite")**: Deep carbon-infused obsidian (`#101012`) paired with `#141417` elevated panels, `#151519` weekend column washes, `#1E1E22` tactile controls, and `1px` `#1F1F24` hairlines—eliminating generic blue-slate (`#0F172A`) plastic dark modes.
   - **Frameless Window Architecture**: No retro 32px gray OS titlebars. Both Compact (`352 × 432`) and Expanded (`520 × 600`) windows use a unified `12px`-radius canvas with subtle frameless window controls (`—`, `×`) in Expanded and Settings views.

2. **90% Quiet Neutrality, 10% Intentional Accent**
   - **Neutral Tactile Controls**: Header navigation buttons (`‹`, `[ ● Today ]`, `›`, `⚙`) and settings actions (`Check for updates`) use neutral monochrome control fills (`#FFFFFF` Light / `#1E1E22` Dark) with `1px` `#E2E2E6` / `#2C2C32` borders and primary ink text (`#141416` / `#F4F4F6`). The `Today` button features a `6px` **Coral-Vermilion (`#E5484D`)** indicator dot on the left—never green-on-green or mint-on-mint.
   - **Distinct `Today (29)` vs `Selected (30)` States**:
     - **Today (`Oct 29`)**: Solid **Coral-Vermilion (`#E5484D`)** rounded tile (`rx: 8px`) with crisp `#FFFFFF` Bold text and white event dots.
     - **Selected (`Oct 30`)**: Soft **Linear Indigo (`#EEF0FF` Light / `#1D203A` Dark)** fill with a `1.5px` `#5E6AD2` inset border, Bold ink numeral, and semantic colored event dots.
   - **Current Month Emphasis**: When viewing the active month (`October 2026`), the month title is accented with a `2px` `#5E6AD2` underline bar and a quiet `Current month` indicator (`6px` `#5E6AD2` dot + muted label). Both are omitted when browsing other months (`November 2026`).

3. **Borderless Calendar Grid**
   - Zero spreadsheet table gridlines between day cells.
   - Non-working weekday columns (`Sa`, `Su`) are tinted with a continuous vertical rounded band (`#F8F8FA` Light / `#151519` Dark, `rx: 8px`).
   - ISO week numbers (`W40`–`W45`) sit quietly in the left margin (`10px` `#A0A0AB` / `#52525B`).

4. **Grouped Settings (`780 × 620`)**
   - Related rows sit in a soft rounded group (`14px` radius, `#F4F4F6` light / `#18181C` dark) on the plain canvas. Dividers stay inside the group. Individual rows are not boxed.
   - The sidebar keeps navigation for General, Appearance, Calendars, and About. The active item is an indigo wash (`#EEF0FF` / `#1D203A`) without a heavy border.
   - Switches are `46 × 28` with a `22px` white knob and a soft shadow. Off state is a filled stone track (`#E4E4EA` / `#3A3A42`), not an empty outline.
   - Selects, fields, and buttons are `36px` / `34px` tall with a `10px` radius. Selects open an in-flow menu under the control. A private ICS address stays visible while it is edited and is stored only on the computer.

5. **Native OS Shell, Taskbar, Menu Bar, Dock & Dynamic Icon System (`06 · Shell, Dock & Icons`)**
   - **App Icon Craft**: `rx: 23%` squircle in Alabaster (`#FFFFFF`) or Obsidian (`#18181C`) with a `30%` top **Coral-Vermilion (`#E5484D`)** header band and `1px` hairline seam.
   - **Three Dynamic Live Modes**:
     - **Mode A — Date Numeral Only (`29` / `OCT 29`)**: Crisp single-numeral badge (`16×16` to `128×128`) for system tray, menu bar, or dock.
     - **Mode B — Weekday + Date (`THU 29` / `ЧТ 29`)**: Localized 2–3 letter weekday in the `#E5484D` coral header band + bold date numeral + optional semantic event dots (`#5E6AD2`, `#F59E0B`).
     - **Mode C — Full System Clock Replacement (`22:01 · Thu, 29 Oct` / `22:01:45`)**: Replaces the native Windows 11 system tray clock or macOS menu bar clock completely, combining the `[29]` dynamic icon with live digital time and localized weekday/date in a single clickable status pill.

---

## Color Token Reference

| Token | Light Mode ("Alabaster & Ink") | Dark Mode ("Obsidian Graphite") | Role |
| :--- | :--- | :--- | :--- |
| `stage` | `#EAEAEF` | `#08080A` | Presentation backdrop mat |
| `canvas` | `#FFFFFF` | `#101012` | Primary window surface |
| `panel` | `#F7F7F8` | `#141417` | Settings sidebar & expanded agenda panel |
| `weekendBand` | `#F8F8FA` | `#151519` | Continuous vertical `Sa`/`Su` column band |
| `control` | `#FFFFFF` | `#1E1E22` | Buttons, dropdowns, active sidebar item |
| `controlSubtle` | `#F2F2F5` | `#18181C` | Subtle pills, inactive weekday pills |
| `borderWindow` | `#DFDFE4` | `#27272A` | `1px` outer window border |
| `hairline` | `#ECECEF` | `#1F1F24` | `1px` structural section & row divider |
| `borderControl` | `#E2E2E6` | `#2C2C32` | `1px` button & select border |
| `ink` | `#141416` | `#F4F4F6` | Primary text & numerals |
| `secondary` | `#64646C` | `#94949E` | Weekday headers, subtitles, secondary labels |
| `muted` | `#A0A0AB` | `#52525B` | Week numbers (`W40`), shortcuts, metadata |
| `outside` | `#C8C8D0` | `#3A3A42` | Adjacent-month overflow dates |
| `todayAccent` | `#E5484D` | `#E5484D` | Coral-Vermilion `Today` tile, icon header & button dot |
| `primaryAccent` | `#5E6AD2` | `#5E6AD2` | Linear Indigo selection ring, toggles, active bars |
| `primarySoft` | `#EEF0FF` | `#1D203A` | Selected date fill & active highlight |
| `dotWork` | `#5E6AD2` | `#6E7AE2` | Work calendar checkbox & event indicator |
| `dotPersonal` | `#F59E0B` | `#FBBF24` | Personal calendar checkbox & event indicator |
| `dotHoliday` | `#10B981` | `#34D399` | Holidays calendar checkbox & event indicator |

---

## Penpot File Structure (`Новый файл 1` — 7 Pages, 39 Boards)

- **`00 · Overview`**: 1 board (`2060 × 860`) — Side-by-side Light and Dark presentation stages showing Compact (`352 × 432`) and Expanded (`520 × 600`) windows.
- **`01 · Calendar`**: 10 boards — Light & Dark Compact (`352 × 432`) and Expanded (`520 × 600`) for Current Month (`October 2026`) and Next Month (`November 2026`), plus Light & Dark Tray Menus (`240 × 216`).
- **`02 · Settings`**: 8 boards (`780 × 620`) — Light & Dark for `General`, `Appearance`, `Calendars`, and `About`.
- **`03 · States`**: 12 boards (`780 × 620`) — Light & Dark interactive & edge states (`Languages Dropdown Open`, `Shortcut Conflict`, `Calendars Access Denied`, `Calendars Empty`, `Checking Updates`, `Update Available`).
- **`04 · Languages`**: 4 boards (`352 × 432`) — Localized Compact calendars in `English (en-US)`, `Русский (ru-RU)`, `Español (es-ES)`, and `日本語 (ja-JP)`.
- **`05 · Design system`**: 1 board (`1640 × 660`) — Complete token swatches, Light & Dark header/grid primitives, and flat hairline settings controls.
- **`06 · Shell, Dock & Icons`**: 3 boards (`1440 × 620`, `1440 × 620`, `1440 × 520`) — Windows 11 Taskbar & System Tray integration (Dark & Light), macOS Menu Bar & Floating Dock integration (Dark & Light), and the multi-resolution App Icon System (`128`, `64`, `32`, `16`) with 3 dynamic modes (`Date 29`, `Weekday THU 29 / ЧТ 29`, and `Clock Replacement 22:01 · Thu, 29 Oct`).

See [`design/penpot/README.md`](./penpot/README.md) for reproducible Penpot generation scripts.
