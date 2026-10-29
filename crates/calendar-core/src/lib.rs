//! Pure Gregorian calendar calculations, navigation state, and event-dot mapping.
//!
//! This crate intentionally contains no OS calls, system-clock queries, or UI
//! toolkit types. Callers pass an explicit `today` date and preferences so
//! every state transition and edge case is deterministically testable.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// ISO weekday from Monday (`0`) to Sunday (`6`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Weekday {
    Monday = 0,
    Tuesday = 1,
    Wednesday = 2,
    Thursday = 3,
    Friday = 4,
    Saturday = 5,
    Sunday = 6,
}

impl Weekday {
    pub const ALL: [Weekday; 7] = [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ];

    pub const fn from_monday_index(index: u8) -> Self {
        match index % 7 {
            0 => Weekday::Monday,
            1 => Weekday::Tuesday,
            2 => Weekday::Wednesday,
            3 => Weekday::Thursday,
            4 => Weekday::Friday,
            5 => Weekday::Saturday,
            _ => Weekday::Sunday,
        }
    }

    pub const fn monday_index(self) -> u8 {
        self as u8
    }

    /// Returns the 7 weekdays ordered starting from `first_weekday`.
    pub fn ordered_from(first_weekday: Weekday) -> [Weekday; 7] {
        let start = first_weekday.monday_index();
        [
            Self::from_monday_index(start),
            Self::from_monday_index(start + 1),
            Self::from_monday_index(start + 2),
            Self::from_monday_index(start + 3),
            Self::from_monday_index(start + 4),
            Self::from_monday_index(start + 5),
            Self::from_monday_index(start + 6),
        ]
    }
}

/// Configurable set of shaded ("gray") weekdays.
///
/// Any subset of Monday–Sunday (including empty or all seven) can be selected,
/// independent of the user's `first_weekday` preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WeekdaySet(u8);

impl WeekdaySet {
    pub const NONE: Self = Self(0);
    pub const ALL: Self = Self(0b0111_1111);
    pub const DEFAULT_WEEKENDS: Self =
        Self((1 << (Weekday::Saturday as u8)) | (1 << (Weekday::Sunday as u8)));

    pub const fn empty() -> Self {
        Self::NONE
    }

    pub const fn default_weekends() -> Self {
        Self::DEFAULT_WEEKENDS
    }

    pub const fn from_bits(bits: u8) -> Self {
        Self(bits & 0b0111_1111)
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn contains(self, day: Weekday) -> bool {
        (self.0 & (1 << (day as u8))) != 0
    }

    pub fn insert(&mut self, day: Weekday) {
        self.0 |= 1 << (day as u8);
    }

    pub fn remove(&mut self, day: Weekday) {
        self.0 &= !(1 << (day as u8));
    }

    pub fn toggle(&mut self, day: Weekday) -> bool {
        if self.contains(day) {
            self.remove(day);
            false
        } else {
            self.insert(day);
            true
        }
    }

    pub fn to_vec(self) -> Vec<Weekday> {
        Weekday::ALL
            .into_iter()
            .filter(|&d| self.contains(d))
            .collect()
    }

    pub fn from_slice(days: &[Weekday]) -> Self {
        let mut set = Self::NONE;
        for &day in days {
            set.insert(day);
        }
        set
    }
}

impl Default for WeekdaySet {
    fn default() -> Self {
        Self::DEFAULT_WEEKENDS
    }
}

/// Proleptic Gregorian year and month (`1..=12`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct YearMonth {
    pub year: i32,
    pub month: u8,
}

impl YearMonth {
    pub fn new(year: i32, month: u8) -> Option<Self> {
        if (1..=12).contains(&month) {
            Some(Self { year, month })
        } else {
            None
        }
    }

    pub fn days_in_month(self) -> u8 {
        days_in_month(self.year, self.month)
    }

