//! Google Calendar delegated read-only API and desktop PKCE sign-in.
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

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const SCOPES: &str = "https://www.googleapis.com/auth/calendar.calendarlist.readonly https://www.googleapis.com/auth/calendar.events.readonly";

pub fn valid_client_id(raw: &str) -> bool {
    let Some((left, right)) = raw.rsplit_once('.') else {
        return false;
    };
    right == "apps.googleusercontent.com"
        && left.contains('-')
        && left.len() > 8
        && left
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

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

pub fn connect(client_id: &str, open_browser: impl FnOnce(String)) -> Result<String, String> {
    if !valid_client_id(client_id) {
        return Err("Configure a Google Desktop OAuth client ID first".into());
    }
    let listener = callback_listener()?;
    let port = listener
        .local_addr()
        .map_err(|_| "Could not configure sign-in port")?
        .port();
    let redirect = format!("http://127.0.0.1:{port}");
    let verifier = random_key()?;
    let state = random_key()?;
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(verifier.as_bytes()));
    let mut auth = url::Url::parse(AUTH_URL).unwrap();
    auth.query_pairs_mut().extend_pairs([
        ("client_id", client_id),
        ("redirect_uri", redirect.as_str()),
        ("response_type", "code"),
        ("scope", SCOPES),
        ("state", state.as_str()),
        ("code_challenge", challenge.as_str()),
        ("code_challenge_method", "S256"),
        ("access_type", "offline"),
        ("prompt", "consent"),
    ]);
    open_browser(auth.into());
    let deadline = Instant::now() + Duration::from_secs(180);
    let code = loop {
        if Instant::now() >= deadline {
            return Err("Google sign-in timed out; try connecting again".into());
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
                let response = format!(
                    "HTTP/1.1 {}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    if success { "200 OK" } else { "400 Bad Request" },
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
                match result {
                    Ok(Some(code)) => break code,
                    Err(error) if error == "Google sign-in was cancelled" => return Err(error),
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
        ],
    )?;
    response["refresh_token"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(String::from)
        .ok_or_else(|| "Google did not grant offline calendar access; reconnect and consent".into())
}

fn callback_code(line: &str, expected_state: &str) -> Result<Option<String>, String> {
    let mut parts = line.split_whitespace();
    if parts.next() != Some("GET") {
        return Ok(None);
    }
    let target = parts.next().ok_or("Invalid sign-in callback")?;
    let path = target.split('?').next().unwrap_or("");
    if path != "/" && !path.is_empty() {
        return Ok(None);
    }
    let callback = url::Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|_| "Invalid sign-in callback")?;
    if callback.fragment().is_some() {
        return Ok(None);
    }
    let params: Vec<_> = callback.query_pairs().collect();
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
        return Err("Google sign-in was cancelled".into());
    }
    let codes: Vec<_> = params.iter().filter(|(key, _)| key == "code").collect();
    if codes.len() != 1 || codes[0].1.is_empty() {
        return Err("Google returned no authorization code".into());
    }
    Ok(Some(codes[0].1.to_string()))
}

pub struct GoogleQuery {
    pub snapshot: ProviderSnapshot,
    pub needs_signin: bool,
    pub label: Option<String>,
}

pub fn query(
    anchor: LocalDate,
    client_id: &str,
    selected: &BTreeSet<String>,
    credentials: &mut Credentials,
    store: &SecureStore,
) -> Result<GoogleQuery, String> {
    let configured = credentials.google_client_id.clone();
    let client_id = configured.as_deref().unwrap_or(client_id);
    if !valid_client_id(client_id) {
        return Err("Set the Google Desktop client ID before refreshing".into());
    }
    let refresh = credentials
        .google_refresh_token
        .as_deref()
        .ok_or("Connect Google first")?;
    let response = crate::net::post_form_detailed(
        TOKEN_URL,
        &[
            ("client_id", client_id),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh),
        ],
    )
    .map_err(|error| error.message())?;
    let token = response["access_token"]
        .as_str()
        .ok_or("Google returned no calendar access token")?;
    if let Some(rotated) = response["refresh_token"].as_str() {
        credentials.google_refresh_token = Some(rotated.to_string());
        store.save(credentials)?;
    }
    let calendars = pages(
        "https://www.googleapis.com/calendar/v3/users/me/calendarList?maxResults=100",
        token,
    )?;
    let label = calendars
        .iter()
        .find(|calendar| calendar["primary"].as_bool() == Some(true))
        .and_then(|calendar| calendar["id"].as_str())
        .filter(|id| id.contains('@'))
        .map(str::to_string);
    let (from, to) = range(anchor);
    let mut snapshot = ProviderSnapshot {
        status: ProviderStatus::Available,
        sources: Vec::new(),
        occurrences: Vec::new(),
    };
    for calendar in calendars {
        let remote_id = calendar["id"]
            .as_str()
            .ok_or("Google returned a calendar without an ID")?;
        let id = format!("google:{remote_id}");
        let color = calendar["backgroundColor"]
            .as_str()
            .filter(|s| {
                s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| b.is_ascii_hexdigit())
            })
            .unwrap_or("#008573");
        snapshot.sources.push(CalendarSource {
            id: id.clone(),
            name: calendar["summary"].as_str().unwrap_or("Google").into(),
            account_label: "Google Calendar".into(),
            color_hex: color.into(),
        });
        if !selected.contains(&id) {
            continue;
        }
        let mut uri = url::Url::parse("https://www.googleapis.com/calendar/v3/calendars/").unwrap();
        uri.path_segments_mut()
            .map_err(|_| "Invalid Google path")?
            .pop_if_empty()
            .push(remote_id)
            .push("events");
        uri.query_pairs_mut()
            .append_pair("singleEvents", "true")
            .append_pair("orderBy", "startTime")
            .append_pair("maxResults", "250")
            .append_pair(
                "timeMin",
                &format!(
                    "{:04}-{:02}-{:02}T00:00:00Z",
                    from.year, from.month, from.day
                ),
            )
            .append_pair(
                "timeMax",
                &format!("{:04}-{:02}-{:02}T00:00:00Z", to.year, to.month, to.day),
            );
        for event in pages(uri.as_str(), token)? {
            if event["status"].as_str() == Some("cancelled") {
                continue;
            }
            snapshot.occurrences.push(parse_event(&event, &id, color)?);
        }
    }
    Ok(GoogleQuery {
        snapshot,
        needs_signin: false,
        label,
    })
}

