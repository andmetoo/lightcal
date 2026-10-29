//! Real read-only Windows WinRT AppointmentStore provider.

use crate::platform::{ProviderSnapshot, ProviderStatus};
use calendar_core::{CalendarOccurrence, CalendarSource, LocalDate};
use chrono::{Datelike, Timelike};
use std::env;

const FILETIME_EPOCH_DIFF_SECS: i64 = 11_644_473_600;
const TICKS_PER_SECOND: i64 = 10_000_000;

#[cfg(windows)]
pub fn query_windows_calendar_store(anchor: LocalDate) -> ProviderSnapshot {
    use windows::ApplicationModel::Appointments::{
        AppointmentManager, AppointmentStoreAccessType, FindAppointmentsOptions,
    };
    use windows::Foundation::{DateTime, TimeSpan};

    let store_op = match AppointmentManager::RequestStoreAsync(
        AppointmentStoreAccessType::AllCalendarsReadOnly,
    ) {
        Ok(op) => op,
        Err(e) => {
            return ProviderSnapshot {
                status: map_winrt_error(&e),
                sources: Vec::new(),
                occurrences: Vec::new(),
            };
        }
    };

    let store = match store_op.get() {
        Ok(s) => s,
        Err(e) => {
            return ProviderSnapshot {
                status: map_winrt_error(&e),
                sources: Vec::new(),
                occurrences: Vec::new(),
            };
        }
    };

    let cals_op = match store.FindAppointmentCalendarsAsync() {
        Ok(op) => op,
        Err(e) => {
            return ProviderSnapshot {
                status: map_winrt_error(&e),
                sources: Vec::new(),
                occurrences: Vec::new(),
            };
        }
    };

    let cals_view = match cals_op.get() {
        Ok(v) => v,
        Err(e) => {
            return ProviderSnapshot {
                status: map_winrt_error(&e),
                sources: Vec::new(),
                occurrences: Vec::new(),
            };
        }
    };

    let cal_count = cals_view.Size().unwrap_or(0);
    if cal_count == 0 {
        return ProviderSnapshot {
            status: ProviderStatus::NoCalendars,
            sources: Vec::new(),
            occurrences: Vec::new(),
        };
    }

    let mut sources = Vec::with_capacity(cal_count as usize);
    for i in 0..cal_count {
        if let Ok(cal) = cals_view.GetAt(i) {
            let id = cal
                .LocalId()
                .map(|h| h.to_string_lossy())
                .unwrap_or_default();
            let name = cal
                .DisplayName()
                .map(|h| h.to_string_lossy())
                .unwrap_or_else(|_| "Calendar".to_string());
            let source_raw = cal
                .SourceDisplayName()
                .map(|h| h.to_string_lossy())
                .unwrap_or_default();
            let account_label = if source_raw.trim().is_empty() {
                format!("Windows · {name}")
            } else {
                format!("Windows · {source_raw}")
            };
            let color_hex = cal
                .DisplayColor()
                .map(|c| format!("#{:02X}{:02X}{:02X}", c.R, c.G, c.B))
                .unwrap_or_else(|_| "#5E6AD2".to_string());
            sources.push(CalendarSource {
                id,
                name,
                account_label,
                color_hex,
            });
        }
    }

    // Cover the entire fixed 42-cell grid, including a short month's trailing rows.
    let ym = anchor.year_month();
    let month_start = LocalDate::new(ym.year, ym.month, 1).unwrap_or(anchor);
    let range_start_date = month_start.add_days(-7);
    let month_end = LocalDate::new(ym.year, ym.month, ym.days_in_month()).unwrap_or(anchor);
    let range_end_date = month_end.add_days(15);
    let duration_days = (range_end_date.to_epoch_days() - range_start_date.to_epoch_days()).max(1);

    let start_ticks =
        (range_start_date.to_epoch_days() * 86_400 + FILETIME_EPOCH_DIFF_SECS) * TICKS_PER_SECOND;
    let duration_ticks = duration_days * 86_400 * TICKS_PER_SECOND;

    let start_winrt = DateTime {
        UniversalTime: start_ticks,
    };
    let duration_winrt = TimeSpan {
        Duration: duration_ticks,
    };

    let mut occurrences = Vec::new();
    let options = match FindAppointmentsOptions::new() {
        Ok(opt) => opt,
        Err(e) => {
            return ProviderSnapshot {
                status: map_winrt_error(&e),
                sources,
                occurrences,
            };
        }
    };

    if let Ok(props) = options.FetchProperties() {
        let _ = props.Append(&windows::core::HSTRING::from("Subject"));
        let _ = props.Append(&windows::core::HSTRING::from("StartTime"));
        let _ = props.Append(&windows::core::HSTRING::from("Duration"));
        let _ = props.Append(&windows::core::HSTRING::from("AllDay"));
        let _ = props.Append(&windows::core::HSTRING::from("CalendarId"));
    }

    let appts_res =
        match store.FindAppointmentsAsyncWithOptions(start_winrt, duration_winrt, &options) {
            Ok(op) => op.get(),
            Err(e) => match store.FindAppointmentsAsync(start_winrt, duration_winrt) {
                Ok(op) => op.get(),
                Err(_) => Err(e),
            },
        };

    if let Ok(appts_view) = appts_res {
        let appt_count = appts_view.Size().unwrap_or(0);
        for i in 0..appt_count {
            if let Ok(appt) = appts_view.GetAt(i) {
                let source_id = appt
                    .CalendarId()
                    .map(|h| h.to_string_lossy())
                    .unwrap_or_default();
                let color_hex = sources
                    .iter()
                    .find(|s| s.id == source_id)
                    .map(|s| s.color_hex.clone())
                    .unwrap_or_else(|| "#5E6AD2".to_string());
                let title = appt
                    .Subject()
                    .map(|h| h.to_string_lossy())
                    .unwrap_or_default();
                let is_all_day = appt.AllDay().unwrap_or(false);

                let start_time_winrt = match appt.StartTime() {
                    Ok(st) => st.UniversalTime,
                    Err(_) => continue,
                };
                let unix_secs = (start_time_winrt / TICKS_PER_SECOND) - FILETIME_EPOCH_DIFF_SECS;
                let utc_dt = match chrono::DateTime::from_timestamp(unix_secs, 0) {
                    Some(dt) => dt,
                    None => continue,
                };
                let (start_date, local_dt) = if is_all_day {
                    let mid_local = utc_dt.with_timezone(&chrono::Local);
                    let d = LocalDate::new(
                        mid_local.year(),
                        mid_local.month() as u8,
                        mid_local.day() as u8,
                    )
                    .unwrap_or(anchor);
                    (d, mid_local)
                } else {
                    let local = utc_dt.with_timezone(&chrono::Local);
                    let d = match LocalDate::new(
                        local.year(),
                        local.month() as u8,
                        local.day() as u8,
                    ) {
                        Some(d) => d,
                        None => continue,
                    };
                    (d, local)
                };

                let duration_secs = appt
                    .Duration()
                    .map(|d| (d.Duration / TICKS_PER_SECOND).max(0))
                    .unwrap_or(0);

                let (end_date_exclusive, time_label) = if is_all_day {
                    let days = (duration_secs / 86_400).max(1) as i32;
                    (start_date.add_days(days), "All day".to_string())
                } else {
                    let end_utc = utc_dt + chrono::Duration::seconds(duration_secs);
                    let end_local = end_utc.with_timezone(&chrono::Local);
                    let end_date = LocalDate::new(
                        end_local.year(),
                        end_local.month() as u8,
                        end_local.day() as u8,
                    )
                    .unwrap_or_else(|| start_date.add_days(1));
                    let end_excl = if end_date > start_date {
                        if end_local.hour() == 0
                            && end_local.minute() == 0
                            && end_local.second() == 0
                        {
                            end_date
                        } else {
                            end_date.add_days(1)
                        }
                    } else {
                        start_date.add_days(1)
                    };
                    let time = format!("{:02}:{:02}", local_dt.hour(), local_dt.minute());
                    (end_excl, time)
                };

                let event_id = appt
                    .LocalId()
                    .map(|h| h.to_string_lossy())
                    .unwrap_or_else(|_| format!("win-evt-{i}"));

                occurrences.push(CalendarOccurrence {
                    source_id,
                    event_id,
                    color_hex,
                    start_date,
                    end_date_exclusive,
                    is_all_day,
                    title,
                    time_label,
                });
            }
        }
    } else if let Err(e) = appts_res {
        return ProviderSnapshot {
            status: map_winrt_error(&e),
            sources,
            occurrences: Vec::new(),
        };
    }

    ProviderSnapshot {
        status: ProviderStatus::Available,
        sources,
        occurrences,
    }
}