    pub fn prev_month(self) -> Self {
        if self.month == 1 {
            Self {
                year: self.year - 1,
                month: 12,
            }
        } else {
            Self {
                year: self.year,
                month: self.month - 1,
            }
        }
    }

    pub fn next_month(self) -> Self {
        if self.month == 12 {
            Self {
                year: self.year + 1,
                month: 1,
            }
        } else {
            Self {
                year: self.year,
                month: self.month + 1,
            }
        }
    }

    pub fn add_months(self, delta: i32) -> Self {
        let total = self.year * 12 + (self.month as i32 - 1) + delta;
        let year = total.div_euclid(12);
        let month = (total.rem_euclid(12) + 1) as u8;
        Self { year, month }
    }
}

/// Proleptic Gregorian calendar date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LocalDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl LocalDate {
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        if !(1..=12).contains(&month) {
            return None;
        }
        let max_day = days_in_month(year, month);
        if (1..=max_day).contains(&day) {
            Some(Self { year, month, day })
        } else {
            None
        }
    }

    pub const fn year_month(self) -> YearMonth {
        YearMonth {
            year: self.year,
            month: self.month,
        }
    }

    /// Converts the date to days since 1970-01-01 (Howard Hinnant's civil algorithm).
    pub const fn to_epoch_days(self) -> i64 {
        let y = self.year as i64 - if self.month <= 2 { 1 } else { 0 };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = (y - era * 400) as u64;
        let m = self.month as u64;
        let d = self.day as u64;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146097 + doe as i64 - 719468
    }

    /// Converts days since 1970-01-01 back to a `LocalDate`.
    pub const fn from_epoch_days(z: i64) -> Self {
        let z = z + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = (z - era * 146097) as u64;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe as i64 + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = (y + if m <= 2 { 1 } else { 0 }) as i32;
        Self {
            year,
            month: m as u8,
            day: d as u8,
        }
    }

    pub fn add_days(self, delta: i32) -> Self {
        Self::from_epoch_days(self.to_epoch_days() + delta as i64)
    }

    pub fn weekday(self) -> Weekday {
        // 1970-01-01 was a Thursday (Monday index = 3).
        let idx = (self.to_epoch_days() + 3).rem_euclid(7) as u8;
        Weekday::from_monday_index(idx)
    }

    /// 1-based ordinal day of the year (`1..=366`).
    pub fn ordinal(self) -> u16 {
        let jan1 = Self {
            year: self.year,
            month: 1,
            day: 1,
        };
        (self.to_epoch_days() - jan1.to_epoch_days() + 1) as u16
    }

    /// ISO-8601 week-year and week number (`1..=53`).
    pub fn iso_year_week(self) -> (i32, u8) {
        // In ISO-8601, a week belongs to the year that contains its Thursday.
        let weekday_idx = self.weekday().monday_index() as i32; // Mon=0..Sun=6
        let thursday = self.add_days(3 - weekday_idx);
        let iso_year = thursday.year;
        let jan4 = Self {
            year: iso_year,
            month: 1,
            day: 4,
        };
        let jan4_mon_offset = jan4.weekday().monday_index() as i32;
        let week1_monday = jan4.add_days(-jan4_mon_offset);
        let week = ((self.to_epoch_days() - week1_monday.to_epoch_days()) / 7 + 1) as u8;
        (iso_year, week)
    }

    pub fn iso_week(self) -> u8 {
        self.iso_year_week().1
    }
}

impl fmt::Display for LocalDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// Returns true if `year` is a Gregorian leap year.
pub const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Returns the number of days in `month` (`1..=12`) for `year`.
pub const fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Single day cell in a 6-row (42-cell) month grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayCell {
    pub date: LocalDate,
    pub in_displayed_month: bool,
    pub is_today: bool,
    pub is_selected: bool,
    pub is_gray_weekday: bool,
    /// Up to 3 calendar colors for event dots below the numeral.
    pub dot_colors: Vec<String>,
    /// Exact total number of deduplicated event occurrences on this date.
    pub total_event_count: usize,
}

