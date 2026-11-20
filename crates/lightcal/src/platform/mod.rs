//! Platform boundary definitions for Windows and macOS.

#[cfg(target_os = "macos")]
pub mod macos;
pub mod windows;

use calendar_core::{CalendarOccurrence, CalendarSource, LocalDate};

/// Status of the read-only OS calendar provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderStatus {
    Available,
    NoCalendars,
    AccessDenied,
    ProviderUnavailable(String),
    StaleData,
}

#[derive(Debug, Clone)]
pub struct ProviderSnapshot {
    pub status: ProviderStatus,
    pub sources: Vec<CalendarSource>,
    pub occurrences: Vec<CalendarOccurrence>,
}

impl ProviderSnapshot {
    pub fn synthetic(anchor: LocalDate) -> Self {
        let sources = vec![
            CalendarSource {
                id: "win-work".to_string(),
                name: "Work".to_string(),
                account_label: "Windows · Exchange".to_string(),
                color_hex: "#008573".to_string(),
            },
            CalendarSource {
                id: "win-personal".to_string(),
                name: "Personal".to_string(),
                account_label: "Windows · Local".to_string(),
                color_hex: "#F59E0B".to_string(),
            },
            CalendarSource {
                id: "win-holidays".to_string(),
                name: "Holidays".to_string(),
                account_label: "Windows · Subscribed".to_string(),
                color_hex: "#10B981".to_string(),
            },
        ];

        let ym = anchor.year_month();
        let d = |day: u8| LocalDate::new(ym.year, ym.month, day.min(ym.days_in_month())).unwrap();
        let mut occurrences = vec![
            CalendarOccurrence {
                source_id: "win-work".to_string(),
                event_id: "syn-1".to_string(),
                color_hex: "#008573".to_string(),
                start_date: d(2),
                end_date_exclusive: d(2).add_days(1),
                is_all_day: false,
                title: "Quarterly roadmap review".to_string(),
                time_label: "09:30".to_string(),
                ..Default::default()
            },
            CalendarOccurrence {
                source_id: "win-personal".to_string(),
                event_id: "syn-2".to_string(),
                color_hex: "#F59E0B".to_string(),
                start_date: d(7),
                end_date_exclusive: d(7).add_days(1),
                is_all_day: false,
                title: "Evening gym session".to_string(),
                time_label: "18:00".to_string(),
                ..Default::default()
            },
            CalendarOccurrence {
                source_id: "win-work".to_string(),
                event_id: "syn-3".to_string(),
                color_hex: "#008573".to_string(),
                start_date: d(14),
                end_date_exclusive: d(14).add_days(1),
                is_all_day: false,
                title: "Design system sync".to_string(),
                time_label: "11:00".to_string(),
                ..Default::default()
            },
            CalendarOccurrence {
                source_id: "win-personal".to_string(),
                event_id: "syn-4".to_string(),
                color_hex: "#F59E0B".to_string(),
                start_date: d(14),
                end_date_exclusive: d(14).add_days(1),
                is_all_day: true,
                title: "Family dinner".to_string(),
                time_label: "19:30".to_string(),
                ..Default::default()
            },
            CalendarOccurrence {
                source_id: "win-holidays".to_string(),
                event_id: "syn-5".to_string(),
                color_hex: "#10B981".to_string(),
                start_date: d(21),
                end_date_exclusive: d(21).add_days(1),
                is_all_day: true,
                title: "Public Holiday".to_string(),
                time_label: "All day".to_string(),
                ..Default::default()
            },
            CalendarOccurrence {
                source_id: "win-work".to_string(),
                event_id: "syn-6".to_string(),
                color_hex: "#008573".to_string(),
                start_date: anchor,
                end_date_exclusive: anchor.add_days(2),
                is_all_day: false,
                title: "Architecture sync".to_string(),
                time_label: "10:00".to_string(),
                ..Default::default()
            },
            CalendarOccurrence {
                source_id: "win-personal".to_string(),
                event_id: "syn-7".to_string(),
                color_hex: "#F59E0B".to_string(),
                start_date: anchor,
                end_date_exclusive: anchor.add_days(2),
                is_all_day: false,
                title: "Dentist checkup".to_string(),
                time_label: "14:30".to_string(),
                ..Default::default()
            },
            CalendarOccurrence {
                source_id: "win-holidays".to_string(),
                event_id: "syn-8".to_string(),
                color_hex: "#10B981".to_string(),
                start_date: anchor,
                end_date_exclusive: anchor.add_days(1),
                is_all_day: true,
                title: "Release milestone".to_string(),
                time_label: "All day".to_string(),
                ..Default::default()
            },
        ];
        // Sample timed events last an hour from their start label.
        for occ in occurrences.iter_mut().filter(|occ| !occ.is_all_day) {
            let mut parts = occ.time_label.split(':').map(|p| p.parse::<u32>().ok());
            if let (Some(Some(hour)), Some(Some(minute))) = (parts.next(), parts.next()) {
                let start = calendar_core::local_minutes(occ.start_date, hour * 60 + minute);
                occ.start_minutes = Some(start);
                occ.end_minutes = Some(start + 60);
            }
        }

        Self {
            status: ProviderStatus::Available,
            sources,
            occurrences,
        }
    }
}

