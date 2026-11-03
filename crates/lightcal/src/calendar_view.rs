//! Calendar popup presentation (ADR 0011): header text, shaded bands and the
//! selected day's agenda with now/next state, details, Copy and Open.
//! Calendar rules live in `calendar_core`; this module only formats them for Slint.

use crate::{mark_color, AppController, AppWindow};
use crate::{AgendaChipRow, AgendaEventItem, ShadedBand};
use calendar_core::{
    agenda_timings, local_minutes, next_occurrence_after, sort_agenda, split_local_minutes,
    AgendaTiming, CalendarOccurrence, LocalDate, MonthGrid,
};
use lightcal::i18n::{
    agenda_copy, catalog, format_date_line, format_duration, format_event_count, format_short_date,
    format_starts_in, month_name,
};
use lightcal::preferences::LocaleCode;
use slint::{Model, ModelRc, VecModel};
use std::{cell::RefCell, rc::Rc};

/// Identifies one occurrence on one agenda day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EventKey {
    date: LocalDate,
    source_id: String,
    event_id: String,
}

impl EventKey {
    const SEPARATOR: char = '\u{1f}';

    fn for_event(date: LocalDate, event: &CalendarOccurrence) -> Self {
        Self {
            date,
            source_id: event.source_id.clone(),
            event_id: event.event_id.clone(),
        }
    }

    /// The row key handed to Slint: date, source and event id joined by U+001F.
    pub(crate) fn encode(&self) -> String {
        let d = self.date;
        let sep = Self::SEPARATOR;
        format!(
            "{}-{}-{}{sep}{}{sep}{}",
            d.year, d.month, d.day, self.source_id, self.event_id
        )
    }

    fn decode(raw: &str) -> Option<Self> {
        let mut parts = raw.splitn(3, Self::SEPARATOR);
        let mut date = parts.next()?.splitn(3, '-');
        let date = LocalDate::new(
            date.next()?.parse().ok()?,
            date.next()?.parse().ok()?,
            date.next()?.parse().ok()?,
        )?;
        Some(Self {
            date,
            source_id: parts.next()?.to_string(),
            event_id: parts.next()?.to_string(),
        })
    }
}

/// Agenda interaction state kept between UI syncs.
#[derive(Debug, Default)]
pub(crate) struct AgendaUiState {
    pub(crate) expanded: Option<EventKey>,
    pub(crate) copied: Option<EventKey>,
    /// Increments on every Copy so a stale "Copied" timeout does not clear a newer one.
    pub(crate) copy_generation: u64,
    /// Fixed local wall-clock time ([`local_minutes`]) for tests and snapshots.
    pub(crate) now_override: Option<i64>,
    /// The event just expanded by a click, so only its details fade in. It lasts until those
    /// details have faded in (the panel reports it), or the day changes, or Settings opens,
    /// so a refresh right after the click cannot cancel the fade and details shown again
    /// later (rows rebuilt, another day and back, Settings and back) appear at once.
    pub(crate) fade: RefCell<Option<EventKey>>,
}

impl AppController {
    /// The selected day's events from checked calendars, all-day first.
    pub(crate) fn day_agenda(&self) -> Vec<CalendarOccurrence> {
        let mut events = self.cal_state.events_for_date(
            self.cal_state.selected_date,
            &self.provider.occurrences,
            &self.prefs.selected_calendar_ids,
        );
        sort_agenda(&mut events);
        events
    }

    /// The row key of the `index`-th event of the selected day (tests and snapshots).
    pub(crate) fn agenda_key_at(&self, index: usize) -> String {
        self.day_agenda()
            .get(index)
            .map(|event| EventKey::for_event(self.cal_state.selected_date, event).encode())
            .unwrap_or_default()
    }

    /// Resolves a row key from Slint to the current occurrence; `None` if it disappeared.
    fn agenda_key(&self, row_key: &str) -> Option<(EventKey, CalendarOccurrence)> {
        let key = EventKey::decode(row_key)?;
        if key.date != self.cal_state.selected_date {
            return None;
        }
        let event = self
            .day_agenda()
            .into_iter()
            .find(|event| event.source_id == key.source_id && event.event_id == key.event_id)?;
        Some((key, event))
    }

    /// Expands the event at `index`, or collapses it when it is already open.
    pub(crate) fn toggle_agenda_event(&mut self, row_key: &str) {
        let Some((key, _)) = self.agenda_key(row_key) else {
            return;
        };
        self.agenda.copied = None;
        self.agenda.expanded = if self.agenda.expanded.as_ref() == Some(&key) {
            None
        } else {
            Some(key)
        };
        *self.agenda.fade.borrow_mut() = self.agenda.expanded.clone();
    }

