//! Native-size calendar date badges with pixel-fitted Onest numerals.
//! The calendar silhouette remains recognizable beside other Windows tray icons.

use crate::preferences::TrayIconMode;
use calendar_core::{LocalDate, Weekday};
use fontdue::{Font, FontSettings};
use std::sync::OnceLock;

static UI_FONT: OnceLock<Font> = OnceLock::new();
static UI_BOLD_FONT: OnceLock<Font> = OnceLock::new();

fn get_bold_font() -> &'static Font {
    UI_BOLD_FONT.get_or_init(|| {
        Font::from_bytes(
            include_bytes!("../../../assets/fonts/Onest-Bold.ttf").as_slice(),
            FontSettings::default(),
        )
        .expect("Failed to parse bundled Onest Bold font")
    })
}

pub fn get_font() -> &'static Font {
    UI_FONT.get_or_init(|| {
        let bytes = include_bytes!("../../../assets/fonts/Onest-Regular.ttf");
        Font::from_bytes(bytes.as_slice(), FontSettings::default())
            .expect("Failed to parse bundled Onest font")
    })
}

/// Renders a 32×32 RGBA system tray icon supporting 3 modes (`DateOnly`, `WeekdayAndDate`, `TimeAndDate`)
/// and both coral and system-monochrome styles.
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