/// Work area rectangle in logical coordinates (`x, y, width, height`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkArea {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl WorkArea {
    /// Clamps a window origin, aligning oversized windows to the work area's leading edge.
    pub fn clamp_window(
        self,
        desired_x: i32,
        desired_y: i32,
        win_w: i32,
        win_h: i32,
    ) -> (i32, i32) {
        let margin = 8;
        let min_x = self.x + margin;
        let min_y = self.y + margin;
        let max_x = (self.x + self.width - win_w - margin).max(min_x);
        let max_y = (self.y + self.height - win_h - margin).max(min_y);
        (desired_x.clamp(min_x, max_x), desired_y.clamp(min_y, max_y))
    }

    /// Anchors a window (`win_w`, `win_h`) in the work-area corner next to the System Tray
    /// for a taskbar on `edge`, with the given `margin`, clamped inside the work area. The
    /// tray sits at the right end of a horizontal taskbar and the bottom of a vertical one.
    pub fn anchor_by_tray(
        self,
        edge: TaskbarEdge,
        win_w: i32,
        win_h: i32,
        margin: i32,
    ) -> (i32, i32) {
        let m = margin.max(8);
        let left = self.x + m;
        let right = self.x + self.width - win_w - m;
        let top = self.y + m;
        let bottom = self.y + self.height - win_h - m;
        let (desired_x, desired_y) = match edge {
            TaskbarEdge::Bottom | TaskbarEdge::Right => (right, bottom),
            TaskbarEdge::Left => (left, bottom),
            TaskbarEdge::Top => (right, top),
        };
        self.clamp_window(desired_x, desired_y, win_w, win_h)
    }

    /// This area without a strip `by` wide along `edge`; a strip that would leave nothing is
    /// ignored.
    fn without_strip(self, edge: TaskbarEdge, by: i32) -> WorkArea {
        let along = match edge {
            TaskbarEdge::Left | TaskbarEdge::Right => self.width,
            TaskbarEdge::Top | TaskbarEdge::Bottom => self.height,
        };
        if by <= 0 || by >= along {
            return self;
        }
        match edge {
            TaskbarEdge::Left => WorkArea {
                x: self.x + by,
                width: self.width - by,
                ..self
            },
            TaskbarEdge::Top => WorkArea {
                y: self.y + by,
                height: self.height - by,
                ..self
            },
            TaskbarEdge::Right => WorkArea {
                width: self.width - by,
                ..self
            },
            TaskbarEdge::Bottom => WorkArea {
                height: self.height - by,
                ..self
            },
        }
    }
}

/// The monitor edge the Windows taskbar is docked to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskbarEdge {
    Left,
    Top,
    Right,
    #[default]
    Bottom,
}