    /// The details of `row_key` have faded in: no later rebuild fades them again.
    pub(crate) fn agenda_details_faded(&self, row_key: &str) {
        let mut fade = self.agenda.fade.borrow_mut();
        if fade.as_ref().map(EventKey::encode).as_deref() == Some(row_key) {
            *fade = None;
        }
    }

    /// Plain-text details of the event at `index` for the clipboard and the copy
    /// generation to pass to [`Self::clear_copied`]; marks the event copied.
    pub(crate) fn copy_agenda_event(&mut self, row_key: &str) -> Option<(String, u64)> {
        let (key, event) = self.agenda_key(row_key)?;
        let locale = self.prefs.locale();
        let mut lines = vec![self.event_title(&event).to_string()];
        let date = format_date_line(locale, self.cal_state.selected_date, true);
        lines.push(format!("{date} · {}", detail_time(locale, &event)));
        if let Some(location) = &event.location {
            lines.push(location.clone());
        }
        lines.push(self.provider_line(&event));
        if let Some(link) = openable_link(&event) {
            lines.push(link.to_string());
        }
        self.agenda.copied = Some(key);
        self.agenda.copy_generation += 1;
        // Windows text controls expect CRLF line breaks.
        Some((lines.join("\r\n"), self.agenda.copy_generation))
    }

    /// Returns the Copy action to its normal label unless a newer copy happened.
    pub(crate) fn clear_copied(&mut self, generation: u64) -> bool {
        let current = self.agenda.copy_generation == generation && self.agenda.copied.is_some();
        if current {
            self.agenda.copied = None;
        }
        current
    }

    /// The validated provider page of the event at `index`.
    pub(crate) fn agenda_event_link(&self, row_key: &str) -> Option<String> {
        let (_, event) = self.agenda_key(row_key)?;
        openable_link(&event).map(String::from)
    }

    /// Selects the date of the first event after the selected day.
    pub(crate) fn go_next_event(&mut self) -> bool {
        let Some(date) = next_occurrence_after(
            self.cal_state.selected_date,
            &self.provider.occurrences,
            &self.prefs.selected_calendar_ids,
        )
        .map(|event| event.start_date) else {
            return false;
        };
        self.cal_state.select_date(date);
        true
    }

    fn now_minutes(&self) -> i64 {
        self.agenda.now_override.unwrap_or_else(|| {
            use chrono::{Datelike, Timelike};
            let now = chrono::Local::now();
            let date = LocalDate::new(now.year(), now.month() as u8, now.day() as u8)
                .unwrap_or(self.cal_state.today);
            local_minutes(date, now.hour() * 60 + now.minute())
        })
    }

