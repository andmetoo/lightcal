//! Platform boundary definitions for Windows (primary) and macOS (future port path).

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
                color_hex: "#5E6AD2".to_string(),
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
        let occurrences = vec![
            CalendarOccurrence {
                source_id: "win-work".to_string(),
                event_id: "syn-1".to_string(),
                color_hex: "#5E6AD2".to_string(),
                start_date: d(2),
                end_date_exclusive: d(2).add_days(1),
                is_all_day: false,
                title: "Quarterly roadmap review".to_string(),
                time_label: "09:30".to_string(),
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
            },
            CalendarOccurrence {
                source_id: "win-work".to_string(),
                event_id: "syn-3".to_string(),
                color_hex: "#5E6AD2".to_string(),
                start_date: d(14),
                end_date_exclusive: d(14).add_days(1),
                is_all_day: false,
                title: "Design system sync".to_string(),
                time_label: "11:00".to_string(),
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
            },
            CalendarOccurrence {
                source_id: "win-work".to_string(),
                event_id: "syn-6".to_string(),
                color_hex: "#5E6AD2".to_string(),
                start_date: anchor,
                end_date_exclusive: anchor.add_days(2),
                is_all_day: false,
                title: "Architecture sync".to_string(),
                time_label: "10:00".to_string(),
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
            },
        ];

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
    /// Clamps a window rectangle (`win_w`, `win_h`) so it remains fully visible inside the work area.
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

    /// Anchors a window (`win_w`, `win_h`) in the bottom-right corner right above the
    /// Windows Taskbar / System Tray with the given `margin`, clamped inside the work area.
    pub fn anchor_above_tray(self, win_w: i32, win_h: i32, margin: i32) -> (i32, i32) {
        let m = margin.max(8);
        let desired_x = self.x + self.width - win_w - m;
        let desired_y = self.y + self.height - win_h - m;
        self.clamp_window(desired_x, desired_y, win_w, win_h)
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
    fn shortcut_validation_and_normalization() {
        assert_eq!(normalize_shortcut("ctrl+alt+c").unwrap(), "Ctrl + Alt + C");
        assert!(normalize_shortcut("C").is_err());
        assert!(normalize_shortcut("Ctrl + ").is_err());
    }
}