/// Render at the notification area's native pixel size to avoid Windows resampling text.
pub fn render_native_tray_icon(
    mode: TrayIconMode,
    monochrome: bool,
    day: u8,
    weekday: Weekday,
    time: (u8, u8),
    dark: bool,
) -> (Vec<u8>, u32) {
    #[cfg(windows)]
    let size = unsafe {
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
    };
    #[cfg(not(windows))]
    let size = 32;
    (
        render_squircle_icon_generic(
            size as usize,
            mode,
            monochrome,
            day,
            weekday,
            time,
            taskbar_prefers_dark_mode(dark),
        ),
        size,
    )
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

fn render_date_calendar(size: usize, day: u8, monochrome: bool, dark: bool) -> Vec<u8> {
    let mut pixels = vec![0; size * size * 4];
    let side = size as f32;
    // The shell already owns spacing around the icon. Use the full native
    // canvas; transparent rounded corners provide the optical breathing room.
    let inset = 0.0;
    let stroke = (side / 16.0).max(1.0);
    let radius = (side * 0.16).max(2.0);
    let cap_bottom = inset + (side * 0.125).max(2.0);
    let bottom = side - inset;
    let inverse = monochrome && dark;
    let paper = if inverse {
        [32, 32, 32]
    } else {
        [255, 255, 255]
    };
    let outline = if inverse {
        [180, 180, 180]
    } else {
        [155, 155, 155]
    };
    let cap = if monochrome {
        if dark {
            [238, 238, 238]
        } else {
            [49, 49, 49]
        }
    } else {
        [229, 72, 77]
    };
    // Only the silhouette is subpixel sampled. Numerals are rasterized directly
    // at the shell's native size; downsampling would blur their thin strokes.
    let half = side * 0.5;
    let rect_half = half - inset - radius;
    for y in 0..size {
        for x in 0..size {
            let mut rgb = [0u32; 3];
            let mut covered = 0u32;
            for sy in 0..4 {
                for sx in 0..4 {
                    let fx = x as f32 + (sx as f32 + 0.5) / 4.0;
                    let fy = y as f32 + (sy as f32 + 0.5) / 4.0;
                    let qx = (fx - half).abs() - rect_half;
                    let qy = (fy - half).abs() - rect_half;
                    let distance =
                        qx.max(0.0).hypot(qy.max(0.0)) + qx.min(0.0).max(qy.min(0.0)) - radius;
                    if distance > 0.0 {
                        continue;
                    }
                    let color = if fy < cap_bottom {
                        cap
                    } else if distance > -stroke {
                        outline
                    } else {
                        paper
                    };
                    for i in 0..3 {
                        rgb[i] += color[i] as u32;
                    }
                    covered += 1;
                }
            }
            if covered > 0 {
                let index = (y * size + x) * 4;
                for i in 0..3 {
                    pixels[index + i] = rgb[i].checked_div(covered).unwrap_or_default() as u8;
                }
                pixels[index + 3] = (covered * 255 / 16) as u8;
            }
        }
    }
    let font = get_bold_font();
    let available_width = (side - 2.0 * (inset + stroke)).max(1.0);
    let available_height = (bottom - cap_bottom - 2.0 * stroke).max(1.0);
    // Fit the entire month at one size: date changes must not change cap height.
    let em = 1024.0;
    let mut widest = 1.0f32;
    let mut tallest = 1.0f32;
    for date in 1..=31 {
        let mut advance = 0.0;
        let mut left = f32::MAX;
        let mut right = f32::MIN;
        for character in date.to_string().chars() {
            let metrics = font.metrics(character, em);
            left = left.min(advance + metrics.xmin as f32);
            right = right.max(advance + metrics.xmin as f32 + metrics.width as f32);
            tallest = tallest.max(metrics.height as f32);
            advance += metrics.advance_width;
        }
        widest = widest.max(right - left);
    }
    let mut font_size = em * (available_width / widest).min(available_height / tallest);
    // Rasterization rounds each glyph independently. Recheck actual pixel
    // metrics so wide dates such as 30 retain a transparent edge.
    loop {
        let fits = (1..=31).all(|date| {
            let mut advance = 0.0;
            let mut left = f32::MAX;
            let mut right = f32::MIN;
            let mut height = 0;
            for character in date.to_string().chars() {
                let metrics = font.metrics(character, font_size);
                left = left.min(advance + metrics.xmin as f32);
                right = right.max(advance + metrics.xmin as f32 + metrics.width as f32);
                height = height.max(metrics.height);
                advance += metrics.advance_width;
            }
            (right - left).ceil() <= available_width && height as f32 <= available_height
        });
        if fits || font_size <= 1.0 {
            break;
        }
        font_size -= 0.125;
    }
    let ink = if inverse {
        [238, 238, 238, 255]
    } else {
        [20, 20, 20, 255]
    };
    let mut date_ink = vec![0; pixels.len()];
    draw_text_centered(
        &mut date_ink,
        size,
        size,
        font,
        &day.clamp(1, 31).to_string(),
        font_size,
        side * 0.5,
        (cap_bottom + bottom) * 0.5,
        ink,
    );
    // Font metrics can include empty columns. Align the visible raster,
    // including for narrow single digits, to the calendar page.
    let mut bounds = (size, size, 0usize, 0usize);
    for (i, pixel) in date_ink.as_chunks::<4>().0.iter().enumerate() {
        if pixel[3] > 0 {
            bounds.0 = bounds.0.min(i % size);
            bounds.1 = bounds.1.min(i / size);
            bounds.2 = bounds.2.max(i % size);
            bounds.3 = bounds.3.max(i / size);
        }
    }
    if bounds.0 < size {
        let dx = ((side - 1.0 - (bounds.0 + bounds.2) as f32) * 0.5).round() as isize;
        let dy =
            ((cap_bottom + bottom - 1.0 - (bounds.1 + bounds.3) as f32) * 0.5).round() as isize;
        for (i, pixel) in date_ink.as_chunks::<4>().0.iter().enumerate() {
            let x = (i % size) as isize + dx;
            let y = (i / size) as isize + dy;
            if x >= 0 && y >= 0 && x < size as isize && y < size as isize {
                let index = (y as usize * size + x as usize) * 4;
                blend_over(&mut pixels[index..index + 4], ink, pixel[3] as u32);
            }
        }
    }
    pixels
}

/// Renders date glyphs directly at 32×32. Legacy modes use a box-filtered 64×64 master.
pub fn render_tray_icon_variant_rgba(
    mode: TrayIconMode,
    monochrome: bool,
    day_of_month: u8,
    weekday: Weekday,
    hour_min: (u8, u8),
    dark_taskbar: bool,
) -> Vec<u8> {
    if mode == TrayIconMode::DateOnly && day_of_month != 0 {
        return render_date_calendar(32, day_of_month, monochrome, dark_taskbar);
    }
    let hi = render_squircle_icon_rgba_64(
        mode,
        monochrome,
        day_of_month,
        weekday,
        hour_min,
        dark_taskbar,
    );
    let mut out = vec![0u8; 32 * 32 * 4];
    for y in 0..32usize {
        for x in 0..32usize {
            let mut r = 0u32;
            let mut g = 0u32;
            let mut b = 0u32;
            let mut a = 0u32;
            for dy in 0..2usize {
                for dx in 0..2usize {
                    let src_idx = ((y * 2 + dy) * 64 + (x * 2 + dx)) * 4;
                    let sa = hi[src_idx + 3] as u32;
                    r += hi[src_idx] as u32 * sa;
                    g += hi[src_idx + 1] as u32 * sa;
                    b += hi[src_idx + 2] as u32 * sa;
                    a += sa;
                }
            }
            let dst_idx = (y * 32 + x) * 4;
            if let Some(div_a) = a.checked_div(4) {
                if div_a > 0 {
                    out[dst_idx] = (r / a).min(255) as u8;
                    out[dst_idx + 1] = (g / a).min(255) as u8;
                    out[dst_idx + 2] = (b / a).min(255) as u8;
                    out[dst_idx + 3] = div_a.min(255) as u8;
                }
            }
        }
    }
    out
}

/// Renders a 64×64 anti-aliased RGBA squircle icon with high-quality font glyph rendering.
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

/// Generic squircle icon renderer for arbitrary square dimensions (16..=256).
pub fn render_squircle_icon_generic(
    size: usize,
    mode: TrayIconMode,
    monochrome: bool,
    day_of_month: u8,
    weekday: Weekday,
    hour_min: (u8, u8),
    dark_taskbar: bool,
) -> Vec<u8> {
    if mode == TrayIconMode::DateOnly && day_of_month != 0 {
        return render_date_calendar(size, day_of_month, monochrome, dark_taskbar);
    }
    let w = size;
    let h = size;
    let mut buf = vec![0u8; w * h * 4];

    // Colored calendars retain a white page and dark ink in both app themes.
    let dark_page = monochrome && dark_taskbar;
    let bg = if dark_page {
        [24u8, 24, 28, 255] // #18181C matte dark surface
    } else {
        [255u8, 255, 255, 255] // #FFFFFF crisp light surface
    };
    let fg = if dark_page {
        [244u8, 244, 246, 255] // #EEEEEE primary ink
    } else {
        [20u8, 20, 22, 255] // #141416 primary ink
    };
    let top_band = if monochrome {
        if dark_taskbar {
            [212u8, 212, 216, 255] // #D4D4D8 monochrome dark-mode cap
        } else {
            [39u8, 39, 42, 255] // #27272A monochrome light-mode cap
        }
    } else {
        [229u8, 72, 77, 255] // #E5484D Signature Coral-Vermilion cap
    };
    let cap_fg = if monochrome && dark_taskbar {
        [24u8, 24, 27, 255] // #18181B dark text on silver cap
    } else {
        [255u8, 255, 255, 255] // #FFFFFF white text on coral/graphite cap
    };
    let border = if monochrome {
        if dark_taskbar {
            [161u8, 161, 170, 255]
        } else {
            [39u8, 39, 42, 255]
        }
    } else if dark_taskbar {
        [63u8, 63, 70, 255] // #3F3F46 quiet outline
    } else {
        [212u8, 212, 220, 255] // #D4D4DC quiet outline
    };

    let s = size as f32;
    let edge = (s * 1.5 / 64.0).max(0.25);
    let corner_rx = (s * 0.23).max(3.0);
    let header_bottom = (s * if mode == TrayIconMode::DateOnly {
        0.23
    } else {
        0.31
    })
    .round();

    let half = s * 0.5;
    let inset = corner_rx;
    let rect_half = half - inset;

    // Draw anti-aliased squircle using 2x2 subpixel sampling
    for y in 0..h {
        for x in 0..w {
            let mut r_acc = 0.0f32;
            let mut g_acc = 0.0f32;
            let mut b_acc = 0.0f32;
            let mut a_acc = 0.0f32;

            for sy in 0..2 {
                for sx in 0..2 {
                    let fx = x as f32 + (sx as f32 + 0.5) * 0.5;
                    let fy = y as f32 + (sy as f32 + 0.5) * 0.5;
                    let qx = (fx - half).abs() - rect_half;
                    let qy = (fy - half).abs() - rect_half;
                    let dist = qx.max(0.0).hypot(qy.max(0.0)) + qx.min(0.0).max(qy.min(0.0))
                        - (corner_rx - edge);
                    if dist <= 0.0 {
                        let c = if dist >= -edge
                            || (fy >= header_bottom - edge * 2.0 / 3.0
                                && fy < header_bottom + edge / 3.0)
                        {
                            border
                        } else if fy < header_bottom {
                            top_band
                        } else {
                            bg
                        };
                        r_acc += c[0] as f32;
                        g_acc += c[1] as f32;
                        b_acc += c[2] as f32;
                        a_acc += 255.0;
                    }
                }
            }

            if a_acc > 0.0 {
                let samples = a_acc / 255.0;
                let idx = (y * w + x) * 4;
                buf[idx] = (r_acc / samples) as u8;
                buf[idx + 1] = (g_acc / samples) as u8;
                buf[idx + 2] = (b_acc / samples) as u8;
                buf[idx + 3] = (a_acc / 4.0) as u8;
            }
        }
    }

    let font = get_bold_font();
    let clamped_day = day_of_month.clamp(1, 31);

    // Header content
    if mode == TrayIconMode::DateOnly {
        if size >= 48 {
            // Subtle horizontal calendar binder accent line inside cap
            let seam_y_start = (s * 0.16).round() as usize;
            let seam_y_end = (s * 0.19).round() as usize;
            let seam_x_start = (s * 0.28).round() as usize;
            let seam_x_end = (s * 0.72).round() as usize;
            for y in seam_y_start..=seam_y_end {
                for x in seam_x_start..seam_x_end {
                    blend_pixel(&mut buf, w, x, y, cap_fg);
                }
            }
        }
    } else {
        let code = match weekday {
            Weekday::Monday => "MON",
            Weekday::Tuesday => "TUE",
            Weekday::Wednesday => "WED",
            Weekday::Thursday => "THU",
            Weekday::Friday => "FRI",
            Weekday::Saturday => "SAT",
            Weekday::Sunday => "SUN",
        };
        let cap_font_size = (s * 0.16).clamp(7.0, 36.0);
        let cap_center_y = header_bottom * 0.52;
        draw_text_centered(
            &mut buf,
            w,
            h,
            font,
            code,
            cap_font_size,
            s * 0.5,
            cap_center_y,
            cap_fg,
        );
    }

    // Undated application branding for installed/pinned icons; tray dates remain dynamic.
    if day_of_month == 0 {
        let cell = (s * 0.12).round().max(1.0) as usize;
        for row in 0..2 {
            for col in 0..3 {
                let x0 = (s * (0.23 + col as f32 * 0.22)).round() as usize;
                let y0 = (s * (0.43 + row as f32 * 0.23)).round() as usize;
                for y in y0..(y0 + cell).min(h) {
                    for x in x0..(x0 + cell).min(w) {
                        blend_pixel(&mut buf, w, x, y, fg);
                    }
                }
            }
        }
        return buf;
    }

    if mode == TrayIconMode::TimeAndDate {
        let hr = hour_min.0.min(23);
        let mn = hour_min.1.min(59);
        let time_str = format!("{hr:02}:{mn:02}");
        let time_font_size = (s * 0.24).clamp(8.0, 48.0);
        let time_center_y = header_bottom + (s - header_bottom) * 0.30;
        draw_text_centered(
            &mut buf,
            w,
            h,
            font,
            &time_str,
            time_font_size,
            s * 0.5,
            time_center_y,
            fg,
        );

        let sub_fg = if dark_taskbar {
            [161u8, 161, 170, 255]
        } else {
            [100u8, 100, 108, 255]
        };
        let day_str = clamped_day.to_string();
        let sub_font_size = (s * 0.19).clamp(7.0, 36.0);
        let sub_center_y = header_bottom + (s - header_bottom) * 0.72;
        draw_text_centered(
            &mut buf,
            w,
            h,
            font,
            &day_str,
            sub_font_size,
            s * 0.5,
            sub_center_y,
            sub_fg,
        );
    } else {
        // Mode DateOnly or WeekdayAndDate: Big prominent date numeral
        let day_str = clamped_day.to_string();
        let num_font_size = (s * if mode == TrayIconMode::DateOnly {
            0.64
        } else {
            0.60
        })
        .max(10.0);
        let num_center_y = header_bottom + (s - header_bottom) * 0.50;
        draw_text_centered(
            &mut buf,
            w,
            h,
            font,
            &day_str,
            num_font_size,
            s * 0.5,
            num_center_y,
            fg,
        );
    }

    buf
}

fn blend_pixel(buf: &mut [u8], stride: usize, x: usize, y: usize, rgba: [u8; 4]) {
    if x < stride && y < stride {
        let idx = (y * stride + x) * 4;
        buf[idx..idx + 4].copy_from_slice(&rgba);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_text_centered(
    buf: &mut [u8],
    w: usize,
    h: usize,
    font: &Font,
    text: &str,
    font_size: f32,
    center_x: f32,
    center_y: f32,
    color: [u8; 4],
) {
    let mut total_advance = 0.0f32;
    let mut ink_left = f32::MAX;
    let mut ink_right = f32::MIN;
    let mut glyphs = Vec::with_capacity(text.len());
    let mut max_top = 0i32;
    let mut min_bottom = 0i32;

    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, font_size);
        let top = metrics.height as i32 + metrics.ymin;
        let bottom = metrics.ymin;
        max_top = max_top.max(top);
        min_bottom = min_bottom.min(bottom);
        ink_left = ink_left.min(total_advance + metrics.xmin as f32);
        ink_right = ink_right.max(total_advance + metrics.xmin as f32 + metrics.width as f32);
        glyphs.push((metrics, bitmap));
        total_advance += metrics.advance_width;
    }

    if glyphs.is_empty() {
        return;
    }
    let start_x = center_x - (ink_left + ink_right) * 0.5;
    // Align visual center of font glyphs with center_y
    let baseline_y = (center_y + (max_top + min_bottom) as f32 * 0.5).round() as i32;

    let mut cur_x = start_x;
    for (metrics, bitmap) in glyphs {
        let glyph_x = (cur_x + metrics.xmin as f32).round() as i32;
        let glyph_y = baseline_y - (metrics.height as i32 + metrics.ymin);

        for row in 0..metrics.height {
            let py = glyph_y + row as i32;
            if py < 0 || py >= h as i32 {
                continue;
            }
            for col in 0..metrics.width {
                let px = glyph_x + col as i32;
                if px < 0 || px >= w as i32 {
                    continue;
                }
                let cov = bitmap[row * metrics.width + col] as u32;
                if cov > 0 {
                    let idx = (py as usize * w + px as usize) * 4;
                    blend_over(&mut buf[idx..idx + 4], color, cov);
                }
            }
        }
        cur_x += metrics.advance_width;
    }
}

fn blend_over(dst: &mut [u8], src: [u8; 4], coverage: u32) {
    let src_a = (src[3] as u32 * coverage) / 255;
    if src_a == 0 {
        return;
    }
    let dst_a = dst[3] as u32;
    let inv = 255u32 - src_a;
    let dst_contrib = (dst_a * inv) / 255;
    let out_a = src_a + dst_contrib;
    if let Some(r) = (src[0] as u32 * src_a + dst[0] as u32 * dst_contrib).checked_div(out_a) {
        dst[0] = r.min(255) as u8;
        dst[1] = ((src[1] as u32 * src_a + dst[1] as u32 * dst_contrib) / out_a).min(255) as u8;
        dst[2] = ((src[2] as u32 * src_a + dst[2] as u32 * dst_contrib) / out_a).min(255) as u8;
        dst[3] = out_a.min(255) as u8;
    }
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

/// Generates a multi-size `.ico` for Lightcal (16, 24, 32, 48, 64, 128, 256).
pub fn generate_multi_size_ico(anchor: LocalDate) -> Vec<u8> {
    generate_ico(anchor.day, anchor.weekday())
}

pub fn generate_application_ico() -> Vec<u8> {
    generate_ico(0, Weekday::Monday)
}

fn generate_ico(day: u8, weekday: Weekday) -> Vec<u8> {
    let sizes = [16u32, 24, 32, 48, 64, 128, 256];
    let mut png_buffers = Vec::new();

    for &size in &sizes {
        let rgba = render_squircle_icon_generic(
            size as usize,
            TrayIconMode::DateOnly,
            false,
            day,
            weekday,
            (12, 0),
            false,
        );
        let png = encode_raw_rgba_to_png(size, size, &rgba);
        png_buffers.push((size, size, png));
    }

    let entries: Vec<(u32, u32, &[u8])> = png_buffers
        .iter()
        .map(|(w, h, png)| (*w, *h, png.as_slice()))
        .collect();
    encode_ico_from_pngs(&entries)
}

/// Helper encoding RGBA pixels into PNG using the uncompressed DEFLATE encoder from `snapshot`.
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

    #[test]
    fn date_badge_has_a_calendar_silhouette_in_both_shell_themes() {
        for (monochrome, dark) in [(false, false), (false, true), (true, false), (true, true)] {
            let rgba = render_squircle_icon_generic(
                32,
                TrayIconMode::DateOnly,
                monochrome,
                31,
                Weekday::Saturday,
                (12, 0),
                dark,
            );
            let pixels = rgba.as_chunks::<4>().0;
            // A continuous page, header and readable ink survive even when the
            // surrounding shell has the opposite theme to the application.
            assert_eq!(pixels[16 * 32 + 2][3], 255);
            assert_eq!(pixels[2 * 32 + 16][3], 255);
            assert_ne!(pixels[2 * 32 + 16][..3], pixels[16 * 32 + 2][..3]);
            assert!(pixels.iter().filter(|p| p[3] == 255).count() > 500);
            assert_eq!(pixels[0][3], 0); // Transparent rounded corner.
            assert_eq!(pixels[16][3], 255); // No unused top/bottom/side rows.
            assert_eq!(pixels[31 * 32 + 16][3], 255);
            assert_eq!(pixels[16 * 32][3], 255);
            assert_eq!(pixels[16 * 32 + 31][3], 255);
        }
    }

    #[test]
    fn every_date_fits_and_centers_at_native_shell_sizes() {
        for size in [16, 18, 20, 24, 32, 36, 40, 48, 64] {
            for day in 1..=31 {
                let rgba = render_squircle_icon_generic(
                    size,
                    TrayIconMode::DateOnly,
                    false,
                    day,
                    Weekday::Saturday,
                    (12, 0),
                    false,
                );
                let mut left = size;
                let mut right = 0;
                let mut top = size;
                let mut bottom = 0;
                for (i, _) in rgba
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p[0] <= 100 && p[1] <= 100 && p[2] <= 100 && p[3] > 160)
                {
                    let (x, y) = (i % size, i / size);
                    left = left.min(x);
                    right = right.max(x);
                    top = top.min(y);
                    bottom = bottom.max(y);
                }
                assert!(
                    left > 0 && top > 0 && right < size - 1 && bottom < size - 1,
                    "date {day} clips at {size}px: {left},{top}..{right},{bottom}"
                );
                assert!(
                    left.abs_diff(size - 1 - right) <= 2,
                    "date {day} is horizontally off-center at {size}px: {left}..{right}"
                );
                let cap_bottom = (size as f32 * 0.125).max(2.0);
                let page_bottom = size;
                assert!(
                    top as f32 > cap_bottom && bottom < page_bottom - 1,
                    "date {day} touches the calendar rail or edge at {size}px"
                );
                assert!(
                    ((top as f32 - cap_bottom) - (page_bottom - 1 - bottom) as f32).abs() <= 2.0,
                    "date {day} is vertically off-center within its page at {size}px"
                );
            }
        }
    }

    #[test]
    fn tray_icon_renders_valid_32x32_rgba_buffer() {
        let dark = render_tray_icon_rgba(29, true);
        let light = render_tray_icon_rgba(5, false);
        assert_eq!(dark.len(), 32 * 32 * 4);
        assert_eq!(light.len(), 32 * 32 * 4);
        assert_ne!(dark, light); // Different dates; coral paper stays light in both themes.
    }

    #[test]
    fn tray_icon_variants_render_valid_rgba() {
        for monochrome in [false, true] {
            for mode in [
                TrayIconMode::DateOnly,
                TrayIconMode::WeekdayAndDate,
                TrayIconMode::TimeAndDate,
            ] {
                let rgba = render_tray_icon_variant_rgba(
                    mode,
                    monochrome,
                    29,
                    Weekday::Thursday,
                    (22, 1),
                    true,
                );
                assert_eq!(rgba.len(), 32 * 32 * 4);
            }
        }
    }

    #[test]
    fn multi_size_ico_generates_valid_headers() {
        let date = LocalDate::new(2026, 10, 29).unwrap();
        let ico = generate_multi_size_ico(date);
        assert!(ico.len() > 1000);
        assert_eq!(&ico[0..4], &[0, 0, 1, 0]); // ICO signature
        let count = u16::from_le_bytes([ico[4], ico[5]]);
        assert_eq!(count, 7);
    }
}
