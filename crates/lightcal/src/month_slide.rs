//! Month change slide (ADR 0016): the day rows enter from 8 px in the direction of travel
//! while fading in, so paging through months is easy to follow. It plays only for changes
//! made while the popup is on screen with Windows Animation effects on; otherwise the rows
//! show settled. A timer runs only while a slide is in progress.

use crate::AppWindow;
use calendar_core::YearMonth;
use slint::ComponentHandle;
use std::cell::Cell;
use std::cmp::Ordering;
use std::time::{Duration, Instant};

const SLIDE_SECS: f64 = 0.18;
const SLIDE_PX: f64 = 8.0;
/// One tick of Windows' default 15.625 ms timer; progress is measured, not counted.
const FRAME: Duration = Duration::from_micros(15_625);

thread_local! {
    static SHOWN_MONTH: Cell<Option<YearMonth>> = const { Cell::new(None) };
    static SLIDE_TIMER: slint::Timer = slint::Timer::default();
    /// Start time and direction (−1 back, +1 forward) of the slide in progress.
    static SLIDE: Cell<Option<(Instant, f64)>> = const { Cell::new(None) };
}

/// Row offset in px and opacity `elapsed` seconds into a slide toward `direction`, and
/// whether it has finished. Ease-out cubic: most of the travel happens early.
fn slide_at(direction: f64, elapsed: f64) -> (f64, f64, bool) {
    let t = (elapsed / SLIDE_SECS).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - t).powi(3);
    (direction * SLIDE_PX * (1.0 - eased), eased, t >= 1.0)
}

/// Called with every calendar sync: slides when the displayed month changed and `animate`
/// allows it, otherwise leaves the rows settled.
pub(crate) fn show_month(ui: &AppWindow, month: YearMonth, animate: bool) {
    let previous = SHOWN_MONTH.with(|shown| shown.replace(Some(month)));
    let direction = match previous.map(|previous| month.cmp(&previous)) {
        Some(Ordering::Greater) => 1.0,
        Some(Ordering::Less) => -1.0,
        _ => return,
    };
    if !animate {
        settle_month_slide(ui);
        return;
    }
    SLIDE.with(|slide| slide.set(Some((Instant::now(), direction))));
    apply(ui, slide_at(direction, 0.0));
    SLIDE_TIMER.with(|timer| {
        if timer.running() {
            return;
        }
        let ui_weak = ui.as_weak();
        timer.start(slint::TimerMode::Repeated, FRAME, move || {
            if let Some(ui) = ui_weak.upgrade() {
                slide_frame(&ui);
            }
        });
    });
}

/// Hidden popup or open Settings: rows settled and no timer.
pub(crate) fn settle_month_slide(ui: &AppWindow) {
    SLIDE.with(|slide| slide.set(None));
    SLIDE_TIMER.with(|timer| timer.stop());
    apply(ui, (0.0, 1.0, true));
}

fn slide_frame(ui: &AppWindow) {
    let Some((start, direction)) = SLIDE.with(Cell::get) else {
        settle_month_slide(ui);
        return;
    };
    let state = slide_at(direction, start.elapsed().as_secs_f64());
    apply(ui, state);
    if state.2 {
        settle_month_slide(ui);
    }
}

fn apply(ui: &AppWindow, (offset, opacity, _): (f64, f64, bool)) {
    ui.set_month_shift(offset as f32);
    ui.set_month_fade(opacity as f32);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slide_enters_from_the_direction_of_travel_and_settles() {
        assert_eq!(slide_at(1.0, 0.0), (8.0, 0.0, false));
        assert_eq!(slide_at(-1.0, 0.0), (-8.0, 0.0, false));
        let (offset, opacity, done) = slide_at(1.0, SLIDE_SECS / 2.0);
        assert!(
            offset > 0.0 && offset < 2.0,
            "ease-out: most travel is early, {offset}"
        );
        assert!(opacity > 0.8 && !done);
        assert_eq!(slide_at(1.0, SLIDE_SECS), (0.0, 1.0, true));
        assert_eq!(slide_at(-1.0, 5.0), (0.0, 1.0, true));
    }

    #[test]
    fn the_slide_never_runs_backwards_or_past_its_place() {
        let mut last = slide_at(1.0, 0.0);
        for step in 1..=20 {
            let next = slide_at(1.0, SLIDE_SECS * f64::from(step) / 20.0);
            assert!(next.0 <= last.0 && next.0 >= 0.0);
            assert!(next.1 >= last.1 && next.1 <= 1.0);
            last = next;
        }
    }
}
