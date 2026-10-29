//! Bounded iCalendar parsing and recurrence expansion; no toolkit or platform APIs.
use super::range;
use crate::secure_store::IcsSubscription;
use calendar_core::{CalendarOccurrence, LocalDate};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Timelike};
use ical::{parser::ical::component::IcalEvent, property::Property};
use rrule::{RRule, RRuleSet, Tz, Unvalidated};
use std::{collections::BTreeMap, io::BufReader};

const MAX_BYTES: u64 = 8 * 1024 * 1024;

pub fn fetch(feed: &IcsSubscription, anchor: LocalDate) -> Result<Vec<CalendarOccurrence>, String> {
    let (status, bytes) = crate::net::get(&feed.url, None, MAX_BYTES)?;
    if status != 200 {
        return Err(format!(
            "Calendar download returned HTTP {status}; use a direct HTTPS ICS link"
        ));
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| "Calendar is not UTF-8 iCalendar data")?;
    parse(text, feed, anchor, Tz::Local(chrono::Local))
}

fn property<'a>(event: &'a IcalEvent, name: &str) -> Option<&'a Property> {
    event.properties.iter().find(|p| p.name == name)
}
fn value<'a>(event: &'a IcalEvent, name: &str) -> Option<&'a str> {
    property(event, name).and_then(|p| p.value.as_deref())
}
fn param<'a>(property: &'a Property, name: &str) -> Option<&'a str> {
    property
        .params
        .as_ref()?
        .iter()
        .find(|(key, _)| key == name)?
        .1
        .first()
        .map(String::as_str)
}

fn stamp(property: &Property, raw: &str, floating: Tz) -> Result<(DateTime<Tz>, bool), String> {
    let raw = raw.trim();
    if raw.len() == 8 {
        let date = NaiveDate::parse_from_str(raw, "%Y%m%d").map_err(|_| "Invalid calendar date")?;
        return Ok((
            Tz::UTC.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap()),
            true,
        ));
    }
    let utc = raw.ends_with('Z');
    let naive = NaiveDateTime::parse_from_str(raw.trim_end_matches('Z'), "%Y%m%dT%H%M%S")
        .map_err(|_| "Invalid calendar date/time")?;
    let zone = if utc {
        Tz::UTC
    } else if let Some(name) = param(property, "TZID") {
        Tz::Tz(
            name.trim_matches('"')
                .parse::<chrono_tz::Tz>()
                .map_err(|_| "Unsupported ICS timezone; use an IANA timezone export")?,
        )
    } else {
        floating
    };
    let date = zone
        .from_local_datetime(&naive)
        .earliest()
        .ok_or("Calendar time falls in an unsupported DST gap")?;
    Ok((date, false))
}

fn duration(raw: &str) -> Result<Duration, String> {
    let mut input = raw
        .strip_prefix('+')
        .unwrap_or(raw)
        .strip_prefix('P')
        .ok_or("Invalid event duration")?;
    let mut seconds = 0i64;
    let mut time = false;
    while !input.is_empty() {
        if let Some(rest) = input.strip_prefix('T') {
            time = true;
            input = rest;
            continue;
        }
        let count = input.bytes().take_while(u8::is_ascii_digit).count();
        if count == 0 || count == input.len() {
            return Err("Invalid event duration".into());
        }
        let number: i64 = input[..count]
            .parse()
            .map_err(|_| "Invalid event duration")?;
        let factor = match (input.as_bytes()[count], time) {
            (b'W', false) => 604800,
            (b'D', false) => 86400,
            (b'H', true) => 3600,
            (b'M', true) => 60,
            (b'S', true) => 1,
            _ => return Err("Unsupported event duration".into()),
        };
        seconds = seconds
            .checked_add(
                number
                    .checked_mul(factor)
                    .ok_or("Event duration is too large")?,
            )
            .ok_or("Event duration is too large")?;
        input = &input[count + 1..];
    }
    Duration::try_seconds(seconds)
        .filter(|d| *d <= Duration::days(3660))
        .ok_or_else(|| "Event duration exceeds the supported limit".into())
}