/// Complete 6-row × 7-column month view projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthGrid {
    pub displayed_month: YearMonth,
    pub is_current_month: bool,
    pub weekdays: [Weekday; 7],
    pub gray_columns: [bool; 7],
    pub iso_weeks: [u8; 6],
    pub cells: Vec<DayCell>,
}

/// Read-only calendar source exposed by a platform provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarSource {
    pub id: String,
    pub name: String,
    pub account_label: String,
    pub color_hex: String,
}

/// Read-only event occurrence mapped to local calendar dates (`[start_date, end_date_exclusive)`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarOccurrence {
    pub source_id: String,
    pub event_id: String,
    pub color_hex: String,
    /// Inclusive local start date of the occurrence.
    pub start_date: LocalDate,
    /// Exclusive local end date (`end_date_exclusive > start_date`).
    /// A single-day event on `2026-10-29` has `start_date = 2026-10-29` and
    /// `end_date_exclusive = 2026-10-30`.
    pub end_date_exclusive: LocalDate,
    pub is_all_day: bool,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub time_label: String,
}

impl CalendarOccurrence {
    /// Returns true if this occurrence covers `date` under exclusive-end semantics.
    pub fn covers_date(&self, date: LocalDate) -> bool {
        date >= self.start_date && date < self.end_date_exclusive
    }
}

/// Projects occurrences onto local dates for the checked `selected_source_ids`.
///
/// Deduplicates per `(source_id, event_id)` on each date and retains up to 3 dot colors
/// while preserving the exact `total_event_count`.
pub fn project_event_dots(
    occurrences: &[CalendarOccurrence],
    selected_source_ids: &BTreeSet<String>,
    range_start_inclusive: LocalDate,
    range_end_inclusive: LocalDate,
) -> BTreeMap<LocalDate, (Vec<String>, usize)> {
    let mut per_day_seen: BTreeMap<LocalDate, BTreeSet<(String, String)>> = BTreeMap::new();
    let mut per_day_colors: BTreeMap<LocalDate, Vec<String>> = BTreeMap::new();

    if selected_source_ids.is_empty() {
        return BTreeMap::new();
    }

    let start_epoch = range_start_inclusive.to_epoch_days();
    let end_epoch = range_end_inclusive.to_epoch_days();

    for occ in occurrences {
        if !selected_source_ids.contains(&occ.source_id) {
            continue;
        }
        let occ_start = occ.start_date.to_epoch_days().max(start_epoch);
        let occ_end_excl = occ.end_date_exclusive.to_epoch_days().min(end_epoch + 1);
        if occ_end_excl <= occ_start {
            continue;
        }
        for day_epoch in occ_start..occ_end_excl {
            let date = LocalDate::from_epoch_days(day_epoch);
            let seen = per_day_seen.entry(date).or_default();
            if seen.insert((occ.source_id.clone(), occ.event_id.clone())) {
                let colors = per_day_colors.entry(date).or_default();
                if colors.len() < 3 {
                    colors.push(occ.color_hex.clone());
                }
            }
        }
    }

    per_day_seen
        .into_iter()
        .map(|(date, seen)| {
            let colors = per_day_colors.remove(&date).unwrap_or_default();
            (date, (colors, seen.len()))
        })
        .collect()
}

/// Pure calendar navigation and selection state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarState {
    pub today: LocalDate,
    pub displayed_month: YearMonth,
    pub selected_date: LocalDate,
    pub first_weekday: Weekday,
    pub gray_weekdays: WeekdaySet,
}

impl CalendarState {
    pub fn new(today: LocalDate, first_weekday: Weekday, gray_weekdays: WeekdaySet) -> Self {
        Self {
            today,
            displayed_month: today.year_month(),
            selected_date: today,
            first_weekday,
            gray_weekdays,
        }
    }

