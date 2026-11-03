//! Native-size Lightcal icons from ADR 0011.
//!
//! The tray shows today's date on the coral tile (Color) or as an outline in
//! the taskbar's ink (Mono). Every raster is drawn at its exact pixel size.
//! Numerals use Segoe UI Variable, read at runtime from the Windows fonts
//! folder; Segoe is never bundled. When no system font can be read, a simple
//! geometric stroke face keeps the date legible.

use crate::preferences::TrayIconMode;
use calendar_core::{LocalDate, Weekday};
use skrifa::instance::Size;
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, MetadataProvider};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use zeno::{Command, Mask, Point};

const CORAL: [u8; 3] = [0xD7, 0x38, 0x40];
const CORAL_BAND: [u8; 3] = [0xA9, 0x2A, 0x31];
const WHITE: [u8; 3] = [0xFF, 0xFF, 0xFF];
/// Mono ink follows the Windows shell theme, not the application theme.
const INK_ON_LIGHT_SHELL: [u8; 3] = [0x1A, 0x1A, 0x1A];
const INK_ON_DARK_SHELL: [u8; 3] = [0xFF, 0xFF, 0xFF];
const APP_GRID_DIM_ALPHA: f32 = 0.55;

/// Characters extracted from the system font: digits, the time colon and the
/// letters of the English weekday codes used by the legacy tray modes.
const GLYPH_CHARS: &str = "0123456789:ADEFHIMNORSTUW";

/// Files in `%WINDIR%\Fonts`, best first, with their variation settings.
/// Segoe UI Variable is a variable font: `wght` 700 selects Bold and `opsz`
/// 10.5 selects the Text optical size used for small UI text.
type Variation = (&'static str, f32);

const SYSTEM_FONTS: [(&str, &str, &[Variation]); 3] = [
    (
        "SegUIVar.ttf",
        "Segoe UI Variable Text Bold",
        &[("wght", 700.0), ("opsz", 10.5)],
    ),
    ("segoeuib.ttf", "Segoe UI Bold", &[]),
    ("seguisb.ttf", "Segoe UI Semibold", &[]),
];

/// One glyph outline in em units (y up) with its advance.
struct Glyph {
    path: Vec<Command>,
    advance: f32,
}

/// The few glyphs the icons need. Only these outlines stay in memory; the
/// font file itself is dropped after extraction.
struct SystemGlyphs {
    glyphs: HashMap<char, Glyph>,
    /// Height of the flat-topped digit `7`, as a fraction of the em.
    digit_height: f32,
    source: &'static str,
}

static SYSTEM_GLYPHS: OnceLock<Option<SystemGlyphs>> = OnceLock::new();

fn system_glyphs() -> Option<&'static SystemGlyphs> {
    SYSTEM_GLYPHS.get_or_init(load_system_glyphs).as_ref()
}

fn load_system_glyphs() -> Option<SystemGlyphs> {
    let fonts = super::system_fonts_dir()?;
    SYSTEM_FONTS.iter().find_map(|(file, source, settings)| {
        let data = std::fs::read(fonts.join(file)).ok()?;
        extract_glyphs(&data, settings, source)
    })
}

#[derive(Default)]
struct CollectPen(Vec<Command>);

impl OutlinePen for CollectPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(Command::MoveTo(Point::new(x, y)));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(Command::LineTo(Point::new(x, y)));
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.0
            .push(Command::QuadTo(Point::new(cx0, cy0), Point::new(x, y)));
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.push(Command::CurveTo(
            Point::new(cx0, cy0),
            Point::new(cx1, cy1),
            Point::new(x, y),
        ));
    }
    fn close(&mut self) {
        self.0.push(Command::Close);
    }
}

fn extract_glyphs(
    data: &[u8],
    settings: &[(&str, f32)],
    source: &'static str,
) -> Option<SystemGlyphs> {
    let font = FontRef::new(data).ok()?;
    let location = font.axes().location(settings.iter().copied());
    let charmap = font.charmap();
    let outlines = font.outline_glyphs();
    let metrics = font.glyph_metrics(Size::new(1.0), &location);
    let mut glyphs = HashMap::new();
    for character in GLYPH_CHARS.chars() {
        let Some(id) = charmap.map(character) else {
            continue;
        };
        let (Some(outline), Some(advance)) = (outlines.get(id), metrics.advance_width(id)) else {
            continue;
        };
        let mut pen = CollectPen::default();
        if outline
            .draw(DrawSettings::unhinted(Size::new(1.0), &location), &mut pen)
            .is_err()
        {
            continue;
        }
        glyphs.insert(
            character,
            Glyph {
                path: pen.0,
                advance,
            },
        );
    }
    if !('0'..='9').all(|digit| glyphs.contains_key(&digit)) {
        return None;
    }
    let digit_height = command_points(&glyphs[&'7'].path)
        .map(|point| point.y)
        .fold(0.0f32, f32::max);
    (digit_height > 0.3).then_some(SystemGlyphs {
        glyphs,
        digit_height,
        source,
    })
}

fn command_points(path: &[Command]) -> impl Iterator<Item = Point> + '_ {
    path.iter().flat_map(|command| {
        let points: [Option<Point>; 3] = match *command {
            Command::MoveTo(p) | Command::LineTo(p) => [Some(p), None, None],
            Command::QuadTo(c, p) => [Some(c), Some(p), None],
            Command::CurveTo(c0, c1, p) => [Some(c0), Some(c1), Some(p)],
            Command::Close => [None, None, None],
        };
        points.into_iter().flatten()
    })
}

/// The name of the face that draws tray numerals on this machine.
pub fn numeral_font_source() -> &'static str {
    system_glyphs().map_or("built-in stroke digits (no system font found)", |glyphs| {
        glyphs.source
    })
}

/// A source of numeral outlines.
#[derive(Clone, Copy)]
enum Face {
    System(&'static SystemGlyphs),
    /// Geometric digits for machines without a readable Segoe UI font.
    Strokes,
}

impl Face {
    fn current() -> Self {
        system_glyphs().map_or(Face::Strokes, Face::System)
    }

    /// Font size, in pixels per em, whose digits are `height` pixels tall.
    fn px_for_digit_height(self, height: f32) -> f32 {
        match self {
            Face::System(glyphs) => height / glyphs.digit_height,
            Face::Strokes => height,
        }
    }

