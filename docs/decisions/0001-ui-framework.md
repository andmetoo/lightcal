# ADR 0001: Rust desktop UI without a web runtime

Date: 2026-10-29

Status: **Accepted by the owner on 2026-10-29: Slint selected.** Platform integration and performance validation remain required before full UI implementation. No framework benchmark has been run.

## Requirements

Windows tray and taskbar entry points, a visually polished small calendar, low idle CPU and memory, Rust, no browser container, and a realistic macOS path.

Here, native execution means a compiled desktop application without a webview. Native controls means actual Win32/WinUI or AppKit widgets. These are different requirements. The owner explicitly requires the former; whether toolkit-drawn widgets satisfy the desired appearance is checked in design review.

## Comparison

| Option | Rendering and OS controls | Windows / macOS path | Fit for Lightcal |
| --- | --- | --- | --- |
| Slint | Compiled declarative UI; toolkit-drawn widgets; software and GPU renderer choices | Shared UI through its desktop backend; shell integration still platform-specific | Recommended: small custom calendar and event-driven UI, subject to measured footprint and licensing review |
| Iced | Rust UI with custom widgets; GPU and software renderer options | Shared cross-platform windowing; separate tray integration | Primary fallback: permissive MIT license and Rust-only view code; same platform prototype still required |
| egui / eframe | Immediate-mode custom UI, not OS controls | Native desktop integration on both platforms | Good prototype/tool UI; more custom visual work for this product. Reactive repainting can sleep at idle; immediate mode does not inherently mean continuous polling |
| Direct Windows APIs + AppKit bindings | Real platform controls and/or native drawing, chosen separately per OS | Shared Rust date/settings core, separate Windows and macOS views | Best alternative if actual OS controls become mandatory; higher duplicated UI and accessibility work |
| Tauri | System webview with Rust backend | Cross-platform | Excluded by the no-web-container requirement |

The ranking is an engineering judgment for this product, not measured proof that one toolkit is smaller or faster. None of the shared UI choices provides the entire tray/taskbar behavior automatically.

## Decision

Prototype **Slint + its winit backend**, initially with the software renderer and only necessary features enabled. Compare with a GPU renderer only if rendering quality or latency requires it. Do not select the default renderer blindly or infer process memory from embedded runtime marketing numbers.

Use platform APIs through `windows` for Windows-specific shell behavior. Evaluate `tray-icon` and `muda` for the tray/menu layer; retain a direct `Shell_NotifyIconW` adapter as the Windows fallback. Their event-loop requirements must be proven with Slint, not assumed. Slint owns the GUI event loop; do not start a competing GUI loop.

Lightcal's chosen GPL-3.0-only license makes Slint's GPLv3 route the intended dependency license. Record the exact selected version, its license expression, and transitive notices when adding dependencies. Do not silently switch to royalty-free/commercial terms. Alternative dependency licenses, attribution requirements, and release source obligations must be reviewed against the actual locked dependency set before distribution.

## Validation gate

Time-box the initial prototype to 2-3 implementation days. Demonstrate tray show/hide, focus loss, correct taskbar identity and pinned-shortcut reactivation, one process, high-DPI placement, keyboard operation, and no idle render loop. Measure using [the validation protocol](../validation.md). Build and launch a basic macOS view and menu-bar entry on real macOS before platform APIs become deeply embedded.

If tray integration or accessibility blocks the prototype, or resource targets fail after one bounded tuning pass, document the evidence and propose revisiting this decision with the owner. Do not switch frameworks without owner approval. Record prototype results in this ADR.

## Evidence

Official sources reviewed on 2026-10-29; recheck against the pinned release during implementation. Some `latest` documentation may describe unreleased functionality.

- [Slint desktop backend and renderer options](https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backend_winit/)
- [Slint licensing FAQ](https://github.com/slint-ui/slint/blob/master/FAQ.md) and [license options](https://slint.dev/pricing)
- [Iced project: platform support, renderers, and license](https://github.com/iced-rs/iced)
- [egui: native appearance, repaint behavior, and licensing](https://github.com/emilk/egui)
- [tray-icon platform and event-loop requirements](https://docs.rs/tray-icon/latest/tray_icon/)
- [Microsoft Rust bindings for Windows](https://github.com/microsoft/windows-rs)
- [Rust Objective-C and AppKit bindings](https://github.com/madsmtm/objc2)
- [Microsoft taskbar APIs](https://learn.microsoft.com/en-us/windows/win32/shell/taskbar)
- [Microsoft notification-area APIs](https://learn.microsoft.com/en-us/windows/win32/shell/notification-area)
