//! Microsoft Graph delegated read-only Calendar API and desktop PKCE sign-in.
use super::account::MicrosoftAccountState;
use super::{random_key, range};
use crate::{
    platform::{ProviderSnapshot, ProviderStatus},
    secure_store::{Credentials, SecureStore},
};
use base64::Engine;
use calendar_core::{CalendarOccurrence, CalendarSource, LocalDate};
use chrono::{Datelike, Timelike};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    time::{Duration, Instant},
};

const TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const SCOPES: &str = "https://graph.microsoft.com/Calendars.Read offline_access";
const CALLBACK_PATH: &str = "/lightcalendar/oauth/microsoft";

fn callback_listener() -> Result<TcpListener, String> {
    let socket = socket2::Socket::new(
        socket2::Domain::IPV4,
        socket2::Type::STREAM,
        Some(socket2::Protocol::TCP),
    )
    .map_err(|_| "Could not create sign-in socket")?;
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawSocket;
        use windows_sys::Win32::Networking::WinSock::{
            setsockopt, SOCKET_ERROR, SOL_SOCKET, SO_EXCLUSIVEADDRUSE,
        };
        let enabled = 1i32;
        // Must be applied before bind. A competing SO_REUSEADDR socket must not share this endpoint.
        let result = unsafe {
            setsockopt(
                socket.as_raw_socket() as usize,
                SOL_SOCKET,
                SO_EXCLUSIVEADDRUSE,
                (&enabled as *const i32).cast(),
                std::mem::size_of_val(&enabled) as i32,
            )
        };
        if result == SOCKET_ERROR {
            return Err("Could not reserve an exclusive Windows sign-in socket".into());
        }
    }
    let address = std::net::SocketAddr::from(([127, 0, 0, 1], 0));
    socket
        .bind(&address.into())
        .map_err(|_| "Could not bind local sign-in callback")?;
    socket
        .listen(8)
        .map_err(|_| "Could not listen for sign-in callback")?;
    let listener: TcpListener = socket.into();
    listener
        .set_nonblocking(true)
        .map_err(|_| "Could not configure sign-in callback")?;
    Ok(listener)
}

pub fn valid_client_id(raw: &str) -> bool {
    raw.len() == 36
        && raw.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

pub fn connect(client_id: &str, open_browser: impl FnOnce(String)) -> Result<String, String> {
    if !valid_client_id(client_id) {
        return Err("Configure a Microsoft Entra Desktop app Client ID first (see provider setup documentation)".into());
    }
    let listener = callback_listener()?;
    let redirect = format!(
        "http://localhost:{}{CALLBACK_PATH}",
        listener
            .local_addr()
            .map_err(|_| "Could not configure sign-in port")?
            .port()
    );
    let verifier = random_key()?;
    let state = random_key()?;
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(verifier.as_bytes()));
    let mut auth =
        url::Url::parse("https://login.microsoftonline.com/common/oauth2/v2.0/authorize").unwrap();
    auth.query_pairs_mut().extend_pairs([
        ("client_id", client_id),
        ("response_type", "code"),
        ("redirect_uri", redirect.as_str()),
        ("response_mode", "query"),
        ("scope", SCOPES),
        ("state", state.as_str()),
        ("code_challenge", challenge.as_str()),
        ("code_challenge_method", "S256"),
        ("prompt", "select_account"),
    ]);
    open_browser(auth.into());
    let deadline = Instant::now() + Duration::from_secs(180);
    let code = loop {
        if Instant::now() >= deadline {
            return Err("Microsoft sign-in timed out; try connecting again".into());
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                let mut line = String::new();
                if BufReader::new((&mut stream).take(8192))
                    .read_line(&mut line)
                    .is_err()
                {
                    continue;
                }
                let result = callback_code(&line, &state);
                let success = result.as_ref().is_ok_and(|code| code.is_some());
                let body = if success {
                    "Authorization response received. Return to Lightcal to see the connection result."
                } else {
                    "Sign-in callback rejected. Return to Lightcal or finish sign-in in the original browser tab."
                };
                let response = format!("HTTP/1.1 {}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", if success { "200 OK" } else { "400 Bad Request" }, body.len(), body);
                let _ = stream.write_all(response.as_bytes());
                match result {
                    Ok(Some(code)) => break code,
                    Err(error) if error == "Microsoft sign-in was cancelled" => return Err(error),
                    _ => {}
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100))
            }
            Err(_) => return Err("Local sign-in callback failed".into()),
        }
    };
    let response = crate::net::post_form(
        TOKEN_URL,
        &[
            ("client_id", client_id),
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect),
            ("code_verifier", &verifier),
            ("scope", SCOPES),
        ],
    )?;
    response["refresh_token"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(String::from)
        .ok_or_else(|| {
            "Microsoft did not grant offline calendar access; reconnect and consent".into()
        })
}

