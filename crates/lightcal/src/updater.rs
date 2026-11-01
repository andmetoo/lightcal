//! Manual GitHub release update checker.
//!
//! Per `docs/features.md` and `docs/architecture.md`:
//! - Manual checks, plus startup/daily checks only when explicitly enabled in Settings.
//! - Compares stable semantic versions; excludes drafts and prereleases.
//! - Accepts release-page links only from `https://github.com/andmetoo/lightcal/`.
//! - Handles offline, timeout, rate-limit, malformed-response, and no-release states.
//! - Never downloads, executes, or restarts the application.

use serde::Deserialize;

/// Uses Windows' TLS stack on a worker thread; no browser or shell process.
#[cfg(windows)]
pub fn check_latest_release(installed_version: &str) -> UpdateCheckState {
    use windows::{core::HSTRING, Foundation::Uri, Web::Http::HttpClient};
    let request = || -> windows::core::Result<UpdateCheckState> {
        let client = HttpClient::new()?;
        let headers = client.DefaultRequestHeaders()?;
        headers.Append(
            &HSTRING::from("User-Agent"),
            &HSTRING::from(concat!("Lightcal/", env!("LIGHTCAL_VERSION"))),
        )?;
        headers.Append(
            &HSTRING::from("Accept"),
            &HSTRING::from("application/vnd.github+json"),
        )?;
        let uri = Uri::CreateUri(&HSTRING::from(
            "https://api.github.com/repos/andmetoo/lightcal/releases/latest",
        ))?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let op = client.GetAsync(&uri)?;
        while op.Status()?.0 == 0 {
            // WinRT AsyncStatus::Started
            if std::time::Instant::now() >= deadline {
                op.Cancel()?;
                return Ok(UpdateCheckState::Timeout);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let response = op.get()?;
        let status = response.StatusCode()?.0 as u16;
        if status != 200 {
            return Ok(evaluate_release_response(installed_version, status, ""));
        }
        let body_op = response.Content()?.ReadAsStringAsync()?;
        while body_op.Status()?.0 == 0 {
            if std::time::Instant::now() >= deadline {
                body_op.Cancel()?;
                return Ok(UpdateCheckState::Timeout);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let body = body_op.get()?.to_string_lossy();
        Ok(evaluate_release_response(installed_version, status, &body))
    };
    request().unwrap_or(UpdateCheckState::Offline)
}

#[cfg(not(windows))]
pub fn check_latest_release(_installed_version: &str) -> UpdateCheckState {
    UpdateCheckState::Offline
}

pub const ALLOWED_RELEASE_URL_PREFIX: &str = "https://github.com/andmetoo/lightcal/releases";
pub const DEFAULT_RELEASE_PAGE: &str = "https://github.com/andmetoo/lightcal/releases";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SemVer {
    pub fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim().strip_prefix('v').unwrap_or(raw.trim());
        // Reject prerelease/build tags for stable release comparison.
        if trimmed.contains('-') || trimmed.contains('+') {
            return None;
        }
        let mut parts = trimmed.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheckState {
    Idle,
    Checking,
    UpToDate {
        current_version: String,
    },
    UpdateAvailable {
        current_version: String,
        latest_version: String,
        release_url: String,
    },
    NoStableRelease,
    Offline,
    Timeout,
    RateLimited,
    InvalidResponse(String),
}

#[derive(Debug, Deserialize)]
struct GithubReleasePayload {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// Evaluates an HTTP response from the GitHub Releases endpoint without performing network IO.
pub fn evaluate_release_response(
    installed_version: &str,
    http_status: u16,
    body: &str,
) -> UpdateCheckState {
    match http_status {
        404 => return UpdateCheckState::NoStableRelease,
        403 | 429 => return UpdateCheckState::RateLimited,
        status if status != 200 => {
            return UpdateCheckState::InvalidResponse(format!("Unexpected HTTP status {status}"));
        }
        _ => {}
    }

    let current_semver = match SemVer::parse(installed_version) {
        Some(v) => v,
        None => {
            return UpdateCheckState::InvalidResponse("Invalid installed version".to_string());
        }
    };

    let payload: GithubReleasePayload = match serde_json::from_str(body) {
        Ok(p) => p,
        Err(err) => {
            return UpdateCheckState::InvalidResponse(format!("Malformed release JSON: {err}"));
        }
    };

    if payload.draft || payload.prerelease {
        return UpdateCheckState::NoStableRelease;
    }

    let latest_semver = match SemVer::parse(&payload.tag_name) {
        Some(v) => v,
        None => {
            return UpdateCheckState::InvalidResponse(format!(
                "Invalid release tag: {}",
                payload.tag_name
            ));
        }
    };

    if !is_verified_github_url(&payload.html_url) {
        return UpdateCheckState::InvalidResponse(
            "Rejected untrusted release URL outside repository".to_string(),
        );
    }

    if latest_semver > current_semver {
        UpdateCheckState::UpdateAvailable {
            current_version: installed_version.to_string(),
            latest_version: payload.tag_name.trim_start_matches('v').to_string(),
            release_url: payload.html_url,
        }
    } else {
        UpdateCheckState::UpToDate {
            current_version: installed_version.to_string(),
        }
    }
}

/// Validates that a URL points strictly to the official repository or koona.app.
pub fn is_verified_github_url(url: &str) -> bool {
    const PATH: &str = "/andmetoo/lightcal/releases";
    if url.contains(|c: char| c.is_control() || c.is_whitespace() || matches!(c, '"' | '\'' | '\\'))
        || url.contains("..")
        || url.to_ascii_lowercase().contains("%2e")
    {
        return false;
    }
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    parsed.scheme() == "https"
        && parsed.host_str() == Some("github.com")
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.port().is_none()
        && (parsed.path() == PATH || parsed.path().starts_with(&format!("{PATH}/")))
}

pub fn is_allowed_external_link(url: &str) -> bool {
    if let Ok(uri) = url::Url::parse(url) {
        if uri.scheme() == "https"
            && uri.username().is_empty()
            && uri.password().is_none()
            && uri.port().is_none()
            && ((uri.host_str() == Some("login.microsoftonline.com")
                && uri.path() == "/common/oauth2/v2.0/authorize")
                || (uri.host_str() == Some("accounts.google.com")
                    && uri.path() == "/o/oauth2/v2/auth"))
        {
            return true;
        }
    }
    url == "https://github.com/andmetoo/lightcal"
        || url == "https://koona.app"
        || url == "ms-settings:privacy-calendar"
        || url == "ms-settings:emailandaccounts"
        || is_verified_github_url(url)
}

/// Provider web pages an agenda event's Open action may launch: Google Calendar
/// `htmlLink` and Microsoft Graph `webLink` addresses. HTTPS only, no
/// credentials or ports. Lightcal opens these on user action and never fetches them.
pub fn is_allowed_event_link(url: &str) -> bool {
    if url.len() > 4096 || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    let Ok(uri) = url::Url::parse(url) else {
        return false;
    };
    if uri.scheme() != "https"
        || !uri.username().is_empty()
        || uri.password().is_some()
        || uri.port().is_some()
    {
        return false;
    }
    let path = uri.path();
    match uri.host_str() {
        // Google Calendar: https://www.google.com/calendar/event?eid=...
        Some("www.google.com") => path == "/calendar/event",
        // Graph webLink: https://outlook.office365.com/owa/?itemid=...&path=/calendar/item
        // (work or school), https://outlook.live.com/owa/?itemid=... (personal), and the
        // newer https://outlook.office.com/calendar/item/... form.
        Some("outlook.office.com" | "outlook.office365.com" | "outlook.live.com") => {
            path.starts_with("/owa/") || path.starts_with("/calendar/")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_links_accept_only_provider_calendar_pages() {
        for allowed in [
            "https://www.google.com/calendar/event?eid=ZXZ0IGV4YW1wbGU",
            "https://outlook.office365.com/owa/?itemid=AAMk%2FAGI&exvsurl=1&path=/calendar/item",
            "https://outlook.live.com/owa/?itemid=AQMk&exvsurl=1&path=/calendar/item",
            "https://outlook.office.com/calendar/item/AAMkAGI",
            "https://outlook.office.com/owa/?itemid=AAMk",
        ] {
            assert!(is_allowed_event_link(allowed), "{allowed}");
        }
        for rejected in [
            "http://www.google.com/calendar/event?eid=1",
            "https://www.google.com:8443/calendar/event?eid=1",
            "https://user:pw@www.google.com/calendar/event?eid=1",
            "https://user@outlook.live.com/owa/?itemid=1",
            "https://www.google.com/url?q=https://evil.example",
            "https://www.google.com/calendar/eventx",
            "https://calendar.google.com.evil.example/calendar/event",
            "https://google.com/calendar/event?eid=1",
            "https://outlook.office365.com/mail/inbox",
            "https://outlook.office365.com.evil.example/owa/",
            "https://evil.example/owa/?r=https://outlook.live.com",
            "https://outlook.live.com/owa/ ?itemid=1",
            "javascript:alert(1)",
            "file:///C:/Temp/run.exe",
            "ms-settings:privacy-calendar",
            "",
        ] {
            assert!(!is_allowed_event_link(rejected), "{rejected}");
        }
        // Event links are a separate allow-list from the app's fixed links.
        assert!(!is_allowed_external_link(
            "https://www.google.com/calendar/event?eid=1"
        ));
    }

    #[test]
    fn evaluates_newer_current_and_prerelease_versions() {
        let newer_json = r#"{
            "tag_name": "v0.2.0",
            "html_url": "https://github.com/andmetoo/lightcal/releases/tag/v0.2.0",
            "draft": false,
            "prerelease": false
        }"#;
        assert_eq!(
            evaluate_release_response("0.1.0", 200, newer_json),
            UpdateCheckState::UpdateAvailable {
                current_version: "0.1.0".into(),
                latest_version: "0.2.0".into(),
                release_url: "https://github.com/andmetoo/lightcal/releases/tag/v0.2.0".into(),
            }
        );

        assert_eq!(
            evaluate_release_response("0.2.0", 200, newer_json),
            UpdateCheckState::UpToDate {
                current_version: "0.2.0".into(),
            }
        );

        let prerelease_json = r#"{
            "tag_name": "v0.3.0-rc.1",
            "html_url": "https://github.com/andmetoo/lightcal/releases/tag/v0.3.0-rc.1",
            "draft": false,
            "prerelease": true
        }"#;
        assert_eq!(
            evaluate_release_response("0.1.0", 200, prerelease_json),
            UpdateCheckState::NoStableRelease
        );
    }

    #[test]
    fn release_url_rejects_traversal_and_authority_tricks() {
        assert!(is_verified_github_url(
            "https://github.com/andmetoo/lightcal/releases"
        ));
        assert!(is_verified_github_url(
            "https://github.com/andmetoo/lightcal/releases/tag/v0.2.0"
        ));
        for bad in [
            "https://github.com/andmetoo/lightcal/releases/../../other/repo",
            "https://github.com/andmetoo/lightcal/releases/%2e%2e/%2E%2e/other",
            "https://github.com/andmetoo/lightcal/releases/..%2f..%2fother",
            "https://github.com/andmetoo/lightcal/releases\\..\\other",
            "https://github.com/andmetoo/lightcal/releases/tag/v1 x",
            "https://github.com/andmetoo/lightcal/releases/\n",
            "https://user@github.com/andmetoo/lightcal/releases",
            "https://github.com:444/andmetoo/lightcal/releases",
            "http://github.com/andmetoo/lightcal/releases",
            "https://github.com.evil.example/andmetoo/lightcal/releases",
            "https://github.com/andmetoo/lightcal/releases-untrusted",
        ] {
            assert!(!is_verified_github_url(bad), "{bad}");
        }
    }

    #[test]
    fn rejects_untrusted_links_and_handles_rate_limits() {
        assert!(!is_verified_github_url(
            "https://github.com/andmetoo/lightcal/releases-untrusted"
        ));
        let untrusted_json = r#"{
            "tag_name": "v0.2.0",
            "html_url": "https://evil.example.com/lightcal.exe",
            "draft": false,
            "prerelease": false
        }"#;
        assert!(matches!(
            evaluate_release_response("0.1.0", 200, untrusted_json),
            UpdateCheckState::InvalidResponse(_)
        ));
        assert_eq!(
            evaluate_release_response("0.1.0", 429, ""),
            UpdateCheckState::RateLimited
        );
        assert_eq!(
            evaluate_release_response("0.1.0", 404, ""),
            UpdateCheckState::NoStableRelease
        );
    }

    #[test]
    fn allowed_external_links_include_system_settings_and_about_urls() {
        assert!(is_allowed_external_link(
            "https://github.com/andmetoo/lightcal"
        ));
        assert!(is_allowed_external_link("https://koona.app"));
        assert!(is_allowed_external_link(
            "https://github.com/andmetoo/lightcal/releases/tag/v0.5.0"
        ));
        assert!(is_allowed_external_link(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id=1234567890-abc.apps.googleusercontent.com&redirect_uri=http%3A%2F%2F127.0.0.1%3A9&response_type=code"
        ));
        assert!(!is_allowed_external_link(
            "https://accounts.google.com/o/oauth2/v2/other"
        ));
        assert!(!is_allowed_external_link(
            "https://evil.accounts.google.com/o/oauth2/v2/auth"
        ));
        assert!(is_allowed_external_link("ms-settings:privacy-calendar"));
        assert!(is_allowed_external_link("ms-settings:emailandaccounts"));
        assert!(!is_allowed_external_link("ms-settings:windowsupdate"));
        assert!(!is_allowed_external_link("https://malicious.site"));
        assert!(!is_allowed_external_link("cmd.exe"));
    }

    #[test]
    #[ignore = "Explicit live HTTPS smoke probe; normal tests use response fixtures"]
    fn live_release_check_finishes() {
        let state = check_latest_release(crate::version::VERSION);
        println!("GitHub update check: {state:?}");
        assert!(!matches!(
            state,
            UpdateCheckState::Idle | UpdateCheckState::Checking
        ));
    }
}
