//! macOS menu-bar (`NSStatusItem`) and Dock activation adapter stub.
//!
//! Preserves the architectural port boundary described in `docs/architecture.md`.

use crate::platform::WorkArea;

pub fn primary_work_area() -> WorkArea {
    WorkArea {
        x: 0,
        y: 25,
        width: 1728,
        height: 1055,
    }
}
