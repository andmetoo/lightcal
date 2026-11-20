//! Direct read-only providers; toolkit-independent inputs/outputs.
pub mod account;
pub mod google;
pub mod ics;
pub mod microsoft;
pub mod sign_in;
use crate::platform::ProviderStatus;
use crate::preferences::LocaleCode;
use crate::{platform::ProviderSnapshot, secure_store::Credentials};
use account::MicrosoftAccountState;
use base64::Engine;
use calendar_core::LocalDate;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub struct GoogleConfiguration {
    pub client_id: String,
    pub client_secret: String,
}

pub struct CalendarLoad {
    pub snapshot: ProviderSnapshot,
    pub microsoft: MicrosoftAccountState,
    pub microsoft_error: String,
    pub google_connected: bool,
    pub google_needs_signin: bool,
    pub google_configuration: Option<GoogleConfiguration>,
    pub google_label: Option<String>,
    pub google_error: String,
    pub google_skipped: usize,
    /// Each ICS feed's error or note by feed id; the cards and the ICS status line both
    /// read it.
    pub ics_feed_errors: BTreeMap<String, String>,
}

pub fn random_key() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|_| "Secure random generator is unavailable".to_string())?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

/// Ends a sign-in callback connection after its response is written. The callback reads
/// only the request line, so the browser's headers can still be unread; closing a socket
/// with unread input makes Windows reset the connection, which can discard the response
/// before the browser reads it. So: flush, send FIN, then read and drop what is left until
/// the browser closes its side, for at most a second in all and 64 KiB, so a client that
/// trickles bytes cannot hold up the sign-in loop.
pub(crate) fn close_callback_stream(stream: &mut std::net::TcpStream) {
    use std::io::{Read, Write};
    use std::time::{Duration, Instant};
    let _ = stream.flush();
    let _ = stream.shutdown(std::net::Shutdown::Write);
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut sink = [0u8; 4096];
    let mut drained = 0usize;
    while drained < 64 * 1024 {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() || stream.set_read_timeout(Some(left)).is_err() {
            break;
        }
        match stream.read(&mut sink) {
            Ok(0) | Err(_) => break,
            Ok(n) => drained += n,
        }
    }
}

pub fn range(anchor: LocalDate) -> (LocalDate, LocalDate) {
    let month = anchor.year_month();
    (
        LocalDate::new(month.year, month.month, 1)
            .unwrap()
            .add_days(-7),
        LocalDate::new(month.year, month.month, month.days_in_month())
            .unwrap()
            .add_days(15),
    )
}

/// Local wall-clock minutes ([`calendar_core::local_minutes`]) for a provider timestamp.
pub(crate) fn wall_minutes(value: chrono::NaiveDateTime) -> Option<i64> {
    use chrono::{Datelike, Timelike};
    let date = LocalDate::new(value.year(), value.month() as u8, value.day() as u8)?;
    Some(calendar_core::local_minutes(
        date,
        value.hour() * 60 + value.minute(),
    ))
}

/// Trimmed, single-line provider text capped at 256 characters; `None` when blank.
pub(crate) fn optional_text(raw: Option<&str>) -> Option<String> {
    let text: String = raw?
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|c| !c.is_control())
        .take(256)
        .collect();
    (!text.is_empty()).then_some(text)
}

/// Keeps a provider event link only when Lightcal would open it.
pub(crate) fn event_link(raw: Option<&str>) -> Option<String> {
    raw.filter(|url| crate::updater::is_allowed_event_link(url))
        .map(String::from)
}