    /// Horizontal ink extent of `text` relative to the pen origin.
    fn extent(self, text: &str, px: f32) -> (f32, f32) {
        let mut left = f32::MAX;
        let mut right = f32::MIN;
        match self {
            Face::System(glyphs) => {
                let mut pen = 0.0;
                for glyph in text.chars().filter_map(|c| glyphs.glyphs.get(&c)) {
                    for point in command_points(&glyph.path) {
                        left = left.min(pen + point.x * px);
                        right = right.max(pen + point.x * px);
                    }
                    pen += glyph.advance * px;
                }
            }
            Face::Strokes => {
                for (segment, _) in stroke_segments(text, px) {
                    let half = stroke_width(px) * 0.5;
                    left = left.min(segment[0].min(segment[2]) - half);
                    right = right.max(segment[0].max(segment[2]) + half);
                }
            }
        }
        if left > right {
            (0.0, 0.0)
        } else {
            (left, right)
        }
    }

    /// Coverage mask (`size`×`size`) of `text` with its pen origin at
    /// (`origin_x`, `baseline`).
    fn draw(self, text: &str, px: f32, origin_x: f32, baseline: f32, size: usize) -> Vec<u8> {
        match self {
            Face::System(glyphs) => {
                let mut path = Vec::new();
                let mut pen = origin_x;
                let map = |p: Point, pen: f32| Point::new(pen + p.x * px, baseline - p.y * px);
                for glyph in text.chars().filter_map(|c| glyphs.glyphs.get(&c)) {
                    path.extend(glyph.path.iter().map(|command| match *command {
                        Command::MoveTo(p) => Command::MoveTo(map(p, pen)),
                        Command::LineTo(p) => Command::LineTo(map(p, pen)),
                        Command::QuadTo(c, p) => Command::QuadTo(map(c, pen), map(p, pen)),
                        Command::CurveTo(c0, c1, p) => {
                            Command::CurveTo(map(c0, pen), map(c1, pen), map(p, pen))
                        }
                        Command::Close => Command::Close,
                    }));
                    pen += glyph.advance * px;
                }
                if path.is_empty() {
                    return vec![0; size * size];
                }
                Mask::new(&path).size(size as u32, size as u32).render().0
            }
            Face::Strokes => {
                let segments: Vec<[f32; 4]> = stroke_segments(text, px)
                    .map(|(s, _)| {
                        [
                            s[0] + origin_x,
                            baseline - s[1],
                            s[2] + origin_x,
                            baseline - s[3],
                        ]
                    })
                    .collect();
                let half = stroke_width(px) * 0.5;
                let mut mask = vec![0u8; size * size];
                for (i, value) in mask.iter_mut().enumerate() {
                    let (x, y) = ((i % size) as f32, (i / size) as f32);
                    let mut covered = 0u32;
                    for sample in 0..16 {
                        let fx = x + ((sample % 4) as f32 + 0.5) / 4.0;
                        let fy = y + ((sample / 4) as f32 + 0.5) / 4.0;
                        if segments.iter().any(|s| segment_distance(fx, fy, s) <= half) {
                            covered += 1;
                        }
                    }
                    *value = (covered * 255 / 16) as u8;
                }
                mask
            }
        }
    }
}

fn stroke_width(height: f32) -> f32 {
    (height * 0.2).max(1.0)
}

/// Centre-line segments `[x0, y0, x1, y1]` (y up from the baseline) of the
/// fallback stroke digits, laid out with `height`-pixel digits.
fn stroke_segments(text: &str, height: f32) -> impl Iterator<Item = ([f32; 4], char)> + '_ {
    const DIGITS: [&[[f32; 4]]; 10] = [
        &[
            [0., 0., 1., 0.],
            [1., 0., 1., 1.],
            [1., 1., 0., 1.],
            [0., 1., 0., 0.],
        ],
        &[[0.6, 0., 0.6, 1.], [0.6, 1., 0.2, 0.75]],
        &[
            [0., 1., 1., 1.],
            [1., 1., 1., 0.5],
            [1., 0.5, 0., 0.5],
            [0., 0.5, 0., 0.],
            [0., 0., 1., 0.],
        ],
        &[
            [0., 1., 1., 1.],
            [1., 1., 1., 0.],
            [1., 0., 0., 0.],
            [0.25, 0.5, 1., 0.5],
        ],
        &[[0., 1., 0., 0.4], [0., 0.4, 1., 0.4], [0.8, 1., 0.8, 0.]],
        &[
            [1., 1., 0., 1.],
            [0., 1., 0., 0.5],
            [0., 0.5, 1., 0.5],
            [1., 0.5, 1., 0.],
            [1., 0., 0., 0.],
        ],
        &[
            [1., 1., 0., 1.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [1., 0., 1., 0.5],
            [1., 0.5, 0., 0.5],
        ],
        &[[0., 1., 1., 1.], [1., 1., 0.35, 0.]],
        &[
            [0., 0., 1., 0.],
            [1., 0., 1., 1.],
            [1., 1., 0., 1.],
            [0., 1., 0., 0.],
            [0., 0.5, 1., 0.5],
        ],
        &[
            [1., 0.5, 0., 0.5],
            [0., 0.5, 0., 1.],
            [0., 1., 1., 1.],
            [1., 1., 1., 0.],
            [1., 0., 0., 0.],
        ],
    ];
    let inset = stroke_width(height) * 0.5;
    let width = (height * 0.62).max(3.0) - 2.0 * inset;
    let tall = height - 2.0 * inset;
    let gap = height * 0.22 + 2.0 * inset;
    let mut pen = 0.0f32;
    text.chars().flat_map(move |character| {
        let start = pen;
        let segments: Vec<([f32; 4], char)> = match character {
            '0'..='9' => {
                pen += width + gap;
                DIGITS[character as usize - '0' as usize]
                    .iter()
                    .map(|s| {
                        (
                            [
                                start + inset + s[0] * width,
                                inset + s[1] * tall,
                                start + inset + s[2] * width,
                                inset + s[3] * tall,
                            ],
                            character,
                        )
                    })
                    .collect()
            }
            ':' => {
                pen += 2.0 * inset + gap;
                [0.25, 0.75]
                    .iter()
                    .map(|y| {
                        (
                            [start + inset, y * height, start + inset, y * height],
                            character,
                        )
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        segments
    })
}

fn segment_distance(x: f32, y: f32, s: &[f32; 4]) -> f32 {
    let (dx, dy) = (s[2] - s[0], s[3] - s[1]);
    let length = dx * dx + dy * dy;
    let t = if length > 0.0 {
        (((x - s[0]) * dx + (y - s[1]) * dy) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (x - s[0] - t * dx).hypot(y - s[1] - t * dy)
}

/// Signed distance from a point to a rounded rectangle (negative inside).
fn rounded_rect_distance(x: f32, y: f32, rect: [f32; 4], radius: f32) -> f32 {
    let half_w = (rect[2] - rect[0]) * 0.5;
    let half_h = (rect[3] - rect[1]) * 0.5;
    let radius = radius.min(half_w).min(half_h);
    let qx = (x - (rect[0] + rect[2]) * 0.5).abs() - (half_w - radius);
    let qy = (y - (rect[1] + rect[3]) * 0.5).abs() - (half_h - radius);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius
}

/// Paints a `size`×`size` straight-alpha RGBA raster with `samples`² supersampling.
/// `shade` returns the premultiplied colour (0..=1 channels) at a point.
fn paint(size: usize, samples: usize, shade: impl Fn(f32, f32) -> [f32; 4]) -> Vec<u8> {
    let mut pixels = vec![0u8; size * size * 4];
    let count = (samples * samples) as f32;
    for y in 0..size {
        for x in 0..size {
            let mut sum = [0.0f32; 4];
            for sy in 0..samples {
                for sx in 0..samples {
                    let color = shade(
                        x as f32 + (sx as f32 + 0.5) / samples as f32,
                        y as f32 + (sy as f32 + 0.5) / samples as f32,
                    );
                    for (total, value) in sum.iter_mut().zip(color) {
                        *total += value;
                    }
                }
            }
            let alpha = sum[3] / count;
            if alpha > 0.0 {
                let index = (y * size + x) * 4;
                for channel in 0..3 {
                    pixels[index + channel] =
                        (sum[channel] / sum[3] * 255.0).round().clamp(0.0, 255.0) as u8;
                }
                pixels[index + 3] = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    pixels
}

fn opaque(rgb: [u8; 3]) -> [f32; 4] {
    [
        rgb[0] as f32 / 255.0,
        rgb[1] as f32 / 255.0,
        rgb[2] as f32 / 255.0,
        1.0,
    ]
}

/// Source-over for premultiplied colours.
fn over(dst: [f32; 4], rgb: [u8; 3], alpha: f32) -> [f32; 4] {
    let src = opaque(rgb);
    [
        src[0] * alpha + dst[0] * (1.0 - alpha),
        src[1] * alpha + dst[1] * (1.0 - alpha),
        src[2] * alpha + dst[2] * (1.0 - alpha),
        alpha + dst[3] * (1.0 - alpha),
    ]
}

/// Pixel geometry of the date tile at one native size. Every value scales
/// from the 32 px master and is snapped where an edge would otherwise blur.
#[derive(Debug, Clone, Copy, PartialEq)]
struct TileGeometry {
    size: usize,
    /// Corner radius: 6/32 of the raster.
    radius: f32,
    /// Bottom of the darker band in whole pixels: 6/32 of the raster.
    band_bottom: usize,
    /// Mono outline width in whole pixels: 2/32 of the raster, at least 1.
    stroke: usize,
}

impl TileGeometry {
    fn new(size: usize, band_ratio: f32) -> Self {
        let side = size as f32;
        Self {
            size,
            radius: side * 6.0 / 32.0,
            band_bottom: ((side * band_ratio).round() as usize).max(3),
            stroke: ((side / 16.0).round() as usize).max(1),
        }
    }

    /// Clear space between numerals and the tile edge. Color and Mono share
    /// it, so both styles use the same numeral size.
    fn margin(&self) -> usize {
        self.stroke + 1
    }
}

const DATE_BAND_RATIO: f32 = 6.0 / 32.0;
/// Legacy two-line modes put a short label inside a taller band.
const LABEL_BAND_RATIO: f32 = 11.0 / 32.0;

#[derive(Clone, Copy)]
enum Style {
    Color,
    Mono { ink: [u8; 3] },
}

impl Style {
    fn new(monochrome: bool, dark_shell: bool) -> Self {
        if monochrome {
            Style::Mono {
                ink: if dark_shell {
                    INK_ON_DARK_SHELL
                } else {
                    INK_ON_LIGHT_SHELL
                },
            }
        } else {
            Style::Color
        }
    }
}

fn paint_tile(geometry: TileGeometry, style: Style) -> Vec<u8> {
    let side = geometry.size as f32;
    let band = geometry.band_bottom as f32;
    let stroke = geometry.stroke as f32;
    paint(geometry.size, 4, |x, y| {
        let distance = rounded_rect_distance(x, y, [0.0, 0.0, side, side], geometry.radius);
        if distance > 0.0 {
            return [0.0; 4];
        }
        match style {
            Style::Color => opaque(if y < band { CORAL_BAND } else { CORAL }),
            Style::Mono { ink } if y < band || distance > -stroke => opaque(ink),
            Style::Mono { .. } => [0.0; 4],
        }
    })
}

/// A text block placed on the tile at one size shared by all its variants.
struct TextFit {
    px: f32,
    height: usize,
}

/// Picks one size for every candidate string: the tallest digit height, up to
/// `target`, at which all candidates fit `max_width` × `max_height` pixels.
fn fit_text(
    face: Face,
    candidates: &[String],
    target: usize,
    max_width: f32,
    max_height: usize,
) -> TextFit {
    let mut height = target.min(max_height).max(3);
    loop {
        let px = face.px_for_digit_height(height as f32);
        let fits = candidates.iter().all(|text| {
            let (left, right) = face.extent(text, px);
            right - left <= max_width
        });
        if fits || height <= 3 {
            return TextFit { px, height };
        }
        height -= 1;
    }
}

#[derive(Clone, Copy)]
enum Ink {
    Paint([u8; 3]),
    /// Cut the text out of a filled band.
    KnockOut,
}

/// Draws `text` with its digit tops at row `top`, centred on the raster's
/// vertical axis by its visible pixels and shifted only by whole pixels.
fn draw_text(
    pixels: &mut [u8],
    size: usize,
    face: Face,
    text: &str,
    fit: &TextFit,
    top: usize,
    ink: Ink,
) {
    let baseline = (top + fit.height) as f32;
    let (left, right) = face.extent(text, fit.px);
    let origin = (size as f32 * 0.5 - (left + right) * 0.5).round();
    let mask = face.draw(text, fit.px, origin, baseline, size);
    let mut columns = (size, 0usize);
    for (i, &coverage) in mask.iter().enumerate() {
        if coverage > 24 {
            columns.0 = columns.0.min(i % size);
            columns.1 = columns.1.max(i % size);
        }
    }
    let shift = if columns.0 <= columns.1 {
        (size as f32 * 0.5 - (columns.0 + columns.1 + 1) as f32 * 0.5).round() as isize
    } else {
        0
    };
    for (i, &coverage) in mask.iter().enumerate() {
        let x = (i % size) as isize + shift;
        if coverage == 0 || x < 0 || x >= size as isize {
            continue;
        }
        let index = ((i / size) * size + x as usize) * 4;
        match ink {
            Ink::Paint(rgb) => blend_over(&mut pixels[index..index + 4], rgb, coverage as u32),
            Ink::KnockOut => {
                let alpha = pixels[index + 3] as u32;
                pixels[index + 3] = (alpha * (255 - coverage as u32) / 255) as u8;
            }
        }
    }
}

fn digit_height_target(size: usize) -> usize {
    (size as f32 * 0.4 + 0.5).round() as usize
}

/// The date tile: an optional label in a taller band, and a body line
/// centred below the band. `body_candidates` share one size.
fn render_tile(
    size: usize,
    face: Face,
    style: Style,
    band_label: Option<(&str, &[String])>,
    body: &str,
    body_candidates: &[String],
    body_target: usize,
) -> Vec<u8> {
    let ratio = if band_label.is_some() {
        LABEL_BAND_RATIO
    } else {
        DATE_BAND_RATIO
    };
    let geometry = TileGeometry::new(size, ratio);
    let mut pixels = paint_tile(geometry, style);
    let margin = geometry.margin();
    let max_width = (size - 2 * margin) as f32;
    let ink = match style {
        Style::Color => WHITE,
        Style::Mono { ink } => ink,
    };
    if let Some((label, candidates)) = band_label {
        let band = geometry.band_bottom;
        let fit = fit_text(
            face,
            candidates,
            band / 2,
            max_width,
            band.saturating_sub(2),
        );
        let top = (band - fit.height).div_ceil(2);
        let label_ink = match style {
            Style::Color => Ink::Paint(WHITE),
            Style::Mono { .. } => Ink::KnockOut,
        };
        draw_text(&mut pixels, size, face, label, &fit, top, label_ink);
    }
    let below = size - geometry.band_bottom;
    let max_height = size.saturating_sub(geometry.band_bottom + 1 + margin);
    let fit = fit_text(face, body_candidates, body_target, max_width, max_height);
    let top = geometry.band_bottom + (below - fit.height).div_ceil(2);
    draw_text(&mut pixels, size, face, body, &fit, top, Ink::Paint(ink));
    pixels
}

fn all_dates() -> Vec<String> {
    (1..=31).map(|day: u8| day.to_string()).collect()
}

fn render_date_tile(size: usize, face: Face, style: Style, day: u8) -> Vec<u8> {
    render_tile(
        size,
        face,
        style,
        None,
        &day.clamp(1, 31).to_string(),
        &all_dates(),
        digit_height_target(size),
    )
}

/// The undated application icon: the coral tile with a darker band and a
/// 2×2 grid, three squares at 55 % white and today's square solid white.
/// Geometry follows `assets/icons/canvas/lightcal-app.svg`, snapped to whole
/// pixels and kept symmetric at every size.
pub fn render_app_icon(size: usize) -> Vec<u8> {
    let scale = size as f32 / 64.0;
    let inset = (4.0 * scale).round().max(1.0);
    let tile = [inset, inset, size as f32 - inset, size as f32 - inset];
    let tile_size = (tile[2] - tile[0]) as usize;
    let radius = 14.0 * scale;
    let cell = ((12.0 * scale).round() as usize).max(2);
    let mut gap = ((4.0 * scale).round() as usize).max(1);
    // Equal clear space on both sides of the grid needs an even remainder.
    if (tile_size - 2 * cell - gap) % 2 == 1 {
        gap += 1;
    }
    let grid = 2 * cell + gap;
    let mut band = (18.0 * scale).round() as usize;
    if (size - inset as usize - band - grid) % 2 == 1 {
        band += 1;
    }
    let left = (size - grid) / 2;
    let top = band + (size - inset as usize - band - grid) / 2;
    let cell_radius = cell as f32 * 0.25;
    let squares: Vec<([f32; 4], f32)> = (0..4)
        .map(|i| {
            let x = (left + (i % 2) * (cell + gap)) as f32;
            let y = (top + (i / 2) * (cell + gap)) as f32;
            let alpha = if i == 3 { 1.0 } else { APP_GRID_DIM_ALPHA };
            ([x, y, x + cell as f32, y + cell as f32], alpha)
        })
        .collect();
    let band = band as f32;
    paint(size, 4, |x, y| {
        if rounded_rect_distance(x, y, tile, radius) > 0.0 {
            return [0.0; 4];
        }
        let mut color = opaque(if y < band { CORAL_BAND } else { CORAL });
        for (rect, alpha) in &squares {
            if rounded_rect_distance(x, y, *rect, cell_radius) <= 0.0 {
                color = over(color, WHITE, *alpha);
            }
        }
        color
    })
}

/// Renders a 32×32 RGBA tray icon for `DateOnly`.
pub fn render_tray_icon_rgba(day_of_month: u8, dark_taskbar: bool) -> Vec<u8> {
    render_tray_icon_variant_rgba(
        TrayIconMode::DateOnly,
        false,
        day_of_month,
        Weekday::Thursday,
        (22, 1),
        dark_taskbar,
    )
}

thread_local! {
    static SMALL_ICON_SIZE: std::cell::Cell<Option<u32>> = const { std::cell::Cell::new(None) };
}

/// The shell's small-icon size in physical pixels (16..=64), at the taskbar's DPI. It is
/// asked once and remembered until `forget_small_icon_size`, so the tray and window icons
/// agree and a UI sync does not query the shell each time.
pub fn native_small_icon_size() -> u32 {
    SMALL_ICON_SIZE.with(|size| {
        let known = size.get().unwrap_or_else(query_small_icon_size);
        size.set(Some(known));
        known
    })
}

/// Makes the next `native_small_icon_size` ask the shell again, after a DPI, display or
/// system setting change.
pub fn forget_small_icon_size() {
    SMALL_ICON_SIZE.with(|size| size.set(None));
}

fn query_small_icon_size() -> u32 {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::{
            HiDpi::{GetDpiForSystem, GetDpiForWindow, GetSystemMetricsForDpi},
            WindowsAndMessaging::{FindWindowW, SM_CXSMICON},
        };
        // Explorer owns the destination. Its DPI can differ from the app's
        // current monitor and must determine the raster's physical dimensions.
        let class: Vec<u16> = "Shell_TrayWnd\0".encode_utf16().collect();
        let taskbar = FindWindowW(class.as_ptr(), std::ptr::null());
        let taskbar_dpi = if taskbar.is_null() {
            0
        } else {
            GetDpiForWindow(taskbar)
        };
        let dpi = if taskbar_dpi == 0 {
            GetDpiForSystem()
        } else {
            taskbar_dpi
        };
        GetSystemMetricsForDpi(SM_CXSMICON, dpi).clamp(16, 64) as u32
    }
    #[cfg(not(windows))]
    32
}

/// The shell's theme is independent of the selected application theme.
pub fn taskbar_prefers_dark_mode(fallback: bool) -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Registry::{
            RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD,
        };
        let key: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\0"
            .encode_utf16()
            .collect();
        let name: Vec<u16> = "SystemUsesLightTheme\0".encode_utf16().collect();
        let mut value: u32 = 0;
        let mut length = std::mem::size_of::<u32>() as u32;
        let result = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                (&mut value as *mut u32).cast(),
                &mut length,
            )
        };
        if result == 0 {
            return value == 0;
        }
    }
    fallback
}

/// Renders a 32×32 tray icon in any mode.
pub fn render_tray_icon_variant_rgba(
    mode: TrayIconMode,
    monochrome: bool,
    day_of_month: u8,
    weekday: Weekday,
    hour_min: (u8, u8),
    dark_taskbar: bool,
) -> Vec<u8> {
    render_squircle_icon_generic(
        32,
        mode,
        monochrome,
        day_of_month,
        weekday,
        hour_min,
        dark_taskbar,
    )
}

/// Renders a 64×64 icon, used for the settings previews and UI snapshots.
pub fn render_squircle_icon_rgba_64(
    mode: TrayIconMode,
    monochrome: bool,
    day_of_month: u8,
    weekday: Weekday,
    hour_min: (u8, u8),
    dark_taskbar: bool,
) -> Vec<u8> {
    render_squircle_icon_generic(
        64,
        mode,
        monochrome,
        day_of_month,
        weekday,
        hour_min,
        dark_taskbar,
    )
}

/// Renders the tile at an exact square size (16..=256). `day_of_month` 0
/// renders the undated application icon. `dark_taskbar` selects Mono ink.
pub fn render_squircle_icon_generic(
    size: usize,
    mode: TrayIconMode,
    monochrome: bool,
    day_of_month: u8,
    weekday: Weekday,
    hour_min: (u8, u8),
    dark_taskbar: bool,
) -> Vec<u8> {
    render_with_face(
        Face::current(),
        size,
        mode,
        monochrome,
        day_of_month,
        weekday,
        hour_min,
        dark_taskbar,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_with_face(
    face: Face,
    size: usize,
    mode: TrayIconMode,
    monochrome: bool,
    day_of_month: u8,
    weekday: Weekday,
    hour_min: (u8, u8),
    dark_taskbar: bool,
) -> Vec<u8> {
    if day_of_month == 0 {
        return render_app_icon(size);
    }
    let style = Style::new(monochrome, dark_taskbar);
    let day = day_of_month.clamp(1, 31).to_string();
    match mode {
        TrayIconMode::DateOnly => render_date_tile(size, face, style, day_of_month),
        TrayIconMode::WeekdayAndDate => {
            let codes: Vec<String> = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"]
                .map(String::from)
                .to_vec();
            let code = &codes[weekday as usize % 7];
            render_tile(
                size,
                face,
                style,
                Some((code, &codes)),
                &day,
                &all_dates(),
                (size as f32 * 0.34).round() as usize,
            )
        }
        TrayIconMode::TimeAndDate => {
            let time = format!("{:02}:{:02}", hour_min.0.min(23), hour_min.1.min(59));
            let times: Vec<String> = ["00:00", "08:48", "23:59", "20:20"]
                .map(String::from)
                .to_vec();
            render_tile(
                size,
                face,
                style,
                Some((&day, &all_dates())),
                &time,
                &times,
                (size as f32 * 0.3).round() as usize,
            )
        }
    }
}

fn blend_over(dst: &mut [u8], rgb: [u8; 3], coverage: u32) {
    let src_a = coverage.min(255);
    if src_a == 0 {
        return;
    }
    let dst_a = dst[3] as u32;
    let dst_contrib = dst_a * (255 - src_a) / 255;
    let out_a = src_a + dst_contrib;
    for channel in 0..3 {
        dst[channel] = ((rgb[channel] as u32 * src_a + dst[channel] as u32 * dst_contrib) / out_a)
            .min(255) as u8;
    }
    dst[3] = out_a.min(255) as u8;
}

/// Encodes an array of `(width, height, png_bytes)` into a valid Windows `.ico` byte buffer.
pub fn encode_ico_from_pngs(entries: &[(u32, u32, &[u8])]) -> Vec<u8> {
    let count = entries.len() as u16;
    let header_len = 6usize;
    let dir_entry_len = 16usize;
    let total_header_len = header_len + dir_entry_len * (count as usize);

    let mut out = Vec::new();
    // ICONDIR
    out.extend_from_slice(&0u16.to_le_bytes()); // Reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // Type = 1 (Icon)
    out.extend_from_slice(&count.to_le_bytes());

    let mut offset = total_header_len as u32;
    for &(w, h, png) in entries {
        let width_byte = if w >= 256 { 0 } else { w as u8 };
        let height_byte = if h >= 256 { 0 } else { h as u8 };
        out.push(width_byte);
        out.push(height_byte);
        out.push(0); // Color count
        out.push(0); // Reserved
        out.extend_from_slice(&1u16.to_le_bytes()); // Planes
        out.extend_from_slice(&32u16.to_le_bytes()); // Bit count
        out.extend_from_slice(&(png.len() as u32).to_le_bytes()); // Bytes in res
        out.extend_from_slice(&offset.to_le_bytes()); // Image offset
        offset += png.len() as u32;
    }

    for &(_, _, png) in entries {
        out.extend_from_slice(png);
    }
    out
}

const ICO_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

/// Generates a multi-size dated `.ico` (16, 24, 32, 48, 64, 128, 256).
pub fn generate_multi_size_ico(anchor: LocalDate) -> Vec<u8> {
    generate_ico(|size| render_date_tile(size, Face::current(), Style::Color, anchor.day))
}

/// Generates the undated application `.ico` (16, 24, 32, 48, 64, 128, 256).
pub fn generate_application_ico() -> Vec<u8> {
    generate_ico(render_app_icon)
}

fn generate_ico(render: impl Fn(usize) -> Vec<u8>) -> Vec<u8> {
    let png_buffers: Vec<(u32, Vec<u8>)> = ICO_SIZES
        .iter()
        .map(|&size| {
            (
                size,
                encode_raw_rgba_to_png(size, size, &render(size as usize)),
            )
        })
        .collect();
    let entries: Vec<(u32, u32, &[u8])> = png_buffers
        .iter()
        .map(|(size, png)| (*size, *size, png.as_slice()))
        .collect();
    encode_ico_from_pngs(&entries)
}

/// Writes review PNGs of every tray size, style and shell for the dates
/// 1, 8, 11, 28 and 31, plus the application icon sizes, into `dir`.
/// Returns the files written.
pub fn export_review_set(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    const TRAY_SIZES: [usize; 7] = [16, 20, 24, 32, 40, 48, 64];
    const DATES: [u8; 5] = [1, 8, 11, 28, 31];
    // Approximate Windows 11 taskbar surfaces.
    const LIGHT_SHELL: [u8; 3] = [0xEE, 0xEE, 0xEE];
    const DARK_SHELL: [u8; 3] = [0x1C, 0x1C, 0x1C];
    let face = Face::current();
    let mut written = Vec::new();
    std::fs::create_dir_all(dir.join("tray"))?;
    std::fs::create_dir_all(dir.join("app"))?;
    let mut write = |path: PathBuf, size: (usize, usize), rgba: &[u8]| -> std::io::Result<()> {
        std::fs::write(
            &path,
            encode_raw_rgba_to_png(size.0 as u32, size.1 as u32, rgba),
        )?;
        written.push(path);
        Ok(())
    };
    let variants = [
        ("color", false, false, LIGHT_SHELL, "light"),
        ("mono", true, false, LIGHT_SHELL, "light"),
        ("color", false, true, DARK_SHELL, "dark"),
        ("mono", true, true, DARK_SHELL, "dark"),
    ];
    let mut actual_rows = Vec::new();
    for size in TRAY_SIZES {
        let mut zoom_rows = Vec::new();
        for (style_name, monochrome, dark, _, shell) in variants {
            let mut row = Vec::new();
            for day in DATES {
                let rgba = render_date_tile(size, face, Style::new(monochrome, dark), day);
                write(
                    dir.join(format!("tray/{style_name}-{shell}-{size}px-{day:02}.png")),
                    (size, size),
                    &rgba,
                )?;
                row.push((size, rgba));
            }
            zoom_rows.push(row);
        }
        let zoom = (192 / size).max(2);
        let backgrounds: Vec<[u8; 3]> = variants.iter().map(|v| v.3).collect();
        let (sheet, sheet_size) = contact_sheet(&zoom_rows, &backgrounds, zoom);
        write(
            dir.join(format!("tray-zoom-{size}px.png")),
            sheet_size,
            &sheet,
        )?;
        actual_rows.push(zoom_rows);
    }
    // One sheet at actual size: a row per style and shell, every size left to right.
    let mut rows: Vec<Vec<(usize, Vec<u8>)>> = vec![Vec::new(); variants.len()];
    for per_size in actual_rows {
        for (row, images) in rows.iter_mut().zip(per_size) {
            row.extend(images);
        }
    }
    let backgrounds: Vec<[u8; 3]> = variants.iter().map(|v| v.3).collect();
    let (sheet, sheet_size) = contact_sheet(&rows, &backgrounds, 1);
    write(dir.join("tray-actual.png"), sheet_size, &sheet)?;
    let (sheet, sheet_size) = contact_sheet(&rows, &backgrounds, 2);
    write(dir.join("tray-actual-2x.png"), sheet_size, &sheet)?;

    let mut app_row = Vec::new();
    for size in ICO_SIZES {
        let rgba = render_app_icon(size as usize);
        write(
            dir.join(format!("app/lightcal-{size}px.png")),
            (size as usize, size as usize),
            &rgba,
        )?;
        app_row.push((size as usize, rgba));
    }
    let app_rows = vec![app_row.clone(), app_row];
    let (sheet, sheet_size) = contact_sheet(&app_rows, &[LIGHT_SHELL, DARK_SHELL], 1);
    write(dir.join("app-actual.png"), sheet_size, &sheet)?;
    let small: Vec<Vec<(usize, Vec<u8>)>> = app_rows
        .iter()
        .map(|row| row.iter().filter(|(s, _)| *s <= 48).cloned().collect())
        .collect();
    let (sheet, sheet_size) = contact_sheet(&small, &[LIGHT_SHELL, DARK_SHELL], 6);
    write(dir.join("app-zoom-6x.png"), sheet_size, &sheet)?;
    Ok(written)
}

/// Lays out rows of square RGBA images, scaled by `zoom` with nearest
/// neighbour, over per-row opaque backgrounds.
fn contact_sheet(
    rows: &[Vec<(usize, Vec<u8>)>],
    backgrounds: &[[u8; 3]],
    zoom: usize,
) -> (Vec<u8>, (usize, usize)) {
    let pad = 8 * zoom.min(2);
    let row_height = |row: &Vec<(usize, Vec<u8>)>| {
        row.iter().map(|(s, _)| s * zoom).max().unwrap_or(0) + 2 * pad
    };
    let width = rows
        .iter()
        .map(|row| row.iter().map(|(s, _)| s * zoom + pad).sum::<usize>() + pad)
        .max()
        .unwrap_or(0);
    let height: usize = rows.iter().map(row_height).sum();
    let mut sheet = vec![0u8; width * height * 4];
    let mut y0 = 0;
    for (row, background) in rows.iter().zip(backgrounds) {
        let h = row_height(row);
        for y in y0..y0 + h {
            for x in 0..width {
                let index = (y * width + x) * 4;
                sheet[index..index + 3].copy_from_slice(background);
                sheet[index + 3] = 255;
            }
        }
        let mut x0 = pad;
        for (size, rgba) in row {
            let top = y0 + (h - size * zoom) / 2;
            for y in 0..size * zoom {
                for x in 0..size * zoom {
                    let src = ((y / zoom) * size + x / zoom) * 4;
                    let index = ((top + y) * width + x0 + x) * 4;
                    let pixel = [rgba[src], rgba[src + 1], rgba[src + 2]];
                    blend_over(&mut sheet[index..index + 4], pixel, rgba[src + 3] as u32);
                }
            }
            x0 += size * zoom + pad;
        }
        y0 += h;
    }
    (sheet, (width, height))
}

/// Helper encoding RGBA pixels into PNG with stored (uncompressed) DEFLATE blocks.
fn encode_raw_rgba_to_png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut raw_scanlines = Vec::with_capacity((width as usize * 4 + 1) * height as usize);
    for y in 0..height as usize {
        raw_scanlines.push(0u8);
        let row_start = y * (width as usize) * 4;
        raw_scanlines.extend_from_slice(&rgba[row_start..row_start + (width as usize) * 4]);
    }

    let mut zlib = Vec::with_capacity(raw_scanlines.len() + 64);
    zlib.push(0x78);
    zlib.push(0x01);

    let chunks: Vec<&[u8]> = raw_scanlines.chunks(65535).collect();
    for (i, chunk) in chunks.iter().enumerate() {
        let is_last = i + 1 == chunks.len();
        zlib.push(if is_last { 0x01 } else { 0x00 });
        let len = chunk.len() as u16;
        let nlen = !len;
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&nlen.to_le_bytes());
        zlib.extend_from_slice(chunk);
    }
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in &raw_scanlines {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    let adler = (b << 16) | a;
    zlib.extend_from_slice(&adler.to_be_bytes());

    let mut out = Vec::with_capacity(zlib.len() + 128);
    out.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8);
    ihdr.push(6);
    ihdr.push(0);
    ihdr.push(0);
    ihdr.push(0);
    append_chunk(&mut out, b"IHDR", &ihdr);
    append_chunk(&mut out, b"IDAT", &zlib);
    append_chunk(&mut out, b"IEND", &[]);
    out
}

fn append_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(chunk_type);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(chunk_type);
    crc_input.extend_from_slice(data);
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in &crc_input {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    out.extend_from_slice(&(!crc).to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRAY_SIZES: [usize; 7] = [16, 20, 24, 32, 40, 48, 64];

    #[test]
    fn small_icon_size_is_remembered_until_forgotten() {
        let shell = query_small_icon_size();
        assert!((16..=64).contains(&shell));
        // A remembered size is returned without asking the shell again.
        SMALL_ICON_SIZE.with(|size| size.set(Some(99)));
        assert_eq!(native_small_icon_size(), 99);
        assert_eq!(native_small_icon_size(), 99);
        forget_small_icon_size();
        assert_eq!(native_small_icon_size(), shell);
    }

    /// The system face when this machine has one, and always the fallback.
    fn faces() -> Vec<Face> {
        let mut faces = vec![Face::Strokes];
        if let Some(glyphs) = system_glyphs() {
            faces.push(Face::System(glyphs));
        }
        faces
    }

    fn pixel(rgba: &[u8], size: usize, x: usize, y: usize) -> [u8; 4] {
        let i = (y * size + x) * 4;
        [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
    }

    #[test]
    fn color_tile_fills_the_raster_with_a_band_and_transparent_corners() {
        for size in TRAY_SIZES {
            let geometry = TileGeometry::new(size, DATE_BAND_RATIO);
            let rgba = render_date_tile(size, Face::Strokes, Style::Color, 1);
            assert_eq!(pixel(&rgba, size, 0, 0)[3], 0, "{size}px corner");
            assert_eq!(pixel(&rgba, size, size - 1, size - 1)[3], 0);
            // Edges reach the raster; no unused outer rows or columns.
            assert_eq!(pixel(&rgba, size, size / 2, 0), [0xA9, 0x2A, 0x31, 255]);
            assert_eq!(pixel(&rgba, size, 0, size / 2), [0xD7, 0x38, 0x40, 255]);
            assert_eq!(pixel(&rgba, size, size - 1, size / 2)[3], 255);
            assert_eq!(
                pixel(&rgba, size, size / 2, size - 1),
                [0xD7, 0x38, 0x40, 255]
            );
            // The band ends on a whole pixel.
            let band = geometry.band_bottom;
            assert_eq!(pixel(&rgba, size, 1, band - 1)[..3], CORAL_BAND);
            assert_eq!(pixel(&rgba, size, 1, band)[..3], CORAL);
        }
    }

    #[test]
    fn mono_tile_is_an_outline_in_the_shell_ink() {
        for (dark, ink) in [(false, INK_ON_LIGHT_SHELL), (true, INK_ON_DARK_SHELL)] {
            for size in TRAY_SIZES {
                let geometry = TileGeometry::new(size, DATE_BAND_RATIO);
                let rgba = paint_tile(geometry, Style::new(true, dark));
                assert_eq!(pixel(&rgba, size, 0, 0)[3], 0);
                assert_eq!(pixel(&rgba, size, size / 2, 1)[..3], ink);
                assert_eq!(
                    pixel(&rgba, size, 0, size / 2),
                    [ink[0], ink[1], ink[2], 255]
                );
                assert_eq!(pixel(&rgba, size, size / 2, size - 1)[3], 255);
                assert_eq!(
                    pixel(&rgba, size, size / 2, size / 2)[3],
                    0,
                    "hollow at {size}px"
                );
                assert_eq!(pixel(&rgba, size, geometry.stroke, size / 2)[3], 0);
            }
        }
    }

    /// Bounds of numeral pixels: white on the coral tile.
    fn numeral_bounds(rgba: &[u8], size: usize) -> (usize, usize, usize, usize) {
        let mut bounds = (size, size, 0, 0);
        for (i, p) in rgba.as_chunks::<4>().0.iter().enumerate() {
            if p[3] == 255 && p[1] > 0x90 {
                let (x, y) = (i % size, i / size);
                bounds = (
                    bounds.0.min(x),
                    bounds.1.min(y),
                    bounds.2.max(x),
                    bounds.3.max(y),
                );
            }
        }
        bounds
    }

    #[test]
    fn every_date_fits_and_centres_at_native_shell_sizes() {
        for face in faces() {
            for size in TRAY_SIZES {
                let geometry = TileGeometry::new(size, DATE_BAND_RATIO);
                let margin = geometry.margin();
                for day in 1..=31 {
                    let rgba = render_date_tile(size, face, Style::Color, day);
                    let (left, top, right, bottom) = numeral_bounds(&rgba, size);
                    assert!(left <= right, "date {day} missing at {size}px");
                    assert!(
                        left >= margin && right < size - margin,
                        "date {day} clips at {size}px: {left}..{right}"
                    );
                    assert!(
                        top > geometry.band_bottom && bottom < size - margin,
                        "date {day} touches the band or edge at {size}px: {top}..{bottom}"
                    );
                    assert!(
                        // Fractional edge coverage can differ by up to two pixels (ADR 0010).
                        left.abs_diff(size - 1 - right) <= 2,
                        "date {day} is off-centre at {size}px: {left}..{right}"
                    );
                    let above = top - geometry.band_bottom;
                    let below = size - 1 - bottom;
                    assert!(
                        above.abs_diff(below) <= 2,
                        "date {day} is vertically off-centre at {size}px: {above} vs {below}"
                    );
                }
            }
        }
    }

    #[test]
    fn one_numeral_size_per_raster_size() {
        // Flat-topped, flat-bottomed digits show the shared cap height directly.
        for face in faces() {
            for size in TRAY_SIZES {
                let rows: Vec<(usize, usize)> = [1u8, 4, 7, 11, 14, 17]
                    .iter()
                    .map(|&day| {
                        let b =
                            numeral_bounds(&render_date_tile(size, face, Style::Color, day), size);
                        (b.1, b.3)
                    })
                    .collect();
                // Baselines are identical; the `1` flag may rise one row above `4`/`7`.
                assert!(
                    rows.windows(2)
                        .all(|w| w[0].1 == w[1].1 && w[0].0.abs_diff(w[1].0) <= 1),
                    "numeral rows differ at {size}px: {rows:?}"
                );
            }
        }
    }

    #[test]
    fn mono_numeral_uses_the_shell_ink() {
        for face in faces() {
            for (dark, ink) in [(false, INK_ON_LIGHT_SHELL), (true, INK_ON_DARK_SHELL)] {
                let rgba = render_date_tile(32, face, Style::new(true, dark), 28);
                let inner = rgba.as_chunks::<4>().0.iter().enumerate().filter(|(i, p)| {
                    let (x, y) = (i % 32, i / 32);
                    (4..28).contains(&x) && (8..28).contains(&y) && p[3] == 255
                });
                let mut count = 0;
                for (_, p) in inner {
                    assert_eq!(p[..3], ink);
                    count += 1;
                }
                assert!(count > 10, "numeral missing in Mono");
            }
        }
    }

    #[test]
    fn app_icon_matches_the_vector_master() {
        let rgba = render_app_icon(256);
        let at = |x: usize, y: usize| pixel(&rgba, 256, x, y);
        assert_eq!(at(0, 0)[3], 0);
        assert_eq!(at(8, 128)[3], 0); // 4/64 transparent inset.
        assert_eq!(at(128, 20), [0xA9, 0x2A, 0x31, 255]);
        assert_eq!(at(20, 128), [0xD7, 0x38, 0x40, 255]);
        // Today's square is solid; the other three are 55 % white.
        assert_eq!(at(160, 188), [255, 255, 255, 255]);
        let dim = at(96, 124);
        assert_eq!(dim[3], 255);
        assert!(
            dim[0].abs_diff(237) <= 2 && dim[1].abs_diff(165) <= 2,
            "{dim:?}"
        );
        for size in ICO_SIZES {
            let rgba = render_app_icon(size as usize);
            assert_eq!(rgba.len(), (size * size * 4) as usize);
            assert_eq!(rgba[3], 0);
        }
    }

    #[test]
    fn missing_system_font_falls_back_to_strokes() {
        assert!(extract_glyphs(b"not a font", &[], "test").is_none());
        let rgba = render_date_tile(16, Face::Strokes, Style::Color, 28);
        let (left, _, right, _) = numeral_bounds(&rgba, 16);
        assert!(left < right);
    }

    #[cfg(windows)]
    #[test]
    fn segoe_is_read_from_the_windows_fonts_folder() {
        let Some(fonts) = super::super::system_fonts_dir() else {
            return;
        };
        if fonts.join("SegUIVar.ttf").exists() || fonts.join("segoeuib.ttf").exists() {
            assert!(numeral_font_source().starts_with("Segoe UI"));
        }
    }

    #[test]
    fn tray_icon_renders_valid_32x32_rgba_buffer() {
        let dark = render_tray_icon_rgba(29, true);
        let light = render_tray_icon_rgba(5, false);
        assert_eq!(dark.len(), 32 * 32 * 4);
        assert_eq!(light.len(), 32 * 32 * 4);
        assert_ne!(dark, light);
    }

    #[test]
    fn tray_icon_variants_render_valid_rgba() {
        for monochrome in [false, true] {
            for mode in [
                TrayIconMode::DateOnly,
                TrayIconMode::WeekdayAndDate,
                TrayIconMode::TimeAndDate,
            ] {
                for size in TRAY_SIZES {
                    for face in faces() {
                        let rgba = render_with_face(
                            face,
                            size,
                            mode,
                            monochrome,
                            29,
                            Weekday::Thursday,
                            (22, 1),
                            true,
                        );
                        assert_eq!(rgba.len(), size * size * 4);
                    }
                }
            }
        }
    }

    #[test]
    fn multi_size_ico_generates_valid_headers() {
        for ico in [
            generate_multi_size_ico(LocalDate::new(2026, 10, 29).unwrap()),
            generate_application_ico(),
        ] {
            assert_eq!(&ico[0..4], &[0, 0, 1, 0]); // ICO signature
            let count = u16::from_le_bytes([ico[4], ico[5]]);
            assert_eq!(count, 7);
            let widths: Vec<u8> = (0..7).map(|i| ico[6 + i * 16]).collect();
            assert_eq!(widths, [16, 24, 32, 48, 64, 128, 0]);
        }
    }
}
