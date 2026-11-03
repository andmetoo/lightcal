//! Month change fade (ADR 0016): the day rows fade in when the displayed month changes, so
//! paging through months is easy to follow without anything moving. It plays only for
//! changes made while the popup is on screen with Windows Animation effects on; otherwise
//! the rows show settled. A month change during a fade carries on from the opacity on
//! screen, so fast paging never blanks the grid. A timer runs only while a fade is in
//! progress.

use crate::AppWindow;
use calendar_core::YearMonth;
use slint::ComponentHandle;
use std::cell::Cell;
use std::time::{Duration, Instant};

const FADE_SECS: f64 = 0.18;
/// One tick of Windows' default 15.625 ms timer; progress is measured, not counted.
const FRAME: Duration = Duration::from_micros(15_625);

thread_local! {
    static SHOWN_MONTH: Cell<Option<YearMonth>> = const { Cell::new(None) };
    static FADE_TIMER: slint::Timer = slint::Timer::default();
    /// The fade in progress.
    static FADE: Cell<Option<Fade>> = const { Cell::new(None) };
}

/// A fade from an opacity to the settled rows.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Fade {
    start: Instant,
    from: f64,
}

impl Fade {
    /// Row opacity `elapsed` seconds in, and whether it has finished. Ease-out cubic: most
    /// of the change happens early.
    fn at(&self, elapsed: f64) -> (f64, bool) {
        let t = (elapsed / FADE_SECS).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - t).powi(3);
        (self.from + (1.0 - self.from) * eased, t >= 1.0)
    }

    fn now(&self) -> (f64, bool) {
        self.at(self.start.elapsed().as_secs_f64())
    }
}

/// Called with every calendar sync: fades the rows in when the displayed month changed and
/// `animate` allows it, otherwise leaves them settled.
pub(crate) fn show_month(ui: &AppWindow, month: YearMonth, animate: bool) {
    let previous = SHOWN_MONTH.with(|shown| shown.replace(Some(month)));
    if previous.is_none_or(|previous| previous == month) {
        return;
    }
    if !animate {
        settle_month_fade(ui);
        return;
    }
    // Carry on from what is on screen, or start from transparent.
    let from = FADE.with(Cell::get).map_or(0.0, |running| running.now().0);
    let fade = Fade {
        start: Instant::now(),
        from,
    };
    FADE.with(|state| state.set(Some(fade)));
    apply(ui, fade.at(0.0).0);
    FADE_TIMER.with(|timer| {
        if timer.running() {
            return;
        }
        let ui_weak = ui.as_weak();
        timer.start(slint::TimerMode::Repeated, FRAME, move || {
            if let Some(ui) = ui_weak.upgrade() {
                fade_frame(&ui);
            }
        });
    });
}

/// Hidden popup or open Settings: rows settled and no timer.
pub(crate) fn settle_month_fade(ui: &AppWindow) {
    FADE.with(|fade| fade.set(None));
    FADE_TIMER.with(|timer| timer.stop());
    apply(ui, 1.0);
}

fn fade_frame(ui: &AppWindow) {
    let Some(fade) = FADE.with(Cell::get) else {
        settle_month_fade(ui);
        return;
    };
    let (opacity, done) = fade.now();
    apply(ui, opacity);
    if done {
        settle_month_fade(ui);
    }
}

fn apply(ui: &AppWindow, opacity: f64) {
    ui.set_month_fade(opacity as f32);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Fade {
        Fade {
            start: Instant::now(),
            from: 0.0,
        }
    }

    #[test]
    fn a_fade_starts_transparent_eases_out_and_settles() {
        assert_eq!(fresh().at(0.0), (0.0, false));
        let (opacity, done) = fresh().at(FADE_SECS / 2.0);
        assert!(
            opacity > 0.8 && !done,
            "ease-out: most change is early, {opacity}"
        );
        assert_eq!(fresh().at(FADE_SECS), (1.0, true));
        assert_eq!(fresh().at(5.0), (1.0, true));
    }

    #[test]
    fn the_fade_never_runs_backwards() {
        let fade = fresh();
        let mut last = fade.at(0.0).0;
        for step in 1..=20 {
            let next = fade.at(FADE_SECS * f64::from(step) / 20.0).0;
            assert!(next >= last && next <= 1.0);
            last = next;
        }
    }

    #[test]
    fn a_fade_carried_on_starts_at_the_opacity_on_screen() {
        // A month change 40 ms into a fade: the rows are already partly visible.
        let (opacity, _) = fresh().at(0.04);
        let carried = Fade {
            start: Instant::now(),
            from: opacity,
        };
        assert_eq!(carried.at(0.0), (opacity, false));
        assert!(opacity > 0.4, "the grid is not blanked: {opacity}");
        assert_eq!(carried.at(FADE_SECS), (1.0, true));
    }
}
