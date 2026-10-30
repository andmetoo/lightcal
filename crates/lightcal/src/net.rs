//! Bounded HTTPS transport. Errors intentionally omit URLs and response bodies.
use std::time::Duration;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

pub fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .max_redirects(0)
        .http_status_as_error(false)
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into()
}

pub fn download_release_asset(raw: &str) -> Result<Vec<u8>, String> {
    let uri = https_url(raw)?;
    if uri.host_str() != Some("github.com")
        || !uri
            .path()
            .starts_with("/andmetoo/lightcal/releases/download/")
        || uri.port().is_some()
    {
        return Err("Rejected an update asset outside the official repository".into());
    }
    let client: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(120)))
        .max_redirects(5)
        .https_only(true)
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into();
    let mut response = client
        .get(raw)
        .header("User-Agent", "Lightcal updater")
        .call()
        .map_err(|_| "Update download failed or timed out")?;
    if response.status().as_u16() != 200 {
        return Err("Update asset is unavailable".into());
    }
    response
        .body_mut()
        .with_config()
        .limit(128 * 1024 * 1024)
        .read_to_vec()
        .map_err(|_| "Update download exceeded the size/time limit".into())
}

pub fn https_url(raw: &str) -> Result<url::Url, String> {
    let url = url::Url::parse(raw).map_err(|_| "Invalid HTTPS address".to_string())?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("Use an HTTPS address without embedded credentials or a fragment".into());
    }
    Ok(url)
}

pub fn get(raw: &str, bearer: Option<&str>, limit: u64) -> Result<(u16, Vec<u8>), String> {
    get_with_headers(raw, bearer, &[], limit)
}

pub fn get_with_headers(
    raw: &str,
    bearer: Option<&str>,
    extra_headers: &[(&str, &str)],
    limit: u64,
) -> Result<(u16, Vec<u8>), String> {
    https_url(raw)?;
    let client = agent(Duration::from_secs(60));
    let mut request = client
        .get(raw)
        .header("User-Agent", concat!("Lightcal/", env!("LIGHTCAL_VERSION")));
    if let Some(token) = bearer {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    for &(name, value) in extra_headers {
        request = request.header(name, value);
    }
    let mut response = request
        .call()
        .map_err(|_| "HTTPS request failed or timed out".to_string())?;
    let status = response.status().as_u16();
    let bytes = response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(|_| "Response is too large, incomplete, or timed out".to_string())?;
    Ok((status, bytes))
}

#[derive(Debug, PartialEq, Eq)]
pub enum AuthenticationError {
    SignInRequired,
    Failed(String),
}
impl AuthenticationError {
    pub fn message(self) -> String {
        match self {
            Self::SignInRequired => {
                "Microsoft session expired or consent was revoked; sign in again".into()
            }
            Self::Failed(message) => message,
        }
    }
}
pub fn post_form(raw: &str, fields: &[(&str, &str)]) -> Result<serde_json::Value, String> {
    post_form_detailed(raw, fields).map_err(AuthenticationError::message)
}
pub fn post_form_detailed(
    raw: &str,
    fields: &[(&str, &str)],
) -> Result<serde_json::Value, AuthenticationError> {
    https_url(raw).map_err(AuthenticationError::Failed)?;
    let body = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(fields.iter().copied())
        .finish();
    let mut response = agent(Duration::from_secs(30))
        .post(raw)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send(body.as_str())
        .map_err(|_| {
            AuthenticationError::Failed("Authentication request failed or timed out".to_string())
        })?;
    let status = response.status().as_u16();
    let text = response
        .body_mut()
        .with_config()
        .limit(1024 * 1024)
        .read_to_string()
        .map_err(|_| AuthenticationError::Failed("Invalid authentication response".to_string()))?;
    evaluate_authentication_response(status, &text)
}
fn evaluate_authentication_response(
    status: u16,
    text: &str,
) -> Result<serde_json::Value, AuthenticationError> {
    let payload: serde_json::Value = serde_json::from_str(text)
        .map_err(|_| AuthenticationError::Failed("Invalid authentication response".to_string()))?;
    if status >= 400 {
        if status == 401
            || matches!(
                payload["error"].as_str(),
                Some(
                    "invalid_grant"
                        | "interaction_required"
                        | "login_required"
                        | "consent_required"
                )
            )
        {
            return Err(AuthenticationError::SignInRequired);
        }
        return Err(AuthenticationError::Failed(format!(
            "Microsoft authentication request failed (HTTP {status})"
        )));
    }
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn revoked_consent_and_network_failures_have_different_account_meanings() {
        assert_eq!(
            evaluate_authentication_response(400, r#"{"error":"invalid_grant"}"#),
            Err(AuthenticationError::SignInRequired)
        );
        assert!(matches!(
            evaluate_authentication_response(400, r#"{"error":"invalid_client"}"#),
            Err(AuthenticationError::Failed(_))
        ));
        assert!(evaluate_authentication_response(200, r#"{"access_token":"fixture"}"#).is_ok());
    }
    #[test]
    fn rejects_non_https_and_embedded_credentials() {
        for raw in [
            "http://example.org/a.ics",
            "file:///private",
            "https://user:secret@example.org/a",
            "https://example.org/#secret",
        ] {
            assert!(https_url(raw).is_err());
        }
        assert!(https_url("https://example.org/private/token.ics").is_ok());
    }
}
