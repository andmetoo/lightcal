//! Windows timezone-name adapter used only at the provider boundary.
use chrono::{Datelike, Timelike};

pub fn calendar_date(
    utc: chrono::DateTime<chrono::Utc>,
    name: &str,
) -> Result<chrono::NaiveDate, String> {
    if let Ok(zone) = name.parse::<chrono_tz::Tz>() {
        return Ok(utc.with_timezone(&zone).date_naive());
    }
    windows_date(utc, name)
}

#[cfg(windows)]
fn windows_date(
    utc: chrono::DateTime<chrono::Utc>,
    name: &str,
) -> Result<chrono::NaiveDate, String> {
    use windows_sys::Win32::{
        Foundation::SYSTEMTIME,
        System::Time::{
            EnumDynamicTimeZoneInformation, SystemTimeToTzSpecificLocalTimeEx,
            DYNAMIC_TIME_ZONE_INFORMATION,
        },
    };
    let input = SYSTEMTIME {
        wYear: utc.year() as u16,
        wMonth: utc.month() as u16,
        wDay: utc.day() as u16,
        wDayOfWeek: 0,
        wHour: utc.hour() as u16,
        wMinute: utc.minute() as u16,
        wSecond: utc.second() as u16,
        wMilliseconds: 0,
    };
    for index in 0..2048 {
        let mut zone: DYNAMIC_TIME_ZONE_INFORMATION = unsafe { std::mem::zeroed() };
        if unsafe { EnumDynamicTimeZoneInformation(index, &mut zone) } != 0 {
            break;
        }
        let length = zone
            .TimeZoneKeyName
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(zone.TimeZoneKeyName.len());
        if String::from_utf16_lossy(&zone.TimeZoneKeyName[..length]) == name {
            let mut output: SYSTEMTIME = unsafe { std::mem::zeroed() };
            if unsafe { SystemTimeToTzSpecificLocalTimeEx(&zone, &input, &mut output) } == 0 {
                return Err("Could not convert the event's Windows timezone".into());
            }
            return chrono::NaiveDate::from_ymd_opt(
                output.wYear as i32,
                output.wMonth as u32,
                output.wDay as u32,
            )
            .ok_or_else(|| "Invalid event date after timezone conversion".into());
        }
    }
    Err("Microsoft returned an unsupported all-day event timezone".into())
}
#[cfg(not(windows))]
fn windows_date(_: chrono::DateTime<chrono::Utc>, _: &str) -> Result<chrono::NaiveDate, String> {
    Err("A Windows timezone mapping is needed for this calendar".into())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use chrono::TimeZone;
    #[test]
    fn graph_windows_timezone_preserves_original_all_day_date() {
        let midnight_in_europe = chrono::Utc
            .with_ymd_and_hms(2026, 10, 29, 23, 0, 0)
            .unwrap();
        assert_eq!(
            calendar_date(midnight_in_europe, "W. Europe Standard Time").unwrap(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()
        );
    }
}
