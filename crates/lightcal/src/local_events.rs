//! Events the user creates in Lightcal, stored only on this PC ([ADR 0017]).
//!
//! The events live in `local-events.json` in the data directory as plain JSON. A save
//! writes a temporary file and swaps it in, keeping the previous file as
//! `local-events.last_good.json`. Loading validates every event on its own, recovers an
//! unfinished save or the backup, and never overwrites a file it cannot read. One
//! process at a time holds `local-events.lock`.
//!
//! [ADR 0017]: ../../../docs/decisions/0017-local-events.md
use crate::preferences::LocaleCode;
use calendar_core::{local_minutes, CalendarOccurrence, CalendarSource, LocalDate};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Condvar, Mutex,
    },
    time::{Duration, SystemTime},
};

/// Calendar source ID of the local calendar.
pub const SOURCE_ID: &str = "local";
/// Event IDs start with this prefix.
pub const EVENT_ID_PREFIX: &str = "local:";
/// Swatch color of the local calendar (Teal 700 from the Gray + Teal palette).
pub const COLOR_HEX: &str = "#0F766E";
pub const SCHEMA_VERSION: u64 = 1;
pub const MAX_EVENTS: usize = 2_000;
/// Largest file the editor still writes to.
pub const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
/// Largest file that loads at all, the same cap as an ICS feed.
pub const READ_CAP_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_TITLE_CHARS: usize = 256;
pub const MAX_LOCATION_CHARS: usize = 256;
pub const MAX_NOTE_CHARS: usize = 2_000;
/// Minute 1440 is `24:00`, the end of the event's own date.
pub const END_OF_DAY: u16 = 1440;
const MIN_YEAR: i32 = 1900;
const MAX_YEAR: i32 = 2200;

const FILE_NAME: &str = "local-events.json";
const BACKUP_NAME: &str = "local-events.last_good.json";
const LOCK_NAME: &str = "local-events.lock";
const TEMP_PREFIX: &str = "local-events.json.";
const TEMP_SUFFIX: &str = ".tmp";
const PRESERVED_PREFIX: &str = "local-events.preserved-";
const MAX_PRESERVED_FILES: usize = 10;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Name of the local calendar in the calendar list.
pub fn calendar_name(locale: LocaleCode) -> &'static str {
    match locale {
        LocaleCode::En => "On this PC",
        LocaleCode::De => "Auf diesem PC",
        LocaleCode::Es => "En este PC",
        LocaleCode::Fr => "Sur ce PC",
        LocaleCode::PtBr => "Neste PC",
        LocaleCode::Ru => "На этом ПК",
        LocaleCode::Uk => "На цьому ПК",
        LocaleCode::Ja => "この PC",
        LocaleCode::Ko => "이 PC",
        LocaleCode::ZhHans => "此电脑",
    }
}

/// Account label of the local calendar.
pub fn account_label(locale: LocaleCode) -> &'static str {
    match locale {
        LocaleCode::En | LocaleCode::Es | LocaleCode::Fr | LocaleCode::PtBr => "Local",
        LocaleCode::De => "Lokal",
        LocaleCode::Ru => "Локальный",
        LocaleCode::Uk => "Локальний",
        LocaleCode::Ja => "ローカル",
        LocaleCode::Ko => "로컬",
        LocaleCode::ZhHans => "本地",
    }
}

/// The local calendar as a calendar source.
pub fn source(locale: LocaleCode) -> CalendarSource {
    CalendarSource {
        id: SOURCE_ID.into(),
        name: calendar_name(locale).into(),
        account_label: account_label(locale).into(),
        color_hex: COLOR_HEX.into(),
    }
}

/// All day, or a start and end in minutes after local midnight on the event's date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timing {
    AllDay,
    /// `start` is 0–1439 and `end` is 1–1440, after `start`.
    Timed {
        start: u16,
        end: u16,
    },
}

/// A validated local event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalEvent {
    pub id: String,
    pub title: String,
    pub date: LocalDate,
    pub timing: Timing,
    pub location: Option<String>,
    pub note: Option<String>,
}

/// What the editor submits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDraft {
    pub title: String,
    pub date: LocalDate,
    pub timing: Timing,
    pub location: String,
    pub note: String,
}

/// Why an event or draft is not valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidEvent {
    Id,
    Title,
    Location,
    Note,
    Date,
    Time,
}

/// Why an edit was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditError {
    Invalid(InvalidEvent),
    TooManyEvents,
    FileTooLarge,
    /// The file is over the limits; only deleting is allowed until it fits again.
    OverLimits,
    /// Local events are loading or unavailable.
    ReadOnly,
    NotFound,
}

impl LocalEvent {
    /// Validates and normalizes every field.
    pub fn validated(
        id: String,
        title: &str,
        date: LocalDate,
        timing: Timing,
        location: Option<&str>,
        note: Option<&str>,
    ) -> Result<Self, InvalidEvent> {
        if !id.starts_with(EVENT_ID_PREFIX)
            || id.len() <= EVENT_ID_PREFIX.len()
            || id.len() > 128
            || id.chars().any(char::is_control)
        {
            return Err(InvalidEvent::Id);
        }
        let title = single_line(title, MAX_TITLE_CHARS)
            .ok_or(InvalidEvent::Title)?
            .ok_or(InvalidEvent::Title)?;
        let location = match location {
            Some(raw) => single_line(raw, MAX_LOCATION_CHARS).ok_or(InvalidEvent::Location)?,
            None => None,
        };
        let note = match note {
            Some(raw) => multi_line(raw, MAX_NOTE_CHARS).ok_or(InvalidEvent::Note)?,
            None => None,
        };
        if !(MIN_YEAR..=MAX_YEAR).contains(&date.year) {
            return Err(InvalidEvent::Date);
        }
        if let Timing::Timed { start, end } = timing {
            if start >= END_OF_DAY || end > END_OF_DAY || end <= start {
                return Err(InvalidEvent::Time);
            }
        }
        Ok(Self {
            id,
            title,
            date,
            timing,
            location,
            note,
        })
    }

    fn from_draft(id: String, draft: &EventDraft) -> Result<Self, InvalidEvent> {
        Self::validated(
            id,
            &draft.title,
            draft.date,
            draft.timing,
            Some(&draft.location),
            Some(&draft.note),
        )
    }

    /// The event as an occurrence for event dots and the agenda.
    pub fn to_occurrence(&self) -> CalendarOccurrence {
        let (is_all_day, start_minutes, end_minutes, time_label) = match self.timing {
            Timing::AllDay => (true, None, None, String::new()),
            Timing::Timed { start, end } => (
                false,
                Some(local_minutes(self.date, start.into())),
                Some(local_minutes(self.date, end.into())),
                format!("{:02}:{:02}", start / 60, start % 60),
            ),
        };
        CalendarOccurrence {
            source_id: SOURCE_ID.into(),
            event_id: self.id.clone(),
            color_hex: COLOR_HEX.into(),
            start_date: self.date,
            end_date_exclusive: self.date.add_days(1),
            is_all_day,
            title: self.title.clone(),
            time_label,
            start_minutes,
            end_minutes,
            location: self.location.clone(),
            web_link: None,
            note: self.note.clone(),
        }
    }
}

/// Collapses whitespace to single spaces. `None` when the text is too long or holds other
/// control characters; `Some(None)` when it is blank.
fn single_line(raw: &str, max_chars: usize) -> Option<Option<String>> {
    let text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().any(char::is_control) || text.chars().count() > max_chars {
        return None;
    }
    Some((!text.is_empty()).then_some(text))
}

/// Keeps line breaks (as `\n`) and tabs, trims the ends. `None` when too long or holding
/// other control characters; `Some(None)` when blank.
fn multi_line(raw: &str, max_chars: usize) -> Option<Option<String>> {
    let text = raw.replace("\r\n", "\n").replace('\r', "\n");
    let text = text.trim();
    if text
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t')
        || text.chars().count() > max_chars
    {
        return None;
    }
    Some((!text.is_empty()).then(|| text.to_string()))
}

