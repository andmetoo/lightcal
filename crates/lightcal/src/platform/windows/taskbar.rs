//! Process-owned taskbar integration; never subclasses Explorer or changes pinning.
use slint::{Rgba8Pixel, SharedPixelBuffer};
use std::cell::{Cell, RefCell};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::{Dwm::*, Gdi::*},
    UI::{Shell::*, WindowsAndMessaging::*},
};

const SUBCLASS_ID: usize = 0x4c43414c;

pub fn initialize_identity() {
    // Packaged applications use the identity assigned by Windows/Partner Center.
    if !crate::installer::package_managed() {
        let app_id: Vec<u16> = "Koona.Lightcal\0".encode_utf16().collect();
        unsafe {
            SetCurrentProcessExplicitAppUserModelID(app_id.as_ptr());
        }
    }
}

struct PreviewState {
    snapshot: Box<dyn Fn() -> Option<SharedPixelBuffer<Rgba8Pixel>>>,
    last_calendar: RefCell<Option<SharedPixelBuffer<Rgba8Pixel>>>,
}

struct HookState {
    /// `None` when DWM refuses custom previews; the restore report still works.
    preview: Option<PreviewState>,
    minimized: Cell<bool>,
    on_restore: Box<dyn Fn()>,
    /// Cleared on `WM_NCDESTROY`, so a destroyed window whose handle is reused never counts.
    attached: Cell<bool>,
}

/// One subclass on the application window for two jobs: calendar snapshots for taskbar
/// thumbnails and Peek (ADR 0005), and a report when the window leaves the minimized state.
///
/// Windows discards a minimized window's pixels, but Slint's software renderer keeps its
/// buffer, believes it is still on screen and presents only dirty regions. Neither Slint's
/// `show()` (a no-op for a window that is only minimized) nor its winit backend repaints
/// everything when the taskbar restores the window, so `on_restore` must request that.
/// The root cause is upstream (Slint 1.18): on Windows the winit software renderer's
/// `occluded(true)` reset is overwritten by the next frame's softbuffer buffer-age check,
/// which does not know the pixels were discarded. Drop the restore report and the
/// `repaint-generation` layer in `app.slint` once Slint repaints fully after a restore.
pub struct WindowHooks {
    hwnd: HWND,
    // The subclass reference points into this stable allocation until Drop.
    state: Box<HookState>,
}

impl WindowHooks {
    /// `on_restore` runs inside the window procedure, before the restored window paints.
    ///
    /// # Safety
    ///
    /// `hwnd` must be null or a live window owned by this thread.
    pub unsafe fn install(
        hwnd: HWND,
        snapshot: impl Fn() -> Option<SharedPixelBuffer<Rgba8Pixel>> + 'static,
        on_restore: impl Fn() + 'static,
    ) -> Result<Self, String> {
        if hwnd.is_null() {
            return Err("No native application window".into());
        }
        let enabled: i32 = 1;
        let previews = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_HAS_ICONIC_BITMAP as u32,
                &enabled as *const _ as _,
                4,
            )
        } >= 0;
        let state = Box::new(HookState {
            preview: previews.then(|| PreviewState {
                last_calendar: RefCell::new(snapshot()),
                snapshot: Box::new(snapshot),
            }),
            minimized: Cell::new(unsafe { IsIconic(hwnd) } != 0),
            on_restore: Box::new(on_restore),
            attached: Cell::new(false),
        });
        let hooks = Self { hwnd, state };
        if unsafe { SetWindowSubclass(hwnd, Some(hook_proc), SUBCLASS_ID, hooks.reference()) } == 0
        {
            let disabled: i32 = 0;
            unsafe {
                DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_HAS_ICONIC_BITMAP as u32,
                    &disabled as *const _ as _,
                    4,
                );
            }
            return Err("Could not attach to the application window".into());
        }
        hooks.state.attached.set(true);
        Ok(hooks)
    }

    fn reference(&self) -> usize {
        &*self.state as *const HookState as usize
    }

    /// Whether these hooks are still attached to `hwnd` (not to a destroyed window whose
    /// handle was reused).
    pub fn is_attached_to(&self, hwnd: HWND) -> bool {
        self.hwnd == hwnd && self.state.attached.get()
    }

    pub fn invalidate_preview(&self) {
        if self.state.preview.is_some() {
            unsafe {
                DwmInvalidateIconicBitmaps(self.hwnd);
            }
        }
    }
}

