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
    (url == ALLOWED_RELEASE_URL_PREFIX
        || url.starts_with(&format!("{ALLOWED_RELEASE_URL_PREFIX}/")))
        && !url.contains(|c: char| c.is_control() || c.is_whitespace() || c == '"' || c == '\'')
}

/// Exact macOS Privacy & Security pane for calendar access. Not a general URL scheme.
pub const MACOS_CALENDAR_PRIVACY_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars";

pub fn is_allowed_external_link(url: &str) -> bool {
    if let Ok(uri) = url::Url::parse(url) {
        if uri.scheme() == "https"
            && uri.host_str() == Some("login.microsoftonline.com")
            && uri.path() == "/common/oauth2/v2.0/authorize"
            && uri.username().is_empty()
            && uri.password().is_none()
            && uri.port().is_none()
        {
            return true;
        }
    }
    url == "https://github.com/andmetoo/lightcal"
        || url == "https://koona.app"
        || url == "ms-settings:privacy-calendar"
        || url == "ms-settings:emailandaccounts"
        || url == MACOS_CALENDAR_PRIVACY_URL
        || is_verified_github_url(url)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(is_allowed_external_link("ms-settings:privacy-calendar"));
        assert!(is_allowed_external_link("ms-settings:emailandaccounts"));
        assert!(is_allowed_external_link(MACOS_CALENDAR_PRIVACY_URL));
        assert!(!is_allowed_external_link("ms-settings:windowsupdate"));
        assert!(!is_allowed_external_link(
            "x-apple.systempreferences:com.apple.preference.security"
        ));
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