    fn event_title<'a>(&self, event: &'a CalendarOccurrence) -> &'a str {
        if event.title.is_empty() {
            catalog(self.prefs.locale()).agenda_event_fallback
        } else {
            &event.title
        }
    }

    fn provider_line(&self, event: &CalendarOccurrence) -> String {
        let strings = catalog(self.prefs.locale());
        let source = self
            .provider
            .sources
            .iter()
            .find(|source| source.id == event.source_id);
        let name = source
            .map(|source| source.name.as_str())
            .unwrap_or(strings.agenda_calendar_fallback);
        let provider = if event.source_id.starts_with("google:") {
            "Google Calendar"
        } else if event.source_id.starts_with("graph:") {
            "Outlook"
        } else if event.source_id.starts_with("ics:") {
            "ICS"
        } else {
            source
                .map(|source| source.account_label.as_str())
                .unwrap_or("")
        };
        if provider.is_empty() || provider == name {
            name.to_string()
        } else {
            format!("{name} · {provider}")
        }
    }

    /// Publishes the calendar popup's header, bands and agenda.
    pub(crate) fn sync_calendar_view(&self, ui: &AppWindow, grid: &MonthGrid) {
        let locale = self.prefs.locale();
        let strings = catalog(locale);
        let copy = agenda_copy(locale);
        let state = &self.cal_state;
        let selected = state.selected_date;
        let is_today = selected == state.today;

        let month = grid.displayed_month;
        // Japanese, Chinese and Korean put the year first, with its own year mark.
        let year_mark = match locale {
            LocaleCode::Ja | LocaleCode::ZhHans => Some("年"),
            LocaleCode::Ko => Some("년"),
            _ => None,
        };
        ui.set_month_name_text(month_name(locale, month.month).into());
        ui.set_year_text(format!("{}{}", month.year, year_mark.unwrap_or_default()).into());
        ui.set_year_first(year_mark.is_some());
        // Animate the Today slot only when this change happens in front of the user, and
        // only with Windows Animation effects on.
        let motion = (self.on_screen)() && (self.animation_effects)();
        ui.set_header_motion(motion);
        crate::month_fade::show_month(ui, month, motion);
        ui.set_today_hidden(grid.is_current_month && is_today);
        ui.set_prev_month_label(copy.prev_month.into());
        ui.set_next_month_label(copy.next_month.into());
        ui.set_hide_to_tray_label(copy.hide_to_tray.into());
        ui.set_shaded_bands(ModelRc::new(VecModel::from(shaded_bands(
            &grid.gray_columns,
        ))));

        let events = self.day_agenda();
        let date_line = format_date_line(locale, selected, self.prefs.show_weekday);
        ui.set_agenda_title(
            if is_today {
                format!("{} · {date_line}", strings.today_button)
            } else {
                date_line
            }
            .into(),
        );
        ui.set_agenda_count(
            if events.is_empty() {
                String::new()
            } else {
                format_event_count(locale, events.len())
            }
            .into(),
        );

        let timings = agenda_timings(&events, is_today, self.now_minutes());
        let mut chips = Vec::new();
        let mut timed = Vec::new();
        let mut expanded_chip = None;
        for (index, (event, timing)) in events.iter().zip(timings).enumerate() {
            let key = EventKey::for_event(selected, event);
            let item = AgendaEventItem {
                index: index as i32,
                key: key.encode().into(),
                time_label: time_range(locale, event).into(),
                title: self.event_title(event).into(),
                calendar_name: self
                    .provider
                    .sources
                    .iter()
                    .find(|source| source.id == event.source_id)
                    .map(|source| source.name.as_str())
                    .unwrap_or(strings.agenda_calendar_fallback)
                    .into(),
                swatch: mark_color(&event.color_hex),
                all_day: event.is_all_day,
                dimmed: timing == AgendaTiming::Past,
                is_now: timing == AgendaTiming::Now,
                badge: match timing {
                    AgendaTiming::Now => copy.now.to_string(),
                    AgendaTiming::Next { minutes } => format_starts_in(locale, minutes),
                    _ => String::new(),
                }
                .into(),
                detail_time: detail_time(locale, event).into(),
                location: event.location.clone().unwrap_or_default().into(),
                provider_line: self.provider_line(event).into(),
                can_open: openable_link(event).is_some(),
                expanded: self.agenda.expanded.as_ref() == Some(&key),
                copied: self.agenda.copied.as_ref() == Some(&key),
            };
            if event.is_all_day {
                if item.expanded {
                    expanded_chip = Some(item.clone());
                }
                chips.push(item);
            } else {
                timed.push(item);
            }
        }
        let chip_rows: Vec<AgendaChipRow> = wrap_chips(chips)
            .into_iter()
            .map(|row| AgendaChipRow {
                chips: ModelRc::new(VecModel::from(row)),
            })
            .collect();
        ui.set_agenda_chip_rows(ModelRc::new(VecModel::from(chip_rows)));
        let fade_key = {
            let mut fade = self.agenda.fade.borrow_mut();
            if self.settings_open || fade.as_ref().is_some_and(|key| key.date != selected) {
                *fade = None;
            }
            fade.as_ref().map(EventKey::encode).unwrap_or_default()
        };
        ui.set_agenda_fade_key(fade_key.into());
        ui.set_agenda_has_expanded_chip(expanded_chip.is_some());
        ui.set_agenda_expanded_chip(expanded_chip.unwrap_or_default());
        let shown = ui.get_agenda_events();
        update_rows_in_place(
            &shown,
            timed,
            |a, b| a.key == b.key,
            |rows| ui.set_agenda_events(rows),
        );
        ui.set_agenda_empty(events.is_empty());
        ui.set_agenda_empty_text(format_event_count(locale, 0).into());
        let next = events
            .is_empty()
            .then(|| {
                next_occurrence_after(
                    selected,
                    &self.provider.occurrences,
                    &self.prefs.selected_calendar_ids,
                )
            })
            .flatten();
        ui.set_agenda_next_text(
            next.map(|event| {
                format!(
                    "{} {} · {} →",
                    copy.next_prefix,
                    format_short_date(locale, event.start_date),
                    self.event_title(event)
                )
            })
            .unwrap_or_default()
            .into(),
        );
        ui.set_agenda_now_label(copy.now.into());
        ui.set_agenda_copy_label(copy.copy.into());
        ui.set_agenda_copied_label(copy.copied.into());
        ui.set_agenda_open_label(copy.open.into());
    }
}