impl TaskbarEdge {
    /// How much of the monitor the work area leaves out along this edge.
    fn strip(self, monitor: WorkArea, work: WorkArea) -> i32 {
        match self {
            TaskbarEdge::Left => work.x - monitor.x,
            TaskbarEdge::Top => work.y - monitor.y,
            TaskbarEdge::Right => (monitor.x + monitor.width) - (work.x + work.width),
            TaskbarEdge::Bottom => (monitor.y + monitor.height) - (work.y + work.height),
        }
    }

    /// The edge whose strip the work area leaves out of the monitor, or `None` when the
    /// work area covers the whole monitor (an auto-hidden taskbar, or none on it). Other
    /// app bars take strips too, so this is only a guess for when the shell cannot say.
    pub fn from_work_area(monitor: WorkArea, work: WorkArea) -> Option<Self> {
        // The taskbar is assumed to be the widest strip; `max_by_key` keeps the last of
        // equal strips, so ties go to the bottom.
        [
            TaskbarEdge::Top,
            TaskbarEdge::Left,
            TaskbarEdge::Right,
            TaskbarEdge::Bottom,
        ]
        .into_iter()
        .map(|edge| (edge.strip(monitor, work), edge))
        .filter(|(strip, _)| *strip > 0)
        .max_by_key(|(strip, _)| *strip)
        .map(|(_, edge)| edge)
    }
}

/// What the shell reports about the taskbar for one monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellTaskbar {
    /// The primary taskbar's edge (`ABM_GETTASKBARPOS`); every taskbar shares it.
    pub edge: TaskbarEdge,
    /// The rectangle of the auto-hidden taskbar on this monitor's `edge`, if there is one
    /// (`ABM_GETAUTOHIDEBAREX`), in the same coordinates as the work area.
    pub auto_hide_bar: Option<WorkArea>,
    /// Whether the taskbar auto-hides (`ABM_GETSTATE`). A secondary monitor's auto-hidden
    /// taskbar is not always reported as a bar of its own.
    pub auto_hide: bool,
    /// Whether this monitor has a taskbar at all: it holds the primary taskbar, or taskbars
    /// show on all displays (`MMTaskbarEnabled`).
    pub taskbar_here: bool,
}

impl ShellTaskbar {
    /// Whether this report, perhaps stale, still matches the monitor's work area: its edge
    /// keeps a strip, or no edge does (the taskbar auto-hides or is not on this monitor).
    /// A strip on another edge means the taskbar moved since the report.
    pub fn fits(self, monitor: WorkArea, work: WorkArea) -> bool {
        self.edge.strip(monitor, work) > 0 || TaskbarEdge::from_work_area(monitor, work).is_none()
    }
}

/// The work area to anchor in on one monitor and the edge of its taskbar. The shell's report
/// decides the edge, so other app bars cannot pass for the taskbar. An auto-hidden taskbar
/// leaves its room in the work area, so that room is cut from it, measured from the monitor
/// edge it slides out of. With auto-hide on, a monitor with a taskbar whose own bar is not
/// reported still takes the shared edge. A monitor without a taskbar keeps the bottom-right
/// corner. Without the shell's report the widest strip is taken for the taskbar.
pub fn tray_area(
    monitor: WorkArea,
    work: WorkArea,
    taskbar: Option<ShellTaskbar>,
) -> (WorkArea, TaskbarEdge) {
    let Some(bar) = taskbar else {
        return (
            work,
            TaskbarEdge::from_work_area(monitor, work).unwrap_or_default(),
        );
    };
    if !bar.taskbar_here {
        return (work, TaskbarEdge::Bottom);
    }
    let edge = bar.edge;
    let strip = edge.strip(monitor, work);
    if let Some(hidden) = bar.auto_hide_bar {
        let thickness = match edge {
            TaskbarEdge::Left | TaskbarEdge::Right => hidden.width,
            TaskbarEdge::Top | TaskbarEdge::Bottom => hidden.height,
        };
        // App bars on the same edge already keep part of it clear.
        return (work.without_strip(edge, thickness - strip), edge);
    }
    if strip > 0 || bar.auto_hide {
        (work, edge)
    } else {
        (work, TaskbarEdge::Bottom)
    }
}

