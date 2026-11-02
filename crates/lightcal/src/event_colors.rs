//! Calendar colors as Lightcal draws them: day dots, agenda marks and swatches.
//!
//! Colors arrive as Google, Outlook, Windows and ICS feeds set them, and many
//! are pastels (or, from Windows, deep shades) that vanish as small marks.
//! These are graphics, so the target is the WCAG 3:1 non-text contrast against
//! the fill a mark sits on that is closest to it: #E8E8E8 (the selected day) in
//! light mode, about 3.7:1 on the white popup, and #313131 (the selected day)
//! in dark mode, which also covers the #2A2A2A chips and the #191919 popup.
//! Only OKLCH lightness moves, so the hue stays and pastels keep their chroma
//! instead of turning muddy.
//!
//! Both variants travel to Slint as a `MarkColor`, and `Theme.mark()` picks
//! one, so marks switch with the rest of the palette.

use crate::MarkColor;
use slint::Color;
use std::cell::RefCell;
use std::collections::HashMap;

/// More distinct colors than any calendar list shows; past it the cache
/// starts over, so recolored or removed calendars don't pile up in a tray app
/// that runs for weeks.
const CACHE_LIMIT: usize = 64;

thread_local! {
    /// Adjusted colors by source hex: the UI syncs every minute, but calendar
    /// colors rarely change.
    static CACHE: RefCell<HashMap<String, MarkColor>> = RefCell::default();
}

/// A calendar's color for both themes.
pub(crate) fn mark_color(hex: &str) -> MarkColor {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(mark) = cache.get(hex) {
            return mark.clone();
        }
        if cache.len() >= CACHE_LIMIT {
            cache.clear();
        }
        let source = parse_hex_color(hex);
        let mark = MarkColor {
            light: legible_event_color(source, false),
            dark: legible_event_color(source, true),
        };
        cache.insert(hex.to_string(), mark.clone());
        mark
    })
}

fn parse_hex_color(hex: &str) -> Color {
    let h = hex.trim().trim_start_matches('#');
    if h.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&h[0..2], 16),
            u8::from_str_radix(&h[2..4], 16),
            u8::from_str_radix(&h[4..6], 16),
        ) {
            return Color::from_rgb_u8(r, g, b);
        }
    }
    Color::from_rgb_u8(94, 106, 210)
}

fn legible_event_color(color: Color, dark_mode: bool) -> Color {
    // 3:1 against #E8E8E8 (L = 0.807): L <= 0.857 / 3 - 0.05.
    const LIGHT_MAX: f32 = (0.807 + 0.05) / 3.0 - 0.05;
    // 3:1 against #313131 (L = 0.0307): L >= 3 * 0.0807 - 0.05.
    const DARK_MIN: f32 = 3.0 * (0.0307 + 0.05) - 0.05;
    let passes = |rgb: [u8; 3]| {
        let lum = luminance(rgb.map(srgb_to_linear));
        if dark_mode {
            lum >= DARK_MIN
        } else {
            lum <= LIGHT_MAX
        }
    };
    let source = [color.red(), color.green(), color.blue()];
    if passes(source) {
        return color;
    }
    let [l, a, b] = linear_to_oklab(source.map(srgb_to_linear));
    // Bisect lightness between the source and white (dark mode) or black
    // (light mode), which always pass; only tested colors are returned.
    let (mut pass, mut fail) = (if dark_mode { 1.0 } else { 0.0 }, l);
    for _ in 0..24 {
        let mid = (pass + fail) / 2.0;
        if passes(oklab_in_gamut(mid, a, b)) {
            pass = mid;
        } else {
            fail = mid;
        }
    }
    let [r, g, b] = oklab_in_gamut(pass, a, b);
    Color::from_argb_u8(color.alpha(), r, g, b)
}

fn srgb_to_linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let encoded = if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round() as u8
}