fn callback_code(line: &str, expected_state: &str) -> Result<Option<String>, String> {
    let mut parts = line.split_whitespace();
    if parts.next() != Some("GET") {
        return Ok(None);
    }
    let target = parts.next().ok_or("Invalid sign-in callback")?;
    if target.split('?').next() != Some(CALLBACK_PATH) {
        return Ok(None);
    }
    let callback = url::Url::parse(&format!("http://localhost{target}"))
        .map_err(|_| "Invalid sign-in callback")?;
    if callback.path() != CALLBACK_PATH || callback.fragment().is_some() {
        return Ok(None);
    }
    let params: Vec<_> = callback.query_pairs().collect();
    if params
        .iter()
        .any(|(key, _)| key == "access_token" || key == "id_token")
    {
        return Err("Unexpected token in authorization-code callback".into());
    }
    let states: Vec<_> = params.iter().filter(|(key, _)| key == "state").collect();
    if states.len() != 1
        || states[0].1.len() != expected_state.len()
        || states[0]
            .1
            .bytes()
            .zip(expected_state.bytes())
            .fold(0u8, |diff, (a, b)| diff | (a ^ b))
            != 0
    {
        return Err("Invalid sign-in state".into());
    }
    if params.iter().any(|(key, _)| key == "error") {
        return Err("Microsoft sign-in was cancelled".into());
    }
    let codes: Vec<_> = params.iter().filter(|(key, _)| key == "code").collect();
    if codes.len() != 1 || codes[0].1.is_empty() {
        return Err("Microsoft returned no authorization code".into());
    }
    Ok(Some(codes[0].1.to_string()))
}

fn graph_pages(
    raw: &str,
    token: &str,
    account: &mut MicrosoftAccountState,
) -> Result<Vec<Value>, String> {
    let mut next = Some(raw.to_string());
    let mut items = Vec::new();
    for _ in 0..100 {
        let Some(current) = next.take() else {
            return Ok(items);
        };
        let url = crate::net::https_url(&current)?;
        if url.host_str() != Some("graph.microsoft.com")
            || url.port().is_some()
            || !url.path().starts_with("/v1.0/")
        {
            return Err("Rejected an unexpected Microsoft Graph page address".into());
        }
        let (status, bytes) = crate::net::get_with_headers(
            &current,
            Some(token),
            &[("Prefer", "outlook.timezone=\"UTC\"")],
            8 * 1024 * 1024,
        )?;
        if status == 401 {
            *account = MicrosoftAccountState::NeedsSignIn;
        }
        if status != 200 {
            return Err(format!(
                "Microsoft calendar request failed (HTTP {status}); reconnect for expired consent"
            ));
        }
        let page: Value =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid Microsoft calendar response")?;
        items.extend(
            page["value"]
                .as_array()
                .ok_or("Microsoft response has no calendar items")?
                .iter()
                .cloned(),
        );
        if items.len() > 50_000 {
            return Err("Microsoft calendar response exceeded the supported event limit".into());
        }
        next = page["@odata.nextLink"].as_str().map(String::from);
    }
    Err("Microsoft calendar pagination exceeded the supported limit".into())
}