/// Remembers slow answers per key until they go stale, and that no answer came for
/// `retry_after`, so a busy source is not waited on again at once. A stale answer is asked
/// for again, but still given while the source is busy, which beats a guess. Used for the
/// shell's taskbar reports.
#[cfg(any(windows, test))]
pub(crate) struct AnswerCache<K, V> {
    /// Each key's latest answer, and whether it is still fresh.
    answers: Vec<(K, V, bool)>,
    unanswered_until: Option<std::time::Instant>,
}

#[cfg(any(windows, test))]
impl<K: PartialEq, V: Copy> AnswerCache<K, V> {
    pub(crate) const fn new() -> Self {
        Self {
            answers: Vec::new(),
            unanswered_until: None,
        }
    }

    /// The fresh answer for `key`, or `ask`'s, with `true`. A `None` from `ask` is
    /// remembered for all keys until `now + retry_after`; meanwhile the stale answer for
    /// `key`, if any, is given with `false`, for the caller to check before trusting it.
    pub(crate) fn get_or_ask(
        &mut self,
        key: K,
        now: std::time::Instant,
        retry_after: std::time::Duration,
        ask: impl FnOnce() -> Option<V>,
    ) -> Option<(V, bool)> {
        let known = self.answers.iter().position(|(k, _, _)| *k == key);
        if let Some(i) = known {
            if self.answers[i].2 {
                return Some((self.answers[i].1, true));
            }
        }
        let stale = known.map(|i| (self.answers[i].1, false));
        if self.unanswered_until.is_some_and(|until| now < until) {
            return stale;
        }
        match ask() {
            Some(value) => {
                self.unanswered_until = None;
                match known {
                    Some(i) => self.answers[i] = (key, value, true),
                    None => self.answers.push((key, value, true)),
                }
                Some((value, true))
            }
            None => {
                self.unanswered_until = Some(now + retry_after);
                stale
            }
        }
    }

    /// Makes every answer stale and ends a busy wait, so the next lookup asks again.
    pub(crate) fn clear(&mut self) {
        for answer in &mut self.answers {
            answer.2 = false;
        }
        self.unanswered_until = None;
    }

    /// Whether any answer is still fresh.
    #[cfg(test)]
    pub(crate) fn has_fresh(&self) -> bool {
        self.answers.iter().any(|(_, _, fresh)| *fresh)
    }
}