/// WCAG relative luminance of linear sRGB.
fn luminance([r, g, b]: [f32; 3]) -> f32 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn linear_to_oklab([r, g, b]: [f32; 3]) -> [f32; 3] {
    let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

fn oklab_to_linear([l, a, b]: [f32; 3]) -> [f32; 3] {
    let l_ = (l + 0.396_337_78 * a + 0.215_803_76 * b).powi(3);
    let m_ = (l - 0.105_561_346 * a - 0.063_854_17 * b).powi(3);
    let s_ = (l - 0.089_484_18 * a - 1.291_485_5 * b).powi(3);
    [
        4.076_741_7 * l_ - 3.307_711_6 * m_ + 0.230_969_94 * s_,
        -1.268_438 * l_ + 2.609_757_4 * m_ - 0.341_319_38 * s_,
        -0.004_196_086_3 * l_ - 0.703_418_6 * m_ + 1.707_614_7 * s_,
    ]
}

/// The sRGB color at OKLab lightness `l` with the hue of `(a, b)`, giving up
/// only as much chroma as the sRGB gamut requires.
fn oklab_in_gamut(l: f32, a: f32, b: f32) -> [u8; 3] {
    let fits = |k: f32| {
        oklab_to_linear([l, a * k, b * k])
            .iter()
            .all(|c| (-1e-4..=1.0 + 1e-4).contains(c))
    };
    let mut keep = 1.0;
    if !fits(keep) {
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..20 {
            let k = (lo + hi) / 2.0;
            if fits(k) {
                lo = k;
            } else {
                hi = k;
            }
        }
        keep = lo;
    }
    oklab_to_linear([l, a * keep, b * keep]).map(linear_to_srgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lch(c: Color) -> [f32; 3] {
        let [l, a, b] = linear_to_oklab([c.red(), c.green(), c.blue()].map(srgb_to_linear));
        [l, a.hypot(b), b.atan2(a).to_degrees()]
    }

    fn lum(c: Color) -> f32 {
        luminance([c.red(), c.green(), c.blue()].map(srgb_to_linear))
    }

    fn contrast(x: Color, y: Color) -> f32 {
        let (x, y) = (lum(x), lum(y));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    // Only OKLCH lightness moves: the hue holds, and chroma (what turns
    // pastels muddy when lost) is kept apart from gamut limits.
    fn assert_same_hue_and_chroma(hex: &str, source: Color, out: Color) {
        let ([_, sc, sh], [_, oc, oh]) = (lch(source), lch(out));
        if sc < 0.03 {
            return; // Grays have no hue to keep.
        }
        let dh = (oh - sh + 540.0).rem_euclid(360.0) - 180.0;
        assert!(
            dh.abs() < 4.0,
            "{hex} -> {out:?}: hue moved {dh:.1} degrees"
        );
        assert!(
            oc >= 0.8 * sc,
            "{hex} -> {out:?}: chroma {sc:.3} -> {oc:.3}"
        );
    }

    #[test]
    fn light_mode_darkens_pale_colors_to_three_to_one_on_the_selected_day() {
        let selected = Color::from_rgb_u8(0xE8, 0xE8, 0xE8);
        // Google calendarList pastels, Outlook-style pastels, sample colors.
        for hex in [
            "#9FE1E7", "#FAD165", "#FBE983", "#FFD84F", "#A4BDFC", "#A6D1F5", "#42D692", "#D9822B",
            "#2E9D5B", "#CCCCCC",
        ] {
            let source = parse_hex_color(hex);
            let out = mark_color(hex).light;
            assert!(contrast(out, selected) >= 3.0, "{hex} -> {out:?}");
            assert!(lch(out)[0] <= lch(source)[0], "{hex} -> {out:?}");
            assert_same_hue_and_chroma(hex, source, out);
            // Dark mode keeps colors that already stand out there.
            assert_eq!(mark_color(hex).dark, source, "{hex}");
        }
        // Colors that already reach 3:1 on the selected day stay as is.
        assert_eq!(mark_color("#3B82C4").light, parse_hex_color("#3B82C4"));
    }

    #[test]
    fn dark_mode_lifts_deep_colors_to_three_to_one_on_every_dark_fill() {
        // Selected day, all-day chip and hover, popup.
        let fills = [0x31, 0x2A, 0x19].map(|v| Color::from_rgb_u8(v, v, v));
        for hex in [
            "#1F4E8C", "#123456", "#5B1A1A", "#3F51B5", "#008573", "#5E6AD2", "#000000",
        ] {
            let source = parse_hex_color(hex);
            let out = mark_color(hex).dark;
            for fill in fills {
                assert!(contrast(out, fill) >= 3.0, "{hex} -> {out:?} on {fill:?}");
            }
            assert!(lch(out)[0] >= lch(source)[0], "{hex} -> {out:?}");
            assert_same_hue_and_chroma(hex, source, out);
            // Light mode keeps colors that already stand out on white.
            assert_eq!(mark_color(hex).light, source, "{hex}");
        }
    }

    #[test]
    fn marks_come_from_a_cache_that_starts_over_when_full() {
        let cached = |hex: &str| CACHE.with(|cache| cache.borrow().get(hex).cloned());
        // A planted entry comes back as is, so lookups are served from the cache.
        let planted = MarkColor {
            light: Color::from_rgb_u8(1, 2, 3),
            dark: Color::from_rgb_u8(4, 5, 6),
        };
        CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.clear();
            cache.insert("#9FE1E7".into(), planted.clone());
        });
        assert_eq!(mark_color("#9FE1E7"), planted);

        // A new color is computed once and kept.
        let fresh = mark_color("#FAD165");
        assert_eq!(cached("#FAD165"), Some(fresh));

        // Filling the cache starts it over instead of growing it.
        for i in 0..CACHE_LIMIT {
            mark_color(&format!("#{i:06X}"));
        }
        assert!(CACHE.with(|cache| cache.borrow().len()) <= CACHE_LIMIT);
        assert_eq!(cached("#9FE1E7"), None);
        assert_ne!(mark_color("#9FE1E7"), planted);

        // Unparseable values fall back to the default calendar color.
        assert_eq!(parse_hex_color("teal"), Color::from_rgb_u8(94, 106, 210));
    }
}