/// A new event ID.
pub fn new_event_id() -> Result<String, String> {
    Ok(format!(
        "{EVENT_ID_PREFIX}{}",
        crate::calendars::random_key()?
    ))
}

// ---------- File format ----------

#[derive(Serialize, Deserialize)]
struct RawEvent {
    id: String,
    title: String,
    date: String,
    all_day: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    start_minute: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    end_minute: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(Serialize)]
struct RawFileOut {
    schema_version: u64,
    events: Vec<RawEvent>,
}

fn format_date(date: LocalDate) -> String {
    format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
}

fn parse_date(raw: &str) -> Option<LocalDate> {
    let mut parts = raw.split('-');
    let (year, month, day) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return None;
    }
    LocalDate::new(year.parse().ok()?, month.parse().ok()?, day.parse().ok()?)
}

fn to_raw(event: &LocalEvent) -> RawEvent {
    let (all_day, start_minute, end_minute) = match event.timing {
        Timing::AllDay => (true, None, None),
        Timing::Timed { start, end } => (false, Some(start.into()), Some(end.into())),
    };
    RawEvent {
        id: event.id.clone(),
        title: event.title.clone(),
        date: format_date(event.date),
        all_day,
        start_minute,
        end_minute,
        location: event.location.clone(),
        note: event.note.clone(),
    }
}

fn from_raw(raw: RawEvent) -> Result<LocalEvent, InvalidEvent> {
    let date = parse_date(&raw.date).ok_or(InvalidEvent::Date)?;
    let timing = match (raw.all_day, raw.start_minute, raw.end_minute) {
        (true, None, None) => Timing::AllDay,
        (false, Some(start), Some(end)) => Timing::Timed {
            start: u16::try_from(start).map_err(|_| InvalidEvent::Time)?,
            end: u16::try_from(end).map_err(|_| InvalidEvent::Time)?,
        },
        _ => return Err(InvalidEvent::Time),
    };
    LocalEvent::validated(
        raw.id,
        &raw.title,
        date,
        timing,
        raw.location.as_deref(),
        raw.note.as_deref(),
    )
}

/// The serialized file for `events`.
pub fn serialize(events: &[LocalEvent]) -> String {
    let file = RawFileOut {
        schema_version: SCHEMA_VERSION,
        events: events.iter().map(to_raw).collect(),
    };
    let mut text = serde_json::to_string_pretty(&file).expect("local events serialize");
    text.push('\n');
    text
}

/// A file that parsed as this schema: its valid events and how many were skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedFile {
    events: Vec<LocalEvent>,
    skipped: usize,
}

#[derive(Debug)]
enum ReadOutcome {
    Missing,
    /// Not JSON, no `events` list, no schema version, or over the read cap.
    Malformed,
    /// Written by a later version with a newer schema.
    Newer,
    /// Exists but cannot be read (permissions, sharing violation).
    Unreadable(String),
    Parsed(ParsedFile),
}

/// Parses file text. Each event is checked on its own; invalid ones are skipped and
/// counted, and a repeated ID keeps only its first event.
fn parse(text: &str) -> ReadOutcome {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Ok(serde_json::Value::Object(mut root)) = serde_json::from_str(text) else {
        return ReadOutcome::Malformed;
    };
    match root
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
    {
        Some(version) if version > SCHEMA_VERSION => return ReadOutcome::Newer,
        Some(version) if version >= 1 => {}
        _ => return ReadOutcome::Malformed,
    }
    let Some(serde_json::Value::Array(items)) = root.remove("events") else {
        return ReadOutcome::Malformed;
    };
    let mut events: Vec<LocalEvent> = Vec::with_capacity(items.len());
    let mut skipped = 0;
    for item in items {
        let event = serde_json::from_value::<RawEvent>(item)
            .ok()
            .and_then(|raw| from_raw(raw).ok());
        match event {
            Some(event) if !events.iter().any(|known| known.id == event.id) => events.push(event),
            _ => skipped += 1,
        }
    }
    ReadOutcome::Parsed(ParsedFile { events, skipped })
}

fn read(path: &Path) -> ReadOutcome {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return ReadOutcome::Missing,
        Err(error) => return ReadOutcome::Unreadable(error.to_string()),
    };
    let mut bytes = Vec::new();
    if let Err(error) = file.take(READ_CAP_BYTES + 1).read_to_end(&mut bytes) {
        return ReadOutcome::Unreadable(error.to_string());
    }
    if bytes.len() as u64 > READ_CAP_BYTES {
        return ReadOutcome::Malformed;
    }
    match String::from_utf8(bytes) {
        Ok(text) => parse(&text),
        Err(_) => ReadOutcome::Malformed,
    }
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

// ---------- Loading ----------

/// Whether local events can be changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    Editable,
    /// Over [`MAX_EVENTS`] or [`MAX_FILE_BYTES`]: only deleting is allowed.
    DeleteOnly,
    /// Nothing can be changed until the problem is solved or the user starts over.
    Unavailable(Unavailable),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unavailable {
    /// Another Lightcal process still holds the events lock.
    Locked,
    /// Written by a later version of Lightcal.
    NewerVersion { file: PathBuf },
    /// Neither the file nor any copy could be read as local events.
    Damaged { file: PathBuf },
    /// The file is fine but nothing can be saved this session (the writer did not
    /// start); a restart may help. Starting over is never offered for it.
    CannotSave { error: String },
    /// The file exists but the system refused to read it.
    Unreadable { file: PathBuf, error: String },
}

/// Something the user is told after loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// A save that never finished was found and used.
    RecoveredUnfinishedSave,
    /// The previous copy was used; recent changes may be missing. `damaged` is where an
    /// unreadable main file was moved.
    RestoredBackup { damaged: Option<PathBuf> },
    /// Invalid events were skipped. The file as it was is kept: copied aside at load
    /// when it was a temporary file or the backup, otherwise on the next save.
    SkippedEvents(usize),
}

/// The result of loading local events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    pub events: Vec<LocalEvent>,
    pub access: Access,
    pub notices: Vec<Notice>,
    /// Keep a copy of the file as it was before the next save (skipped events).
    pub copy_before_next_save: bool,
    /// The events came from a temporary file or the backup, so they should be saved
    /// right away to repair the main file.
    pub repair: bool,
    /// The file size these events serialize to, the measure every limit check uses.
    pub bytes: u64,
}

impl Loaded {
    pub fn unavailable(reason: Unavailable) -> Self {
        Self {
            events: Vec::new(),
            access: Access::Unavailable(reason),
            notices: Vec::new(),
            copy_before_next_save: false,
            repair: false,
            bytes: empty_file_bytes(),
        }
    }

    fn from_parsed(parsed: ParsedFile, mut notices: Vec<Notice>) -> Self {
        let repair = notices.iter().any(|notice| {
            matches!(
                notice,
                Notice::RecoveredUnfinishedSave | Notice::RestoredBackup { .. }
            )
        });
        if parsed.skipped > 0 {
            notices.push(Notice::SkippedEvents(parsed.skipped));
        }
        // The same size measure the editor uses, so a file that loads as editable never
        // refuses an edit that keeps it within the limits.
        let bytes = file_bytes(&parsed.events);
        let access = if parsed.events.len() > MAX_EVENTS || bytes > MAX_FILE_BYTES {
            Access::DeleteOnly
        } else {
            Access::Editable
        };
        Self {
            events: parsed.events,
            access,
            notices,
            copy_before_next_save: parsed.skipped > 0,
            repair,
            bytes,
        }
    }
}

/// Where local events are stored.
#[derive(Debug, Clone)]
pub struct Storage {
    dir: PathBuf,
}

impl Storage {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn file_path(&self) -> PathBuf {
        self.dir.join(FILE_NAME)
    }

    fn backup_path(&self) -> PathBuf {
        self.dir.join(BACKUP_NAME)
    }