    /// True when the displayed year and month equal `today`'s year and month.
    pub fn is_current_month(&self) -> bool {
        self.displayed_month == self.today.year_month()
    }

    /// Navigates to the previous month.
    pub fn prev_month(&mut self) {
        self.displayed_month = self.displayed_month.prev_month();
    }

    /// Navigates to the next month.
    pub fn next_month(&mut self) {
        self.displayed_month = self.displayed_month.next_month();
    }

    /// Navigates by `delta` months (`-12` / `+12` for year jumps).
    pub fn navigate_months(&mut self, delta: i32) {
        self.displayed_month = self.displayed_month.add_months(delta);
    }

    /// Restores both `displayed_month` and `selected_date` to `today`.
    pub fn return_to_today(&mut self) {
        self.displayed_month = self.today.year_month();
        self.selected_date = self.today;
    }

    /// Selects a specific date and switches `displayed_month` if the date is in an adjacent month.
    pub fn select_date(&mut self, date: LocalDate) {
        self.selected_date = date;
        self.displayed_month = date.year_month();
    }

    /// Moves `selected_date` by `delta_days` (e.g. `-1`, `+1`, `-7`, `+7` for arrow keys)
    /// and keeps `displayed_month` synchronized with the new selection.
    pub fn move_selection_by_days(&mut self, delta_days: i32) {
        let next = self.selected_date.add_days(delta_days);
        self.select_date(next);
    }

    /// Updates `today` after local midnight rollover or timezone change.
    /// If the user was previously on `old_today` viewing `old_today`'s month,
    /// keeps them tracking the new `today`.
    pub fn update_today(&mut self, new_today: LocalDate) {
        let was_on_today =
            self.selected_date == self.today && self.displayed_month == self.today.year_month();
        self.today = new_today;
        if was_on_today {
            self.selected_date = new_today;
            self.displayed_month = new_today.year_month();
        }
    }

    /// Builds the 42-cell (6-row × 7-column) month grid with optional event occurrences.
    pub fn build_grid(
        &self,
        occurrences: &[CalendarOccurrence],
        selected_source_ids: &BTreeSet<String>,
    ) -> MonthGrid {
        let weekdays = Weekday::ordered_from(self.first_weekday);
        let gray_columns = weekdays.map(|d| self.gray_weekdays.contains(d));

        let first_of_month = LocalDate {
            year: self.displayed_month.year,
            month: self.displayed_month.month,
            day: 1,
        };
        let first_wd = first_of_month.weekday().monday_index();
        let start_wd = self.first_weekday.monday_index();
        let leading_days = (first_wd + 7 - start_wd) % 7;
        let grid_start = first_of_month.add_days(-(leading_days as i32));
        let grid_end = grid_start.add_days(41);

        let dots_by_date =
            project_event_dots(occurrences, selected_source_ids, grid_start, grid_end);

        let mut iso_weeks = [0u8; 6];
        let mut cells = Vec::with_capacity(42);

        for idx in 0..42 {
            let date = grid_start.add_days(idx as i32);
            let row = idx / 7;
            let col = idx % 7;
            if col == 0 {
                // Use Thursday of the displayed row (or the row's first date) for the ISO week number.
                iso_weeks[row] = date.add_days(3).iso_week();
            }
            let (dot_colors, total_event_count) = dots_by_date
                .get(&date)
                .cloned()
                .unwrap_or_else(|| (Vec::new(), 0));

            cells.push(DayCell {
                date,
                in_displayed_month: date.year_month() == self.displayed_month,
                is_today: date == self.today,
                is_selected: date == self.selected_date,
                is_gray_weekday: gray_columns[col],
                dot_colors,
                total_event_count,
            });
        }

        MonthGrid {
            displayed_month: self.displayed_month,
            is_current_month: self.is_current_month(),
            weekdays,
            gray_columns,
            iso_weeks,
            cells,
        }
    }

