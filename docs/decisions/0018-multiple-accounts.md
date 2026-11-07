# ADR 0018: Several accounts from the same provider

Date: 2026-11-04. Status: accepted by the owner on 2026-11-04 (answers
under [Owner decisions](#owner-decisions)).

## Context

A user asked to connect more than one Google account
([issue #54](https://github.com/andmetoo/lightcal/issues/54)). Today Lightcal
connects at most one Google account and one Microsoft account. Connecting
again replaces the saved account, as
[provider setup](../provider-setup.md) states for Microsoft: "Current
implementation connects one Microsoft account at a time; reconnecting
successfully replaces it." Private ICS feeds are already a list.

[ADR 0002](0002-direct-calendars-and-updates.md) approved Microsoft Graph and
[ADR 0008](0008-google-calendar-and-platform-design.md) approved read-only
Google Calendar. Neither limits the number of accounts, but the shipped
behavior and the documentation do, so this record asks the owner to change
that behavior.

Where the code assumes one account per provider:

- **Credentials.** `secure_store::Credentials` has one
  `google_refresh_token` with its client ID and secret, and one
  `microsoft_refresh_token` with its client ID. `merge_rotated_token` picks
  the slot by `RefreshProvider` alone.
- **Connect and disconnect.** `CalendarAction::ConnectGoogle` and
  `ConnectMicrosoft` overwrite those slots. Disconnect clears them and drops
  every selected calendar ID that starts with `google:` or `graph:`.
- **Load results.** `calendars::CalendarLoad` returns one
  `MicrosoftAccountState` and one set of Google fields (`google_connected`,
  `google_needs_signin`, `google_label`, `google_error`, `google_skipped`).
  The controller mirrors them, including the `held_google_*` values kept
  during a sign-in.
- **Calendar IDs.** Sources are `google:{calendarId}` and `graph:{calendarId}`.
  Google calendar IDs are global: a calendar shared with two accounts, or a
  public holiday calendar such as `en.usa#holiday@group.v.calendar.google.com`,
  has the same ID in both. Two accounts would share one checkbox state and
  load the same events twice. Graph calendar IDs are per mailbox, so a clash
  there is unlikely but not ruled out.
- **Settings.** `AccountCardItem.id` is `"outlook"` or `"google"`, and the
  card callbacks dispatch on that string. The card list itself is already a
  model of any length.
- **Google sign-in.** The authorization request sends `prompt=consent`, which
  does not force Google's account chooser. With one signed-in browser session
  the user may not get to pick a second account. Microsoft already sends
  `prompt=select_account`.

What already fits:

- `calendars::query` merges any number of providers into one
  `ProviderSnapshot`, and one provider's failure does not hide another's
  calendars.
- Only one browser sign-in runs at a time: the controller refuses account
  actions while `connection_busy` is set. `SIGN_IN_GATE` does not serialize
  sign-ins; `begin` starts a new generation so a superseded or cancelled
  sign-in cannot commit. Both stay as they are.
- Each account's label already comes from data Lightcal reads today: the
  primary calendar ID for Google and the default calendar's `owner.address`
  for Microsoft. No new scope is needed to tell accounts apart.

## Decision

### Scope

- **Google and Microsoft both.** Each provider holds a list of accounts. The
  same model serves both, and doing only Google would leave Microsoft with a
  second, older code path.
- **At most five accounts per provider.** Every account adds sequential
  requests to each refresh (every 15 minutes, on month navigation and on
  manual refresh). The limit keeps the worst case bounded. At the limit, the
  provider's row in **Add an account** is disabled with a short reason.
- **Unchanged:** read-only scopes, the system browser with loopback redirect
  and PKCE, DPAPI storage, one sign-in at a time, and the rule that a new
  account starts with no calendars selected.

### Account identity

- Each account gets a random **key** when it is first saved, used in
  calendar IDs and card IDs: 8 lowercase hexadecimal characters from the
  system random generator, drawn again if it matches a saved key. The
  existing `random_key()` (43 base64url characters) is not used for this. The
  only other key is the fixed `legacy` from migration, which a hex key cannot
  spell. The key never leaves the PC and carries no personal data.
- Right after a successful sign-in, Lightcal reads two values from calendar
  data it already requests, with no new scope:
  - The **account ID**, used to recognize the account: the primary
    calendar's ID from `calendarList` for Google, and the default calendar's
    `id` from `/me/calendars` for Microsoft. It does not need to be an email
    address. The comparison is exact for Microsoft and ignores case for
    Google.
  - **No account ID** means nothing is saved and the sign-in counts as
    failed, keeping the previous state, as a failed sign-in already keeps
    the old credentials today. Two causes, each with its own message and
    **Try again**: the calendar request failed (network, throttling, a
    server error), or the account has no calendar to read (Google returns
    no primary calendar, or Microsoft lists no calendar marked
    `isDefaultCalendar`, as with an organizational account without an
    Exchange Online mailbox). Such an account would have no calendars to
    show anyway.
  - The **address**, shown as the label: the Google primary calendar ID when
    it contains `@`, and the default calendar's `owner.address` for
    Microsoft. A Microsoft `owner.address` can be an alias that differs from
    the sign-in name, which is why it labels the account but does not
    identify it. With no address, the card shows the provider name and its
    calendar names.
- The account ID is stored with the account. Calendar IDs, selection and
  card actions use the key, never the account ID or the address.
- This refines owner answer 3 below, and the owner approved the refinement
  on 2026-11-04: the account is still recognized from calendar data with no
  new scope. For Google the account ID is the address; for Microsoft it is
  the default calendar's ID, because the address alone is not reliable.

Two sign-in paths, each with fixed outcomes:

- **Add an account** (provider row). Starts only below the limit; at the
  limit the row is disabled before any browser opens.
  - Same account ID as a saved account: the new token replaces that
    account's token and keeps its key, so its selected calendars stay
    checked. Settings shows that the account was already connected and is
    updated.
  - A new account ID: a new account is added.
- **Sign in again** (an account card). Always targets that card's key and
  never adds an account, so it cannot fill the limit with duplicates.
  - The account ID of another saved account of the same provider: nothing is
    saved, because one account must not sit under two keys. The card says
    this account is already connected on another card. This check comes
    first.
  - Same account ID as the card: the token replaces that key's token.
  - The card has no stored account ID yet: the token replaces that key's
    token and the account ID is stored with it.
  - Any other account ID: nothing is saved. The card says a different
    account was chosen.
  - When nothing is saved, the card keeps its state so the user can try
    again.
- An account saved before the account ID existed (the migrated `legacy`
  account) gets it on its first successful load. If it is expired at the
  upgrade and never loads, two rules apply until it has an account ID:
  - Its **Sign in again** follows the no-account-ID rule above, but asks
    before saving: "This card will now show {address}", with **Keep** and
    **Cancel**. Cancel saves nothing. Lightcal makes no "same account"
    decision here: selection follows calendar IDs, so calendars of the same
    account stay checked and another account's calendars start unchecked,
    as for any new account. On the first successful load after **Keep**
    only, selected IDs with that key that the account does not list are
    removed from preferences, since they belonged to the previous account.
    Later loads never prune, so a calendar that is briefly missing (hidden
    in the provider, or a shared calendar that is unavailable) stays
    checked.
  - **Add an account** for that provider first asks whether to reconnect
    the expired account instead. **Reconnect** runs that card's **Sign in
    again**; **Add another** continues with the normal Add. The choice comes
    before the browser opens because Lightcal cannot tell afterwards: if the
    user picks **Add another** and signs in as the expired account anyway,
    a second card is added for it, and the old card's **Sign in again** is
    then refused as "already connected on another card". The old card can
    only be disconnected.

Lightcal never revokes a grant, as with Disconnect today. A token from a
refused **Sign in again** is dropped; the user's provider account still lists
Lightcal under third-party access until they remove it there. The limit check
before **Add an account** means only the two **Sign in again** refusals above
can drop a token.

### Storage

- Accounts move to a new file, `accounts.dpapi`, also current-user DPAPI in
  the data directory. It holds `google_accounts` and `microsoft_accounts`,
  each a list of `{ key, account_id, label, refresh_token, client_id }`, with
  `client_secret` for Google, plus the ICS subscriptions. The list order is
  the order in which accounts were added.
- **Migration.** Runs only when `accounts.dpapi` is missing:
  - `credentials.dpapi` absent: `accounts.dpapi` is written with empty
    lists.
  - `credentials.dpapi` read successfully: each old single account becomes a
    one-item list with the fixed key `legacy`, and each ICS feed is copied
    with its ID. Each migrated item records what it was migrated from as a
    SHA-256: of the refresh token for an account, of the address for a feed.
    No second copy of a secret is stored.
  - `credentials.dpapi` present but the read failed (locked, read error):
    nothing is written. Accounts shows a quiet notice with **Try now**, and
    the migration is tried again on the next load.
  - `credentials.dpapi` present with unreadable content (DPAPI cannot
    decrypt it, or it does not parse): nothing is written, and Accounts shows
    the existing "Stored credentials are invalid" notice. **Start over** is
    offered as described under [unreadable files](#unreadable-files).
    **Start over** renames it to
    `credentials-unreadable-{timestamp}.dpapi` and writes `accounts.dpapi`
    with empty lists, so the user can connect accounts again. The notice
    says the set-aside file may hold encrypted tokens and feed addresses,
    with **Show in folder**.
  - An empty `accounts.dpapi` is never written in place of accounts that
    could not be read, unless the user chose **Start over**.
  - **While migration is pending** (`accounts.dpapi` missing and not yet
    written by one of the cases above), connecting, disconnecting and adding
    feeds are unavailable. Otherwise the first new account would create
    `accounts.dpapi`, migration would never run, and the old accounts would
    be left behind. No provider calendars load while migration is pending:
    Accounts shows the pending notice, and the calendar shows only local
    events. Nothing reads tokens from `credentials.dpapi` for loading, so no
    rotated token can be lost. ICS feeds wait as well.
- After migration, only `accounts.dpapi` is read. `credentials.dpapi` is
  never deleted. The new build changes it in three cases only: clearing a
  migrated copy when it is removed (see [Consequences](#consequences)),
  renaming it in its own **Start over**, and clearing or renaming it in the
  `accounts.dpapi` **Start over** below. An older build that still uses it
  therefore cannot overwrite or erase the new account lists. A fixed key
  makes the step repeatable: an interrupted migration produces the same IDs
  next time.
- **An `accounts.dpapi` that cannot be loaded** is never treated as missing:
  Lightcal does not migrate again and does not write the file. Two cases:
  - **Read failed** (the file is locked or a read error occurs): Accounts
    shows a quiet notice with **Try now**, and the next refresh tries again.
    Nothing is set aside.
  - **Unreadable content** (DPAPI cannot decrypt it, or it does not parse):
    Accounts shows no provider accounts or feeds and one notice. **Start
    over** is offered as described under
    [unreadable files](#unreadable-files), as the local events file does
    ([ADR 0017](0017-local-events.md)).
  - In both cases connecting, disconnecting and adding feeds stay
    unavailable until the file loads or the user starts over.
- **Start over** for `accounts.dpapi` runs two steps. Each is safe to
  repeat, so if a step fails, Start over stops with **Try now** and running
  it again finishes the job:
  1. Clear the account and feed fields of `credentials.dpapi`, whatever
     they hold, since the record of what was migrated is in the unreadable
     file. If it is missing or already cleared, this step does nothing. If
     it does not parse, it is renamed to
     `credentials-unreadable-{timestamp}.dpapi` instead. If it fails to
     decrypt, the [unreadable files](#unreadable-files) rule applies: on a
     first failure Start over stops with **Try now** and changes nothing,
     and only a repeated failure renames it.
  2. Rename `accounts.dpapi` to `accounts-unreadable-{timestamp}.dpapi`
     and start with empty lists. Migration never runs again.
  - If step 1 cannot complete (the file is locked), nothing is changed. If
    step 2 fails after step 1, `credentials.dpapi` is already cleared and
    `accounts.dpapi` is still in place; the next attempt skips step 1's
    work and retries step 2.
  - The notice lists each set-aside file, says it still holds encrypted
    tokens and feed addresses, and offers **Show in folder** so the user
    can delete it.

- `merge_rotated_token` and `save_rotated_token` match by provider and key. A
  rotated token for an account that was removed or reconnected meanwhile is
  still dropped.

#### Unreadable files

- A file that decrypts but does not parse is damaged for good: **Start
  over** is offered at once.
- A DPAPI decryption failure can be temporary, for example when Lightcal
  starts at sign-in before the user's keys are available. The first failure
  shows only **Try now**. **Start over** appears once decryption has failed
  again on any later attempt in the same process: **Try now**, the
  15-minute refresh, month navigation or Refresh. A healthy file is
  therefore never set aside because of one early failure, and a user who
  never clicks **Try now** is still offered **Start over**. The count lives in memory for the running process
  only; after a restart the first failure again shows **Try now**.

### Calendar IDs

- New format: `google:{key}:{calendarId}` and `graph:{key}:{calendarId}`.
  Keys contain no `:`, so everything after the second `:` is the provider's
  ID unchanged.
- **Migration.** On every preferences load, each selected `google:` or
  `graph:` ID that is not already in the new format is rewritten to the
  `legacy` key: `google:{calendarId}` becomes `google:legacy:{calendarId}`,
  and the same for `graph:`. An ID is in the new format when the text after
  the provider prefix starts with `legacy:` or with 8 lowercase hex
  characters and `:`. Google calendar IDs contain `@` and no `:`, and Graph
  calendar IDs are base64 with no `:`, so neither can start that way.
- The rule needs no credentials and no marker in the file, so it runs when
  `accounts.dpapi` cannot be read, and running it again changes nothing. A
  marker field would not survive a downgrade: 1.2.0 ignores unknown keys
  when loading preferences and drops them on its next save, and the next
  upgrade would then prefix the IDs a second time. The schema version stays
  at version 1. A bump would not help: 1.2.0 would fall back to its
  newer-schema recovery (the `last_good` copy or defaults) and, on its next
  save, write a version 1 file from that copy that still contains new-format
  IDs. The rule above handles such a file anyway.
- Removing an account clears only the IDs with its prefix
  (`google:{key}:`).
- Event details keep choosing **Open** by the `google:` and `graph:`
  prefixes, so they need no change.

### Loading and state

- `calendars::query` loops over each provider's accounts in list order and
  returns one state per account: connected with label, needs sign-in, or
  unavailable with its error and skipped-event count. An expired or offline
  account never changes another account's state.
- `MicrosoftAccountState` becomes the per-account state for both providers.
  The controller keeps a list keyed by provider and key in place of the
  single Google fields and `held_microsoft`. During a sign-in it holds only
  the account being replaced, if any.
- Accounts are loaded one after another, as today. Parallel loading is not
  part of this decision.

### Settings › Accounts

- One card per account. The order stays as today, with each provider's
  accounts in list order: the **On this PC** card
  ([ADR 0017](0017-local-events.md)), Outlook accounts, Google accounts, ICS
  feeds, then the legacy Windows card when it is shown.
- The card ID is `account:outlook:{key}` or `account:google:{key}`. The
  `account:` namespace keeps card IDs apart from calendar source IDs
  (`google:…`, `graph:…`, `ics:…`), so card callbacks never reach the
  source-removal path by prefix. The current `"outlook"`, `"google"` and
  prefix checks in `CalendarAction::Remove` are replaced, not extended.
- The title stays **Outlook** or **Google Calendar**, and the detail shows the
  account address, so two Google cards are told apart by address.
- **Disconnect**, **Sign in again** and the calendar checkboxes act on that
  card's account only. **Refresh** still refreshes every source, as it does
  today.
- **Add an account → Google Calendar** and **→ Outlook** stay available after
  the first account, up to the limit. The Google sign-in sends
  `prompt=select_account consent` so the account chooser always appears.
- New strings are translated into all ten languages from
  [ADR 0014](0014-ten-languages.md): the limit reason; "already connected,
  updated"; "a different account was chosen"; "this account is already
  connected on another card"; the two failed sign-in messages (the calendar
  request failed; the account has no calendar to read) with **Try again**;
  the read failure of either credentials file with **Try now**; the
  unreadable-file notice of either file with **Start over** and the
  set-aside file note; the "This card will now show {address}" question
  with **Keep** and **Cancel**; the "reconnect the expired account?"
  question with **Reconnect** and **Add another**.

### Out of scope

- A combined or merged view of the same shared calendar from two accounts.
  Each account lists it separately; the user checks the one they want.
- Reordering accounts, renaming them, or giving them custom colors.
- Any write access, push subscriptions or new scopes.

## Consequences

- `accounts.dpapi` stores up to ten refresh tokens instead of two.
  `credentials.dpapi` stays on disk after migration with the tokens it had at
  the time.
- **Downgrade.** An older build ignores `accounts.dpapi` and keeps using
  `credentials.dpapi`, so it never deletes the new account lists. It sees the
  accounts it had before the upgrade; a refresh token rotated since then may
  have expired, and that account asks to sign in again. Calendars checked
  after the upgrade use the new ID format, which the older build does not
  recognize, so they show as unchecked there. Disconnect or Remove of a
  Google or Outlook source in the older build clears every selected ID with
  that provider prefix, including other accounts' new-format IDs, so those
  calendars are unchecked after upgrading again. Calendars the older build
  checks are written in the old format and migrated to `legacy` on the next
  upgrade. Upgrading again otherwise uses `accounts.dpapi` as it was left.
  Downgrades are not supported beyond this.
- A migrated `credentials.dpapi` still holds copies of what it had at
  migration: the `legacy` tokens and the private ICS feed addresses. Removing
  any migrated item in the new build (disconnecting a `legacy` account or
  removing a feed that came from migration) removes it from `accounts.dpapi`
  and also from `credentials.dpapi`, but only while the copy there is still
  the one migrated: the stored token's SHA-256 matches the recorded one, or
  the SHA-256 of the address of the feed with that ID does. If an older build
  replaced it after a downgrade, the newer entry belongs to that build and is
  left alone. These removals and **Start over** are the only writes the new
  build makes to that file, so removal still clears every local copy of a
  token or feed address. Accounts and feeds added after migration exist only
  in `accounts.dpapi`.
- Google's OAuth client is still subject to its publishing status. While the
  app is in Testing, refresh tokens expire after about seven days and only
  listed test users can sign in. Several accounts make that more visible but
  do not change it. Verification for the sensitive calendar scopes remains
  the open item recorded in ADR 0008.
- [Required features](../features.md) and
  [provider setup](../provider-setup.md) link this decision now and keep
  describing the current one-account build. Their one-account statements are
  replaced when this ships.

## Validation

- Unit tests with temporary data directories, for storage and migration:
  - Migration from `credentials.dpapi` to `accounts.dpapi` leaves the old
    file byte-for-byte unchanged, and later saves never touch it except the
    removals and **Start over** described above.
  - An interrupted migration repeats with the same `legacy` keys.
  - A missing `credentials.dpapi` writes empty lists. A read failure writes
    nothing and migrates on a later load once it reads. Unreadable content
    writes nothing until **Start over**, which sets the file aside and
    writes empty lists.
  - Migrated items store SHA-256 values only, never a second copy of a
    token or feed address.
  - A locked `accounts.dpapi` shows **Try now** and loads on retry without
    being set aside. Unreadable content is neither migrated over nor
    written.
  - **Start over** keeps the file under a new name and clears the account
    and feed fields of `credentials.dpapi`. A `credentials.dpapi` that does
    not parse is set aside too; one that fails to decrypt for the first time,
    or is locked, stops Start over with **Try now** and nothing changes. A
    failure in step 2 after step 1 is finished by the next attempt.
  - While migration is pending, connecting, disconnecting and adding feeds
    are unavailable, no action creates `accounts.dpapi`, no provider or feed
    loads, and no token is read from or written to `credentials.dpapi`.
  - After an ID-less card is kept for another account, selected IDs with
    its key that the account does not list are removed on the first
    successful load after **Keep** only; a later load that misses a
    calendar leaves it checked.
  - A second decryption failure from any later attempt (Try now, the
    automatic refresh, month navigation, Refresh) offers **Start over**.
  - A first decryption failure offers only **Try now**; **Start over**
    appears after a second failure. A file that decrypts but does not parse
    offers **Start over** at once.
  - Disconnecting the `legacy` account or removing a migrated ICS feed
    clears it from `credentials.dpapi` only while the recorded token hash
    or address still matches, and leaves an entry an older build replaced.
- Unit tests for calendar IDs and keys:
  - Selected-ID migration leaves new-format IDs alone and is a no-op on a
    second run, also when `accounts.dpapi` cannot be read.
  - A downgrade round trip (new build, then a version 1 save without
    unknown keys that keeps new-format IDs, then new build) produces no
    doubled prefix.
  - Keys are 8 lowercase hex characters and never repeat a saved key.
- Unit tests for sign-in outcomes:
  - Add with a saved account ID keeps the key and selection and reports
    "already connected, updated"; Add with a new account ID adds an
    account, including one with no address; adding the same no-address
    account twice keeps one account. Add is refused at the limit before
    sign-in.
  - **Sign in again** with the card's account ID replaces its token; with
    another saved account's ID or any other ID it saves nothing and shows
    the matching notice.
  - The `legacy` account gets its account ID on first load. An expired
    `legacy` account with no account ID: its first **Sign in again** asks
    before saving and **Cancel** saves nothing; it keeps its checked
    calendars for the same account and is refused for another saved
    account. **Add an account** for that provider asks to reconnect it
    first, and **Reconnect** runs its **Sign in again**.
  - A failed calendar request after sign-in, a Google account with no
    primary calendar and a Microsoft account with no default calendar each
    save nothing, keep the previous state and show their message.
  - A rotated token after the account was removed is dropped, and removing
    one account leaves the other's calendars and selection.
- Card actions with `account:` IDs never reach `CalendarAction::Remove` for a
  source, and source IDs never match a card action.
- Two Google accounts that both list the same shared calendar produce two
  sources with different IDs and no duplicate events.
- Load tests with fixture HTTP: one account expired and one working leaves
  the working account's calendars loaded and marks only the other card.
- Headless snapshots (`--render-snapshots`) for two Google cards, two Outlook
  cards, one expired card among working ones, and the disabled provider row
  at the limit, in both themes.
- Live sign-in with two real Google accounts and two Microsoft accounts is an
  owner-run check; automated tests never use real accounts.

## Owner decisions

Answered by the owner on 2026-11-04:

1. **Providers:** Google and Microsoft together, as proposed.
2. **Limit:** five accounts per provider.
3. **Identity:** the account is recognized from calendar data, with no new
   scope. An account whose ID in that data changes shows up as a new
   account; that is accepted. Refined after review and approved by the
   owner on 2026-11-04 (see [Account identity](#account-identity)): Google
   uses the primary calendar ID (the address), Microsoft the default
   calendar ID.
4. **Refresh:** one **Refresh** keeps reloading every source.
5. **Workaround reply:** until this ships, issue #54 points the reporter at
   Google's "Secret address in iCal format" with
   **Add an account → Private ICS link**.