/// Wires the agenda's expand, Copy, Open and Next actions. `reload` refreshes
/// provider data after the Next link moves to another month.
pub(crate) fn connect_agenda_callbacks(
    ui: &AppWindow,
    ctrl: &Rc<RefCell<AppController>>,
    reload: impl Fn(slint::Weak<AppWindow>) + 'static,
) {
    use slint::ComponentHandle;
    {
        // Reported while Slint builds the panel, possibly during a sync that holds the
        // controller: then it is handled right after.
        let ctrl = Rc::clone(ctrl);
        ui.on_agenda_details_faded(move |row_key| match ctrl.try_borrow() {
            Ok(c) => c.agenda_details_faded(&row_key),
            Err(_) => {
                let ctrl = Rc::clone(&ctrl);
                slint::Timer::single_shot(std::time::Duration::ZERO, move || {
                    ctrl.borrow().agenda_details_faded(&row_key);
                });
            }
        });
    }
    {
        let ctrl = Rc::clone(ctrl);
        let weak = ui.as_weak();
        ui.on_toggle_agenda_event(move |row_key| {
            let mut c = ctrl.borrow_mut();
            c.toggle_agenda_event(&row_key);
            if let Some(w) = weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(ctrl);
        let weak = ui.as_weak();
        ui.on_copy_agenda_event(move |row_key| {
            let mut c = ctrl.borrow_mut();
            let Some((text, generation)) = c.copy_agenda_event(&row_key) else {
                return;
            };
            if lightcal::platform::windows::clipboard::copy_text(&text).is_err() {
                c.clear_copied(generation);
                return;
            }
            if let Some(w) = weak.upgrade() {
                c.sync_to_ui(&w);
            }
            let ctrl = Rc::downgrade(&ctrl);
            let weak = weak.clone();
            slint::Timer::single_shot(std::time::Duration::from_millis(1500), move || {
                let (Some(ctrl), Some(w)) = (ctrl.upgrade(), weak.upgrade()) else {
                    return;
                };
                let mut c = ctrl.borrow_mut();
                if c.clear_copied(generation) {
                    c.sync_to_ui(&w);
                }
            });
        });
    }
    {
        let ctrl = Rc::clone(ctrl);
        ui.on_open_agenda_event(move |row_key| {
            // Opened only on this click; the address is never fetched by Lightcal.
            if let Some(link) = ctrl.borrow().agenda_event_link(&row_key) {
                #[cfg(windows)]
                crate::open_allowed_browser_link(&link);
                #[cfg(not(windows))]
                let _ = link;
            }
        });
    }
    {
        let ctrl = Rc::clone(ctrl);
        let weak = ui.as_weak();
        ui.on_go_next_event(move || {
            let mut c = ctrl.borrow_mut();
            let previous_month = c.cal_state.displayed_month;
            if !c.go_next_event() {
                return;
            }
            if let Some(w) = weak.upgrade() {
                c.sync_to_ui(&w);
            }
            let changed_month = previous_month != c.cal_state.displayed_month;
            drop(c);
            if changed_month {
                reload(weak.clone());
            }
        });
    }
}

/// Runs of adjacent shaded columns, so neighbouring days share one band.
pub(crate) fn shaded_bands(columns: &[bool; 7]) -> Vec<ShadedBand> {
    let mut bands: Vec<ShadedBand> = Vec::new();
    for (col, shaded) in columns.iter().enumerate() {
        if !shaded {
            continue;
        }
        match bands.last_mut() {
            Some(band) if band.start + band.span == col as i32 => band.span += 1,
            _ => bands.push(ShadedBand {
                start: col as i32,
                span: 1,
            }),
        }
    }
    bands
}

/// Wraps chips into lines by an estimate of their 12 px width (328 px agenda, 8 px padding).
fn wrap_chips(chips: Vec<AgendaEventItem>) -> Vec<Vec<AgendaEventItem>> {
    const LINE: f32 = 320.0;
    const GAP: f32 = 6.0;
    let mut rows: Vec<Vec<AgendaEventItem>> = Vec::new();
    let mut used = 0.0f32;
    for chip in chips {
        let width = (chip
            .title
            .chars()
            .map(|c| if c.is_ascii() { 6.6 } else { 12.0 })
            .sum::<f32>()
            + 28.0)
            .min(LINE);
        match rows.last_mut() {
            Some(row) if used + GAP + width <= LINE => {
                used += GAP + width;
                row.push(chip);
            }
            _ => {
                used = width;
                rows.push(vec![chip]);
            }
        }
    }
    rows
}

fn openable_link(event: &CalendarOccurrence) -> Option<&str> {
    if event.source_id.starts_with("ics:") {
        return None;
    }
    event
        .web_link
        .as_deref()
        .filter(|link| lightcal::updater::is_allowed_event_link(link))
}

fn clock(minutes: i64) -> String {
    let (_, hour, minute) = split_local_minutes(minutes);
    format!("{hour:02}:{minute:02}")
}

/// "10:00–11:00", "All day", or the provider start label for older cached data.
fn time_range(locale: LocaleCode, event: &CalendarOccurrence) -> String {
    if event.is_all_day {
        return catalog(locale).agenda_all_day.to_string();
    }
    match event.timed_span() {
        Some((start, end)) if end > start => format!("{}–{}", clock(start), clock(end)),
        Some((start, _)) => clock(start),
        None => event.time_label.clone(),
    }
}

/// "10:00–11:00 · 1 h" for timed events, "All day" otherwise.
fn detail_time(locale: LocaleCode, event: &CalendarOccurrence) -> String {
    match event.timed_span() {
        Some((start, end)) if end > start => format!(
            "{} · {}",
            time_range(locale, event),
            format_duration(locale, end - start)
        ),
        _ => time_range(locale, event),
    }
}

/// Updates the rows already shown when they are the same rows (`same_row` pairwise), so
/// Slint keeps each row and its transitions (an opening details panel, Copy → Copied)
/// instead of rebuilding them on every sync. Different rows, such as another day's
/// events, get a new model, so no transition plays between unrelated rows.
fn update_rows_in_place<T: Clone + PartialEq + 'static>(
    shown: &ModelRc<T>,
    rows: Vec<T>,
    same_row: impl Fn(&T, &T) -> bool,
    publish: impl FnOnce(ModelRc<T>),
) {
    if let Some(model) = shown.as_any().downcast_ref::<VecModel<T>>() {
        let same = model.row_count() == rows.len()
            && rows.iter().enumerate().all(|(index, row)| {
                model
                    .row_data(index)
                    .is_some_and(|shown| same_row(&shown, row))
            });
        if same {
            for (index, row) in rows.into_iter().enumerate() {
                if model.row_data(index).as_ref() != Some(&row) {
                    model.set_row_data(index, row);
                }
            }
            return;
        }
    }
    publish(ModelRc::new(VecModel::from(rows)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_stay_in_place_only_when_they_are_the_same_rows() {
        let shown = ModelRc::new(VecModel::from(vec![(1, "a"), (2, "b")]));
        let same_key = |a: &(i32, &str), b: &(i32, &str)| a.0 == b.0;
        // Same events, new data (Copy → Copied): updated in place.
        let mut published = false;
        update_rows_in_place(&shown, vec![(1, "a"), (2, "B")], same_key, |_| {
            published = true
        });
        assert!(!published);
        assert_eq!(shown.row_data(1), Some((2, "B")));
        // Another day's events, same count: a new model, so no transition plays.
        update_rows_in_place(&shown, vec![(3, "c"), (4, "d")], same_key, |_| {
            published = true
        });
        assert!(published);
        assert_eq!(shown.row_data(0), Some((1, "a")));
    }

    #[test]
    fn adjacent_shaded_columns_merge_into_one_band() {
        let bands = shaded_bands(&[true, false, false, false, false, true, true]);
        let spans: Vec<_> = bands.iter().map(|b| (b.start, b.span)).collect();
        assert_eq!(spans, [(0, 1), (5, 2)]);
        assert!(shaded_bands(&[false; 7]).is_empty());
    }

    #[test]
    fn ics_and_unknown_links_are_never_openable() {
        let mut event = CalendarOccurrence {
            source_id: "google:primary".into(),
            web_link: Some("https://www.google.com/calendar/event?eid=1".into()),
            ..Default::default()
        };
        assert!(openable_link(&event).is_some());
        event.source_id = "ics:feed".into();
        assert!(openable_link(&event).is_none());
        event.source_id = "graph:a".into();
        event.web_link = Some("https://example.org/owa/".into());
        assert!(openable_link(&event).is_none());
    }
}