fn unescape(raw: &str) -> String {
    let mut output = String::new();
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                output.push(if next == 'n' || next == 'N' {
                    '\n'
                } else {
                    next
                });
            }
        } else {
            output.push(ch);
        }
    }
    output
}

pub fn parse(
    text: &str,
    feed: &IcsSubscription,
    anchor: LocalDate,
    display_zone: Tz,
) -> Result<Vec<CalendarOccurrence>, String> {
    if text.len() > MAX_BYTES as usize {
        return Err("Calendar exceeds the 8 MiB limit".into());
    }
    let mut parser = ical::IcalParser::new(BufReader::new(text.as_bytes()));
    let calendar = parser
        .next()
        .ok_or("No VCALENDAR data found")?
        .map_err(|_| "Invalid iCalendar structure")?;
    if parser.next().is_some() {
        return Err("Use one VCALENDAR per subscription".into());
    }
    if calendar.events.len() > 5000 {
        return Err("Calendar exceeds the 5000-component limit".into());
    }
    let floating = if let Some(zone) = calendar
        .properties
        .iter()
        .find(|p| p.name == "X-WR-TIMEZONE")
        .and_then(|p| p.value.as_deref())
    {
        Tz::Tz(
            zone.parse::<chrono_tz::Tz>()
                .map_err(|_| "Unsupported calendar timezone")?,
        )
    } else {
        display_zone
    };
    let (from, to) = range(anchor);
    let mut groups: BTreeMap<String, Vec<IcalEvent>> = BTreeMap::new();
    for event in calendar.events {
        let uid = value(&event, "UID")
            .ok_or("An ICS event is missing its UID")?
            .to_string();
        groups.entry(uid).or_default().push(event);
    }
    let mut output = Vec::new();
    for (uid, events) in groups {
        let mut overrides = BTreeMap::new();
        for event in &events {
            if let Some(id) = property(event, "RECURRENCE-ID") {
                if param(id, "RANGE").is_some() {
                    return Err("ICS RANGE recurrence overrides are not supported".into());
                }
                let (date, _) = stamp(
                    id,
                    id.value.as_deref().ok_or("Empty recurrence ID")?,
                    floating,
                )?;
                overrides.insert(date.timestamp(), event);
            }
        }
        let master = events
            .iter()
            .filter(|e| property(e, "RECURRENCE-ID").is_none())
            .max_by_key(|e| {
                value(e, "SEQUENCE")
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0)
            });
        if let Some(master) = master {
            if value(master, "STATUS") != Some("CANCELLED") {
                let (starts, length, all_day) = instances(master, floating, from, to)?;
                for start in starts {
                    if !overrides.contains_key(&start.timestamp()) {
                        emit(
                            &mut output,
                            feed,
                            &uid,
                            master,
                            start,
                            length,
                            all_day,
                            display_zone,
                            from,
                            to,
                        )?;
                    }
                }
            }
        }
        for event in overrides.values() {
            if value(event, "STATUS") == Some("CANCELLED") {
                continue;
            }
            let (starts, length, all_day) = instances(event, floating, from, to)?;
            for start in starts {
                emit(
                    &mut output,
                    feed,
                    &uid,
                    event,
                    start,
                    length,
                    all_day,
                    display_zone,
                    from,
                    to,
                )?;
            }
        }
        if output.len() > 20_000 {
            return Err("Calendar expansion exceeds the supported event limit".into());
        }
    }
    Ok(output)
}

