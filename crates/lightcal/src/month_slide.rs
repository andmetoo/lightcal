//! Month change slide (ADR 0016): the day rows enter from 8 px in the direction of travel
//! while fading in, so paging through months is easy to follow. It plays only for changes
//! made while the popup is on screen with Windows Animation effects on; otherwise the rows
//! show settled. A month change during a slide carries on from the offset and opacity on
//! screen, so fast paging glides instead of blanking the grid. A timer runs only while a
//! slide is in progress.

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
    /// The slide in progress.
    static SLIDE: Cell<Option<Slide>> = const { Cell::new(None) };
}

/// A slide from an offset and opacity toward the settled rows.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Slide {
    start: Instant,
    from_offset: f64,
    from_opacity: f64,
}

impl Slide {
    /// A new slide toward `direction` (−1 back, +1 forward): from 8 px on that side,
    /// transparent.
    fn toward(direction: f64, start: Instant) -> Slide {
        Slide {
            start,
            from_offset: direction * SLIDE_PX,
            from_opacity: 0.0,
        }
    }

    /// Row offset in px and opacity `elapsed` seconds in, and whether it has finished.
    /// Ease-out cubic: most of the travel happens early.
    fn at(&self, elapsed: f64) -> (f64, f64, bool) {
        let t = (elapsed / SLIDE_SECS).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - t).powi(3);
        (
            self.from_offset * (1.0 - eased),
            self.from_opacity + (1.0 - self.from_opacity) * eased,
            t >= 1.0,
        )
    }

    fn now(&self) -> (f64, f64, bool) {
        self.at(self.start.elapsed().as_secs_f64())
    }
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
    let now = Instant::now();
    let slide = match SLIDE.with(Cell::get) {
        // Carry on from what is on screen: no jump back to 8 px, no blank grid.
        Some(running) => {
            let (from_offset, from_opacity, _) = running.now();
            Slide {
                start: now,
                from_offset,
                from_opacity,
            }
        }
        None => Slide::toward(direction, now),
    };
    SLIDE.with(|state| state.set(Some(slide)));
    apply(ui, slide.at(0.0));
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
    let Some(slide) = SLIDE.with(Cell::get) else {
        settle_month_slide(ui);
        return;
    };
    let state = slide.now();
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

    fn fresh(direction: f64) -> Slide {
        Slide::toward(direction, Instant::now())
    }

    #[test]
    fn a_slide_enters_from_the_direction_of_travel_and_settles() {
        assert_eq!(fresh(1.0).at(0.0), (8.0, 0.0, false));
        assert_eq!(fresh(-1.0).at(0.0), (-8.0, 0.0, false));
        let (offset, opacity, done) = fresh(1.0).at(SLIDE_SECS / 2.0);
        assert!(
            offset > 0.0 && offset < 2.0,
            "ease-out: most travel is early, {offset}"
        );
        assert!(opacity > 0.8 && !done);
        assert_eq!(fresh(1.0).at(SLIDE_SECS), (0.0, 1.0, true));
        assert_eq!(fresh(-1.0).at(5.0), (0.0, 1.0, true));
    }

    #[test]
    fn the_slide_never_runs_backwards_or_past_its_place() {
        let slide = fresh(1.0);
        let mut last = slide.at(0.0);
        for step in 1..=20 {
            let next = slide.at(SLIDE_SECS * f64::from(step) / 20.0);
            assert!(next.0 <= last.0 && next.0 >= 0.0);
            assert!(next.1 >= last.1 && next.1 <= 1.0);
            last = next;
        }
    }

    #[test]
    fn a_slide_carried_on_starts_where_the_rows_are() {
        // A month change 40 ms into a slide: the rows are partly in and partly visible.
        let (offset, opacity, _) = fresh(1.0).at(0.04);
        let carried = Slide {
            start: Instant::now(),
            from_offset: offset,
            from_opacity: opacity,
        };
        assert_eq!(carried.at(0.0), (offset, opacity, false));
        assert!(opacity > 0.4, "the grid is not blanked: {opacity}");
        assert_eq!(carried.at(SLIDE_SECS), (0.0, 1.0, true));
    }
}