/// Validates and normalizes a global shortcut string (e.g. `Ctrl + Alt + C`).
pub fn normalize_shortcut(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Shortcut cannot be empty".to_string());
    }
    let parts: Vec<&str> = trimmed
        .split('+')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() < 2 {
        return Err("Include at least one modifier key (Ctrl, Alt, Shift, or Win)".to_string());
    }
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut win = false;
    let mut key: Option<String> = None;

    for part in parts {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => ctrl = true,
            "alt" => alt = true,
            "shift" => shift = true,
            "win" | "super" | "meta" | "cmd" => win = true,
            k if k.len() == 1 && k.chars().all(|c| c.is_ascii_alphanumeric()) => {
                key = Some(k.to_ascii_uppercase());
            }
            "space" => key = Some("Space".to_string()),
            "f1" | "f2" | "f3" | "f4" | "f5" | "f6" | "f7" | "f8" | "f9" | "f10" | "f11"
            | "f12" => {
                key = Some(part.to_ascii_uppercase());
            }
            _ => return Err(format!("Unsupported key token: {part}")),
        }
    }

    if !(ctrl || alt || shift || win) {
        return Err("Shortcut requires a modifier key".to_string());
    }
    let main_key =
        key.ok_or_else(|| "Shortcut requires a primary key (e.g. C, L, Space)".to_string())?;

    let mut normalized = Vec::new();
    if ctrl {
        normalized.push("Ctrl".to_string());
    }
    if alt {
        normalized.push("Alt".to_string());
    }
    if shift {
        normalized.push("Shift".to_string());
    }
    if win {
        normalized.push("Win".to_string());
    }
    normalized.push(main_key);
    Ok(normalized.join(" + "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_offscreen_window_into_monitor_work_area() {
        let area = WorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
        };
        assert_eq!(area.clamp_window(2200, 1200, 352, 432), (1560, 600));
        assert_eq!(area.clamp_window(-200, -50, 520, 600), (8, 8));
    }

    #[test]
    fn anchors_oversized_settings_without_invalid_clamp_bounds() {
        let area = WorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
        };
        // Oversized logical window at high DPI must still clamp into the work area.
        // Legacy 780×620 at 200% DPI used to exceed a 1080p work height; keep that
        // stress case so anchoring stays valid if a larger panel returns.
        assert_eq!(
            area.anchor_by_tray(TaskbarEdge::Bottom, 1560, 1240, 24),
            (336, 8)
        );
        // Current 448×360 settings at 200% DPI (896×720) fits a 1080p work area.
        assert_eq!(
            area.anchor_by_tray(TaskbarEdge::Bottom, 896, 720, 24),
            (1000, 296)
        );
        let left_monitor = WorkArea {
            x: -1280,
            y: -900,
            width: 1280,
            height: 720,
        };
        assert_eq!(
            left_monitor.anchor_by_tray(TaskbarEdge::Bottom, 1560, 1240, 24),
            (-1272, -892)
        );
        assert_eq!(
            area.anchor_by_tray(TaskbarEdge::Bottom, 1920, 1040, 24),
            (8, 8)
        );
    }

    #[test]
    fn anchors_next_to_the_tray_for_every_taskbar_edge() {
        let monitor = WorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let work = |x, y, width, height| WorkArea {
            x,
            y,
            width,
            height,
        };
        let cases = [
            (work(0, 0, 1920, 1032), TaskbarEdge::Bottom, (1556, 588)),
            (work(0, 48, 1920, 1032), TaskbarEdge::Top, (1556, 60)),
            (work(62, 0, 1858, 1080), TaskbarEdge::Left, (74, 636)),
            (work(0, 0, 1858, 1080), TaskbarEdge::Right, (1494, 636)),
        ];
        for (area, edge, origin) in cases {
            assert_eq!(TaskbarEdge::from_work_area(monitor, area), Some(edge));
            assert_eq!(area.anchor_by_tray(edge, 352, 432, 12), origin, "{edge:?}");
        }
        // An auto-hidden taskbar leaves the whole monitor to the work area.
        assert_eq!(TaskbarEdge::from_work_area(monitor, monitor), None);
        // A second monitor left of the primary, taskbar on top.
        let second = work(-1280, 0, 1280, 1024);
        assert_eq!(
            TaskbarEdge::from_work_area(second, work(-1280, 40, 1280, 984)),
            Some(TaskbarEdge::Top)
        );
    }

    #[test]
    fn widest_strip_wins_and_ties_go_to_the_bottom() {
        let monitor = WorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let work = |x, y, width, height| WorkArea {
            x,
            y,
            width,
            height,
        };
        // A 300 px left app bar beside a 48 px bottom taskbar looks like the taskbar.
        assert_eq!(
            TaskbarEdge::from_work_area(monitor, work(300, 0, 1620, 1032)),
            Some(TaskbarEdge::Left)
        );
        assert_eq!(
            TaskbarEdge::from_work_area(monitor, work(0, 48, 1920, 984)),
            Some(TaskbarEdge::Bottom)
        );
    }

    #[test]
    fn shell_taskbar_decides_the_edge_and_auto_hide_leaves_room() {
        let monitor = WorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let work = |x, y, width, height| WorkArea {
            x,
            y,
            width,
            height,
        };
        let bar = |edge, auto_hide_bar: Option<WorkArea>| {
            Some(ShellTaskbar {
                edge,
                auto_hide_bar,
                auto_hide: auto_hide_bar.is_some(),
                taskbar_here: true,
            })
        };
        // A wide left app bar does not pass for the taskbar.
        let with_app_bar = work(300, 0, 1620, 1032);
        assert_eq!(
            tray_area(monitor, with_app_bar, bar(TaskbarEdge::Bottom, None)),
            (with_app_bar, TaskbarEdge::Bottom)
        );
        // An auto-hidden left taskbar: the work area loses the bar's width on the left.
        assert_eq!(
            tray_area(
                monitor,
                monitor,
                bar(TaskbarEdge::Left, Some(work(-60, 0, 62, 1080)))
            ),
            (work(62, 0, 1858, 1080), TaskbarEdge::Left)
        );
        // A 30 px bottom app bar under a 48 px auto-hidden taskbar: 18 px more is kept clear.
        assert_eq!(
            tray_area(
                monitor,
                work(0, 0, 1920, 1050),
                bar(TaskbarEdge::Bottom, Some(work(0, 1078, 1920, 48)))
            ),
            (work(0, 0, 1920, 1032), TaskbarEdge::Bottom)
        );
        // A second monitor at 200 % keeps its own, thicker auto-hidden bar clear.
        let second = work(1920, 0, 3840, 2160);
        assert_eq!(
            tray_area(
                second,
                second,
                bar(TaskbarEdge::Bottom, Some(work(1920, 2158, 3840, 96)))
            ),
            (work(1920, 0, 3840, 2064), TaskbarEdge::Bottom)
        );
        // A second monitor's own taskbar shares the primary's edge.
        assert_eq!(
            tray_area(
                second,
                work(1920, 96, 3840, 2064),
                bar(TaskbarEdge::Top, None)
            ),
            (work(1920, 96, 3840, 2064), TaskbarEdge::Top)
        );
        // Auto-hide on, but the second monitor's own bar is not reported: the shared edge.
        assert_eq!(
            tray_area(
                second,
                second,
                Some(ShellTaskbar {
                    edge: TaskbarEdge::Top,
                    auto_hide_bar: None,
                    auto_hide: true,
                    taskbar_here: true,
                })
            ),
            (second, TaskbarEdge::Top)
        );
        // A second monitor without a taskbar keeps the bottom-right corner, with auto-hide
        // on or off, even when an app bar takes a strip on the shared edge.
        assert_eq!(
            tray_area(second, second, bar(TaskbarEdge::Left, None)),
            (second, TaskbarEdge::Bottom)
        );
        for auto_hide in [false, true] {
            let app_bar_strip = work(1980, 0, 3780, 2160);
            assert_eq!(
                tray_area(
                    second,
                    app_bar_strip,
                    Some(ShellTaskbar {
                        edge: TaskbarEdge::Left,
                        auto_hide_bar: None,
                        auto_hide,
                        taskbar_here: false,
                    })
                ),
                (app_bar_strip, TaskbarEdge::Bottom)
            );
        }
        // Without the shell's answer the widest strip is the guess.
        assert_eq!(
            tray_area(monitor, work(0, 0, 1858, 1080), None),
            (work(0, 0, 1858, 1080), TaskbarEdge::Right)
        );
    }

    #[test]
    fn stale_report_is_trusted_only_while_it_fits_the_work_area() {
        let monitor = WorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let bottom = ShellTaskbar {
            edge: TaskbarEdge::Bottom,
            auto_hide_bar: None,
            auto_hide: false,
            taskbar_here: true,
        };
        let work = |x, y, width, height| WorkArea {
            x,
            y,
            width,
            height,
        };
        // Still a bottom strip, or no strip at all (auto-hide): the report fits.
        assert!(bottom.fits(monitor, work(0, 0, 1920, 1032)));
        assert!(bottom.fits(monitor, monitor));
        // The taskbar moved to the left since the report: it does not.
        assert!(!bottom.fits(monitor, work(62, 0, 1858, 1080)));
    }

    #[test]
    fn answer_cache_remembers_answers_and_waits_out_a_busy_source() {
        use std::cell::Cell;
        use std::time::{Duration, Instant};
        let asked = Cell::new(0);
        let answer = |value: Option<u8>| {
            asked.set(asked.get() + 1);
            value
        };
        let retry = Duration::from_secs(2);
        let start = Instant::now();
        let mut cache = AnswerCache::<u8, u8>::new();
        // A busy source is asked once, then not again until the retry time.
        assert_eq!(cache.get_or_ask(1, start, retry, || answer(None)), None);
        assert_eq!(cache.get_or_ask(1, start, retry, || answer(Some(7))), None);
        assert_eq!(asked.get(), 1);
        let later = start + retry;
        assert_eq!(
            cache.get_or_ask(1, later, retry, || answer(Some(7))),
            Some((7, true))
        );
        // A fresh answer is kept per key.
        assert_eq!(
            cache.get_or_ask(1, later, retry, || answer(Some(9))),
            Some((7, true))
        );
        assert_eq!(
            cache.get_or_ask(2, later, retry, || answer(Some(9))),
            Some((9, true))
        );
        assert_eq!(asked.get(), 3);
        assert!(cache.has_fresh());
        // Cleared answers are asked for again.
        cache.clear();
        assert!(!cache.has_fresh());
        assert_eq!(
            cache.get_or_ask(1, later, retry, || answer(Some(8))),
            Some((8, true))
        );
        assert_eq!(asked.get(), 4);
        // A busy source after a clear: the stale answer stands in, then a new key gets
        // nothing until the retry time, and clearing ends that wait.
        cache.clear();
        assert_eq!(
            cache.get_or_ask(2, later, retry, || answer(None)),
            Some((9, false))
        );
        assert_eq!(cache.get_or_ask(3, later, retry, || answer(Some(5))), None);
        assert_eq!(asked.get(), 5);
        cache.clear();
        assert_eq!(
            cache.get_or_ask(3, later, retry, || answer(Some(5))),
            Some((5, true))
        );
        assert_eq!(asked.get(), 6);
    }

    #[test]
    fn shortcut_validation_and_normalization() {
        assert_eq!(normalize_shortcut("ctrl+alt+c").unwrap(), "Ctrl + Alt + C");
        assert!(normalize_shortcut("C").is_err());
        assert!(normalize_shortcut("Ctrl + ").is_err());
    }
}