fn pages(raw: &str, token: &str) -> Result<Vec<Value>, String> {
    let mut next = Some(raw.to_string());
    let mut items = Vec::new();
    for _ in 0..20 {
        let Some(current) = next.take() else {
            return Ok(items);
        };
        let url = crate::net::https_url(&current)?;
        if url.host_str() != Some("www.googleapis.com") || url.port().is_some() {
            return Err("Rejected an unexpected Google Calendar address".into());
        }
        let (status, bytes) = crate::net::get(&current, Some(token), 8 * 1024 * 1024)?;
        if status == 401 {
            return Err("Sign-in expired or consent was revoked; sign in again".into());
        }
        if status != 200 {
            return Err(format!("Google calendar request failed (HTTP {status})"));
        }
        let page: Value =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid Google calendar response")?;
        let list = page["items"].as_array().cloned().unwrap_or_default();
        items.extend(list);
        if items.len() > 20_000 {
            return Err("Google calendar response exceeded the supported event limit".into());
        }
        next = page["nextPageToken"].as_str().map(|token| {
            let mut continued = url::Url::parse(&current).unwrap();
            continued.query_pairs_mut().append_pair("pageToken", token);
            continued.to_string()
        });
    }
    Err("Google calendar pagination exceeded the supported limit".into())
}

fn parse_event(event: &Value, source: &str, color: &str) -> Result<CalendarOccurrence, String> {
    let all_day = event["start"]["date"].is_string();
    let start = google_bound(&event["start"], true)?;
    let end = google_bound(&event["end"], false)?;
    if end.0 < start.0 && !all_day {
        return Err("Google event ends before its start".into());
    }
    let date = |dt: chrono::NaiveDate| {
        LocalDate::new(dt.year(), dt.month() as u8, dt.day() as u8)
            .ok_or_else(|| "Event date is outside supported range".to_string())
    };
    let start_date = date(start.0)?;
    let mut end_date = date(end.0)?;
    if all_day {
        // Google all-day end.date is already exclusive.
    } else if end.1 == chrono::NaiveTime::MIN && end_date > start_date {
    } else {
        end_date = end_date.add_days(1);
    }
    let end_exclusive = end_date.max(start_date.add_days(1));
    Ok(CalendarOccurrence {
        source_id: source.into(),
        event_id: event["id"].as_str().unwrap_or("").into(),
        color_hex: color.into(),
        start_date,
        end_date_exclusive: end_exclusive,
        is_all_day: all_day,
        title: event["summary"].as_str().unwrap_or("").into(),
        time_label: if all_day {
            String::new()
        } else {
            format!("{:02}:{:02}", start.1.hour(), start.1.minute())
        },
    })
}

fn google_bound(
    value: &Value,
    start: bool,
) -> Result<(chrono::NaiveDate, chrono::NaiveTime), String> {
    if let Some(day) = value["date"].as_str() {
        let date = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d")
            .map_err(|_| "Google event has an invalid date")?;
        return Ok((date, chrono::NaiveTime::MIN));
    }
    let raw = value["dateTime"].as_str().ok_or(if start {
        "Google event has no start"
    } else {
        "Google event has no end"
    })?;
    let instant = chrono::DateTime::parse_from_rfc3339(raw)
        .map_err(|_| "Google event has an invalid date/time")?
        .with_timezone(&chrono::Local);
    Ok((instant.date_naive(), instant.time()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_desktop_client_ids_only() {
        assert!(valid_client_id(
            "1234567890-abc_DEF.apps.googleusercontent.com"
        ));
        assert!(!valid_client_id("not-a-google-id"));
        assert!(!valid_client_id("secret"));
    }

    #[test]
    fn callback_accepts_loopback_root_and_rejects_a_mismatch() {
        assert_eq!(
            callback_code("GET /?code=abc&state=nonce HTTP/1.1", "nonce").unwrap(),
            Some("abc".into())
        );
        assert!(callback_code("GET /?error=access_denied&state=nonce HTTP/1.1", "nonce").is_err());
        assert!(callback_code("GET /?code=abc&state=other HTTP/1.1", "nonce").is_err());
    }
}
