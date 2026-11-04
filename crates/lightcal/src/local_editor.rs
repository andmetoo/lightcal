//! The local event editor, its panel, the row actions and the agenda strip
//! ([ADR 0017](../../docs/decisions/0017-local-events.md)). Event rules and storage live in
//! `lightcal::local_events`; this module turns UI input into edits and edits into text.
//!
//! The field text lives in Slint while the editor is open; Rust sets it when the editor
//! opens and reads it back on Save.

use crate::calendar_view::EventKey;
use crate::{AppController, AppWindow, LocalEditorText};
use calendar_core::LocalDate;
use lightcal::i18n::{format_date_line, local_count_text, local_event_text, LocalEventText};
use lightcal::local_events::{
    Access, EditError, EventDraft, InvalidEvent, LocalEvent, Notice, Timing, END_OF_DAY,
    MAX_EVENTS, MAX_LOCATION_CHARS, MAX_NOTE_CHARS, MAX_TITLE_CHARS, SOURCE_ID,
};
use slint::ComponentHandle;
use std::{cell::RefCell, rc::Rc};

/// What the editor fields hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EditorFields {
    pub(crate) title: String,
    pub(crate) all_day: bool,
    pub(crate) start: String,
    pub(crate) end: String,
    pub(crate) location: String,
    pub(crate) note: String,
}

impl EditorFields {
    /// A new event: timed, 09:00–10:00, as on the design board.
    pub(crate) fn blank() -> Self {
        Self {
            title: String::new(),
            all_day: false,
            start: "09:00".into(),
            end: "10:00".into(),
            location: String::new(),
            note: String::new(),
        }
    }

    fn from_event(event: &LocalEvent) -> Self {
        let (all_day, start, end) = match event.timing {
            Timing::AllDay => (true, "09:00".to_string(), "10:00".to_string()),
            Timing::Timed { start, end } => (false, clock(start), clock(end)),
        };
        Self {
            title: event.title.clone(),
            all_day,
            start,
            end,
            location: event.location.clone().unwrap_or_default(),
            note: event.note.clone().unwrap_or_default(),
        }
    }

    pub(crate) fn read(ui: &AppWindow) -> Self {
        Self {
            title: ui.get_editor_title().into(),
            all_day: ui.get_editor_all_day(),
            start: ui.get_editor_start().into(),
            end: ui.get_editor_end().into(),
            location: ui.get_editor_location().into(),
            note: ui.get_editor_note().into(),
        }
    }

    pub(crate) fn write(&self, ui: &AppWindow) {
        ui.set_editor_title(self.title.as_str().into());
        ui.set_editor_all_day(self.all_day);
        ui.set_editor_start(self.start.as_str().into());
        ui.set_editor_end(self.end.as_str().into());
        ui.set_editor_location(self.location.as_str().into());
        ui.set_editor_note(self.note.as_str().into());
    }
}

