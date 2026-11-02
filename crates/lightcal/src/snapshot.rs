//! Pure-Rust uncompressed PNG encoder for visual regression tests and icon/UI snapshot inspection.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;

pub struct HeadlessPlatform {
    pub window: Rc<slint::platform::software_renderer::MinimalSoftwareWindow>,
}

impl Default for HeadlessPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadlessPlatform {
    pub fn new() -> Self {
        let window = slint::platform::software_renderer::MinimalSoftwareWindow::new(
            slint::platform::software_renderer::RepaintBufferType::NewBuffer,
        );
        Self { window }
    }

    pub fn render_to_png(&self, path: &Path, width: u32, height: u32) -> std::io::Result<()> {
        self.window
            .set_size(slint::PhysicalSize::new(width, height));
        self.window.request_redraw();
        slint::platform::update_timers_and_animations();
        let mut pixels = vec![
            slint::platform::software_renderer::PremultipliedRgbaColor::default();
            (width * height) as usize
        ];
        self.window.draw_if_needed(|renderer| {
            renderer.render(&mut pixels, width as usize);
        });
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for px in pixels {
            if px.alpha == 255 {
                rgba.push(px.red);
                rgba.push(px.green);
                rgba.push(px.blue);
                rgba.push(255);
            } else if px.alpha == 0 {
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
            } else {
                let a = px.alpha as u16;
                let r = ((px.red as u16 * 255 + a / 2) / a).min(255) as u8;
                let g = ((px.green as u16 * 255 + a / 2) / a).min(255) as u8;
                let b = ((px.blue as u16 * 255 + a / 2) / a).min(255) as u8;
                rgba.push(r);
                rgba.push(g);
                rgba.push(b);
                rgba.push(px.alpha);
            }
        }
        write_rgba_png(path, width, height, &rgba)
    }
}

impl slint::platform::Platform for HeadlessPlatform {
    fn create_window_adapter(
        &self,
    ) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }
}

pub fn write_rgba_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> std::io::Result<()> {
    assert_eq!(rgba.len(), (width as usize) * (height as usize) * 4);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut raw_scanlines = Vec::with_capacity((width as usize * 4 + 1) * height as usize);
    for y in 0..height as usize {
        raw_scanlines.push(0u8); // Filter type 0 (None)
        let row_start = y * (width as usize) * 4;
        raw_scanlines.extend_from_slice(&rgba[row_start..row_start + (width as usize) * 4]);
    }

    // Wrap raw_scanlines in valid zlib stream using uncompressed DEFLATE blocks (BTYPE=00)
    let mut zlib = Vec::with_capacity(raw_scanlines.len() + 64);
    zlib.push(0x78); // CMF (Deflate, 32K window)
    zlib.push(0x01); // FLG (No dict, check bits)

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
    let adler = adler32(&raw_scanlines);
    zlib.extend_from_slice(&adler.to_be_bytes());

    let mut out = Vec::with_capacity(zlib.len() + 128);
    // PNG Signature
    out.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    // IHDR
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(6); // color type 6 = RGBA
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace
    write_png_chunk(&mut out, b"IHDR", &ihdr);

    // IDAT
    write_png_chunk(&mut out, b"IDAT", &zlib);

    // IEND
    write_png_chunk(&mut out, b"IEND", &[]);

    let mut file = fs::File::create(path)?;
    file.write_all(&out)?;
    Ok(())
}

fn write_png_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(chunk_type);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(chunk_type);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}