    /// Temporary files left by saves that did not finish.
    fn temp_files(&self) -> Vec<PathBuf> {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.starts_with(TEMP_PREFIX) && name.ends_with(TEMP_SUFFIX)
                    })
            })
            .collect()
    }

    /// The newest temporary file that parses as this schema, with its time and path.
    fn newest_valid_temp(&self) -> Option<(SystemTime, PathBuf, ParsedFile)> {
        self.temp_files()
            .into_iter()
            .filter_map(|path| match read(&path) {
                ReadOutcome::Parsed(parsed) => Some((modified(&path)?, path, parsed)),
                _ => None,
            })
            .max_by_key(|(time, _, _)| *time)
    }

    /// Events read from `source`, a temporary file or the backup that the next save
    /// removes or overwrites. With skipped events, `source` itself is copied aside now,
    /// since copying the main file on save would miss them.
    fn loaded_from_copy(&self, parsed: ParsedFile, notices: Vec<Notice>, source: &Path) -> Loaded {
        let mut loaded = Loaded::from_parsed(parsed, notices);
        if loaded.copy_before_next_save && self.copy_aside_once(source).is_ok() {
            loaded.copy_before_next_save = false;
        }
        loaded
    }

    /// Copies `source` aside unless a preserved copy with the same contents exists, so
    /// loading the same file again (a repair save kept failing) adds no duplicates that
    /// would push other copies out.
    fn copy_aside_once(&self, source: &Path) -> io::Result<PathBuf> {
        let contents = fs::read(source)?;
        let same = self.preserved_files().into_iter().find(|path| {
            fs::metadata(path).is_ok_and(|m| m.len() == contents.len() as u64)
                && fs::read(path).is_ok_and(|other| other == contents)
        });
        match same {
            Some(existing) => Ok(existing),
            None => self.copy_aside(source),
        }
    }

    /// Loads local events. Reads only, except that an unreadable main file is moved aside
    /// once a readable copy replaces it, and a temporary file or backup with skipped
    /// events is copied aside before the next save removes it.
    pub fn load(&self) -> Loaded {
        let main_path = self.file_path();
        let main = read(&main_path);
        match main {
            // A newer version's file is never mixed with older copies.
            ReadOutcome::Newer => {
                return Loaded::unavailable(Unavailable::NewerVersion { file: main_path })
            }
            ReadOutcome::Unreadable(error) => {
                return Loaded::unavailable(Unavailable::Unreadable {
                    file: main_path,
                    error,
                })
            }
            _ => {}
        }
        let temp = self.newest_valid_temp();
        if let ReadOutcome::Parsed(parsed) = main {
            return match temp {
                Some((time, path, newer))
                    if modified(&main_path).is_none_or(|main| time > main) =>
                {
                    self.loaded_from_copy(newer, vec![Notice::RecoveredUnfinishedSave], &path)
                }
                _ => Loaded::from_parsed(parsed, Vec::new()),
            };
        }
        let main_damaged = matches!(main, ReadOutcome::Malformed);
        let backup_path = self.backup_path();
        let backup = read(&backup_path);
        let backup_time = modified(&backup_path);
        // A newer version's copy is never replaced by an older leftover temp file.
        if matches!(backup, ReadOutcome::Newer) {
            return Loaded::unavailable(Unavailable::NewerVersion { file: backup_path });
        }
        let temp_wins = match (&temp, &backup) {
            (Some((time, _, _)), ReadOutcome::Parsed(_)) => backup_time.is_none_or(|b| *time > b),
            (Some(_), _) => true,
            (None, _) => false,
        };
        let (parsed, recovered) = if temp_wins {
            (temp.map(|(_, path, parsed)| (path, parsed)), true)
        } else {
            match backup {
                ReadOutcome::Parsed(parsed) => (Some((backup_path.clone(), parsed)), false),
                ReadOutcome::Missing if !main_damaged => {
                    // Nothing was ever saved: a fresh, empty calendar.
                    return Loaded::from_parsed(
                        ParsedFile {
                            events: Vec::new(),
                            skipped: 0,
                        },
                        Vec::new(),
                    );
                }
                ReadOutcome::Newer => {
                    return Loaded::unavailable(Unavailable::NewerVersion { file: backup_path })
                }
                ReadOutcome::Unreadable(error) => {
                    return Loaded::unavailable(Unavailable::Unreadable {
                        file: backup_path,
                        error,
                    })
                }
                _ => (None, false),
            }
        };
        let Some((source, parsed)) = parsed else {
            let file = if main_damaged { main_path } else { backup_path };
            return Loaded::unavailable(Unavailable::Damaged { file });
        };
        let damaged = if main_damaged {
            match self.move_aside(&main_path) {
                Ok(path) => Some(path),
                // Keep the damaged file where it is rather than lose the readable copy.
                Err(_) => {
                    return Loaded::unavailable(Unavailable::Damaged { file: main_path });
                }
            }
        } else {
            None
        };
        let notice = if recovered {
            Notice::RecoveredUnfinishedSave
        } else {
            Notice::RestoredBackup { damaged }
        };
        self.loaded_from_copy(parsed, vec![notice], &source)
    }

    /// A free `local-events.preserved-<seconds>[-n].json` path.
    fn preserved_path(&self) -> io::Result<PathBuf> {
        let seconds = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        for attempt in 0..1000u32 {
            let name = if attempt == 0 {
                format!("{PRESERVED_PREFIX}{seconds}.json")
            } else {
                format!("{PRESERVED_PREFIX}{seconds}-{attempt}.json")
            };
            let path = self.dir.join(name);
            if !path.exists() {
                return Ok(path);
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "no free name for a preserved copy",
        ))
    }

    /// Keeps the newest preserved copies, by the time in their names: a rename or copy
    /// keeps the old file's modified time, so a fresh copy could otherwise go first.
    /// Names this module did not write (a copy the user renamed) are never pruned.
    fn prune_preserved(&self) {
        let mut files: Vec<((u64, u32), PathBuf)> = self
            .preserved_files()
            .into_iter()
            .filter_map(|path| Some((preserved_order(&path)?, path)))
            .collect();
        files.sort();
        if files.len() > MAX_PRESERVED_FILES {
            for (_, stale) in &files[..files.len() - MAX_PRESERVED_FILES] {
                let _ = fs::remove_file(stale);
            }
        }
    }

    fn preserved_files(&self) -> Vec<PathBuf> {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(PRESERVED_PREFIX))
            })
            .collect()
    }

    fn move_aside(&self, path: &Path) -> io::Result<PathBuf> {
        let destination = self.preserved_path()?;
        fs::rename(path, &destination)?;
        self.prune_preserved();
        Ok(destination)
    }

    fn copy_aside(&self, path: &Path) -> io::Result<PathBuf> {
        let destination = self.preserved_path()?;
        fs::copy(path, &destination)?;
        self.prune_preserved();
        Ok(destination)
    }

    /// Writes `events`. With `copy_first`, the current file is first copied to a preserved
    /// name. A failed save leaves its temporary file for the next load to recover; at most
    /// one is kept, since each new one holds everything the older ones held.
    pub fn save(&self, events: &[LocalEvent], copy_first: bool) -> Result<(), String> {
        fs::create_dir_all(&self.dir)
            .map_err(|e| format!("Failed to create the data directory: {e}"))?;
        let main = self.file_path();
        let temp = self.dir.join(format!(
            "{TEMP_PREFIX}{}-{}{TEMP_SUFFIX}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let written = fs::File::create(&temp).and_then(|mut file| {
            file.write_all(serialize(events).as_bytes())?;
            file.sync_all()
        });
        if let Err(error) = written {
            // A partial file holds nothing worth recovering.
            let _ = fs::remove_file(&temp);
            return Err(format!(
                "Failed to write the temporary events file: {error}"
            ));
        }
        // Only this process writes (the events lock), so older temporary files are
        // earlier versions of what was just written.
        self.remove_temp_files_except(Some(&temp));
        if copy_first && main.exists() {
            // A failed copy does not block the save: `replace` still keeps the previous
            // file as `last_good`.
            let _ = self.copy_aside(&main);
        }
        replace(&temp, &main, &self.backup_path())?;
        self.remove_temp_files_except(None);
        Ok(())
    }

    fn remove_temp_files_except(&self, keep: Option<&Path>) {
        for path in self.temp_files() {
            if Some(path.as_path()) != keep {
                let _ = fs::remove_file(path);
            }
        }
    }

    /// Starts an empty file after local events were unavailable. The unreadable file, if
    /// any, is moved to a preserved name first; returns where it went.
    pub fn start_over(&self) -> Result<Option<PathBuf>, String> {
        let main = self.file_path();
        let preserved = if main.exists() {
            Some(
                self.move_aside(&main)
                    .map_err(|e| format!("Failed to keep the events file: {e}"))?,
            )
        } else {
            None
        };
        self.save(&[], false)?;
        Ok(preserved)
    }
}