pub fn query(
    anchor: LocalDate,
    client_id: &str,
    selected: &BTreeSet<String>,
    credentials: &mut Credentials,
    store: &SecureStore,
    account: &mut MicrosoftAccountState,
) -> Result<ProviderSnapshot, String> {
    let configured_client = credentials.microsoft_client_id.clone();
    let client_id = configured_client.as_deref().unwrap_or(client_id);
    if !valid_client_id(client_id) {
        return Err("Set the Microsoft Desktop app Client ID before refreshing".into());
    }
    let refresh = credentials
        .microsoft_refresh_token
        .as_deref()
        .ok_or("Connect Microsoft first")?;
    let response = crate::net::post_form_detailed(
        TOKEN_URL,
        &[
            ("client_id", client_id),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh),
            ("scope", SCOPES),
        ],
    )
    .map_err(|error| {
        if matches!(error, crate::net::AuthenticationError::SignInRequired) {
            *account = MicrosoftAccountState::NeedsSignIn;
        }
        error.message()
    })?;
    let token = response["access_token"]
        .as_str()
        .ok_or("Microsoft returned no calendar access token")?;
    if let Some(rotated) = response["refresh_token"].as_str() {
        credentials.microsoft_refresh_token = Some(rotated.to_string());
        store.save(credentials)?;
    }
    *account = MicrosoftAccountState::Connected { label: None };
    let calendars = graph_pages(
        "https://graph.microsoft.com/v1.0/me/calendars?$top=100",
        token,
        account,
    )?;
    let owner = calendars
        .iter()
        .find(|calendar| calendar["isDefaultCalendar"].as_bool() == Some(true))
        .and_then(|calendar| calendar["owner"]["address"].as_str())
        .filter(|address| !address.is_empty())
        .map(String::from);
    if let MicrosoftAccountState::Connected { label } = account {
        *label = owner;
    }
    let (from, to) = range(anchor);
    let mut result = ProviderSnapshot {
        status: ProviderStatus::Available,
        sources: Vec::new(),
        occurrences: Vec::new(),
    };
    for calendar in calendars {
        let remote_id = calendar["id"]
            .as_str()
            .ok_or("Microsoft returned a calendar without an ID")?;
        let id = format!("graph:{remote_id}");
        let color = calendar["hexColor"]
            .as_str()
            .filter(|s| {
                s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| b.is_ascii_hexdigit())
            })
            .unwrap_or("#008573");
        result.sources.push(CalendarSource {
            id: id.clone(),
            name: calendar["name"].as_str().unwrap_or("Outlook").into(),
            account_label: "Microsoft Graph · Outlook".into(),
            color_hex: color.into(),
        });
        if !selected.contains(&id) {
            continue;
        }
        let mut uri = url::Url::parse("https://graph.microsoft.com/v1.0/me/calendars/").unwrap();
        uri.path_segments_mut()
            .map_err(|_| "Invalid Graph path")?
            .pop_if_empty()
            .push(remote_id)
            .push("calendarView");
        uri.query_pairs_mut()
            .append_pair(
                "startDateTime",
                &format!(
                    "{:04}-{:02}-{:02}T00:00:00Z",
                    from.year, from.month, from.day
                ),
            )
            .append_pair(
                "endDateTime",
                &format!("{:04}-{:02}-{:02}T00:00:00Z", to.year, to.month, to.day),
            )
            .append_pair("$top", "1000");
        for event in graph_pages(uri.as_str(), token, account)? {
            if event["isCancelled"].as_bool() == Some(true) {
                continue;
            }
            result.occurrences.push(parse_event(&event, &id, color)?);
        }
    }
    Ok(result)
}

fn graph_datetime(value: &Value) -> Result<chrono::DateTime<chrono::Utc>, String> {
    let raw = value["dateTime"]
        .as_str()
        .ok_or("Microsoft event has no date/time")?;
    if let Ok(date) = chrono::DateTime::parse_from_rfc3339(raw) {
        return Ok(date.with_timezone(&chrono::Utc));
    }
    if value["timeZone"].as_str() != Some("UTC") {
        return Err("Microsoft returned an unsupported event timezone".into());
    }
    chrono::DateTime::parse_from_rfc3339(&format!("{raw}Z"))
        .map(|d| d.with_timezone(&chrono::Utc))
        .map_err(|_| "Microsoft event has an invalid date/time".into())
}

