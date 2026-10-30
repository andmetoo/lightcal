//! Direct read-only providers; toolkit-independent inputs/outputs.
pub mod account;
pub mod ics;
pub mod microsoft;
use crate::{
    platform::{ProviderSnapshot, ProviderStatus},
    secure_store::Credentials,
};
use account::MicrosoftAccountState;
use base64::Engine;
use calendar_core::LocalDate;
use std::{collections::BTreeSet, path::Path};

pub struct CalendarLoad {
    pub snapshot: ProviderSnapshot,
    pub microsoft: MicrosoftAccountState,
}

pub fn random_key() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|_| "Secure random generator is unavailable".to_string())?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
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

pub fn query(
    anchor: LocalDate,
    dir: &Path,
    client_id: &str,
    selected: &BTreeSet<String>,
    legacy_windows: bool,
) -> CalendarLoad {
    let _guard = match crate::secure_store::credential_lock() {
        Ok(guard) => guard,
        Err(error) => return failure(error),
    };
    let store = crate::secure_store::SecureStore::new(dir);
    let mut credentials = match store.load() {
        Ok(c) => c,
        Err(error) => return failure(error),
    };
    let mut snapshot = ProviderSnapshot {
        status: ProviderStatus::Available,
        sources: Vec::new(),
        occurrences: Vec::new(),
    };
    let mut errors = Vec::new();
    let mut account = MicrosoftAccountState::Disconnected;
    if credentials.microsoft_refresh_token.is_some() {
        account = MicrosoftAccountState::Unavailable {
            saved_credentials: true,
        };
        match microsoft::query(
            anchor,
            client_id,
            selected,
            &mut credentials,
            &store,
            &mut account,
        ) {
            Ok(data) => {
                append_provider(&mut snapshot, data, &mut errors);
            }
            Err(e) => errors.push(e),
        }
    }
    for feed in &credentials.ics {
        snapshot.sources.push(calendar_core::CalendarSource {
            id: feed.id.clone(),
            name: feed.name.clone(),
            account_label: "ICS · HTTPS".into(),
            color_hex: feed.color.clone(),
        });
        if selected.contains(&feed.id) {
            match ics::fetch(feed, anchor) {
                Ok(events) => snapshot.occurrences.extend(events),
                Err(e) => errors.push(format!("{}: {e}", feed.name)),
            }
        }
    }
    if legacy_windows {
        let data = crate::platform::windows::query_calendar_provider(anchor, false);
        append_provider(&mut snapshot, data, &mut errors);
    }
    finish_status(&mut snapshot, &errors);
    CalendarLoad {
        snapshot,
        microsoft: account,
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
    snapshot.status = if !errors.is_empty() {
        ProviderStatus::ProviderUnavailable(errors.join("\n"))
    } else if snapshot.sources.is_empty() {
        ProviderStatus::NoCalendars
    } else {
        ProviderStatus::Available
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    fn empty(status: ProviderStatus) -> ProviderSnapshot {
        ProviderSnapshot {
            status,
            sources: Vec::new(),
            occurrences: Vec::new(),
        }
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
}

fn failure(error: String) -> CalendarLoad {
    CalendarLoad {
        microsoft: MicrosoftAccountState::Unavailable {
            saved_credentials: false,
        },
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
