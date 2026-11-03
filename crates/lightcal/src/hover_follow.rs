//! Hover follow (ADR 0015): one highlight pulled toward the day cell under the pointer by a
//! critically damped spring. A new cell only moves the target, so the highlight carries on
//! from where it is drawn, at the speed it has, and never visits cells passed on the way.
//! A timer advances the spring only while something still moves.

use crate::AppWindow;
use slint::ComponentHandle;
use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

/// Stiffness in 1/s, chosen by the owner: 95 % of a step in about 237 ms, settled in 332 ms.
const OMEGA: f64 = 20.0;
/// A stalled frame advances at most this far, so the highlight never leaps.
const MAX_DT: f64 = 0.032;
const FADE_IN_SECS: f64 = 0.100;
const FADE_OUT_SECS: f64 = 0.120;
/// Positions are in cells; a column is about 47 px and a row 32–34 px, so these are about
/// 0.5 px and 10 px/s, where the last step onto the cell is too small to see.
const SETTLE_POS: f64 = 0.01;
const SETTLE_VEL: f64 = 0.2;
/// One tick of Windows' default 15.625 ms timer; frame time is measured, not assumed.
const FRAME: Duration = Duration::from_micros(15_625);

/// One axis of the spring, in cells.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Axis {
    pos: f64,
    vel: f64,
    target: f64,
}

impl Axis {
    fn place(&mut self, at: f64) {
        *self = Axis {
            pos: at,
            vel: 0.0,
            target: at,
        };
    }

    /// Keeps position and velocity. Velocity toward the new target is capped at
    /// `OMEGA × distance`: beyond that a critically damped spring passes its target once,
    /// so the cap is what keeps a fast sweep that stops or reverses from overshooting.
    fn retarget(&mut self, target: f64) {
        self.target = target;
        let e = self.pos - target;
        let limit = OMEGA * e.abs();
        if self.vel * -e.signum() > limit {
            self.vel = -e.signum() * limit;
        }
    }

    /// The exact critically damped step, so any frame length lands on the same path.
    fn step(&mut self, dt: f64) {
        let e = self.pos - self.target;
        let c = self.vel + OMEGA * e;
        let decay = (-OMEGA * dt).exp();
        self.pos = self.target + (e + c * dt) * decay;
        self.vel = (self.vel - OMEGA * c * dt) * decay;
        if self.settled() {
            self.place(self.target);
        }
    }

    fn at_rest(&self) -> bool {
        self.pos == self.target && self.vel == 0.0
    }

    fn settled(&self) -> bool {
        (self.pos - self.target).abs() < SETTLE_POS && self.vel.abs() < SETTLE_VEL
    }
}

/// The hover highlight: column and row in cells, and opacity.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct HoverFollow {
    col: Axis,
    row: Axis,
    opacity: f64,
    shown: bool,
}

impl HoverFollow {
    /// The pointer is over `cell` (column, row), or off the day rows. Without `animate`
    /// (Windows Animation effects off) the final state applies at once.
    pub(crate) fn point(&mut self, cell: Option<(i32, i32)>, animate: bool) {
        match cell {
            Some((col, row)) => {
                let (col, row) = (f64::from(col), f64::from(row));
                // Entering from nothing starts at the cell, never from an old position.
                if !animate || self.opacity == 0.0 {
                    self.col.place(col);
                    self.row.place(row);
                } else {
                    self.col.retarget(col);
                    self.row.retarget(row);
                }
                self.shown = true;
            }
            None => self.shown = false,
        }
        if !animate {
            self.opacity = if self.shown { 1.0 } else { 0.0 };
        }
    }

    /// Hides at once and settles, for a hidden popup or open Settings.
    pub(crate) fn clear(&mut self) {
        *self = HoverFollow::default();
    }

    /// Advances `dt` seconds.
    pub(crate) fn step(&mut self, dt: f64) {
        let dt = dt.clamp(0.0, MAX_DT);
        if self.shown {
            self.opacity = (self.opacity + dt / FADE_IN_SECS).min(1.0);
        } else {
            self.opacity = (self.opacity - dt / FADE_OUT_SECS).max(0.0);
        }
        if self.opacity == 0.0 {
            // Nothing to see: the next entry starts fresh at its cell anyway.
            self.col.place(self.col.target);
            self.row.place(self.row.target);
        } else {
            self.col.step(dt);
            self.row.step(dt);
        }
    }

    /// Whether position or opacity is still changing.
    pub(crate) fn moving(&self) -> bool {
        let fading = self.opacity != if self.shown { 1.0 } else { 0.0 };
        fading || !self.col.at_rest() || !self.row.at_rest()
    }