/// `HH:MM`; minute 1440 is `24:00`.
fn clock(minutes: u16) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// A 24-hour `H:MM` or `HH:MM` time as minutes after midnight, `24:00` included.
pub(crate) fn parse_time(text: &str) -> Option<u16> {
    let (hours, minutes) = text.trim().split_once(':')?;
    if hours.is_empty()
        || hours.len() > 2
        || minutes.len() != 2
        || !hours
            .bytes()
            .chain(minutes.bytes())
            .all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let (hours, minutes): (u16, u16) = (hours.parse().ok()?, minutes.parse().ok()?);
    let total = hours * 60 + minutes;
    (minutes < 60 && total <= END_OF_DAY).then_some(total)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimeError {
    Format,
    Order,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LimitError {
    TooMany,
    TooLarge,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EditorErrors {
    pub(crate) title: bool,
    pub(crate) time: Option<TimeError>,
    /// Only the start is malformed, so only it turns red with the end.
    pub(crate) start_invalid: bool,
    pub(crate) limit: Option<LimitError>,
}

impl EditorErrors {
    fn any_field(&self) -> bool {
        self.title || self.time.is_some()
    }
}

/// The timing the fields describe, or what is wrong with them.
pub(crate) fn validate(fields: &EditorFields) -> Result<Timing, EditorErrors> {
    let mut errors = EditorErrors {
        title: fields.title.trim().is_empty(),
        ..EditorErrors::default()
    };
    let timing = if fields.all_day {
        Some(Timing::AllDay)
    } else {
        match (parse_time(&fields.start), parse_time(&fields.end)) {
            (Some(start), Some(end)) if start < END_OF_DAY && end > 0 => {
                if end > start {
                    Some(Timing::Timed { start, end })
                } else {
                    errors.time = Some(TimeError::Order);
                    None
                }
            }
            (start, _) => {
                errors.time = Some(TimeError::Format);
                errors.start_invalid = !start.is_some_and(|start| start < END_OF_DAY);
                None
            }
        }
    };
    match timing {
        Some(timing) if !errors.any_field() => Ok(timing),
        _ => Err(errors),
    }
}

/// Text beyond the stored limits is cut, so a pasted note never blocks Save.
fn clipped(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

/// The open editor: a new event, or the local event it edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EditorState {
    pub(crate) editing: Option<String>,
    pub(crate) errors: EditorErrors,
}

/// Shown in place of the editor when events cannot be added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorPanel {
    Unavailable,
    OverLimits,
}

impl AppController {
    /// The panel to show instead of the editor, if events cannot be added now.
    fn blocked_panel(&self) -> Option<EditorPanel> {
        if self.local.is_loading() {
            return Some(EditorPanel::Unavailable);
        }
        match self.local.access() {
            Access::Editable => None,
            Access::DeleteOnly => Some(EditorPanel::OverLimits),
            Access::Unavailable(_) => Some(EditorPanel::Unavailable),
        }
    }

    /// Opens the editor for a new event on the selected day. Returns the fields to show,
    /// or `None` when the panel opened instead.
    pub(crate) fn open_new_editor(&mut self) -> Option<EditorFields> {
        // An open editor stays as it is: a right-click on a day only moves its event,
        // like a left-click, so nothing typed is lost.
        if self.editor.is_some() {
            return None;
        }
        self.agenda.confirm_delete = None;
        if let Some(panel) = self.blocked_panel() {
            self.editor = None;
            self.editor_panel = Some(panel);
            return None;
        }
        self.editor_panel = None;
        self.editor = Some(EditorState {
            editing: None,
            errors: EditorErrors::default(),
        });
        Some(EditorFields::blank())
    }

    /// Opens the editor for the local event in the agenda row `row_key`.
    pub(crate) fn open_edit_editor(&mut self, row_key: &str) -> Option<EditorFields> {
        let (key, _) = self.agenda_key(row_key)?;
        let id = key.local_event_id()?;
        if let Some(panel) = self.blocked_panel() {
            self.editor_panel = Some(panel);
            return None;
        }
        let event = self.local.events().iter().find(|event| event.id == id)?;
        let fields = EditorFields::from_event(event);
        self.agenda.confirm_delete = None;
        self.editor_panel = None;
        self.editor = Some(EditorState {
            editing: Some(id.to_string()),
            errors: EditorErrors::default(),
        });
        Some(fields)
    }

    pub(crate) fn cancel_editor(&mut self) {
        self.editor = None;
    }

    pub(crate) fn close_editor_panel(&mut self) {
        self.editor_panel = None;
    }

    /// Saves the editor's event on the selected day. True when it was saved and the editor
    /// closed; otherwise the editor shows why.
    pub(crate) fn save_editor(&mut self, fields: &EditorFields) -> bool {
        let Some(editor) = self.editor.as_mut() else {
            return false;
        };
        let timing = match validate(fields) {
            Ok(timing) => timing,
            Err(errors) => {
                editor.errors = errors;
                return false;
            }
        };
        let date = self.cal_state.selected_date;
        let draft = EventDraft {
            title: clipped(&fields.title, MAX_TITLE_CHARS),
            date,
            timing,
            location: clipped(&fields.location, MAX_LOCATION_CHARS),
            note: clipped(&fields.note, MAX_NOTE_CHARS),
        };
        let editing = editor.editing.clone();
        let result = match &editing {
            None => self.local.add(&draft),
            Some(id) => self.local.update(id, &draft).map(|()| id.clone()),
        };
        let id = match result {
            Ok(id) => id,
            Err(error) => {
                self.editor_failed(error);
                return false;
            }
        };
        self.submit_local_save();
        // A new event never disappears from view: saving selects On this PC again, as
        // the user's own choice, so demo mode saves it too.
        let selected = self
            .prefs
            .selected_calendar_ids
            .insert(SOURCE_ID.to_string());
        let shown_by_demo = self.demo_shown.remove(SOURCE_ID);
        if selected || shown_by_demo {
            self.persist();
        }
        self.editor = None;
        self.refresh_local_view();
        let key = EventKey::for_local(date, &id);
        self.agenda.copied = None;
        self.agenda.expanded = Some(key.clone());
        *self.agenda.fade.borrow_mut() = Some(key);
        true
    }

    fn editor_failed(&mut self, error: EditError) {
        let limit = match error {
            EditError::TooManyEvents => Some(LimitError::TooMany),
            EditError::FileTooLarge => Some(LimitError::TooLarge),
            EditError::Invalid(InvalidEvent::Title) => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.errors.title = true;
                }
                return;
            }
            EditError::Invalid(InvalidEvent::Time | InvalidEvent::Date) => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.errors.time = Some(TimeError::Format);
                }
                return;
            }
            // Clipped before saving, and the ID is generated: only a broken random source
            // gets here. The size message is the closest honest answer.
            EditError::Invalid(_) => Some(LimitError::TooLarge),
            EditError::OverLimits | EditError::ReadOnly => {
                self.editor = None;
                self.editor_panel = self.blocked_panel();
                return;
            }
            // The event was deleted meanwhile: nothing is left to edit.
            EditError::NotFound => {
                self.editor = None;
                return;
            }
        };
        if let Some(editor) = self.editor.as_mut() {
            editor.errors = EditorErrors {
                limit,
                ..EditorErrors::default()
            };
        }
    }

    /// Hands the current events to the writer.
    fn submit_local_save(&mut self) {
        let (events, copy_first) = self.local.take_save();
        if let Some(writer) = &self.local_writer {
            let _ = writer.submit(events, copy_first);
        }
    }

    pub(crate) fn ask_delete_event(&mut self, row_key: &str) {
        if let Some((key, _)) = self.agenda_key(row_key) {
            if key.local_event_id().is_some() {
                self.agenda.confirm_delete = Some(key);
            }
        }
    }

    pub(crate) fn keep_event(&mut self) {
        self.agenda.confirm_delete = None;
    }

    /// Deletes the local event in `row_key` after its confirmation.
    pub(crate) fn confirm_delete_event(&mut self, row_key: &str) -> bool {
        let Some((key, _)) = self.agenda_key(row_key) else {
            return false;
        };
        if self.agenda.confirm_delete.as_ref() != Some(&key) {
            return false;
        }
        self.agenda.confirm_delete = None;
        let Some(id) = key.local_event_id() else {
            return false;
        };
        if self.local.delete(id).is_err() {
            return false;
        }
        self.submit_local_save();
        if self.agenda.expanded.as_ref() == Some(&key) {
            self.agenda.expanded = None;
        }
        self.refresh_local_view();
        true
    }

    /// The agenda strip: text, whether it is an error, and its action label.
    fn local_strip(&self, text: &LocalEventText) -> Option<(String, bool, &'static str)> {
        let locale = self.prefs.locale();
        if self.local.is_loading() {
            return None;
        }
        match self.local.access() {
            Access::Unavailable(_) => {
                return Some((text.unavailable.to_string(), true, text.accounts));
            }
            Access::DeleteOnly => {
                let count = self.local.events().len();
                let message = if count > MAX_EVENTS {
                    local_count_text(locale, text.over_count, count, MAX_EVENTS)
                } else {
                    text.over_size.to_string()
                };
                return Some((message, false, ""));
            }
            Access::Editable => {}
        }
        self.local
            .notices()
            .iter()
            .any(|notice| matches!(notice, Notice::RestoredBackup { .. }))
            .then(|| (text.restored.to_string(), false, text.details))
    }

    /// Publishes the editor, panel and strip.
    pub(crate) fn sync_local_editor(&self, ui: &AppWindow) {
        let locale = self.prefs.locale();
        let text = local_event_text(locale);
        ui.set_local_text(LocalEditorText {
            new_event: text.new_event.into(),
            edit_event: text.edit_event.into(),
            new_event_key: text.new_event_key.into(),
            title: text.title.into(),
            all_day: text.all_day.into(),
            start: text.start.into(),
            end: text.end.into(),
            location: text.location.into(),
            note: text.note.into(),
            date: text.date.into(),
            date_hint: text.date_hint.into(),
            save: text.save.into(),
            save_hint: text.save_hint.into(),
            cancel: text.cancel.into(),
            add_event: text.add_event.into(),
            edit: text.edit.into(),
            delete: text.delete.into(),
            delete_question: text.delete_question.into(),
            open_accounts: text.open_accounts.into(),
            back: text.back.into(),
        });

        let editor = self.editor.as_ref();
        ui.set_editor_open(editor.is_some());
        ui.set_editor_editing(editor.is_some_and(|editor| editor.editing.is_some()));
        let errors = editor
            .map(|editor| editor.errors.clone())
            .unwrap_or_default();
        ui.set_editor_title_error(if errors.title { text.err_title } else { "" }.into());
        ui.set_editor_time_error(
            match errors.time {
                Some(TimeError::Format) => text.err_time_format,
                Some(TimeError::Order) => text.err_time_order,
                None => "",
            }
            .into(),
        );
        ui.set_editor_start_invalid(errors.start_invalid);
        ui.set_editor_limit_error(
            match errors.limit {
                Some(LimitError::TooMany) => {
                    local_count_text(locale, text.err_too_many, MAX_EVENTS, MAX_EVENTS)
                }
                Some(LimitError::TooLarge) => text.err_too_large.to_string(),
                None => String::new(),
            }
            .into(),
        );
        ui.set_editor_date(editor_date(locale, self.cal_state.selected_date).into());

        let panel = self.editor_panel.filter(|_| editor.is_none());
        ui.set_panel_open(panel.is_some());
        let count = self.local.events().len();
        let (title, message, warn) = match panel {
            Some(EditorPanel::OverLimits) => (
                text.panel_over_title,
                if count > MAX_EVENTS {
                    local_count_text(locale, text.panel_over_count, count, MAX_EVENTS)
                } else {
                    text.panel_over_size.to_string()
                },
                true,
            ),
            _ => (
                text.panel_unavailable_title,
                text.panel_unavailable_text.to_string(),
                false,
            ),
        };
        ui.set_panel_title(title.into());
        ui.set_panel_message(message.into());
        ui.set_panel_warn(warn);
        ui.set_panel_accounts(panel == Some(EditorPanel::Unavailable));

        let strip = self.local_strip(text);
        ui.set_strip_text(strip.as_ref().map(|s| s.0.as_str()).unwrap_or("").into());
        ui.set_strip_error(strip.as_ref().is_some_and(|s| s.1));
        ui.set_strip_action(strip.as_ref().map(|s| s.2).unwrap_or("").into());
        ui.set_local_card(self.local_card());
    }
}

