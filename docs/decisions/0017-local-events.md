# ADR 0017: Local events stored on this PC

Date: 2026-11-04. Status: accepted by the owner on 2026-11-04 (answers
under [Owner decisions](#owner-decisions)).

## Context

A user asked to create events in Lightcal itself, with no account connected:
"right click, add event, done"
([issue #45](https://github.com/andmetoo/lightcal/issues/45)). Today every
event comes from Microsoft Graph, Google Calendar or a private ICS feed, and
all of them are read-only. Two current rules block the request:

- [Required features](../features.md), Event details: "Nothing in the app
  edits events."
- [Architecture](../architecture.md): "Do not persist event titles or
  credentials without another design decision."

[AGENTS.md](../../AGENTS.md) also lists other event providers as an owner
decision. This record is that decision.

What the code already offers:

- Providers produce `CalendarSource` and `CalendarOccurrence` values
  (`calendar-core`). Event dots, the agenda, Now/Next, Copy and calendar colors
  work from those values alone, so a local source needs no change to them.
- `calendars::query` merges the providers into one `ProviderSnapshot`.
- `PreferencesStore::save` already writes through a temporary file, an atomic
  rename and a `last_good` copy, and preserves a file it cannot read instead of
  overwriting it.
- All writable data lives in `resolve_data_dir()` (`%APPDATA%\Lightcal`, or
  `LIGHTCAL_DATA_DIR` in tests). The NSIS uninstaller deletes its own files,
  shortcuts and registry entries, and its script states that "Preferences,
  DPAPI credentials, and unrelated files are deliberately retained", so this
  data survives uninstalling and reinstalling.
- The `ical` and `rrule` crates and `ics::parse` already read iCalendar text.

## Decision

### Scope of the first version

- **One local calendar.** A source with ID `local`, name **On this PC** and
  account label **Local**, always listed in Settings › Accounts › Calendars,
  with its own color from the design canvas. It needs no account and makes no
  network request.
- **Single-day events.** An event has a title, a date, and either **All day**
  or a start and end time on that date (end after start). The start is
  `00:00`–`23:59`; the end is `00:01`–`24:00`, where `24:00` (minute 1440)
  lets an event run to midnight. Time labels show an end of 1440 as `24:00`
  on the event's own date, never as `00:00` of the next day. An optional
  location and an optional note complete it.
- **Edit and delete.** Only local events can be changed. Provider events stay
  read-only, and nothing is ever written to Microsoft, Google or an ICS feed.
- **Out of scope for now:** recurrence, multi-day and overnight events,
  reminders and notifications, more than one local calendar, choosing its
  color, and `.ics` import or export. Each needs its own decision.

### Creating and editing

- **Entry points.** Right-clicking a day selects it and opens the editor for
  that date. A **+** button in the agenda header and the **N** key open it for
  the selected date. **N** works only in the calendar view with no text field
  focused and no dialog open; it is ignored while Settings or an account
  dialog is open. It matches `n` and `N`, so Shift and Caps Lock do not
  matter, with no Ctrl or Alt held. On layouts where that key types another
  letter, such as Russian or Ukrainian, the shortcut does not fire; the
  **+** button and right-click remain. The empty-day agenda also offers
  **Add event**.
- **Editor.** The editor replaces the agenda area inside the 352 × 432 popup;
  the month grid stays visible above it, and the popup does not resize. Fields
  appear in this order: Title, All day switch, Start and End time, Location,
  Note, then **Save** and **Cancel**. Enter saves from any single-line field;
  in the multi-line Note, Enter adds a line and Ctrl+Enter saves. Slint's
  `TextEdit` may handle Ctrl+Enter itself, so a headless test in
  `ui_tests.rs` confirms it reaches the editor; if it does not, Ctrl+S saves
  from every field instead, and the **Save** button always works. Escape
  cancels, and Tab moves through the fields in order. The root key handler in
  `ui/app.slint` hides the popup on Escape today, so it gains an
  `editor-open` guard, like the existing account-dialog guard, that sends
  Escape to the editor instead. Times are 24-hour `HH:MM`, the format the
  agenda already shows in every language.
- **Editing.** A local event's expanded agenda row shows **Edit** and
  **Delete** in place of **Open**, next to **Copy**. Delete asks for
  confirmation in the row (**Delete event?** · **Delete** · **Cancel**).
- **Selection.** On this PC starts selected, both on a fresh install and on
  upgrade. This is a deliberate exception to the "first connection selects
  none" rule in [Required features](../features.md): the calendar holds only
  events the user created. Preferences gain a `local_calendar_offered` flag,
  missing in older files: on the first start where it is missing or false,
  `local` is added to `selected_calendar_ids` and the flag is set to true.
  After that, the selection is the user's, so an unchecked On this PC stays
  unchecked across restarts. Saving a new local event while the calendar is
  unchecked selects it again, every time, so a new event never disappears
  from view. Otherwise unchecking the calendar hides local events like any
  other calendar.
- **Accounts.** On this PC is the first row of the calendar list and is
  always shown. With no account connected, the list shows that row and a
  **Connect an account** prompt below it, in place of the current "no
  calendars" empty state. Provider error and access-denied messages appear
  below the local row as they do today.
- **Limits.** Title 1–256 characters, location up to 256, note up to 2,000,
  on one line apart from the note. At most 2,000 events and 2 MiB for the
  whole file; the editor reports which limit was reached. A typical file of a
  few hundred short events is well under 100 KB, so a save rewrites little.
  A file over these limits, from a hand edit or a later version, still loads
  up to a hard read cap of 8 MiB, the same as an ICS feed. Until it is back
  within the limits, the editor allows only deletion and says why. A file
  over 8 MiB counts as malformed.

### Storage

- **File.** `local-events.json` in the data directory. A save writes and
  flushes a temporary file, then calls `ReplaceFileW` with
  `local-events.last_good.json` as the backup name. That one call puts the
  new file in place and keeps the previous one as `last_good`, so the data
  is written once, and the main file is missing only in a rare failure. The
  first save, with no main file yet, renames the temporary file into place.
  `ReplaceFileW` can fail with `ERROR_UNABLE_TO_MOVE_REPLACEMENT_2` after
  moving the original to the backup name but before moving the new file in.
  The writer then renames the temporary file into place itself, and if that
  fails too, the temporary file is kept. `ReplaceFileW` can also fail with
  `ERROR_UNABLE_TO_MOVE_REPLACEMENT`, leaving the old main file in place and
  the newest save only in the temporary file. So on every load, a valid
  temporary `local-events.json.*.tmp` newer than the main file (or than
  `last_good` when the main file is missing) wins, and Lightcal says that it
  recovered an unfinished save. A missing main file counts as the restore
  case below whenever a valid temporary file or a `last_good` exists, never
  as an empty calendar; only a folder with none of them starts empty.
  After the next successful save, the writer deletes every leftover
  `local-events.json.*.tmp`, so old copies of event titles do not pile up
  and a later restore never picks a stale one.
- **Portable path.** `ReplaceFileW` is called only under `cfg(windows)`.
  Other builds, which `just check` tests on Linux, first copy the current
  main file to `last_good`, then rename the temporary file over the main
  file. That rename replaces it atomically, so the main file always exists.
  Shared storage tests run on both; tests of the Win32 failure codes run
  only on Windows.
- **Unreadable file.** When the file is malformed or missing, Lightcal
  tries `last_good`. If that loads, a damaged file is kept under a unique
  name, and a notice says that an earlier copy was restored, that recent
  changes may be missing, and where any damaged file is. When the file has
  a newer schema, Lightcal never falls back to `last_good`, because that
  copy may be older than what the newer version wrote. In both cases, if no
  current file can be read, the original is kept untouched under a unique
  name, local events are shown as unavailable with a message that names the
  preserved file, and adding, editing and deleting local events are
  disabled. Nothing replaces the file until the user chooses **Start over**
  in that message, which begins an empty file and keeps the preserved copy.
- **Format.**

  ```json
  {
    "schema_version": 1,
    "events": [
      {
        "id": "local:3q2-…",
        "title": "Dentist",
        "date": "2026-11-12",
        "all_day": false,
        "start_minute": 540,
        "end_minute": 600,
        "location": "Main St 4",
        "note": "Bring the card"
      }
    ]
  }
  ```

  IDs come from `calendars::random_key`. `date` is an ISO `YYYY-MM-DD`
  string, not the derived `{year, month, day}` form of `LocalDate`. Start and
  end are minutes after local midnight in the ranges given above, and are
  absent for all-day events. No timestamps are stored, since nothing reads
  them; a future export can add its own.
- **Validation on load.** Users may edit the file, so each event is checked
  on its own: the date is parsed through `LocalDate::new`, minutes and
  lengths must be in range, and IDs must be unique. An invalid event is
  skipped and counted, like unreadable events in `ics::parse`, and a notice
  gives the count; the rest still load. A file that is not JSON, or has no
  `events` list, counts as malformed (above). The first save after a load
  that skipped events keeps the file as it was under a unique name, so a
  skipped event is never silently dropped. Later saves in the same session
  make no further copies.
- **Floating local time.** Times are local wall-clock values with no
  timezone. A 09:00 event stays at 09:00 after the Windows timezone changes or
  during travel. This matches how the core already stores occurrences.
- **Plain text.** The file is readable JSON, not DPAPI-encrypted like account
  credentials, so a user can back it up, inspect or move it. Its privacy is
  that of the user's Windows profile. The Privacy section of the
  [README](../../README.md) and the planned privacy statement
  ([open-source readiness](../open-source.md)) say so.
- **Provider events are still not stored.** This decision allows saving
  events the user creates. Events from providers stay in memory only.
- **Demo and snapshots.** `--synthetic-calendars` (`just run-demo`) and
  `--render-snapshots` never open the real `local-events.json`, take the
  events lock or write anything. They show a few in-memory sample local
  events, so screenshots never contain the user's own events, and edits
  made in the demo are discarded on exit.

### Runtime

- Local events load once at startup into their own in-memory store, separate
  from the provider snapshot. They are not part of `calendars::query`, and
  each provider refresh (`ctrl.provider = snapshot`) or provider failure
  leaves them alone. The local source and its occurrences are combined with
  the provider snapshot only when the UI builds the calendar list, event dots
  and agenda. So local events appear before providers finish loading and stay
  visible when every provider fails or credentials cannot be read.
- Provider status is computed from provider sources only. The always-present
  local source never turns NoCalendars or ProviderUnavailable into Available,
  so provider status still drives the Connect an account prompt and the
  error and access-denied messages described under Accounts.
- A save updates memory and the UI at once. The write runs off the UI thread
  in one background writer that applies writes in order, so the popup never
  waits for the disk. A failed write keeps the edit in memory, shows the
  error, and retries on the next change and on a backoff timer: 5 s,
  doubling up to 5 min. The timer runs only while a write is pending.
- **Quit.** The UI thread never waits on the writer
  ([architecture](../architecture.md)), except at session end (below).
  With nothing pending, Quit exits as today. If the last write failed,
  Quit first shows that unsaved local events will be lost and lets the
  user cancel; that prompt appears before anything blocks. With a write
  still in progress, Quit hides the window and tray icon at once and
  releases the single-instance lock, so a new launch starts normally; the
  process exits when the writer finishes, or after 2 s. It frees the global
  shortcut before it releases the lock, so the new launch can register it.
  A failed write waiting out its backoff is tried again at once, so the
  grace period is spent on an attempt. If that last write fails, nothing is
  shown, the process exits at once, and the edit is lost; this is listed
  with the known limits below.
- **One writer at a time.** Each process takes an exclusive lock on
  `local-events.lock` in the data directory at startup, before it loads
  local events, and holds it until it exits. It uses the standard
  `std::fs::File::lock` and `try_lock`, on a handle the process keeps open;
  std implements them with `LockFileEx` on Windows and `flock` on Unix, so
  no platform code is written for it. These need Rust 1.89, so the
  workspace `rust-version` rises from 1.80 to 1.89. `fcntl` locks are not
  used, because a process loses them when it closes any handle to the
  file. A file lock belongs to the process, not to the thread
  that took it, so no thread exiting can release it early, and the system
  releases it when the process ends, even after a crash. Because the lock
  file sits in the data directory, instances with their own
  `LIGHTCAL_DATA_DIR` never share it. When Quit releases the
  single-instance lock early (above), the events lock stays held. A new
  instance tries for it on a background thread and shows local events as
  loading meanwhile. So it never reads a file that the exiting process is
  still writing, and two processes never write it at once. After 3 s it
  shows them as unavailable, read only, but keeps trying, and loads them as
  soon as it holds the lock. If the lock file cannot be opened at all (for
  example, the data folder does not allow creating it), local events are
  neither read nor written for the session: Settings › Accounts says they
  could not be opened, and Start over is not offered.
- **Sign-out and shutdown.** On `WM_QUERYENDSESSION` Lightcal does not
  prompt or delay the session. On `WM_ENDSESSION` a failed write waiting
  out its backoff is tried again at once, and the UI thread waits at most
  1 s for the writer's last attempt; if it fails, the edit is lost.
  This is the one deliberate exception to the rule that the UI thread never
  waits on disk writes ([architecture](../architecture.md)): the process
  ends right after, so no further UI is lost by waiting.
- **Known limits.** An edit is safe only once its write has succeeded. It
  can be lost when that write fails during Quit's 2 s grace period, or at
  sign-out or shutdown.
- `CalendarOccurrence` gains an optional `note`, shown in the expanded agenda
  row and included in Copy. Local occurrences have no `web_link`.

### Text, accessibility and design

- All new labels, confirmations, errors and accessible names are added to the
  ten catalogs ([ADR 0014](0014-ten-languages.md)). User text is never
  translated.
- Every field has an accessible label and visible focus. The editor and the
  delete confirmation can be used with the keyboard alone.
- The editor, the row actions, the right-click result and the Accounts row are
  drawn on the design canvas before implementation.

## Alternatives considered

- **Write to the Windows appointment store or to a provider.** This needs
  write scopes or package capabilities, and it breaks the read-only promise
  for connected accounts. Rejected.
- **An `.ics` file as primary storage.** It would be portable, but writing
  valid iCalendar on every edit adds work and failure modes, and the parser
  is built for untrusted feeds. JSON fits the preferences store; `.ics`
  export can come later.
- **SQLite.** Too heavy for at most 2,000 small records in a file capped at
  2 MiB, where rewriting the whole file on a save stays cheap;
  [architecture](../architecture.md) avoids a database unless a feature
  needs one. A higher limit would need a measurement of the full rewrite
  first.
- **DPAPI encryption.** It would hide titles from other software running as
  the same user. But data encrypted with the user's DPAPI key cannot be read
  on another PC at all, by Lightcal or anything else, so moving a copy of the
  folder would lose the events unless Lightcal first exported them. It can be
  added later together with export if the owner prefers it.

## Consequences

- Lightcal is no longer strictly read-only. Approval updates
  [Required features](../features.md) (Event details, Calendar selection, a
  new Local events row), [Architecture](../architecture.md) (event
  persistence), [Product](../product.md) and the privacy statement.
- The workspace minimum Rust version rises from 1.80 to 1.89 for
  `File::lock`.
- The editor is the largest part of the work: new controls in
  `ui/calendar.slint`, text input focus handling, translations, headless UI
  tests in `ui_tests.rs` and new snapshots.
- Tests use a temporary `LIGHTCAL_DATA_DIR` and cover validation, the
  crash-safe save, a malformed or newer file (newer never using
  `last_good`), the limits, invalid events skipped and counted on load, the
  restored-copy notice, `24:00` end labels, Quit with a pending write, and
  that provider events are never written. They also cover demo mode
  leaving the real file untouched, a temporary file newer than an existing
  main file winning on load, the
  `ReplaceFileW` save and its partial failure (Windows only), the portable
  save, a missing main file restored from a newer temporary file or from
  `last_good`, including a first save with no `last_good`, leftover
  temporary files deleted after a save, an oversized file loading as
  delete-only, the
  `local_calendar_offered` migration running once, the events lock wait,
  its timeout and later acquisition, the lock surviving its acquiring
  thread and released by a killed process, the write retry backoff,
  Ctrl+Enter in the note, and **N** matching both cases.
- Memory, startup and save cost for a full 2 MiB file are not measured
  yet; a release build measurement belongs in
  [validation](../validation.md).
- Saved local events are lost when the user deletes `%APPDATA%\Lightcal`
  or moves to another PC without copying it; export and import are the
  planned answer. An edit whose write never succeeded can also be lost (see
  Known limits).

## Owner decisions

Answered by the owner on 2026-11-04:

1. **Scope:** approved as proposed: one local calendar, single-day all-day
   or timed events up to `24:00`, a title with an optional location and
   note, and edit and delete for local events only. Recurrence, multi-day
   events, reminders, and import and export are deferred.
2. **Entry points:** keep all of them: right-clicking a day, the **+**
   button in the agenda header, the **N** key, and **Add event** in the
   empty-day agenda.
3. **Storage:** plain JSON. Portability, easy backups and recovery on
   another PC outweigh DPAPI for local event titles.
4. **`.ics` export:** not in the first version. Export and import need
   their own decision, so the first version stays focused on the editor,
   storage and locking.
5. **Name:** **On this PC**, with account label **Local** and ID `local`.