    /// Returns deduplicated event occurrences covering `date` from `selected_source_ids`.
    pub fn events_for_date(
        &self,
        date: LocalDate,
        occurrences: &[CalendarOccurrence],
        selected_source_ids: &BTreeSet<String>,
    ) -> Vec<CalendarOccurrence> {
        if selected_source_ids.is_empty() {
            return Vec::new();
        }
        let mut seen = BTreeSet::new();
        let mut result = Vec::new();
        for occ in occurrences {
            if !selected_source_ids.contains(&occ.source_id) {
                continue;
            }
            if occ.covers_date(date) && seen.insert((occ.source_id.clone(), occ.event_id.clone())) {
                result.push(occ.clone());
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leap_year_century_rules() {
        assert!(is_leap_year(2000));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(1900));
        assert!(!is_leap_year(2026));
        assert!(!is_leap_year(2100));
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
        assert!(LocalDate::new(2024, 2, 29).is_some());
        assert!(LocalDate::new(2026, 2, 29).is_none());
    }

    #[test]
    fn epoch_roundtrip_and_weekdays() {
        let oct29 = LocalDate::new(2026, 10, 29).unwrap();
        assert_eq!(oct29.weekday(), Weekday::Thursday);
        assert_eq!(LocalDate::from_epoch_days(oct29.to_epoch_days()), oct29);

        let leap_day = LocalDate::new(2024, 2, 29).unwrap();
        assert_eq!(leap_day.add_days(1), LocalDate::new(2024, 3, 1).unwrap());
        assert_eq!(
            LocalDate::new(2026, 12, 31).unwrap().add_days(1),
            LocalDate::new(2027, 1, 1).unwrap()
        );
    }

    #[test]
    fn iso_week_year_boundaries() {
        // 2026-10-29 is in ISO Week 44 of 2026.
        assert_eq!(
            LocalDate::new(2026, 10, 29).unwrap().iso_year_week(),
            (2026, 44)
        );
        // 2026 has 53 ISO weeks: 2026-12-31 is Thursday of W53, and 2027-01-01..03 stay in 2026-W53.
        assert_eq!(
            LocalDate::new(2026, 12, 31).unwrap().iso_year_week(),
            (2026, 53)
        );
        assert_eq!(
            LocalDate::new(2027, 1, 1).unwrap().iso_year_week(),
            (2026, 53)
        );
        assert_eq!(
            LocalDate::new(2027, 1, 4).unwrap().iso_year_week(),
            (2027, 1)
        );
    }

    #[test]
    fn configurable_gray_weekdays_independent_of_first_weekday() {
        let today = LocalDate::new(2026, 10, 29).unwrap();
        // Custom subset: Wednesday and Friday shaded, with Sunday as first day of week.
        let gray = WeekdaySet::from_slice(&[Weekday::Wednesday, Weekday::Friday]);
        let state = CalendarState::new(today, Weekday::Sunday, gray);
        let grid = state.build_grid(&[], &BTreeSet::new());

        assert_eq!(grid.weekdays[0], Weekday::Sunday);
        assert_eq!(
            grid.gray_columns,
            [false, false, false, true, false, true, false]
        );
    }

    #[test]
    fn current_month_emphasis_and_navigation() {
        let today = LocalDate::new(2026, 10, 29).unwrap();
        let mut state = CalendarState::new(today, Weekday::Monday, WeekdaySet::default_weekends());
        assert!(state.is_current_month());

        state.select_date(LocalDate::new(2026, 10, 30).unwrap());
        assert!(state.is_current_month());

        state.next_month();
        assert_eq!(
            state.displayed_month,
            YearMonth {
                year: 2026,
                month: 11
            }
        );
        assert!(!state.is_current_month());

        // Same month in a different year must NOT be marked as current month.
        state.displayed_month = YearMonth {
            year: 2025,
            month: 10,
        };
        assert!(!state.is_current_month());

        state.return_to_today();
        assert!(state.is_current_month());
        assert_eq!(state.selected_date, today);
    }

    #[test]
    fn event_dot_filtering_exclusive_end_and_three_dot_cap() {
        let today = LocalDate::new(2026, 10, 29).unwrap();
        let state = CalendarState::new(today, Weekday::Monday, WeekdaySet::default_weekends());

        let occurrences = vec![
            // All-day event on Oct 29 only (exclusive end Oct 30 -> must NOT appear on Oct 30)
            CalendarOccurrence {
                source_id: "work".into(),
                event_id: "evt-1".into(),
                color_hex: "#5E6AD2".into(),
                start_date: LocalDate::new(2026, 10, 29).unwrap(),
                end_date_exclusive: LocalDate::new(2026, 10, 30).unwrap(),
                is_all_day: true,
                title: String::new(),
                time_label: String::new(),
            },
            // Duplicate of evt-1 from same source on Oct 29 -> must be deduplicated
            CalendarOccurrence {
                source_id: "work".into(),
                event_id: "evt-1".into(),
                color_hex: "#5E6AD2".into(),
                start_date: LocalDate::new(2026, 10, 29).unwrap(),
                end_date_exclusive: LocalDate::new(2026, 10, 30).unwrap(),
                is_all_day: false,
                title: String::new(),
                time_label: String::new(),
            },
            // Multi-day event Oct 29..Nov 1 (covers Oct 29, 30, 31)
            CalendarOccurrence {
                source_id: "personal".into(),
                event_id: "evt-2".into(),
                color_hex: "#F59E0B".into(),
                start_date: LocalDate::new(2026, 10, 29).unwrap(),
                end_date_exclusive: LocalDate::new(2026, 11, 1).unwrap(),
                is_all_day: true,
                title: String::new(),
                time_label: String::new(),
            },
            // Two more events on Oct 29 to test >3 events cap
            CalendarOccurrence {
                source_id: "work".into(),
                event_id: "evt-3".into(),
                color_hex: "#5E6AD2".into(),
                start_date: LocalDate::new(2026, 10, 29).unwrap(),
                end_date_exclusive: LocalDate::new(2026, 10, 30).unwrap(),
                is_all_day: false,
                title: String::new(),
                time_label: String::new(),
            },
            CalendarOccurrence {
                source_id: "personal".into(),
                event_id: "evt-4".into(),
                color_hex: "#F59E0B".into(),
                start_date: LocalDate::new(2026, 10, 29).unwrap(),
                end_date_exclusive: LocalDate::new(2026, 10, 30).unwrap(),
                is_all_day: false,
                title: String::new(),
                time_label: String::new(),
            },
            // Unchecked calendar source -> must be ignored
            CalendarOccurrence {
                source_id: "holidays".into(),
                event_id: "evt-5".into(),
                color_hex: "#10B981".into(),
                start_date: LocalDate::new(2026, 10, 29).unwrap(),
                end_date_exclusive: LocalDate::new(2026, 10, 30).unwrap(),
                is_all_day: true,
                title: String::new(),
                time_label: String::new(),
            },
        ];

        let mut selected = BTreeSet::new();
        selected.insert("work".to_string());
        selected.insert("personal".to_string());

        let grid = state.build_grid(&occurrences, &selected);
        let oct29_cell = grid
            .cells
            .iter()
            .find(|c| c.date == LocalDate::new(2026, 10, 29).unwrap())
            .unwrap();
        assert_eq!(oct29_cell.total_event_count, 4);
        assert_eq!(oct29_cell.dot_colors.len(), 3); // Capped at 3 dots!

        let oct30_cell = grid
            .cells
            .iter()
            .find(|c| c.date == LocalDate::new(2026, 10, 30).unwrap())
            .unwrap();
        assert_eq!(oct30_cell.total_event_count, 1); // Only multi-day evt-2 covers Oct 30
        assert_eq!(oct30_cell.dot_colors, vec!["#F59E0B".to_string()]);
    }
}
