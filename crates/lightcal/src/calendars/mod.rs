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
                snapshot.sources.extend(data.sources);
                snapshot.occurrences.extend(data.occurrences);
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
        if let ProviderStatus::ProviderUnavailable(error) = data.status {
            errors.push(error);
        }
        snapshot.sources.extend(data.sources);
        snapshot.occurrences.extend(data.occurrences);
    }
    if !errors.is_empty() {
        snapshot.status = ProviderStatus::ProviderUnavailable(errors.join("\n"));
    } else if snapshot.sources.is_empty() {
        snapshot.status = ProviderStatus::NoCalendars;
    }
    CalendarLoad {
        snapshot,
        microsoft: account,
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