/// "Thu, 12 Nov", the editor's footer date.
fn editor_date(locale: lightcal::preferences::LocaleCode, date: LocalDate) -> String {
    format_date_line(locale, date, true)
}

/// Wires the entry points, the editor, the panel, Delete and the strip.
pub(crate) fn connect_local_editor_callbacks(
    ui: &AppWindow,
    ctrl: &Rc<RefCell<AppController>>,
    reload: impl Fn(slint::Weak<AppWindow>) + 'static,
) {
    let reload = Rc::new(reload);
    {
        let ctrl = Rc::clone(ctrl);
        let weak = ui.as_weak();
        let reload = Rc::clone(&reload);
        // −1 is the selected day (N, + and Add event); otherwise a right-clicked cell.
        ui.on_new_event(move |cell| {
            let Some(w) = weak.upgrade() else {
                return;
            };
            let mut c = ctrl.borrow_mut();
            let previous_month = c.cal_state.displayed_month;
            if cell >= 0 {
                let grid = c
                    .cal_state
                    .build_grid(&c.provider.occurrences, &c.prefs.selected_calendar_ids);
                if let Some(cell) = grid.cells.get(cell as usize) {
                    c.cal_state.select_date(cell.date);
                }
            }
            if let Some(fields) = c.open_new_editor() {
                fields.write(&w);
            }
            c.sync_to_ui(&w);
            let changed_month = previous_month != c.cal_state.displayed_month;
            drop(c);
            if changed_month {
                reload(weak.clone());
            }
        });
    }
    {
        let ctrl = Rc::clone(ctrl);
        let weak = ui.as_weak();
        ui.on_edit_event(move |row_key| {
            let Some(w) = weak.upgrade() else {
                return;
            };
            let mut c = ctrl.borrow_mut();
            if let Some(fields) = c.open_edit_editor(&row_key) {
                fields.write(&w);
            }
            c.sync_to_ui(&w);
        });
    }
    {
        let ctrl = Rc::clone(ctrl);
        let weak = ui.as_weak();
        ui.on_save_editor(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            let mut c = ctrl.borrow_mut();
            if c.save_editor(&EditorFields::read(&w)) {
                w.invoke_focus_calendar_keys();
            }
            c.sync_to_ui(&w);
        });
    }
    {
        let ctrl = Rc::clone(ctrl);
        let weak = ui.as_weak();
        ui.on_cancel_editor(move || {
            let mut c = ctrl.borrow_mut();
            c.cancel_editor();
            if let Some(w) = weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(ctrl);
        let weak = ui.as_weak();
        ui.on_panel_back(move || {
            let mut c = ctrl.borrow_mut();
            c.close_editor_panel();
            if let Some(w) = weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(ctrl);
        let weak = ui.as_weak();
        let open_accounts = move || {
            ctrl.borrow_mut().close_editor_panel();
            if let Some(w) = weak.upgrade() {
                // Settings › Accounts, through the same path as the gear button.
                w.invoke_open_settings(1);
            }
        };
        let open_accounts = Rc::new(open_accounts);
        let from_panel = Rc::clone(&open_accounts);
        ui.on_panel_open_accounts(move || from_panel());
        ui.on_strip_action_clicked(move || open_accounts());
    }
    {
        let ctrl = Rc::clone(ctrl);
        let weak = ui.as_weak();
        ui.on_ask_delete_event(move |row_key| {
            let mut c = ctrl.borrow_mut();
            c.ask_delete_event(&row_key);
            if let Some(w) = weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(ctrl);
        let weak = ui.as_weak();
        ui.on_keep_event(move |_| {
            let mut c = ctrl.borrow_mut();
            c.keep_event();
            if let Some(w) = weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(ctrl);
        let weak = ui.as_weak();
        ui.on_confirm_delete_event(move |row_key| {
            let mut c = ctrl.borrow_mut();
            c.confirm_delete_event(&row_key);
            if let Some(w) = weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(title: &str, start: &str, end: &str) -> EditorFields {
        EditorFields {
            title: title.into(),
            start: start.into(),
            end: end.into(),
            ..EditorFields::blank()
        }
    }

    #[test]
    fn times_are_24_hour_with_24_00_allowed() {
        assert_eq!(parse_time("09:30"), Some(570));
        assert_eq!(parse_time(" 9:05 "), Some(545));
        assert_eq!(parse_time("24:00"), Some(1440));
        for bad in ["24:01", "9", "9:5", "12:60", "ab:cd", "", "-1:00", "123:00"] {
            assert_eq!(parse_time(bad), None, "{bad}");
        }
    }

    #[test]
    fn validation_reports_each_problem() {
        assert_eq!(
            validate(&fields("Dentist", "09:00", "10:00")),
            Ok(Timing::Timed {
                start: 540,
                end: 600
            })
        );
        assert_eq!(
            validate(&fields("Night shift", "16:00", "24:00")),
            Ok(Timing::Timed {
                start: 960,
                end: 1440
            })
        );
        let both = validate(&fields("  ", "18:00", "17:30")).unwrap_err();
        assert!(both.title);
        assert_eq!(both.time, Some(TimeError::Order));
        let format = validate(&fields("x", "24:00", "10:00")).unwrap_err();
        assert_eq!(format.time, Some(TimeError::Format));
        assert!(
            format.start_invalid,
            "a start of 24:00 is the start's fault"
        );
        let end_only = validate(&fields("x", "09:00", "9")).unwrap_err();
        assert!(!end_only.start_invalid);
        let all_day = EditorFields {
            all_day: true,
            ..fields("Birthday", "nonsense", "")
        };
        assert_eq!(validate(&all_day), Ok(Timing::AllDay));
    }

    #[test]
    fn an_edited_event_fills_the_fields() {
        let event = LocalEvent::validated(
            "local:a".into(),
            "Night shift",
            LocalDate::new(2026, 11, 20).unwrap(),
            Timing::Timed {
                start: 960,
                end: 1440,
            },
            Some("Riverside"),
            Some("Badge\nCall Ana"),
        )
        .unwrap();
        let fields = EditorFields::from_event(&event);
        assert_eq!(
            (fields.start.as_str(), fields.end.as_str()),
            ("16:00", "24:00")
        );
        assert_eq!(fields.note, "Badge\nCall Ana");
        assert_eq!(validate(&fields), Ok(event.timing));
    }
}

// ---------- Settings › Accounts: the On this PC card ----------

/// `path` for display, with the user's `%APPDATA%` folder written as such.
fn display_path(path: &std::path::Path) -> String {
    let text = path.display().to_string();
    match std::env::var("APPDATA") {
        Ok(appdata) if !appdata.is_empty() && text.starts_with(&appdata) => {
            format!("%APPDATA%{}", &text[appdata.len()..])
        }
        _ => text,
    }
}

impl AppController {
    /// The file the card names, for its text and Show in folder.
    pub(crate) fn local_card_file(&self) -> Option<std::path::PathBuf> {
        use lightcal::local_events::Unavailable;
        match self.local.access() {
            Access::Unavailable(
                Unavailable::NewerVersion { file }
                | Unavailable::Damaged { file }
                | Unavailable::Unreadable { file, .. },
            ) => Some(file.clone()),
            Access::Unavailable(_) => None,
            _ => self.local.notices().iter().find_map(|notice| match notice {
                Notice::RestoredBackup { damaged } => damaged.clone(),
                _ => None,
            }),
        }
    }

    /// The On this PC card: its line under the name, and at most one notice, the most
    /// pressing first: unavailable, a failed save, over the limits, then load notices.
    pub(crate) fn local_card(&self) -> crate::LocalCardData {
        use lightcal::i18n::{format_event_count, local_settings_text};
        use lightcal::local_events::{account_label, calendar_name, Unavailable};
        let locale = self.prefs.locale();
        let text = local_settings_text(locale);
        let label = account_label(locale);
        let count = self.local.events().len();
        let file = self
            .local_card_file()
            .map(|path| display_path(&path))
            .unwrap_or_default();
        let mut card = crate::LocalCardData {
            name: calendar_name(locale).into(),
            sub: format!("{label} · {}", format_event_count(locale, count)).into(),
            checked: self.prefs.selected_calendar_ids.contains(SOURCE_ID),
            ok_label: text.ok.into(),
            folder_label: text.show_in_folder.into(),
            retry_label: text.try_now.into(),
            start_over_label: text.start_over.into(),
            cancel_label: local_event_text(locale).cancel.into(),
            ..Default::default()
        };
        let red = |card: &mut crate::LocalCardData, head: &str, body: String, start_over: bool| {
            card.notice = 3;
            card.head = head.into();
            card.text = body.into();
            card.file = file.as_str().into();
            card.folder = !file.is_empty();
            card.start_over = start_over;
        };
        if self.local.is_loading() {
            card.sub = label.into();
            return card;
        }
        match self.local.access() {
            Access::Unavailable(reason) => {
                card.sub = format!("{label} · {}", text.unavailable_sub).into();
                match reason {
                    Unavailable::Locked => {
                        card.sub = format!("{label} · {}", text.read_only_sub).into();
                        card.notice = 1;
                        card.spinner = true;
                        card.text = text.locked.into();
                    }
                    Unavailable::NewerVersion { .. } => {
                        red(&mut card, text.newer_head, text.newer.to_string(), true);
                    }
                    Unavailable::Damaged { .. } => {
                        red(&mut card, text.damaged_head, text.damaged.to_string(), true);
                    }
                    Unavailable::Unreadable { error, .. } => {
                        let body = text
                            .unreadable
                            .replace("{error}", error.trim_end_matches('.'));
                        red(&mut card, text.unreadable_head, body, true);
                    }
                    Unavailable::CannotSave { error } => {
                        card.sub = format!("{label} · {}", text.read_only_sub).into();
                        let body = text
                            .cannot_save
                            .replace("{error}", error.trim_end_matches('.'));
                        red(&mut card, text.cannot_save_head, body, false);
                        card.file = "".into();
                        card.folder = false;
                    }
                }
                if self.start_over_confirm && card.start_over {
                    card.head = text.start_over_head.into();
                    card.text = text.start_over_text.into();
                    card.confirm = true;
                }
                if let Some(error) = &self.start_over_error {
                    card.text = format!(
                        "{}
{error}",
                        card.text
                    )
                    .into();
                }
                // While Start over runs, its buttons are gone, so it cannot run twice.
                if self.start_over_busy {
                    card.confirm = false;
                    card.start_over = false;
                }
                return card;
            }
            Access::DeleteOnly => {
                card.sub = format!(
                    "{label} · {} · {}",
                    format_event_count(locale, count),
                    text.delete_only_sub
                )
                .into();
            }
            Access::Editable => {}
        }
        if let Some(error) = &self.local_save_error {
            card.notice = 2;
            card.text = text
                .retrying
                .replace("{error}", error.trim_end_matches('.'))
                .into();
            card.retry = true;
            return card;
        }
        if *self.local.access() == Access::DeleteOnly {
            card.notice = 2;
            card.text = if count > MAX_EVENTS {
                text.over_count
                    .replace("{events}", &format_event_count(locale, count))
                    .replace("{max}", &local_count_text(locale, "{max}", 0, MAX_EVENTS))
            } else {
                text.over_size.to_string()
            }
            .into();
            return card;
        }
        let notices = self.local.notices();
        if let Some(Notice::RestoredBackup { damaged }) = notices
            .iter()
            .find(|notice| matches!(notice, Notice::RestoredBackup { .. }))
        {
            card.notice = 2;
            card.text = if damaged.is_some() {
                text.restored_kept
            } else {
                text.restored
            }
            .into();
            card.file = file.as_str().into();
            card.folder = !file.is_empty();
            card.ok = true;
        } else if let Some(Notice::SkippedEvents(skipped)) = notices
            .iter()
            .find(|notice| matches!(notice, Notice::SkippedEvents(_)))
        {
            card.notice = 2;
            card.text = text
                .skipped
                .replace("{events}", &format_event_count(locale, *skipped))
                .into();
            card.ok = true;
        } else if notices
            .iter()
            .any(|notice| matches!(notice, Notice::RecoveredUnfinishedSave))
        {
            card.notice = 1;
            card.check = true;
            card.text = text.recovered.into();
            card.ok = true;
        }
        card
    }
}

/// Wires the On this PC card's actions.
pub(crate) fn connect_local_card_callbacks(ui: &AppWindow, ctrl: &Rc<RefCell<AppController>>) {
    {
        let ctrl = Rc::clone(ctrl);
        let weak = ui.as_weak();
        ui.on_local_ok(move || {
            let mut c = ctrl.borrow_mut();
            c.local.dismiss_notices();
            if let Some(w) = weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(ctrl);
        ui.on_local_show_folder(move || {
            // Opens Explorer with the kept file selected, only on this click.
            if let Some(file) = ctrl.borrow().local_card_file() {
                lightcal::platform::windows::reveal_in_explorer(&file);
            }
        });
    }
    {
        let ctrl = Rc::clone(ctrl);
        ui.on_local_try_now(move || {
            if let Some(writer) = &ctrl.borrow().local_writer {
                writer.retry_now();
            }
        });
    }
    {
        let ctrl = Rc::clone(ctrl);
        let weak = ui.as_weak();
        ui.on_local_ask_start_over(move || {
            let mut c = ctrl.borrow_mut();
            c.start_over_confirm = true;
            if let Some(w) = weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(ctrl);
        let weak = ui.as_weak();
        ui.on_local_keep(move || {
            let mut c = ctrl.borrow_mut();
            c.start_over_confirm = false;
            if let Some(w) = weak.upgrade() {
                c.sync_to_ui(&w);
            }
        });
    }
    {
        let ctrl = Rc::clone(ctrl);
        let weak = ui.as_weak();
        ui.on_local_start_over(move || {
            let mut c = ctrl.borrow_mut();
            c.start_over_confirm = false;
            if c.start_over_busy {
                return;
            }
            // Only an unavailable store that holds the lock can start over; Locked waits.
            let allowed = matches!(
                c.local.access(),
                Access::Unavailable(
                    lightcal::local_events::Unavailable::NewerVersion { .. }
                        | lightcal::local_events::Unavailable::Damaged { .. }
                        | lightcal::local_events::Unavailable::Unreadable { .. }
                )
            );
            if !allowed {
                return;
            }
            let dir = c.store.dir().to_path_buf();
            c.start_over_busy = true;
            c.start_over_error = None;
            if let Some(w) = weak.upgrade() {
                c.sync_to_ui(&w);
            }
            let weak = weak.clone();
            // The file work runs off the UI thread; the fresh, empty store, or the error,
            // comes back here.
            let _ = std::thread::Builder::new()
                .name("lightcal-local-events-start-over".into())
                .spawn(move || {
                    let storage = lightcal::local_events::Storage::new(&dir);
                    let result = storage
                        .start_over()
                        .map(|_| lightcal::local_events::LocalEvents::from_loaded(storage.load()));
                    let _ = slint::invoke_from_event_loop(move || {
                        crate::with_active_controller(&weak, |c, w| {
                            c.start_over_busy = false;
                            match result {
                                Ok(local) => c.apply_local_events(local),
                                Err(error) => c.start_over_error = Some(error),
                            }
                            c.sync_to_ui(w);
                        });
                    });
                });
        });
    }
}

#[cfg(test)]
mod card_tests {
    use super::*;

    #[test]
    fn appdata_paths_are_written_with_the_variable() {
        let Ok(appdata) = std::env::var("APPDATA") else {
            return;
        };
        let path = std::path::Path::new(&appdata)
            .join("Lightcal")
            .join("local-events.json");
        assert_eq!(
            display_path(&path),
            format!(
                "%APPDATA%{}Lightcal{}local-events.json",
                std::path::MAIN_SEPARATOR,
                std::path::MAIN_SEPARATOR
            )
        );
    }
}