fn parse_event(event: &Value, source: &str, color: &str) -> Result<CalendarOccurrence, String> {
    let start = graph_datetime(&event["start"])?;
    let end = graph_datetime(&event["end"])?;
    if end < start {
        return Err("Microsoft event ends before its start".into());
    }
    let all_day = event["isAllDay"].as_bool().unwrap_or(false);
    // Graph all-day dates are calendar-local dates; preserve them before local conversion.
    let date = |dt: chrono::NaiveDate| {
        LocalDate::new(dt.year(), dt.month() as u8, dt.day() as u8)
            .ok_or_else(|| "Event date is outside supported range".to_string())
    };
    let original_zone = event["originalStartTimeZone"].as_str().unwrap_or("UTC");
    let start_date = date(if all_day {
        crate::platform::windows::timezone::calendar_date(start, original_zone)?
    } else {
        start.with_timezone(&chrono::Local).date_naive()
    })?;
    let end_local = end.with_timezone(&chrono::Local);
    let end_date = date(if all_day {
        crate::platform::windows::timezone::calendar_date(
            end,
            event["originalEndTimeZone"]
                .as_str()
                .unwrap_or(original_zone),
        )?
    } else {
        end_local.date_naive()
    })?;
    let end_exclusive =
        if all_day || (end_local.time() == chrono::NaiveTime::MIN && end_date > start_date) {
            end_date
        } else {
            end_date.add_days(1)
        }
        .max(start_date.add_days(1));
    Ok(CalendarOccurrence {
        source_id: source.into(),
        event_id: event["id"]
            .as_str()
            .ok_or("Microsoft event has no ID")?
            .into(),
        color_hex: color.into(),
        start_date,
        end_date_exclusive: end_exclusive,
        is_all_day: all_day,
        title: event["subject"].as_str().unwrap_or("").into(),
        time_label: if all_day {
            String::new()
        } else {
            format!(
                "{:02}:{:02}",
                start.with_timezone(&chrono::Local).hour(),
                start.with_timezone(&chrono::Local).minute()
            )
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    #[test]
    fn desktop_callback_uses_registered_path_and_cancellation_never_reaches_token_exchange() {
        let mut responder = None;
        let result = connect("00000000-0000-0000-0000-000000000000", |raw| {
            let auth = url::Url::parse(&raw).unwrap();
            let params = auth.query_pairs().collect::<BTreeMap<_, _>>();
            let redirect = url::Url::parse(params.get("redirect_uri").unwrap()).unwrap();
            assert_eq!(redirect.path(), "/lightcalendar/oauth/microsoft");
            assert_eq!(params.get("code_challenge_method").unwrap(), "S256");
            let state = params.get("state").unwrap().to_string();
            let port = redirect.port().unwrap();
            responder = Some(std::thread::spawn(move || {
                let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                write!(stream,"GET /lightcalendar/oauth/microsoft?error=access_denied&state={state} HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n").unwrap();
                let mut response = String::new();
                stream.read_to_string(&mut response).unwrap();
                assert!(response.starts_with("HTTP/1.1 400"));
            }));
        });
        assert_eq!(result.unwrap_err(), "Microsoft sign-in was cancelled");
        responder.unwrap().join().unwrap();
    }

    #[test]
    fn callback_requires_matching_state_and_one_code() {
        assert_eq!(
            callback_code(
                "GET /lightcalendar/oauth/microsoft?code=abc&state=nonce HTTP/1.1",
                "nonce"
            )
            .unwrap(),
            Some("abc".into())
        );
        assert!(callback_code(
            "GET /lightcalendar/oauth/microsoft?code=abc&state=wrong HTTP/1.1",
            "nonce"
        )
        .is_err());
        assert!(callback_code(
            "GET /lightcalendar/oauth/microsoft?code=a&code=b&state=nonce HTTP/1.1",
            "nonce"
        )
        .is_err());
        assert!(callback_code(
            "GET /lightcalendar/oauth/microsoft?error=access_denied&state=nonce HTTP/1.1",
            "nonce"
        )
        .is_err());
        assert!(callback_code(
            "GET /lightcalendar/oauth/microsoft?code=abc&state=nonce&access_token=unexpected HTTP/1.1",
            "nonce"
        )
        .is_err());
    }

    #[test]
    fn callback_rejects_root_wrong_path_and_path_aliases_even_with_valid_state() {
        for path in [
            "/",
            "/wrong",
            "/lightcalendar/oauth/microsoft/",
            "/lightcalendar/oauth/other/../microsoft",
            "/lightcalendar/oauth/%6dicrosoft",
            "/oauth/microsoft/callback",
            "/lighcalendar/oauth/microsoft",
        ] {
            assert_eq!(
                callback_code(
                    &format!("GET {path}?code=abc&state=nonce HTTP/1.1"),
                    "nonce"
                )
                .unwrap(),
                None
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_callback_socket_refuses_competing_reuse_bind() {
        let listener = callback_listener().unwrap();
        let address = listener.local_addr().unwrap();
        assert!(address.ip().is_loopback());
        let competing = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )
        .unwrap();
        competing.set_reuse_address(true).unwrap();
        assert!(competing.bind(&address.into()).is_err());
    }
    #[test]
    fn graph_all_day_end_is_exclusive_and_cancelled_days_are_not_invented() {
        let raw = serde_json::json!({"id":"a","subject":"trip","isAllDay":true,"start":{"dateTime":"2026-10-30T00:00:00","timeZone":"UTC"},"end":{"dateTime":"2026-11-02T00:00:00","timeZone":"UTC"}});
        let event = parse_event(&raw, "graph:a", "#123456").unwrap();
        assert_eq!(event.start_date, LocalDate::new(2026, 10, 30).unwrap());
        assert_eq!(
            event.end_date_exclusive,
            LocalDate::new(2026, 11, 2).unwrap()
        );
        let shifted = serde_json::json!({"id":"b","isAllDay":true,"originalStartTimeZone":"Europe/Belgrade","originalEndTimeZone":"Europe/Belgrade","start":{"dateTime":"2026-10-29T23:00:00","timeZone":"UTC"},"end":{"dateTime":"2026-10-30T23:00:00","timeZone":"UTC"}});
        let event = parse_event(&shifted, "graph:a", "#123456").unwrap();
        assert_eq!(event.start_date, LocalDate::new(2026, 10, 30).unwrap());
        assert_eq!(
            event.end_date_exclusive,
            LocalDate::new(2026, 10, 31).unwrap()
        );
    }
}