#[cfg(windows)]
fn map_winrt_error(err: &windows::core::Error) -> ProviderStatus {
    // 0x80070005 is E_ACCESSDENIED (-2147024891 as i32)
    let code = err.code().0;
    if code == -2147024891
        || code == 0x8007_0005u32 as i32
        || code == 5
        || err.message().contains("Access is denied")
    {
        ProviderStatus::AccessDenied
    } else {
        ProviderStatus::ProviderUnavailable(err.message().to_string())
    }
}

/// Fallback for non-Windows platforms.
#[cfg(not(windows))]
pub fn query_windows_calendar_store(anchor: LocalDate) -> ProviderSnapshot {
    ProviderSnapshot::synthetic(anchor)
}

/// Unified entry point respecting `--synthetic-calendars` and `LIGHTCAL_SYNTHETIC_CALENDARS`.
pub fn query_calendar_provider(anchor: LocalDate, use_synthetic: bool) -> ProviderSnapshot {
    if use_synthetic || env::var_os("LIGHTCAL_SYNTHETIC_CALENDARS").is_some() {
        return ProviderSnapshot::synthetic(anchor);
    }
    query_windows_calendar_store(anchor)
}

/// Asynchronously loads the provider snapshot on a background thread so the UI thread
/// is never blocked, dispatching the result back to Slint event loop.
pub fn load_provider_snapshot_async<F>(anchor: LocalDate, use_synthetic: bool, on_loaded: F)
where
    F: FnOnce(ProviderSnapshot) + Send + 'static,
{
    std::thread::spawn(move || {
        let snapshot = query_calendar_provider(anchor, use_synthetic);
        let _ = slint::invoke_from_event_loop(move || {
            on_loaded(snapshot);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_mode_returns_available_sample_data() {
        let anchor = LocalDate::new(2026, 10, 29).unwrap();
        let snapshot = query_calendar_provider(anchor, true);
        assert_eq!(snapshot.status, ProviderStatus::Available);
        assert!(!snapshot.sources.is_empty());
        assert!(!snapshot.occurrences.is_empty());
    }

    #[test]
    #[cfg(windows)]
    #[ignore = "Explicit read-only live-provider probe; normal tests use synthetic data"]
    fn real_windows_store_does_not_crash() {
        let anchor = LocalDate::new(2026, 10, 29).unwrap();
        let snapshot = query_windows_calendar_store(anchor);
        // On any Windows setup, this must cleanly return Available, NoCalendars, AccessDenied,
        // or ProviderUnavailable, never panicking.
        match snapshot.status {
            ProviderStatus::Available => {
                println!(
                    "Windows calendar available with {} sources",
                    snapshot.sources.len()
                );
                for s in &snapshot.sources {
                    println!(
                        "  Source: id='{}', name='{}', account_label='{}', color='{}'",
                        s.id, s.name, s.account_label, s.color_hex
                    );
                }
                println!("  Occurrences found: {}", snapshot.occurrences.len());
            }
            ProviderStatus::NoCalendars => {
                println!("Windows calendar store returned no calendars");
            }
            ProviderStatus::AccessDenied => {
                println!("Windows calendar store access denied (unpackaged/privacy)");
            }
            ProviderStatus::ProviderUnavailable(msg) => {
                println!("Windows calendar provider unavailable: {msg}");
            }
            ProviderStatus::StaleData => {}
        }
    }
}