/// What the user can see of the application window, for decorations that animate
/// (the koi pond, ADR 0013).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowView {
    /// Shown and not minimized.
    pub on_screen: bool,
    /// On screen with the display on, not cloaked on another virtual desktop, not under the
    /// lock screen and not completely covered by opaque windows.
    pub seen: bool,
    /// Windows Settings › Accessibility › Visual effects › Animation effects.
    pub animation_effects: bool,
}

/// Windows Settings › Accessibility › Visual effects › Animation effects. Off Windows it
/// reads as on.
pub fn animation_effects_enabled() -> bool {
    #[cfg(windows)]
    {
        windows::animation_effects_enabled()
    }
    #[cfg(target_os = "macos")]
    {
        macos::motion_enabled()
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        true
    }
}

/// Reads the native application window. Off Windows, and before the window exists, the
/// window counts as off screen.
pub fn main_window_view() -> WindowView {
    #[cfg(windows)]
    {
        let on_screen = windows::main_window_is_open() == Some(true);
        WindowView {
            on_screen,
            seen: on_screen && windows::main_window_is_seen(),
            animation_effects: windows::animation_effects_enabled(),
        }
    }
    #[cfg(target_os = "macos")]
    {
        let (on_screen, seen) = macos::window_view();
        WindowView {
            on_screen,
            seen,
            animation_effects: macos::motion_enabled(),
        }
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        WindowView {
            on_screen: false,
            seen: false,
            animation_effects: true,
        }
    }
}

/// Calls `on_change` when another app's window changes in a way that can uncover this one
/// (foreground, move or resize, minimize or restore). Does nothing off Windows.
pub fn watch_window_changes(on_change: impl Fn() + 'static) {
    #[cfg(windows)]
    windows::watch_window_changes(on_change);
    #[cfg(not(windows))]
    drop(on_change);
}

/// Stops `watch_window_changes`.
pub fn stop_watching_window_changes() {
    #[cfg(windows)]
    windows::stop_watching_window_changes();
}