    pub(crate) fn col(&self) -> f64 {
        self.col.pos
    }

    pub(crate) fn row(&self) -> f64 {
        self.row.pos
    }

    pub(crate) fn opacity(&self) -> f64 {
        self.opacity
    }
}

thread_local! {
    static HOVER: RefCell<HoverFollow> = RefCell::default();
    static HOVER_TIMER: slint::Timer = slint::Timer::default();
    static LAST_FRAME: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// The day grid reports the cell under the pointer (row × 7 + column), or −1 off the rows.
pub(crate) fn wire_hover_follow(ui: &AppWindow) {
    let ui_weak = ui.as_weak();
    ui.on_hover_cell(move |index| {
        if let Some(ui) = ui_weak.upgrade() {
            let cell = (index >= 0).then_some((index % 7, index / 7));
            let animate = lightcal::platform::animation_effects_enabled();
            HOVER.with_borrow_mut(|hover| hover.point(cell, animate));
            publish(&ui);
        }
    });
}

/// Arrow keys hide the highlight until the pointer moves again.
pub(crate) fn hide_hover_for_keys(ui: &AppWindow) {
    ui.set_hover_cell_index(-1);
    let animate = lightcal::platform::animation_effects_enabled();
    HOVER.with_borrow_mut(|hover| hover.point(None, animate));
    publish(ui);
}

/// Hidden popup or open Settings: no highlight and no timer.
pub(crate) fn clear_hover(ui: &AppWindow) {
    ui.set_hover_cell_index(-1);
    HOVER.with_borrow_mut(HoverFollow::clear);
    publish(ui);
}

/// Shows the current state and runs the frame timer only while it still changes.
fn publish(ui: &AppWindow) {
    let hover = HOVER.with_borrow(|hover| *hover);
    ui.set_hover_col(hover.col() as f32);
    ui.set_hover_row(hover.row() as f32);
    ui.set_hover_opacity(hover.opacity() as f32);
    HOVER_TIMER.with(|timer| {
        if !hover.moving() {
            timer.stop();
            LAST_FRAME.with(|last| last.set(None));
            return;
        }
        if timer.running() {
            return;
        }
        LAST_FRAME.with(|last| last.set(Some(Instant::now())));
        let ui_weak = ui.as_weak();
        timer.start(slint::TimerMode::Repeated, FRAME, move || {
            if let Some(ui) = ui_weak.upgrade() {
                hover_frame(&ui);
            }
        });
    });
}

fn hover_frame(ui: &AppWindow) {
    let now = Instant::now();
    let dt = LAST_FRAME
        .with(|last| last.replace(Some(now)))
        .map_or(0.0, |last| (now - last).as_secs_f64());
    HOVER.with_borrow_mut(|hover| hover.step(dt));
    publish(ui);
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 1.0 / 64.0;

    fn run(hover: &mut HoverFollow, secs: f64) -> Vec<(f64, f64)> {
        let mut path = Vec::new();
        let mut t = 0.0;
        while t < secs {
            hover.step(DT);
            path.push((hover.col(), hover.row()));
            t += DT;
        }
        path
    }

    fn shown_at(col: i32, row: i32) -> HoverFollow {
        let mut hover = HoverFollow::default();
        hover.point(Some((col, row)), true);
        run(&mut hover, 0.5);
        hover
    }

    #[test]
    fn entering_starts_at_the_cell_and_fades_in() {
        let mut hover = HoverFollow::default();
        hover.point(Some((3, 2)), true);
        assert_eq!((hover.col(), hover.row(), hover.opacity()), (3.0, 2.0, 0.0));
        assert!(hover.moving());
        run(&mut hover, FADE_IN_SECS + DT);
        assert_eq!((hover.col(), hover.row(), hover.opacity()), (3.0, 2.0, 1.0));
        assert!(!hover.moving());
    }

    #[test]
    fn one_column_settles_in_about_a_third_of_a_second() {
        let mut hover = shown_at(0, 0);
        hover.point(Some((1, 0)), true);
        let path = run(&mut hover, 0.5);
        let at = |secs: f64| path[(secs / DT) as usize - 1].0;
        assert!((0.93..0.97).contains(&at(0.237)), "{}", at(0.237));
        let settled = path.iter().position(|&(col, _)| col == 1.0).unwrap();
        let settled_secs = (settled + 1) as f64 * DT;
        assert!((0.25..0.36).contains(&settled_secs), "{settled_secs}");
        assert!(!hover.moving());
    }

    #[test]
    fn a_new_target_continues_from_the_drawn_position_without_a_jump() {
        let mut hover = shown_at(0, 0);
        hover.point(Some((3, 0)), true);
        run(&mut hover, 0.05);
        let before = hover.col();
        hover.step(DT);
        let stride = hover.col() - before;
        // Mid-flight the pointer moves on to a cell further along and one row down.
        let here = hover.col();
        hover.point(Some((5, 2)), true);
        assert_eq!(hover.col(), here, "retargeting never moves the highlight");
        hover.step(DT);
        let next = hover.col() - here;
        assert!(
            next > 0.0 && (next - stride).abs() < stride / 2.0,
            "{stride} then {next}"
        );
        let path = run(&mut hover, 0.6);
        assert_eq!(*path.last().unwrap(), (5.0, 2.0));
    }

    #[test]
    fn a_rapid_sweep_never_visits_cells_passed_on_the_way() {
        let mut hover = shown_at(0, 0);
        // The pointer crosses five cells in 40 ms, then rests on the last one.
        for col in 1..=5 {
            hover.point(Some((col, 0)), true);
            hover.step(0.008);
        }
        let path = run(&mut hover, 0.6);
        // Monotonic toward column 5: no queue back through earlier cells, no overshoot.
        assert!(path.windows(2).all(|w| w[1].0 >= w[0].0));
        assert!(path.iter().all(|&(col, _)| col <= 5.0));
        assert_eq!(path.last().unwrap().0, 5.0);
    }

    #[test]
    fn a_sweep_that_reverses_does_not_overshoot() {
        let mut hover = shown_at(0, 0);
        hover.point(Some((6, 0)), true);
        run(&mut hover, 0.06);
        // Moving right fast, the pointer returns to the cell the highlight is entering.
        let target = hover.col().ceil() as i32;
        hover.point(Some((target, 0)), true);
        let path = run(&mut hover, 0.6);
        assert!(path.iter().all(|&(col, _)| col <= f64::from(target)));
        assert_eq!(path.last().unwrap().0, f64::from(target));
    }

    #[test]
    fn frame_length_does_not_change_the_path() {
        let mut a = shown_at(0, 0);
        let mut b = a;
        a.point(Some((4, 3)), true);
        b.point(Some((4, 3)), true);
        for _ in 0..4 {
            a.step(DT / 2.0);
            a.step(DT / 2.0);
            b.step(DT);
        }
        assert!((a.col() - b.col()).abs() < 1e-9 && (a.row() - b.row()).abs() < 1e-9);
    }

    #[test]
    fn a_stalled_frame_cannot_leap() {
        let mut hover = shown_at(0, 0);
        hover.point(Some((6, 0)), true);
        hover.step(2.0);
        let mut capped = shown_at(0, 0);
        capped.point(Some((6, 0)), true);
        capped.step(MAX_DT);
        assert_eq!(hover, capped);
    }

    #[test]
    fn leaving_fades_out_in_place_and_reentry_starts_fresh() {
        let mut hover = shown_at(1, 1);
        hover.point(None, true);
        run(&mut hover, FADE_OUT_SECS / 2.0);
        assert!(hover.opacity() > 0.0 && hover.opacity() < 1.0);
        assert_eq!((hover.col(), hover.row()), (1.0, 1.0));
        run(&mut hover, FADE_OUT_SECS);
        assert_eq!(hover.opacity(), 0.0);
        assert!(!hover.moving());
        hover.point(Some((5, 4)), true);
        assert_eq!((hover.col(), hover.row()), (5.0, 4.0));
    }

    #[test]
    fn without_animation_effects_the_final_state_applies_at_once() {
        let mut hover = HoverFollow::default();
        hover.point(Some((2, 3)), false);
        assert_eq!((hover.col(), hover.row(), hover.opacity()), (2.0, 3.0, 1.0));
        assert!(!hover.moving());
        hover.point(Some((6, 0)), false);
        assert_eq!((hover.col(), hover.row()), (6.0, 0.0));
        assert!(!hover.moving());
        hover.point(None, false);
        assert_eq!(hover.opacity(), 0.0);
        assert!(!hover.moving());
    }

    #[test]
    fn clear_stops_everything() {
        let mut hover = shown_at(0, 0);
        hover.point(Some((6, 5)), true);
        hover.step(DT);
        hover.clear();
        assert_eq!(hover.opacity(), 0.0);
        assert!(!hover.moving());
    }
}