fn instances(
    event: &IcalEvent,
    floating: Tz,
    from: LocalDate,
    to: LocalDate,
) -> Result<(Vec<DateTime<Tz>>, Duration, bool), String> {
    let start_property = property(event, "DTSTART").ok_or("An ICS event is missing DTSTART")?;
    let (start, all_day) = stamp(
        start_property,
        start_property.value.as_deref().ok_or("Empty DTSTART")?,
        floating,
    )?;
    let length = if let Some(end) = property(event, "DTEND") {
        let (end, end_all_day) = stamp(end, end.value.as_deref().ok_or("Empty DTEND")?, floating)?;
        if end_all_day != all_day {
            return Err("DTSTART and DTEND have different value types".into());
        }
        end.signed_duration_since(start)
    } else if let Some(raw) = value(event, "DURATION") {
        duration(raw)?
    } else if all_day {
        Duration::days(1)
    } else {
        Duration::zero()
    };
    if length < Duration::zero() || length > Duration::days(3660) {
        return Err("Invalid or excessively long ICS event".into());
    }
    let mut set = RRuleSet::new(start).rdate(start);
    for prop in &event.properties {
        let Some(raw) = prop.value.as_deref() else {
            continue;
        };
        match prop.name.as_str() {
            "RRULE" => {
                let normalized = raw
                    .split(';')
                    .map(|part| {
                        if let Some(date) =
                            part.strip_prefix("UNTIL=").filter(|date| date.len() == 8)
                        {
                            format!("UNTIL={date}T235959Z")
                        } else {
                            part.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(";");
                let rule = normalized
                    .parse::<RRule<Unvalidated>>()
                    .map_err(|_| "Unsupported ICS recurrence rule")?
                    .validate(start)
                    .map_err(|_| "Invalid ICS recurrence rule")?;
                set = set.rrule(rule);
            }
            "RDATE" | "EXDATE" => {
                for item in raw.split(',') {
                    let (date, date_only) = stamp(prop, item, floating)?;
                    if date_only != all_day {
                        return Err("Recurrence date types do not match DTSTART".into());
                    }
                    set = if prop.name == "EXDATE" {
                        set.exdate(date)
                    } else {
                        set.rdate(date)
                    };
                }
            }
            "EXRULE" => {
                return Err("Deprecated EXRULE is not supported; export EXDATE exceptions".into())
            }
            _ => {}
        }
    }
    let at_midnight = |date: LocalDate| -> Result<DateTime<Tz>, String> {
        Tz::UTC
            .with_ymd_and_hms(date.year, date.month as u32, date.day as u32, 0, 0, 0)
            .single()
            .ok_or_else(|| "Unsupported recurrence range".into())
    };
    let dates = set
        .after(at_midnight(from)? - length - Duration::days(2))
        .before(at_midnight(to)? + Duration::days(2))
        .all(10_000);
    if dates.limited {
        return Err("ICS recurrence expansion reached its safety limit".into());
    }
    let mut starts = dates.dates;
    starts.sort_unstable();
    starts.dedup();
    Ok((starts, length, all_day))
}

#[allow(clippy::too_many_arguments)]
fn emit(
    output: &mut Vec<CalendarOccurrence>,
    feed: &IcsSubscription,
    uid: &str,
    event: &IcalEvent,
    start: DateTime<Tz>,
    length: Duration,
    all_day: bool,
    display: Tz,
    from: LocalDate,
    to: LocalDate,
) -> Result<(), String> {
    let end = start
        .checked_add_signed(length)
        .ok_or("ICS event end overflow")?;
    let local_start = if all_day {
        start
    } else {
        start.with_timezone(&display)
    };
    let local_end = if all_day {
        end
    } else {
        end.with_timezone(&display)
    };
    let date = |dt: DateTime<Tz>| {
        LocalDate::new(dt.year(), dt.month() as u8, dt.day() as u8)
            .ok_or_else(|| "Unsupported ICS event date".to_string())
    };
    let start_date = date(local_start)?;
    let end_date = date(local_end)?;
    let exclusive =
        if all_day || (local_end.time() == chrono::NaiveTime::MIN && end_date > start_date) {
            end_date
        } else {
            end_date.add_days(1)
        }
        .max(start_date.add_days(1));
    if start_date >= to || exclusive <= from {
        return Ok(());
    }
    output.push(CalendarOccurrence {
        source_id: feed.id.clone(),
        event_id: format!("{uid}@{}", start.timestamp()),
        color_hex: feed.color.clone(),
        start_date,
        end_date_exclusive: exclusive,
        is_all_day: all_day,
        title: unescape(value(event, "SUMMARY").unwrap_or("")),
        time_label: if all_day {
            String::new()
        } else {
            format!("{:02}:{:02}", local_start.hour(), local_start.minute())
        },
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn feed() -> IcsSubscription {
        IcsSubscription {
            id: "ics:test".into(),
            name: "Test".into(),
            url: "https://example.org/calendar.ics".into(),
            color: "#123456".into(),
        }
    }
    fn parse_events(events: &str) -> Vec<CalendarOccurrence> {
        parse(
            &format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\n{events}END:VCALENDAR\r\n"),
            &feed(),
            LocalDate::new(2026, 10, 30).unwrap(),
            Tz::UTC,
        )
        .unwrap()
    }
    #[test]
    fn folded_text_escaped_summary_and_exclusive_all_day_end() {
        let events=parse_events("BEGIN:VEVENT\r\nUID:a\r\nDTSTART;VALUE=DATE:20261030\r\nDTEND;VALUE=DATE:20261102\r\nSUMMARY:Hello\\, world\r\n !\r\nEND:VEVENT\r\n");
        assert_eq!(events[0].title, "Hello, world!");
        assert_eq!(
            events[0].end_date_exclusive,
            LocalDate::new(2026, 11, 2).unwrap()
        );
    }
    #[test]
    fn recurring_event_exclusion_and_moved_override() {
        let events=parse_events("BEGIN:VEVENT\r\nUID:a\r\nDTSTART:20261026T090000Z\r\nDURATION:PT1H\r\nRRULE:FREQ=DAILY;COUNT=5\r\nEXDATE:20261027T090000Z\r\nSUMMARY:Original\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:a\r\nRECURRENCE-ID:20261028T090000Z\r\nDTSTART:20261028T110000Z\r\nDURATION:PT1H\r\nSUMMARY:Moved\r\nEND:VEVENT\r\n");
        assert_eq!(events.len(), 4);
        assert!(!events.iter().any(|e| e.start_date.day == 27));
        assert_eq!(
            events
                .iter()
                .find(|e| e.title == "Moved")
                .unwrap()
                .time_label,
            "11:00"
        );
    }
    #[test]
    fn weekly_iana_recurrence_preserves_wall_clock_across_dst() {
        let events=parse_events("BEGIN:VEVENT\r\nUID:dst\r\nDTSTART;TZID=Europe/Belgrade:20261023T090000\r\nDURATION:PT1H\r\nRRULE:FREQ=WEEKLY;COUNT=2\r\nEND:VEVENT\r\n");
        assert_eq!(
            events
                .iter()
                .map(|e| e.time_label.as_str())
                .collect::<Vec<_>>(),
            vec!["07:00", "08:00"]
        );
    }
    #[test]
    fn whitespace_around_dates_is_trimmed() {
        let events = parse_events(
            "BEGIN:VEVENT\r\nUID:trim\r\nDTSTART;VALUE=DATE: 20261030 \r\nDTEND;VALUE=DATE: 20261031 \r\nSUMMARY:Trimmed\r\nEND:VEVENT\r\n",
        );
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].start_date, LocalDate::new(2026, 10, 30).unwrap());
        assert_eq!(
            events[0].end_date_exclusive,
            LocalDate::new(2026, 10, 31).unwrap()
        );
    }
    #[test]
    fn unknown_timezone_fails_instead_of_silently_moving_events() {
        assert!(parse("BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:a\r\nDTSTART;TZID=MadeUp/Zone:20261030T090000\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",&feed(),LocalDate::new(2026,10,30).unwrap(),Tz::UTC).is_err());
    }
}