impl Drop for WindowHooks {
    fn drop(&mut self) {
        if self.state.attached.get() {
            unsafe {
                RemoveWindowSubclass(self.hwnd, Some(hook_proc), SUBCLASS_ID);
            }
        }
    }
}

/// Tracks the minimized state from a `WM_SIZE` kind and reports a restore from it.
fn leaves_minimized(minimized: &Cell<bool>, size_kind: u32) -> bool {
    match size_kind {
        SIZE_MINIMIZED => {
            minimized.set(true);
            false
        }
        SIZE_RESTORED | SIZE_MAXIMIZED => minimized.replace(false),
        _ => false,
    }
}

unsafe extern "system" fn hook_proc(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    let state = &*(data as *const HookState);
    match message {
        WM_NCDESTROY => {
            RemoveWindowSubclass(hwnd, Some(hook_proc), SUBCLASS_ID);
            state.attached.set(false);
        }
        WM_SIZE => {
            if leaves_minimized(&state.minimized, wp as u32) {
                (state.on_restore)();
            }
        }
        WM_DWMSENDICONICTHUMBNAIL | WM_DWMSENDICONICLIVEPREVIEWBITMAP => {
            if let Some(preview) = &state.preview {
                send_preview(hwnd, message, lp, preview);
                return 0;
            }
        }
        _ => {}
    }
    DefSubclassProc(hwnd, message, wp, lp)
}

unsafe fn send_preview(hwnd: HWND, message: u32, lp: LPARAM, state: &PreviewState) {
    // A nested DWM request must not borrow the cache recursively or panic across FFI.
    if let Ok(mut cached) = state.last_calendar.try_borrow_mut() {
        if let Some(image) = (state.snapshot)() {
            *cached = Some(image);
        }
        if let Some(image) = cached.as_ref() {
            let (max_w, max_h) = if message == WM_DWMSENDICONICTHUMBNAIL {
                (
                    ((lp as usize >> 16) & 0xffff) as u32,
                    (lp as usize & 0xffff) as u32,
                )
            } else {
                (image.width(), image.height())
            };
            if let Some(bitmap) = Bitmap::from_image(image, max_w, max_h) {
                if message == WM_DWMSENDICONICTHUMBNAIL {
                    DwmSetIconicThumbnail(hwnd, bitmap.0, 0);
                } else {
                    DwmSetIconicLivePreviewBitmap(hwnd, bitmap.0, std::ptr::null(), 0);
                }
            }
        }
    }
}

fn preview_pixels(
    image: &SharedPixelBuffer<Rgba8Pixel>,
    max_w: u32,
    max_h: u32,
) -> Option<(u32, u32, Vec<u8>)> {
    let (sw, sh) = (image.width(), image.height());
    if sw == 0 || sh == 0 || max_w == 0 || max_h == 0 {
        return None;
    }
    let scale = (max_w.min(4096) as f64 / sw as f64)
        .min(max_h.min(4096) as f64 / sh as f64)
        .min(1.0);
    let (w, h) = (
        ((sw as f64 * scale).floor() as u32).max(1),
        ((sh as f64 * scale).floor() as u32).max(1),
    );
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    let source = image.as_bytes();
    for y in 0..h {
        for x in 0..w {
            let i = (((y as u64 * sh as u64 / h as u64) * sw as u64
                + x as u64 * sw as u64 / w as u64)
                * 4) as usize;
            let a = source[i + 3] as u16;
            pixels.extend_from_slice(&[
                (source[i + 2] as u16 * a / 255) as u8,
                (source[i + 1] as u16 * a / 255) as u8,
                (source[i] as u16 * a / 255) as u8,
                a as u8,
            ]);
        }
    }
    Some((w, h, pixels))
}

