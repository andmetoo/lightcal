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

pub struct TaskbarPreview {
    hwnd: HWND,
    // The subclass reference points into this stable allocation until Drop.
    _state: Box<PreviewState>,
}

impl TaskbarPreview {
    pub fn install(
        snapshot: impl Fn() -> Option<SharedPixelBuffer<Rgba8Pixel>> + 'static,
    ) -> Result<Self, String> {
        let hwnd = application_window();
        Self::install_for_window(hwnd, snapshot)
    }

    fn install_for_window(
        hwnd: HWND,
        snapshot: impl Fn() -> Option<SharedPixelBuffer<Rgba8Pixel>> + 'static,
    ) -> Result<Self, String> {
        if hwnd.is_null() {
            return Err("No native application window".into());
        }
        let mut state = Box::new(PreviewState {
            last_calendar: RefCell::new(snapshot()),
            snapshot: Box::new(snapshot),
        });
        unsafe {
            if SetWindowSubclass(
                hwnd,
                Some(preview_proc),
                SUBCLASS_ID,
                (&mut *state as *mut PreviewState) as usize,
            ) == 0
            {
                return Err("Could not attach taskbar preview".into());
            }
            let enabled: i32 = 1;
            if DwmSetWindowAttribute(
                hwnd,
                DWMWA_HAS_ICONIC_BITMAP as u32,
                &enabled as *const _ as _,
                4,
            ) < 0
            {
                RemoveWindowSubclass(hwnd, Some(preview_proc), SUBCLASS_ID);
                return Err("DWM custom previews are unavailable".into());
            }
        }
        Ok(Self {
            hwnd,
            _state: state,
        })
    }

    pub fn invalidate(&self) {
        unsafe {
            DwmInvalidateIconicBitmaps(self.hwnd);
        }
    }
}

impl Drop for TaskbarPreview {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(self.hwnd) != 0 {
                RemoveWindowSubclass(self.hwnd, Some(preview_proc), SUBCLASS_ID);
            }
        }
    }
}

fn application_window() -> HWND {
    crate::platform::windows::shell::find_lightcal_hwnd()
}

const RESTORE_SUBCLASS_ID: usize = 0x4c435253;

struct RestoreState {
    minimized: Cell<bool>,
    on_restore: Box<dyn Fn()>,
}

/// Reports when the window comes back from the minimized state, for example when the taskbar
/// button restores it. Windows discards a minimized window's pixels, but Slint's software
/// renderer still treats its buffer as on screen and presents only dirty regions, so the
/// caller must request a full repaint. Restores through Slint's own `show()` already do that.
pub struct RestoreWatcher {
    hwnd: HWND,
    // The subclass reference points into this stable allocation until Drop.
    _state: Box<RestoreState>,
}

impl RestoreWatcher {
    pub fn install(on_restore: impl Fn() + 'static) -> Result<Self, String> {
        Self::install_for_window(application_window(), on_restore)
    }

    fn install_for_window(hwnd: HWND, on_restore: impl Fn() + 'static) -> Result<Self, String> {
        if hwnd.is_null() {
            return Err("No native application window".into());
        }
        let mut state = Box::new(RestoreState {
            minimized: Cell::new(unsafe { IsIconic(hwnd) } != 0),
            on_restore: Box::new(on_restore),
        });
        unsafe {
            if SetWindowSubclass(
                hwnd,
                Some(restore_proc),
                RESTORE_SUBCLASS_ID,
                (&mut *state as *mut RestoreState) as usize,
            ) == 0
            {
                return Err("Could not watch window restores".into());
            }
        }
        Ok(Self {
            hwnd,
            _state: state,
        })
    }

    /// Whether this watcher is attached to the current application window.
    pub fn watches_application_window(&self) -> bool {
        self.hwnd == application_window()
    }
}

impl Drop for RestoreWatcher {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(self.hwnd) != 0 {
                RemoveWindowSubclass(self.hwnd, Some(restore_proc), RESTORE_SUBCLASS_ID);
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

unsafe extern "system" fn restore_proc(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    if message == WM_NCDESTROY {
        RemoveWindowSubclass(hwnd, Some(restore_proc), RESTORE_SUBCLASS_ID);
    } else if message == WM_SIZE {
        let state = &*(data as *const RestoreState);
        if leaves_minimized(&state.minimized, wp as u32) {
            (state.on_restore)();
        }
    }
    DefSubclassProc(hwnd, message, wp, lp)
}

unsafe extern "system" fn preview_proc(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    if message == WM_NCDESTROY {
        RemoveWindowSubclass(hwnd, Some(preview_proc), SUBCLASS_ID);
    }
    if message != WM_DWMSENDICONICTHUMBNAIL && message != WM_DWMSENDICONICLIVEPREVIEWBITMAP {
        return DefSubclassProc(hwnd, message, wp, lp);
    }
    let state = &*(data as *const PreviewState);
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
    0
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
    fn restore_watcher_reports_native_restores_from_minimized() {
        struct TestWindow(HWND);
        impl Drop for TestWindow {
            fn drop(&mut self) {
                unsafe {
                    DestroyWindow(self.0);
                }
            }
        }
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        let title: Vec<u16> = "Lightcal restore test\0".encode_utf16().collect();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW,
                -10000,
                -10000,
                352,
                432,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        assert!(!hwnd.is_null());
        let window = TestWindow(hwnd);
        let restores = std::rc::Rc::new(Cell::new(0));
        let counter = restores.clone();
        let watcher =
            RestoreWatcher::install_for_window(hwnd, move || counter.set(counter.get() + 1))
                .unwrap();
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
        drop(watcher);
        unsafe {
            ShowWindow(hwnd, SW_MINIMIZE);
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        assert_eq!(restores.get(), 1);
        drop(window);
    }
}