/// "name: N events could not be read" with count-aware grammar in every supported language.
pub fn skipped_events_note(locale: LocaleCode, name: &str, count: usize) -> String {
    match locale {
        LocaleCode::En => {
            let noun = if count == 1 { "event" } else { "events" };
            format!("{name}: {count} {noun} could not be read")
        }
        LocaleCode::Ru => {
            let noun = crate::i18n::slavic_plural(count, "событие", "события", "событий");
            format!("{name}: не удалось прочитать {count} {noun}")
        }
        LocaleCode::Es => {
            if count == 1 {
                format!("{name}: no se pudo leer 1 evento")
            } else {
                format!("{name}: no se pudieron leer {count} eventos")
            }
        }
        LocaleCode::Ja => format!("{name}: {count} 件の予定を読み込めませんでした"),
        LocaleCode::De => {
            if count == 1 {
                format!("{name}: 1 Termin konnte nicht gelesen werden")
            } else {
                format!("{name}: {count} Termine konnten nicht gelesen werden")
            }
        }
        LocaleCode::Fr => {
            if count <= 1 {
                format!("{name}\u{a0}: {count} événement n’a pas pu être lu")
            } else {
                format!("{name}\u{a0}: {count} événements n’ont pas pu être lus")
            }
        }
        LocaleCode::PtBr => {
            if count == 1 {
                format!("{name}: não foi possível ler 1 evento")
            } else {
                format!("{name}: não foi possível ler {count} eventos")
            }
        }
        LocaleCode::Uk => {
            let noun = crate::i18n::slavic_plural(count, "подію", "події", "подій");
            format!("{name}: не вдалося прочитати {count} {noun}")
        }
        LocaleCode::ZhHans => format!("{name}：有 {count} 个事件无法读取"),
        LocaleCode::Ko => format!("{name}: 일정 {count}개를 읽을 수 없습니다"),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn query(
    anchor: LocalDate,
    dir: &Path,
    microsoft_client_id: &str,
    google_client_id: &str,
    google_client_secret: &str,
    selected: &BTreeSet<String>,
    legacy_windows: bool,
    locale: LocaleCode,
) -> CalendarLoad {
    let store = crate::secure_store::SecureStore::new(dir);
    // The credential lock covers only this local read. Provider requests below can hang, so
    // they run without it; refreshed tokens are written back through `save_rotated_token`.
    let mut credentials = {
        let _guard = match crate::secure_store::credential_lock() {
            Ok(guard) => guard,
            Err(error) => return failure(error),
        };
        match store.load() {
            Ok(c) => c,
            Err(error) => return failure(error),
        }
    };
    let mut snapshot = ProviderSnapshot {
        status: ProviderStatus::Available,
        sources: Vec::new(),
        occurrences: Vec::new(),
    };
    let mut errors = Vec::new();
    let mut account = MicrosoftAccountState::Disconnected;
    let mut microsoft_error = String::new();
    if credentials.microsoft_refresh_token.is_some() {
        account = MicrosoftAccountState::Unavailable {
            saved_credentials: true,
        };
        match microsoft::query(
            anchor,
            microsoft_client_id,
            selected,
            &mut credentials,
            &store,
            &mut account,
            locale,
        ) {
            Ok(data) => {
                if let ProviderStatus::ProviderUnavailable(ref e) = data.status {
                    microsoft_error = e.clone();
                }
                append_provider(&mut snapshot, data, &mut errors);
            }
            Err(e) => {
                // A hard failure (token refresh, Graph rejecting the request) also belongs
                // on the Outlook card; the card decides whether it reads as offline.
                microsoft_error = e.clone();
                errors.push(e);
            }
        }
    }
    // A transport/configuration failure does not disconnect saved credentials.
    // Keep manual refresh available unless authorization explicitly expired.
    let mut google_connected = credentials.google_refresh_token.is_some();
    let google_configuration = google_connected.then(|| GoogleConfiguration {
        client_id: credentials
            .google_client_id
            .clone()
            .unwrap_or_else(|| google_client_id.to_string()),
        client_secret: credentials
            .google_client_secret
            .clone()
            .unwrap_or_else(|| google_client_secret.to_string()),
    });
    let mut google_needs_signin = false;
    let mut google_label = None;
    let mut google_error = String::new();
    let mut google_skipped = 0;
    if credentials.google_refresh_token.is_some() {
        match google::query(
            anchor,
            google_client_id,
            google_client_secret,
            selected,
            &mut credentials,
            &store,
        ) {
            Ok(data) => {
                google_needs_signin = data.needs_signin;
                google_connected = !data.needs_signin;
                google_label = data.label;
                google_skipped = data.skipped;
                if let ProviderStatus::ProviderUnavailable(ref e) = data.snapshot.status {
                    google_error = e.clone();
                }
                append_provider(&mut snapshot, data.snapshot, &mut errors);
            }
            Err(e) => {
                google_needs_signin = crate::net::is_sign_in_required(&e);
                if google_needs_signin {
                    google_connected = false;
                } else {
                    google_error = e.clone();
                    errors.push(e);
                }
            }
        }
    }
    let mut ics_feed_errors = BTreeMap::new();
    for feed in credentials
        .ics
        .iter()
        .filter(|_| !cfg!(target_os = "macos"))
    {
        snapshot.sources.push(calendar_core::CalendarSource {
            id: feed.id.clone(),
            name: feed.name.clone(),
            account_label: "ICS · HTTPS".into(),
            color_hex: feed.color.clone(),
        });
        if selected.contains(&feed.id) {
            match ics::fetch(feed, anchor) {
                Ok(parsed) => {
                    // Cache canonical diagnostics; render warnings in the UI's
                    // current language, including after a language change.
                    if let Some(note) = parsed.note(LocaleCode::En, &feed.name) {
                        ics_feed_errors.insert(feed.id.clone(), note.clone());
                        errors.push(note);
                    }
                    snapshot.occurrences.extend(parsed.occurrences);
                }
                Err(e) => {
                    let message = format!("{}: {e}", feed.name);
                    ics_feed_errors.insert(feed.id.clone(), message.clone());
                    errors.push(message);
                }
            }
        }
    }
    if legacy_windows {
        #[cfg(target_os = "macos")]
        let data = crate::platform::macos::load(anchor, true).snapshot;
        #[cfg(not(target_os = "macos"))]
        let data = crate::platform::windows::query_calendar_provider(anchor, false);
        append_provider(&mut snapshot, data, &mut errors);
    }
    finish_status(&mut snapshot, &errors);
    CalendarLoad {
        snapshot,
        microsoft: account,
        microsoft_error,
        google_connected,
        google_needs_signin,
        google_configuration,
        google_label,
        google_error,
        google_skipped,
        ics_feed_errors,
    }
}

/// Folds one calendar's outcome (readable events and the count of unreadable ones) into the
/// snapshot and returns that count. A `hard` failure (sign-in) aborts the provider; any other
/// failure is recorded as "name: error" and the remaining calendars still load.
pub(crate) fn merge_calendar(
    snapshot: &mut ProviderSnapshot,
    errors: &mut Vec<String>,
    outcome: Result<(Vec<calendar_core::CalendarOccurrence>, usize), String>,
    name: &str,
    hard: bool,
) -> Result<usize, String> {
    match outcome {
        Ok((events, skipped)) => {
            snapshot.occurrences.extend(events);
            Ok(skipped)
        }
        Err(error) if hard => Err(error),
        Err(error) => {
            errors.push(format!("{name}: {error}"));
            Ok(0)
        }
    }
}

fn append_provider(
    snapshot: &mut ProviderSnapshot,
    data: ProviderSnapshot,
    errors: &mut Vec<String>,
) {
    match data.status {
        ProviderStatus::AccessDenied => snapshot.status = ProviderStatus::AccessDenied,
        ProviderStatus::ProviderUnavailable(error) => errors.push(error),
        _ => {}
    }
    snapshot.sources.extend(data.sources);
    snapshot.occurrences.extend(data.occurrences);
}

fn finish_status(snapshot: &mut ProviderSnapshot, errors: &[String]) {
    // Keep permission recovery available even when another provider reports a transient error.
    if snapshot.status == ProviderStatus::AccessDenied {
        return;
    }
    // A failure from one provider must not hide calendars another provider already loaded.
    snapshot.status = if snapshot.sources.is_empty() && !errors.is_empty() {
        ProviderStatus::ProviderUnavailable(errors.join("\n"))
    } else if snapshot.sources.is_empty() {
        ProviderStatus::NoCalendars
    } else {
        ProviderStatus::Available
    };
}

fn failure(error: String) -> CalendarLoad {
    CalendarLoad {
        microsoft: MicrosoftAccountState::Unavailable {
            saved_credentials: false,
        },
        microsoft_error: String::new(),
        google_connected: false,
        google_needs_signin: false,
        google_configuration: None,
        google_label: None,
        google_error: String::new(),
        google_skipped: 0,
        ics_feed_errors: BTreeMap::new(),
        snapshot: ProviderSnapshot {
            status: ProviderStatus::ProviderUnavailable(error),
            sources: Vec::new(),
            occurrences: Vec::new(),
        },
    }
}

pub fn add_feed(credentials: &mut Credentials, name: &str, raw: &str) -> Result<String, String> {
    let url = crate::net::https_url(raw.trim())?;
    if name.trim().is_empty() || name.len() > 100 {
        return Err("Enter a calendar name (1–100 characters)".into());
    }
    if credentials.ics.iter().any(|feed| feed.url == url.as_str()) {
        return Err("This subscription is already connected".into());
    }
    let id = format!("ics:{}", random_key()?);
    credentials.ics.push(crate::secure_store::IcsSubscription {
        id: id.clone(),
        name: name.trim().into(),
        url: url.into(),
        color: "#10B981".into(),
    });
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::time::{Duration, Instant};

    /// A loopback listener, or `None` where the sandbox forbids local sockets.
    fn loopback() -> Option<(TcpListener, u16)> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).ok()?;
        let port = listener.local_addr().ok()?.port();
        Some((listener, port))
    }

    #[test]
    fn callback_response_survives_unread_request_headers() {
        let Some((listener, port)) = loopback() else {
            return;
        };
        for _ in 0..20 {
            let client = std::thread::spawn(move || {
                let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                write!(
                    stream,
                    "GET /callback?code=x HTTP/1.1\r\nHost: localhost\r\nUser-Agent: test\r\nAccept: */*\r\n\r\n"
                )
                .unwrap();
                let mut response = String::new();
                stream.read_to_string(&mut response).unwrap();
                response
            });
            let (mut stream, _) = listener.accept().unwrap();
            let mut line = String::new();
            BufReader::new((&mut stream).take(64))
                .read_line(&mut line)
                .unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
                .unwrap();
            close_callback_stream(&mut stream);
            drop(stream);
            assert!(client.join().unwrap().ends_with("\r\n\r\nok"));
        }
    }

    #[test]
    fn callback_drain_stops_at_its_deadline_for_a_trickling_client() {
        let Some((listener, port)) = loopback() else {
            return;
        };
        let client = std::thread::spawn(move || {
            let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
            // One byte every 300 ms for 6 s: never idle long enough for a per-read timeout.
            for _ in 0..20 {
                if stream.write_all(b"x").is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(300));
            }
        });
        let (mut stream, _) = listener.accept().unwrap();
        let started = Instant::now();
        close_callback_stream(&mut stream);
        let took = started.elapsed();
        drop(stream);
        assert!(took < Duration::from_secs(3), "drain took {took:?}");
        client.join().unwrap();
    }

    fn empty(status: ProviderStatus) -> ProviderSnapshot {
        ProviderSnapshot {
            status,
            sources: Vec::new(),
            occurrences: Vec::new(),
        }
    }
    #[test]
    fn skipped_note_is_count_aware_in_english() {
        assert_eq!(
            skipped_events_note(LocaleCode::En, "Work", 1),
            "Work: 1 event could not be read"
        );
        assert_eq!(
            skipped_events_note(LocaleCode::En, "Work", 2),
            "Work: 2 events could not be read"
        );
    }

    #[test]
    fn skipped_note_uses_russian_plural_forms() {
        let note = |count| skipped_events_note(LocaleCode::Ru, "Работа", count);
        for (count, noun) in [
            (1, "событие"),
            (2, "события"),
            (4, "события"),
            (5, "событий"),
            (11, "событий"),
            (12, "событий"),
            (14, "событий"),
            (21, "событие"),
            (22, "события"),
            (25, "событий"),
            (100, "событий"),
            (101, "событие"),
            (111, "событий"),
        ] {
            assert_eq!(
                note(count),
                format!("Работа: не удалось прочитать {count} {noun}")
            );
        }
    }

    #[test]
    fn skipped_note_is_localized_for_spanish_and_japanese() {
        assert_eq!(
            skipped_events_note(LocaleCode::Es, "Trabajo", 1),
            "Trabajo: no se pudo leer 1 evento"
        );
        assert_eq!(
            skipped_events_note(LocaleCode::Es, "Trabajo", 3),
            "Trabajo: no se pudieron leer 3 eventos"
        );
        assert_eq!(
            skipped_events_note(LocaleCode::Ja, "仕事", 3),
            "仕事: 3 件の予定を読み込めませんでした"
        );
    }

    #[test]
    fn denied_provider_is_not_downgraded_to_no_calendars() {
        let mut snapshot = empty(ProviderStatus::Available);
        let mut errors = Vec::new();
        append_provider(
            &mut snapshot,
            empty(ProviderStatus::AccessDenied),
            &mut errors,
        );
        finish_status(&mut snapshot, &errors);
        assert_eq!(snapshot.status, ProviderStatus::AccessDenied);
    }
    #[test]
    fn permission_denial_retains_other_providers_data() {
        let anchor = LocalDate::new(2026, 10, 30).unwrap();
        let mut snapshot = ProviderSnapshot::synthetic(anchor);
        let sources = snapshot.sources.clone();
        let occurrences = snapshot.occurrences.clone();
        let mut errors = vec!["Outlook is temporarily offline".into()];
        append_provider(
            &mut snapshot,
            empty(ProviderStatus::AccessDenied),
            &mut errors,
        );
        append_provider(&mut snapshot, empty(ProviderStatus::Available), &mut errors);
        finish_status(&mut snapshot, &errors);
        assert_eq!(snapshot.status, ProviderStatus::AccessDenied);
        assert_eq!(snapshot.sources, sources);
        assert_eq!(snapshot.occurrences, occurrences);
    }
    #[test]
    fn empty_and_unavailable_providers_keep_distinct_statuses() {
        let mut snapshot = empty(ProviderStatus::Available);
        finish_status(&mut snapshot, &[]);
        assert_eq!(snapshot.status, ProviderStatus::NoCalendars);
        let mut errors = Vec::new();
        append_provider(
            &mut snapshot,
            empty(ProviderStatus::ProviderUnavailable("offline".into())),
            &mut errors,
        );
        finish_status(&mut snapshot, &errors);
        assert_eq!(
            snapshot.status,
            ProviderStatus::ProviderUnavailable("offline".into())
        );
    }
    #[test]
    fn a_provider_error_keeps_calendars_already_loaded() {
        let mut snapshot = empty(ProviderStatus::Available);
        snapshot.sources.push(calendar_core::CalendarSource {
            id: "graph:primary".into(),
            name: "Calendar".into(),
            account_label: "Outlook".into(),
            color_hex: "#00796B".into(),
        });
        finish_status(
            &mut snapshot,
            &["Google calendar request failed (HTTP 500)".into()],
        );
        assert_eq!(snapshot.status, ProviderStatus::Available);
        assert_eq!(snapshot.sources.len(), 1);
    }

    #[cfg(windows)]
    #[test]
    fn a_saved_google_account_remains_retryable_after_a_query_failure() {
        let dir =
            std::env::temp_dir().join(format!("lightcal-google-retry-{}", random_key().unwrap()));
        let store = crate::secure_store::SecureStore::new(&dir);
        let credentials = Credentials {
            google_refresh_token: Some("fixture-refresh-token".into()),
            // Invalid configuration exercises the failure branch without network traffic.
            google_client_id: Some("invalid-fixture-client".into()),
            google_client_secret: Some("fixture-desktop-secret".into()),
            ..Credentials::default()
        };
        store.save(&credentials).unwrap();
        let result = query(
            LocalDate::new(2026, 11, 1).unwrap(),
            &dir,
            "",
            "",
            "",
            &BTreeSet::new(),
            false,
            LocaleCode::En,
        );
        assert!(result.google_connected);
        assert!(!result.google_needs_signin);
        assert!(!result.google_error.is_empty());
        let configuration = result.google_configuration.unwrap();
        assert_eq!(configuration.client_id, "invalid-fixture-client");
        assert_eq!(configuration.client_secret, "fixture-desktop-secret");
        assert!(store.load().unwrap().google_refresh_token.is_some());
        std::fs::remove_file(dir.join("credentials.dpapi")).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