/// When a preserved copy was made: the seconds and attempt in its name, as
/// `preserved_path` writes it; `None` for any other name. A clock that was wrong when a
/// copy was made still misorders it; the name is the only time a rename or copy keeps.
fn preserved_order(path: &Path) -> Option<(u64, u32)> {
    let stem = path
        .file_name()?
        .to_str()?
        .strip_prefix(PRESERVED_PREFIX)?
        .strip_suffix(".json")?;
    let (seconds, attempt) = match stem.split_once('-') {
        Some((seconds, attempt)) => (seconds, attempt.parse().ok()?),
        None => (stem, 0),
    };
    Some((seconds.parse().ok()?, attempt))
}

/// Puts `temp` in place of `main`, keeping the previous `main` as `backup`.
#[cfg(windows)]
fn replace(temp: &Path, main: &Path, backup: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::{GetLastError, ERROR_UNABLE_TO_MOVE_REPLACEMENT_2},
        Storage::FileSystem::{ReplaceFileW, REPLACEFILE_IGNORE_MERGE_ERRORS},
    };
    if !main.exists() {
        return fs::rename(temp, main)
            .map_err(|e| format!("Failed to move the events file into place: {e}"));
    }
    let wide = |path: &Path| -> Vec<u16> {
        path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    };
    let (main_w, temp_w, backup_w) = (wide(main), wide(temp), wide(backup));
    // SAFETY: all three are NUL-terminated UTF-16 paths that outlive the call; the
    // reserved pointers are null as documented.
    let replaced = unsafe {
        ReplaceFileW(
            main_w.as_ptr(),
            temp_w.as_ptr(),
            backup_w.as_ptr(),
            REPLACEFILE_IGNORE_MERGE_ERRORS,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if replaced != 0 {
        return Ok(());
    }
    // SAFETY: reads the calling thread's last-error value, set by ReplaceFileW above.
    let error = unsafe { GetLastError() };
    if error == ERROR_UNABLE_TO_MOVE_REPLACEMENT_2 {
        // The old file is already the backup; finish by moving the new one in.
        return fs::rename(temp, main)
            .map_err(|e| format!("Failed to move the events file into place: {e}"));
    }
    Err(format!(
        "Failed to replace the events file: {}",
        io::Error::from_raw_os_error(error as i32)
    ))
}

/// Puts `temp` in place of `main`, keeping the previous `main` as `backup`. The rename
/// replaces `main` atomically, so it always exists.
#[cfg(not(windows))]
fn replace(temp: &Path, main: &Path, backup: &Path) -> Result<(), String> {
    if main.exists() {
        fs::copy(main, backup).map_err(|e| format!("Failed to keep the previous events: {e}"))?;
    }
    fs::rename(temp, main).map_err(|e| format!("Failed to move the events file into place: {e}"))
}

// ---------- One writer at a time ----------

/// Exclusive lock on `local-events.lock`, held for the life of the process. It belongs to
/// the open handle, so no thread exiting releases it, and the system releases it when the
/// process ends.
#[derive(Debug)]
pub struct EventsLock {
    _file: fs::File,
}

impl EventsLock {
    fn open(dir: &Path) -> io::Result<fs::File> {
        fs::create_dir_all(dir)?;
        fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(dir.join(LOCK_NAME))
    }

    /// Takes the lock without waiting; `Ok(None)` when another process holds it.
    pub fn try_acquire(dir: &Path) -> io::Result<Option<Self>> {
        let file = Self::open(dir)?;
        match file.try_lock() {
            Ok(()) => Ok(Some(Self { _file: file })),
            Err(fs::TryLockError::WouldBlock) => Ok(None),
            Err(fs::TryLockError::Error(error)) => Err(error),
        }
    }

    /// Waits until the lock is free and takes it.
    pub fn acquire(dir: &Path) -> io::Result<Self> {
        let file = Self::open(dir)?;
        file.lock()?;
        Ok(Self { _file: file })
    }
}

// ---------- In-memory store ----------

/// Local events as the app holds them.
#[derive(Debug, Clone)]
pub struct LocalEvents {
    events: Vec<LocalEvent>,
    access: Access,
    loading: bool,
    notices: Vec<Notice>,
    copy_before_next_save: bool,
    /// Save right away to repair the main file (recovered or restored on load).
    repair_pending: bool,
    /// Size of the file these events serialize to, kept up to date on every edit so the
    /// limit check never serializes everything.
    bytes: u64,
}

/// Size of a file with no events, computed once.
fn empty_file_bytes() -> u64 {
    static EMPTY: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *EMPTY.get_or_init(|| serialize(&[]).len() as u64)
}

/// Bytes one event adds to the file, a slight overestimate (separators included).
fn event_bytes(event: &LocalEvent) -> u64 {
    (serialize(std::slice::from_ref(event)).len() as u64).saturating_sub(empty_file_bytes()) + 2
}

/// Estimated size of the file for `events`; never below the real size.
fn file_bytes(events: &[LocalEvent]) -> u64 {
    empty_file_bytes() + events.iter().map(event_bytes).sum::<u64>()
}

impl LocalEvents {
    /// Waiting for the lock or the file.
    pub fn loading() -> Self {
        Self {
            events: Vec::new(),
            access: Access::Unavailable(Unavailable::Locked),
            loading: true,
            notices: Vec::new(),
            copy_before_next_save: false,
            repair_pending: false,
            bytes: empty_file_bytes(),
        }
    }

    /// Editable events that are never written, for demo mode and tests.
    pub fn in_memory(events: Vec<LocalEvent>) -> Self {
        let bytes = file_bytes(&events);
        Self {
            events,
            access: Access::Editable,
            loading: false,
            notices: Vec::new(),
            copy_before_next_save: false,
            repair_pending: false,
            bytes,
        }
    }

    pub fn from_loaded(loaded: Loaded) -> Self {
        let bytes = loaded.bytes;
        let writable = !matches!(loaded.access, Access::Unavailable(_));
        Self {
            events: loaded.events,
            access: loaded.access,
            loading: false,
            notices: loaded.notices,
            copy_before_next_save: loaded.copy_before_next_save,
            repair_pending: loaded.repair && writable,
            bytes,
        }
    }

    /// True once after a load that recovered a temporary file or restored the backup:
    /// the caller saves at once, so the main file is repaired and the notice is not
    /// shown again on the next start.
    pub fn take_repair(&mut self) -> bool {
        std::mem::take(&mut self.repair_pending)
    }

    /// Still waiting after the lock timeout: shown as unavailable, but the wait goes on.
    pub fn mark_lock_timeout(&mut self) {
        if self.loading {
            self.loading = false;
            self.access = Access::Unavailable(Unavailable::Locked);
        }
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn events(&self) -> &[LocalEvent] {
        &self.events
    }

    pub fn access(&self) -> &Access {
        &self.access
    }

    pub fn notices(&self) -> &[Notice] {
        &self.notices
    }

    pub fn dismiss_notices(&mut self) {
        self.notices.clear();
    }

    /// Occurrences for event dots and the agenda; empty while loading.
    pub fn occurrences(&self) -> Vec<CalendarOccurrence> {
        if self.loading {
            return Vec::new();
        }
        self.events.iter().map(LocalEvent::to_occurrence).collect()
    }

    fn check_writable(&self) -> Result<(), EditError> {
        match self.access {
            Access::Editable => Ok(()),
            Access::DeleteOnly => Err(EditError::OverLimits),
            Access::Unavailable(_) => Err(EditError::ReadOnly),
        }
    }

    fn fits(count: usize, bytes: u64) -> Result<(), EditError> {
        if count > MAX_EVENTS {
            return Err(EditError::TooManyEvents);
        }
        if bytes > MAX_FILE_BYTES {
            return Err(EditError::FileTooLarge);
        }
        Ok(())
    }

    /// Adds a new event and returns its ID.
    pub fn add(&mut self, draft: &EventDraft) -> Result<String, EditError> {
        self.check_writable()?;
        let id = new_event_id().map_err(|_| EditError::Invalid(InvalidEvent::Id))?;
        let event = LocalEvent::from_draft(id.clone(), draft).map_err(EditError::Invalid)?;
        let bytes = self.bytes + event_bytes(&event);
        Self::fits(self.events.len() + 1, bytes)?;
        self.events.push(event);
        self.bytes = bytes;
        Ok(id)
    }

    /// Replaces the event with `id`.
    pub fn update(&mut self, id: &str, draft: &EventDraft) -> Result<(), EditError> {
        self.check_writable()?;
        let index = self
            .events
            .iter()
            .position(|event| event.id == id)
            .ok_or(EditError::NotFound)?;
        let event = LocalEvent::from_draft(id.to_string(), draft).map_err(EditError::Invalid)?;
        let bytes =
            self.bytes.saturating_sub(event_bytes(&self.events[index])) + event_bytes(&event);
        Self::fits(self.events.len(), bytes)?;
        self.events[index] = event;
        self.bytes = bytes;
        Ok(())
    }

    /// Removes the event with `id`. Allowed while over the limits; once the events fit
    /// again, editing is allowed again.
    pub fn delete(&mut self, id: &str) -> Result<(), EditError> {
        if matches!(self.access, Access::Unavailable(_)) || self.loading {
            return Err(EditError::ReadOnly);
        }
        let index = self
            .events
            .iter()
            .position(|event| event.id == id)
            .ok_or(EditError::NotFound)?;
        let removed = self.events.remove(index);
        self.bytes = self.bytes.saturating_sub(event_bytes(&removed));
        if self.access == Access::DeleteOnly && Self::fits(self.events.len(), self.bytes).is_ok() {
            self.access = Access::Editable;
        }
        Ok(())
    }

    /// What to write after a change: the events, and whether to keep a copy of the file
    /// as it was first. The copy is asked for once per load.
    pub fn take_save(&mut self) -> (Vec<LocalEvent>, bool) {
        let copy_first = std::mem::take(&mut self.copy_before_next_save);
        (self.events.clone(), copy_first)
    }
}

/// Sample events for demo mode, around `today`.
pub fn samples(today: LocalDate) -> Vec<LocalEvent> {
    let make = |n: u8, title: &str, offset: i32, timing: Timing, location: Option<&str>| {
        LocalEvent::validated(
            format!("{EVENT_ID_PREFIX}sample-{n}"),
            title,
            today.add_days(offset),
            timing,
            location,
            None,
        )
        .expect("sample local events are valid")
    };
    vec![
        make(
            1,
            "Dentist",
            1,
            Timing::Timed {
                start: 9 * 60,
                end: 10 * 60,
            },
            Some("Main St 4"),
        ),
        make(2, "Mum's birthday", 4, Timing::AllDay, None),
        make(
            3,
            "Pick up the bike",
            0,
            Timing::Timed {
                start: 17 * 60 + 30,
                end: 18 * 60,
            },
            None,
        ),
    ]
}

// ---------- Background writer ----------

/// Outcome of a write, reported from the writer thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteReport {
    Saved,
    Failed(String),
}

#[derive(Default)]
struct WriterState {
    pending: Option<(Vec<LocalEvent>, bool)>,
    busy: bool,
    failed: bool,
    shutdown: bool,
}

/// Writes local events off the UI thread, in order. Only the latest pending state is
/// written. A failed write is retried on the next change and on a backoff timer that runs
/// only while a write is pending.
#[derive(Clone)]
pub struct Writer {
    shared: Arc<(Mutex<WriterState>, Condvar)>,
}

impl Writer {
    /// Starts the writer with the ADR's backoff: 5 s, doubling up to 5 min.
    pub fn spawn(
        storage: Storage,
        report: impl Fn(WriteReport) + Send + 'static,
    ) -> io::Result<Self> {
        Self::spawn_with_backoff(
            storage,
            Duration::from_secs(5),
            Duration::from_secs(300),
            report,
        )
    }

    pub fn spawn_with_backoff(
        storage: Storage,
        first_retry: Duration,
        max_retry: Duration,
        report: impl Fn(WriteReport) + Send + 'static,
    ) -> io::Result<Self> {
        let shared = Arc::new((Mutex::new(WriterState::default()), Condvar::new()));
        let thread_shared = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("lightcal-local-events".into())
            .spawn(move || {
                let (lock, signal) = &*thread_shared;
                let mut retry = first_retry;
                let mut state = lock.lock().unwrap_or_else(|e| e.into_inner());
                loop {
                    while state.pending.is_none() && !state.shutdown {
                        state = signal.wait(state).unwrap_or_else(|e| e.into_inner());
                    }
                    if state.shutdown && state.pending.is_none() {
                        return;
                    }
                    let (events, copy_first) = state.pending.take().expect("pending write");
                    state.busy = true;
                    drop(state);
                    let result = storage.save(&events, copy_first);
                    state = lock.lock().unwrap_or_else(|e| e.into_inner());
                    state.busy = false;
                    match result {
                        Ok(()) => {
                            state.failed = false;
                            retry = first_retry;
                            signal.notify_all();
                            drop(state);
                            report(WriteReport::Saved);
                        }
                        Err(error) => {
                            let requeued = state.record_failure(events, copy_first);
                            signal.notify_all();
                            drop(state);
                            report(WriteReport::Failed(error));
                            state = lock.lock().unwrap_or_else(|e| e.into_inner());
                            // Only a retry of the same change backs off further.
                            if requeued && !state.shutdown {
                                let wait = retry;
                                retry = (retry * 2).min(max_retry);
                                // Wakes early when a new change arrives.
                                let (guard, _) = signal
                                    .wait_timeout_while(state, wait, |s| {
                                        !s.shutdown && !s.newer_than_failure()
                                    })
                                    .unwrap_or_else(|e| e.into_inner());
                                state = guard;
                            }
                            continue;
                        }
                    }
                    state = lock.lock().unwrap_or_else(|e| e.into_inner());
                }
            })?;
        Ok(Self { shared })
    }

    /// Queues a write of `events`, replacing any write not started yet.
    pub fn submit(&self, events: Vec<LocalEvent>, copy_first: bool) {
        let (lock, signal) = &*self.shared;
        let mut state = lock.lock().unwrap_or_else(|e| e.into_inner());
        let copy_first = copy_first || state.pending.as_ref().is_some_and(|p| p.1);
        state.pending = Some((events, copy_first));
        state.failed_superseded();
        signal.notify_all();
    }

    /// True while a change has not been written yet.
    pub fn has_unsaved(&self) -> bool {
        let state = self.shared.0.lock().unwrap_or_else(|e| e.into_inner());
        state.pending.is_some() || state.busy
    }

    /// True when the last attempt failed and its change is still unsaved.
    pub fn last_write_failed(&self) -> bool {
        let state = self.shared.0.lock().unwrap_or_else(|e| e.into_inner());
        state.failed && (state.pending.is_some() || state.busy)
    }

    /// Waits up to `timeout` for every change to be written, without retrying a failed
    /// one; true when nothing is left unsaved. Never call it on the UI thread except at
    /// session end.
    pub fn wait_saved(&self, timeout: Duration) -> bool {
        let (lock, signal) = &*self.shared;
        let state = lock.lock().unwrap_or_else(|e| e.into_inner());
        let (state, _) = signal
            .wait_timeout_while(state, timeout, |s| {
                (s.pending.is_some() || s.busy) && !(s.failed && !s.busy)
            })
            .unwrap_or_else(|e| e.into_inner());
        state.pending.is_none() && !state.busy
    }

    /// Stops the thread once the pending write, if any, has been attempted.
    pub fn shutdown(&self) {
        let (lock, signal) = &*self.shared;
        lock.lock().unwrap_or_else(|e| e.into_inner()).shutdown = true;
        signal.notify_all();
    }
}

impl WriterState {
    /// Records a failed write. A newer change replaces the failed one and is written at
    /// once; otherwise the failed one is queued again and marked failed, to be retried
    /// after the backoff. Returns whether it was queued again.
    fn record_failure(&mut self, events: Vec<LocalEvent>, copy_first: bool) -> bool {
        if let Some(pending) = self.pending.as_mut() {
            pending.1 |= copy_first;
            false
        } else {
            self.failed = true;
            self.pending = Some((events, copy_first));
            true
        }
    }

    /// A change submitted after the last failure, so it should be written at once.
    fn newer_than_failure(&self) -> bool {
        self.pending.is_some() && !self.failed
    }

    fn failed_superseded(&mut self) {
        self.failed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lightcal-local-events-{label}-{}",
            crate::calendars::random_key().unwrap()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn date(day: u8) -> LocalDate {
        LocalDate::new(2026, 11, day).unwrap()
    }

    fn draft(title: &str) -> EventDraft {
        EventDraft {
            title: title.into(),
            date: date(12),
            timing: Timing::Timed {
                start: 540,
                end: 600,
            },
            location: String::new(),
            note: String::new(),
        }
    }

    fn event(n: u32, title: &str) -> LocalEvent {
        LocalEvent::validated(
            format!("local:test-{n}"),
            title,
            date(12),
            Timing::AllDay,
            None,
            None,
        )
        .unwrap()
    }

    fn set_age(path: &Path, seconds_ago: u64) {
        let time = SystemTime::now() - Duration::from_secs(seconds_ago);
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(time)
            .unwrap();
    }

    #[test]
    fn validation_normalizes_text_and_checks_ranges() {
        let ok = LocalEvent::validated(
            "local:a".into(),
            "  Dentist \t visit ",
            date(1),
            Timing::Timed {
                start: 0,
                end: 1440,
            },
            Some("   "),
            Some("line one\r\nline two  "),
        )
        .unwrap();
        assert_eq!(ok.title, "Dentist visit");
        assert_eq!(ok.location, None);
        assert_eq!(ok.note.as_deref(), Some("line one\nline two"));
        let timed = |start, end| {
            LocalEvent::validated(
                "local:a".into(),
                "x",
                date(1),
                Timing::Timed { start, end },
                None,
                None,
            )
        };
        assert_eq!(timed(1440, 1440), Err(InvalidEvent::Time));
        assert_eq!(timed(600, 600), Err(InvalidEvent::Time));
        assert_eq!(timed(600, 1441), Err(InvalidEvent::Time));
        assert!(timed(1439, 1440).is_ok());
        let title = |t: &str| {
            LocalEvent::validated("local:a".into(), t, date(1), Timing::AllDay, None, None)
        };
        assert_eq!(title(" "), Err(InvalidEvent::Title));
        assert_eq!(title(&"a".repeat(257)), Err(InvalidEvent::Title));
        assert!(title(&"a".repeat(256)).is_ok());
        assert_eq!(title("bell\u{7}"), Err(InvalidEvent::Title));
        assert_eq!(
            LocalEvent::validated("ics:x".into(), "x", date(1), Timing::AllDay, None, None),
            Err(InvalidEvent::Id)
        );
    }

    #[test]
    fn an_event_ending_at_midnight_stays_on_its_date() {
        let event = LocalEvent::validated(
            "local:a".into(),
            "Late",
            date(12),
            Timing::Timed {
                start: 23 * 60,
                end: 1440,
            },
            None,
            None,
        )
        .unwrap();
        let occurrence = event.to_occurrence();
        assert!(occurrence.covers_date(date(12)));
        assert!(!occurrence.covers_date(date(13)));
        assert_eq!(
            occurrence.end_minutes,
            Some(local_minutes(date(13), 0)),
            "minute 1440 is next midnight in local minutes"
        );
        assert_eq!(occurrence.source_id, SOURCE_ID);
        assert_eq!(occurrence.web_link, None);
    }

    #[test]
    fn invalid_and_duplicate_events_are_skipped_and_counted() {
        let text = r#"{
          "schema_version": 1,
          "events": [
            {"id":"local:a","title":"Ok","date":"2026-11-12","all_day":true},
            {"id":"local:b","title":"Bad date","date":"2026-02-30","all_day":true},
            {"id":"local:c","title":"Bad time","date":"2026-11-12","all_day":false,
             "start_minute":600,"end_minute":5000},
            {"id":"local:a","title":"Duplicate","date":"2026-11-13","all_day":true},
            {"title":"No id","date":"2026-11-12","all_day":true},
            "not an object"
          ]
        }"#;
        let ReadOutcome::Parsed(parsed) = parse(text) else {
            panic!("expected a parsed file");
        };
        assert_eq!(parsed.events.len(), 1);
        assert_eq!(parsed.events[0].title, "Ok");
        assert_eq!(parsed.skipped, 5);
    }

    #[test]
    fn malformed_and_newer_files_are_classified() {
        assert!(matches!(parse("not json"), ReadOutcome::Malformed));
        assert!(matches!(parse(r#"{"events":[]}"#), ReadOutcome::Malformed));
        assert!(matches!(
            parse(r#"{"schema_version":1}"#),
            ReadOutcome::Malformed
        ));
        assert!(matches!(
            parse(r#"{"schema_version":2,"events":[]}"#),
            ReadOutcome::Newer
        ));
    }

    #[test]
    fn save_and_load_round_trip_and_keep_the_previous_file() {
        let dir = temp_dir("round-trip");
        let storage = Storage::new(&dir);
        let fresh = storage.load();
        assert_eq!(fresh.access, Access::Editable);
        assert!(fresh.events.is_empty() && fresh.notices.is_empty());

        let mut store = LocalEvents::from_loaded(fresh);
        let id = store.add(&draft("Dentist")).unwrap();
        let (events, copy) = store.take_save();
        assert!(!copy);
        storage.save(&events, copy).unwrap();
        store.update(&id, &draft("Dentist, moved")).unwrap();
        storage.save(&store.take_save().0, false).unwrap();

        let loaded = storage.load();
        assert_eq!(loaded.events, store.events());
        assert!(loaded.notices.is_empty());
        let backup = read(&storage.backup_path());
        let ReadOutcome::Parsed(previous) = backup else {
            panic!("backup should hold the previous save");
        };
        assert_eq!(previous.events[0].title, "Dentist");
        assert!(storage.temp_files().is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_damaged_file_is_moved_aside_and_the_backup_restored() {
        let dir = temp_dir("damaged");
        let storage = Storage::new(&dir);
        storage.save(&[event(1, "First")], false).unwrap();
        storage.save(&[event(1, "Second")], false).unwrap();
        fs::write(storage.file_path(), "{ broken").unwrap();

        let loaded = storage.load();
        assert_eq!(loaded.events[0].title, "First");
        let [Notice::RestoredBackup {
            damaged: Some(damaged),
        }] = loaded.notices.as_slice()
        else {
            panic!("expected a restore notice, got {:?}", loaded.notices);
        };
        assert_eq!(fs::read_to_string(damaged).unwrap(), "{ broken");
        assert!(!storage.file_path().exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_damaged_file_with_no_copy_is_unavailable_and_left_alone() {
        let dir = temp_dir("unavailable");
        let storage = Storage::new(&dir);
        fs::write(storage.file_path(), "{ broken").unwrap();
        let loaded = storage.load();
        assert_eq!(
            loaded.access,
            Access::Unavailable(Unavailable::Damaged {
                file: storage.file_path()
            })
        );
        assert_eq!(fs::read_to_string(storage.file_path()).unwrap(), "{ broken");

        let preserved = storage.start_over().unwrap().unwrap();
        assert_eq!(fs::read_to_string(preserved).unwrap(), "{ broken");
        let fresh = storage.load();
        assert_eq!(fresh.access, Access::Editable);
        assert!(fresh.events.is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_newer_schema_never_falls_back_to_the_backup() {
        let dir = temp_dir("newer");
        let storage = Storage::new(&dir);
        storage.save(&[event(1, "Old")], false).unwrap();
        storage.save(&[event(1, "Older backup")], false).unwrap();
        fs::write(
            storage.file_path(),
            r#"{"schema_version":2,"events":[],"calendars":[]}"#,
        )
        .unwrap();
        let loaded = storage.load();
        assert_eq!(
            loaded.access,
            Access::Unavailable(Unavailable::NewerVersion {
                file: storage.file_path()
            })
        );
        assert!(loaded.events.is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_missing_main_file_restores_from_a_newer_temp_or_the_backup() {
        let dir = temp_dir("missing-main");
        let storage = Storage::new(&dir);
        storage.save(&[event(1, "Saved")], false).unwrap();
        storage.save(&[event(1, "Saved again")], false).unwrap();
        fs::remove_file(storage.file_path()).unwrap();
        set_age(&storage.backup_path(), 60);

        let from_backup = storage.load();
        assert_eq!(from_backup.events[0].title, "Saved");
        assert_eq!(
            from_backup.notices,
            vec![Notice::RestoredBackup { damaged: None }]
        );

        let temp = dir.join(format!("{TEMP_PREFIX}1-1{TEMP_SUFFIX}"));
        fs::write(&temp, serialize(&[event(1, "Unfinished")])).unwrap();
        let from_temp = storage.load();
        assert_eq!(from_temp.events[0].title, "Unfinished");
        assert_eq!(from_temp.notices, vec![Notice::RecoveredUnfinishedSave]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_first_save_left_in_a_temp_file_is_recovered() {
        let dir = temp_dir("first-save");
        let storage = Storage::new(&dir);
        let temp = dir.join(format!("{TEMP_PREFIX}7-0{TEMP_SUFFIX}"));
        fs::write(&temp, serialize(&[event(1, "First ever")])).unwrap();
        let loaded = storage.load();
        assert_eq!(loaded.events[0].title, "First ever");
        assert_eq!(loaded.notices, vec![Notice::RecoveredUnfinishedSave]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_temp_newer_than_the_main_file_wins_and_is_cleaned_up_after_a_save() {
        let dir = temp_dir("newer-temp");
        let storage = Storage::new(&dir);
        storage.save(&[event(1, "On disk")], false).unwrap();
        set_age(&storage.file_path(), 60);
        let temp = dir.join(format!("{TEMP_PREFIX}3-0{TEMP_SUFFIX}"));
        fs::write(&temp, serialize(&[event(1, "Newest")])).unwrap();
        let stale = dir.join(format!("{TEMP_PREFIX}2-0{TEMP_SUFFIX}"));
        fs::write(&stale, "{ partial").unwrap();

        let loaded = storage.load();
        assert_eq!(loaded.events[0].title, "Newest");
        assert_eq!(loaded.notices, vec![Notice::RecoveredUnfinishedSave]);

        storage.save(&loaded.events, false).unwrap();
        assert!(!temp.exists() && !stale.exists());
        assert!(storage.load().notices.is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn skipped_events_keep_one_copy_of_the_file_on_the_first_save() {
        let dir = temp_dir("skipped");
        let storage = Storage::new(&dir);
        let original = r#"{"schema_version":1,"events":[
            {"id":"local:a","title":"Ok","date":"2026-11-12","all_day":true},
            {"id":"local:b","title":"Bad","date":"2026-13-01","all_day":true}]}"#;
        fs::write(storage.file_path(), original).unwrap();
        let mut store = LocalEvents::from_loaded(storage.load());
        assert_eq!(store.notices(), &[Notice::SkippedEvents(1)]);
        store.add(&draft("New")).unwrap();
        let (events, copy) = store.take_save();
        assert!(copy);
        storage.save(&events, copy).unwrap();
        store.add(&draft("Another")).unwrap();
        let (events, copy) = store.take_save();
        assert!(!copy, "only the first save keeps a copy");
        storage.save(&events, copy).unwrap();

        let preserved: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(PRESERVED_PREFIX)
            })
            .collect();
        assert_eq!(preserved.len(), 1);
        assert_eq!(fs::read_to_string(preserved[0].path()).unwrap(), original);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn skipped_events_in_a_recovered_temp_file_are_kept() {
        let dir = temp_dir("skipped-temp");
        let storage = Storage::new(&dir);
        let original = r#"{"schema_version":1,"events":[
            {"id":"local:a","title":"Ok","date":"2026-11-12","all_day":true},
            {"id":"local:b","title":"Bad","date":"2026-13-01","all_day":true}]}"#;
        let temp = dir.join(format!("{TEMP_PREFIX}9-0{TEMP_SUFFIX}"));
        fs::write(&temp, original).unwrap();
        let mut store = LocalEvents::from_loaded(storage.load());
        assert_eq!(
            store.notices(),
            &[Notice::RecoveredUnfinishedSave, Notice::SkippedEvents(1)]
        );
        // The repair save removes the temp file; its copy stays.
        let (events, copy) = store.take_save();
        assert!(!copy, "already copied while loading");
        storage.save(&events, copy).unwrap();
        assert!(!temp.exists());
        let preserved: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(PRESERVED_PREFIX)
            })
            .collect();
        assert_eq!(preserved.len(), 1);
        assert_eq!(fs::read_to_string(preserved[0].path()).unwrap(), original);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn pruning_keeps_the_newest_preserved_copies_by_name() {
        let dir = temp_dir("prune");
        let storage = Storage::new(&dir);
        for n in 0..=MAX_PRESERVED_FILES as u64 {
            fs::write(
                dir.join(format!("{PRESERVED_PREFIX}{}.json", 1000 + n)),
                "x",
            )
            .unwrap();
        }
        // The newest copy keeps an old file's modified time, as a rename or copy does.
        let newest = dir.join(format!(
            "{PRESERVED_PREFIX}{}-1.json",
            1000 + MAX_PRESERVED_FILES as u64
        ));
        fs::write(&newest, "x").unwrap();
        set_age(&newest, 86_400 * 30);
        let renamed = dir.join(format!("{PRESERVED_PREFIX}keep.json"));
        fs::write(&renamed, "x").unwrap();
        set_age(&renamed, 86_400 * 60);
        storage.prune_preserved();
        assert!(newest.exists());
        assert!(
            renamed.exists(),
            "a name Lightcal did not write is never pruned"
        );
        assert!(!dir.join(format!("{PRESERVED_PREFIX}1000.json")).exists());
        assert!(!dir.join(format!("{PRESERVED_PREFIX}1001.json")).exists());
        assert!(dir.join(format!("{PRESERVED_PREFIX}1002.json")).exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_oversized_file_loads_as_delete_only_until_it_fits() {
        let dir = temp_dir("oversized");
        let storage = Storage::new(&dir);
        let events: Vec<_> = (0..=MAX_EVENTS as u32).map(|n| event(n, "Many")).collect();
        fs::write(storage.file_path(), serialize(&events)).unwrap();
        let mut store = LocalEvents::from_loaded(storage.load());
        assert_eq!(store.access(), &Access::DeleteOnly);
        assert_eq!(store.add(&draft("More")), Err(EditError::OverLimits));
        store.delete("local:test-0").unwrap();
        assert_eq!(store.access(), &Access::Editable);
        assert_eq!(store.add(&draft("One more")), Err(EditError::TooManyEvents));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_file_over_the_read_cap_is_malformed() {
        let dir = temp_dir("read-cap");
        let path = dir.join("big.json");
        let file = fs::File::create(&path).unwrap();
        file.set_len(READ_CAP_BYTES + 1).unwrap();
        assert!(matches!(read(&path), ReadOutcome::Malformed));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unavailable_events_cannot_be_changed() {
        let mut store = LocalEvents::loading();
        assert!(store.occurrences().is_empty());
        assert_eq!(store.add(&draft("x")), Err(EditError::ReadOnly));
        store.mark_lock_timeout();
        assert!(!store.is_loading());
        assert_eq!(store.access(), &Access::Unavailable(Unavailable::Locked));
        assert_eq!(store.delete("local:a"), Err(EditError::ReadOnly));
    }

    #[test]
    fn the_lock_admits_one_holder_and_survives_its_thread() {
        let dir = temp_dir("lock");
        let held = std::thread::spawn({
            let dir = dir.clone();
            move || EventsLock::try_acquire(&dir).unwrap().unwrap()
        })
        .join()
        .unwrap();
        // The acquiring thread has exited; the lock is still held through its handle.
        assert!(EventsLock::try_acquire(&dir).unwrap().is_none());
        drop(held);
        let again = EventsLock::try_acquire(&dir).unwrap();
        assert!(again.is_some());
        drop(again);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn separate_data_directories_never_share_the_lock() {
        let (a, b) = (temp_dir("lock-a"), temp_dir("lock-b"));
        let first = EventsLock::try_acquire(&a).unwrap();
        assert!(first.is_some());
        assert!(EventsLock::try_acquire(&b).unwrap().is_some());
        drop(first);
        fs::remove_dir_all(a).unwrap();
        fs::remove_dir_all(b).unwrap();
    }

    #[test]
    fn the_writer_saves_in_order_and_retries_a_failed_write() {
        let dir = temp_dir("writer");
        // A file where the data directory should be makes every save fail.
        let blocked = dir.join("blocked");
        fs::write(&blocked, "not a directory").unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let writer = Writer::spawn_with_backoff(
            Storage::new(&blocked),
            Duration::from_millis(20),
            Duration::from_millis(40),
            move |report| {
                let _ = sender.send(report);
            },
        )
        .unwrap();
        writer.submit(vec![event(1, "Waiting")], false);
        let first = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(first, WriteReport::Failed(_)));
        let retried = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(
            matches!(retried, WriteReport::Failed(_)),
            "retried on backoff"
        );
        assert!(writer.has_unsaved());
        assert!(!writer.wait_saved(Duration::from_millis(10)));

        // Clearing the obstacle lets the next retry succeed.
        fs::remove_file(&blocked).unwrap();
        let saved = loop {
            match receiver.recv_timeout(Duration::from_secs(5)).unwrap() {
                WriteReport::Saved => break true,
                WriteReport::Failed(_) => continue,
            }
        };
        assert!(saved);
        assert!(!writer.has_unsaved());
        assert!(!writer.last_write_failed());
        writer.submit(vec![event(1, "Later")], false);
        assert!(writer.wait_saved(Duration::from_secs(5)));
        assert_eq!(Storage::new(&blocked).load().events[0].title, "Later");
        writer.shutdown();
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_change_made_during_a_failed_save_is_not_marked_failed() {
        let mut state = WriterState {
            pending: Some((vec![event(2, "Newer")], false)),
            ..WriterState::default()
        };
        assert!(!state.record_failure(vec![event(1, "Older")], true));
        assert!(!state.failed, "the newer change has not been tried");
        assert!(state.newer_than_failure(), "it is written at once");
        let (events, copy) = state.pending.take().unwrap();
        assert_eq!(events[0].title, "Newer");
        assert!(copy, "the failed save's copy request carries over");

        assert!(state.record_failure(vec![event(1, "Older")], false));
        assert!(
            state.failed && !state.newer_than_failure(),
            "retried after the backoff"
        );
    }

    #[test]
    fn loading_the_same_file_again_adds_no_duplicate_copy() {
        let dir = temp_dir("copy-once");
        let storage = Storage::new(&dir);
        let original = r#"{"schema_version":1,"events":[
            {"id":"local:a","title":"Ok","date":"2026-11-12","all_day":true},
            {"id":"local:b","title":"Bad","date":"2026-13-01","all_day":true}]}"#;
        fs::write(dir.join(format!("{TEMP_PREFIX}4-0{TEMP_SUFFIX}")), original).unwrap();
        storage.load();
        storage.load();
        assert_eq!(storage.preserved_files().len(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failing_saves_keep_at_most_one_temp_file() {
        let dir = temp_dir("temp-pile");
        let storage = Storage::new(&dir);
        // A directory where the main file belongs makes every replace fail.
        fs::create_dir(storage.file_path()).unwrap();
        for n in 0..3 {
            assert!(storage.save(&[event(n, "Retry")], false).is_err());
        }
        assert_eq!(storage.temp_files().len(), 1, "older attempts are removed");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_recovered_load_asks_for_a_repair_save_once() {
        let dir = temp_dir("repair");
        let storage = Storage::new(&dir);
        storage.save(&[event(1, "First")], false).unwrap();
        storage.save(&[event(1, "Second")], false).unwrap();
        fs::remove_file(storage.file_path()).unwrap();
        let loaded = storage.load();
        assert!(loaded.repair);
        let mut store = LocalEvents::from_loaded(loaded);
        assert!(store.take_repair());
        assert!(!store.take_repair());
        let (events, copy) = store.take_save();
        storage.save(&events, copy).unwrap();
        let again = storage.load();
        assert!(again.notices.is_empty() && !again.repair);
        assert_eq!(again.events[0].title, "First");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_old_temp_never_beats_a_newer_schema_backup() {
        let dir = temp_dir("newer-backup");
        let storage = Storage::new(&dir);
        fs::write(storage.backup_path(), r#"{"schema_version":2,"events":[]}"#).unwrap();
        set_age(&storage.backup_path(), 60);
        let temp = dir.join(format!("{TEMP_PREFIX}1-0{TEMP_SUFFIX}"));
        fs::write(&temp, serialize(&[event(1, "Stale")])).unwrap();
        let loaded = storage.load();
        assert_eq!(
            loaded.access,
            Access::Unavailable(Unavailable::NewerVersion {
                file: storage.backup_path()
            })
        );
        assert!(!loaded.repair);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_cached_size_never_underestimates_the_file() {
        let mut store = LocalEvents::in_memory(Vec::new());
        let id = store.add(&draft("Dentist")).unwrap();
        let mut long = draft("Long");
        long.note = "n".repeat(MAX_NOTE_CHARS);
        store.add(&long).unwrap();
        store.update(&id, &draft("Dentist, moved")).unwrap();
        assert!(store.bytes >= serialize(store.events()).len() as u64);
        store.delete(&id).unwrap();
        assert!(store.bytes >= serialize(store.events()).len() as u64);
    }

    #[test]
    fn load_and_edit_use_the_same_size_measure() {
        let dir = temp_dir("size-measure");
        let storage = Storage::new(&dir);
        let events: Vec<_> = (0..3).map(|n| event(n, "Sized")).collect();
        storage.save(&events, false).unwrap();
        let loaded = storage.load();
        assert_eq!(loaded.bytes, file_bytes(&events));
        let store = LocalEvents::from_loaded(loaded);
        assert_eq!(store.bytes, file_bytes(store.events()));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn samples_are_valid_local_events() {
        let today = date(4);
        let samples = samples(today);
        assert_eq!(samples.len(), 3);
        assert!(samples.iter().all(|e| e.id.starts_with(EVENT_ID_PREFIX)));
        let store = LocalEvents::in_memory(samples);
        assert_eq!(store.occurrences().len(), 3);
    }

    #[test]
    fn every_language_names_the_local_calendar() {
        for locale in crate::preferences::LocaleCode::ALL {
            assert!(!calendar_name(locale).is_empty());
            assert!(!account_label(locale).is_empty());
        }
        assert_eq!(source(LocaleCode::En).name, "On this PC");
        assert_eq!(source(LocaleCode::En).account_label, "Local");
    }
}