struct Bitmap(HBITMAP);
impl Bitmap {
    unsafe fn from_image(
        image: &SharedPixelBuffer<Rgba8Pixel>,
        max_w: u32,
        max_h: u32,
    ) -> Option<Self> {
        let (w, h, pixels) = preview_pixels(image, max_w, max_h)?;
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = w as i32;
        info.bmiHeader.biHeight = -(h as i32);
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(
            std::ptr::null_mut(),
            &info,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        );
        if bitmap.is_null() {
            return None;
        }
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits as *mut u8, pixels.len());
        Some(Self(bitmap))
    }
}
impl Drop for Bitmap {
    fn drop(&mut self) {
        unsafe {
            DeleteObject(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_preview_bitmap_is_32_bit_and_fits_dwm_bounds() {
        let image = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
            &[240u8, 40, 50, 255].repeat(352 * 432),
            352,
            432,
        );
        let bitmap = unsafe { Bitmap::from_image(&image, 200, 150) }.unwrap();
        let mut dib: DIBSECTION = unsafe { std::mem::zeroed() };
        let bytes = unsafe {
            GetObjectW(
                bitmap.0,
                std::mem::size_of::<DIBSECTION>() as i32,
                &mut dib as *mut _ as _,
            )
        };
        assert_eq!(bytes, std::mem::size_of::<DIBSECTION>() as i32);
        assert_eq!(dib.dsBm.bmBitsPixel, 32);
        assert!(dib.dsBm.bmWidth > 0 && dib.dsBm.bmWidth <= 200);
        assert!(dib.dsBm.bmHeight > 0 && dib.dsBm.bmHeight <= 150);
        assert_eq!(dib.dsBmih.biHeight.abs(), dib.dsBm.bmHeight);
        assert_eq!(
            unsafe { std::slice::from_raw_parts(dib.dsBm.bmBits as *const u8, 4) },
            &[50, 40, 240, 255]
        );
    }
    #[test]
    fn preview_fits_dwm_bounds_and_uses_premultiplied_bgra() {
        let image = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
            &[255u8, 64, 32, 128].repeat(8),
            4,
            2,
        );
        let (w, h, bytes) = preview_pixels(&image, 2, 2).unwrap();
        assert_eq!((w, h), (2, 1));
        assert_eq!(&bytes[..4], &[16, 32, 128, 128]);
        assert!(preview_pixels(&image, 0, 2).is_none());
        assert_eq!(preview_pixels(&image, 100, 100).unwrap().0, 4);
    }
    #[test]
    fn only_leaving_the_minimized_state_counts_as_a_restore() {
        let minimized = Cell::new(false);
        // Resizing a visible window (opening settings) keeps its pixels.
        assert!(!leaves_minimized(&minimized, SIZE_RESTORED));
        assert!(!leaves_minimized(&minimized, SIZE_MINIMIZED));
        assert!(minimized.get());
        assert!(leaves_minimized(&minimized, SIZE_RESTORED));
        assert!(!leaves_minimized(&minimized, SIZE_RESTORED));
        assert!(!leaves_minimized(&minimized, SIZE_MINIMIZED));
        assert!(leaves_minimized(&minimized, SIZE_MAXIMIZED));
        assert!(!leaves_minimized(&minimized, SIZE_MAXSHOW));
    }
    #[test]
    fn hooks_report_native_restores_from_minimized() {
        let window =
            crate::platform::windows::test_window::TestWindow::new("Lightcal restore test");
        let hwnd = window.0;
        let restores = std::rc::Rc::new(Cell::new(0));
        let counter = restores.clone();
        let hooks =
            unsafe { WindowHooks::install(hwnd, || None, move || counter.set(counter.get() + 1)) }
                .unwrap();
        assert!(hooks.is_attached_to(hwnd));
        unsafe {
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                448,
                360,
                SWP_NOMOVE | SWP_NOZORDER,
            );
        }
        assert_eq!(restores.get(), 0);
        // The taskbar button minimizes and restores the window without any app code.
        unsafe {
            ShowWindow(hwnd, SW_MINIMIZE);
        }
        assert_eq!(restores.get(), 0);
        unsafe {
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        assert_eq!(restores.get(), 1);
        drop(hooks);
        unsafe {
            ShowWindow(hwnd, SW_MINIMIZE);
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        assert_eq!(restores.get(), 1);
        // A destroyed window detaches the hooks, even if Windows reuses its handle later.
        let hooks = unsafe { WindowHooks::install(hwnd, || None, || {}) }.unwrap();
        drop(window);
        assert!(!hooks.is_attached_to(hwnd));
    }
}
