//! Process-owned taskbar integration; never subclasses Explorer or changes pinning.
use slint::{Rgba8Pixel, SharedPixelBuffer};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::{Dwm::*, Gdi::*},
    System::{Diagnostics::Debug::OutputDebugStringW, Threading::GetCurrentThreadId},
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
    /// Answers DWM thumbnail and Peek requests, which DWM sends only after
    /// `request_previews` has opted the window in.
    preview: PreviewState,
    /// Set by the first `request_previews` and kept even if DWM later refuses one, so cache
    /// invalidation is skipped only where previews were never requested.
    previews_requested: Cell<bool>,
    minimized: Cell<bool>,
    on_restore: Box<dyn Fn()>,
    /// Runs when the window becomes active, a system setting changes, the session locks or
    /// unlocks, or the display turns off or on; see `WindowHooks::set_on_wake`.
    on_wake: RefCell<Option<Box<dyn Fn()>>>,
    /// Runs when the DPI, the display or a system setting (such as the taskbar's place)
    /// changes; see `WindowHooks::set_on_display_change`.
    on_display_change: RefCell<Option<Box<dyn Fn()>>>,
    /// Session and display notifications, registered by `subscribe_session_and_display`.
    session_registered: Cell<bool>,
    display_notify: Cell<windows_sys::Win32::System::Power::HPOWERNOTIFY>,
    /// Set once the subclass is registered and cleared on `WM_NCDESTROY`, so a destroyed
    /// window whose handle is reused never counts. comctl32's `GetWindowSubclass` could
    /// answer this, but the comctl32 that unmanifested test binaries load exports it only by
    /// ordinal.
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
    /// Attaches the subclass. `on_restore` runs inside the window procedure, before the
    /// restored window paints. `snapshot` answers DWM preview requests; once attached, it is
    /// also taken immediately, so a calendar showing at install time is cached before DWM
    /// first asks. While it returns `None` (settings open) the last cached calendar is kept.
    /// A panic in either callback is caught inside the window procedure.
    ///
    /// # Safety
    ///
    /// `hwnd` must be null or a live window owned by this thread.
    pub unsafe fn install(
        hwnd: HWND,
        on_restore: impl Fn() + 'static,
        snapshot: impl Fn() -> Option<SharedPixelBuffer<Rgba8Pixel>> + 'static,
    ) -> Result<Self, String> {
        if hwnd.is_null() {
            return Err("No native application window".into());
        }
        let state = Box::new(HookState {
            preview: PreviewState {
                last_calendar: RefCell::new(None),
                snapshot: Box::new(snapshot),
            },
            previews_requested: Cell::new(false),
            minimized: Cell::new(unsafe { IsIconic(hwnd) } != 0),
            on_restore: Box::new(on_restore),
            on_wake: RefCell::new(None),
            on_display_change: RefCell::new(None),
            session_registered: Cell::new(false),
            display_notify: Cell::new(0),
            attached: Cell::new(false),
        });
        let reference = &*state as *const HookState as usize;
        if unsafe { SetWindowSubclass(hwnd, Some(hook_proc), SUBCLASS_ID, reference) } == 0 {
            return Err("Could not attach to the application window".into());
        }
        state.attached.set(true);
        // Rendered only after a successful attach, so a failed attempt costs no snapshot.
        *state.preview.last_calendar.borrow_mut() = (state.preview.snapshot)();
        Ok(Self { hwnd, state })
    }

    /// Opts the window into custom DWM thumbnails and Peek. Call it on every show: the
    /// request is idempotent, DWM can refuse it while composition restarts, and a restarted
    /// DWM may have forgotten it. The subclass is already attached, so every request DWM
    /// then sends is answered.
    pub fn request_previews(&self) {
        self.state.previews_requested.set(true);
        let enabled: i32 = 1;
        unsafe {
            DwmSetWindowAttribute(
                self.hwnd,
                DWMWA_HAS_ICONIC_BITMAP as u32,
                &enabled as *const _ as _,
                4,
            );
        }
    }

    /// Sets the callback for the window becoming active, a system setting change, a session
    /// lock or unlock and the display turning off or on. It runs inside the window
    /// procedure, like `on_restore`.
    pub fn set_on_wake(&self, on_wake: impl Fn() + 'static) {
        *self.state.on_wake.borrow_mut() = Some(Box::new(on_wake));
    }

    /// Sets the callback for a DPI, display or system setting change, which can change the
    /// shell's icon sizes. It runs inside the window procedure, like `on_restore`.
    pub fn set_on_display_change(&self, on_display_change: impl Fn() + 'static) {
        *self.state.on_display_change.borrow_mut() = Some(Box::new(on_display_change));
    }

    /// Asks Windows for session lock and unlock and display on and off notifications to this
    /// window. Windows reports the display state right away, which also sets
    /// `shell::set_display_on`. Repeated calls register once; Drop unregisters.
    pub fn subscribe_session_and_display(&self) {
        use windows_sys::Win32::System::Power::RegisterPowerSettingNotification;
        use windows_sys::Win32::System::RemoteDesktop::{
            WTSRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION,
        };
        use windows_sys::Win32::System::SystemServices::GUID_CONSOLE_DISPLAY_STATE;
        // SAFETY: both register this live window for notifications, undone in Drop.
        unsafe {
            if !self.state.session_registered.get() {
                self.state
                    .session_registered
                    .set(WTSRegisterSessionNotification(self.hwnd, NOTIFY_FOR_THIS_SESSION) != 0);
            }
            if self.state.display_notify.get() == 0 {
                self.state
                    .display_notify
                    .set(RegisterPowerSettingNotification(
                        self.hwnd as _,
                        &GUID_CONSOLE_DISPLAY_STATE,
                        DEVICE_NOTIFY_WINDOW_HANDLE,
                    ));
            }
        }
    }

    /// Whether these hooks are still attached to `hwnd` (not to a destroyed window whose
    /// handle was reused).
    pub fn is_attached_to(&self, hwnd: HWND) -> bool {
        self.hwnd == hwnd && self.state.attached.get()
    }

    /// Drops DWM's cached previews once previews have been requested. It does not depend on
    /// DWM's last answer, so a transient refusal never leaves a stale thumbnail.
    pub fn invalidate_preview(&self) {
        if self.state.previews_requested.get() {
            unsafe {
                DwmInvalidateIconicBitmaps(self.hwnd);
            }
        }
    }
}

impl Drop for WindowHooks {
    fn drop(&mut self) {
        use windows_sys::Win32::System::Power::UnregisterPowerSettingNotification;
        use windows_sys::Win32::System::RemoteDesktop::WTSUnRegisterSessionNotification;
        unsafe {
            if self.state.session_registered.get() {
                WTSUnRegisterSessionNotification(self.hwnd);
            }
            if self.state.display_notify.get() != 0 {
                UnregisterPowerSettingNotification(self.state.display_notify.get());
            }
        }
        if self.state.attached.get() {
            unsafe {
                RemoveWindowSubclass(self.hwnd, Some(hook_proc), SUBCLASS_ID);
            }
        }
    }
}

/// The hooks for the current application window, reinstalled when that window changes.
pub struct WindowHookSlot(Option<WindowHooks>);

impl WindowHookSlot {
    pub const fn new() -> Self {
        Self(None)
    }

    /// Builds hooks for `hwnd` with `install` unless hooks are already attached to it (a
    /// failed attempt is retried next time), then asks DWM for previews again, as
    /// `WindowHooks::request_previews` recommends for every show.
    ///
    /// # Safety
    ///
    /// `hwnd` must be null or a live window owned by this thread.
    pub unsafe fn ensure(
        &mut self,
        hwnd: HWND,
        install: impl FnOnce(HWND) -> Result<WindowHooks, String>,
    ) {
        if hwnd.is_null() {
            return;
        }
        if !self
            .0
            .as_ref()
            .is_some_and(|hooks| hooks.is_attached_to(hwnd))
        {
            self.0 = install(hwnd).ok();
        }
        if let Some(hooks) = &self.0 {
            hooks.request_previews();
        }
    }

    /// Whether hooks are attached to a live window, which then reports display changes.
    pub fn is_attached(&self) -> bool {
        self.0
            .as_ref()
            .is_some_and(|hooks| hooks.state.attached.get())
    }

    pub fn invalidate_preview(&self) {
        if let Some(hooks) = &self.0 {
            hooks.invalidate_preview();
        }
    }
}

impl Default for WindowHookSlot {
    fn default() -> Self {
        Self::new()
    }
}

/// The icon Alt+Tab and an unpinned taskbar button show for the application window. A
/// pinned button draws its shortcut's static icon instead and ignores this one (ADR 0010).
/// Slint 1.18.1's winit backend forwards a window icon only when the image's cache key
/// changes, and pixel-buffer images have none, so a rendered icon never reaches Windows
/// that way. The `slint_still_needs_the_native_window_icon` test (main.rs) fails on any
/// Slint update, so this is checked again before Slint can start setting a second icon.
///
/// `K` describes what the pixels are drawn from; the icon is rebuilt only when it changes.
pub struct WindowIcon<K> {
    /// The inputs `icons` were built from.
    key: Option<K>,
    icons: Option<IconPair>,
    /// The inputs of the last failed build and when it failed, so a failure is retried
    /// after `RETRY_AFTER` rather than on every update.
    failed: Option<(K, std::time::Instant)>,
    /// Windows given `icons`; each one still showing them is moved to rebuilt icons before
    /// the old handles are destroyed, so none keeps drawing a dead icon.
    windows: Vec<HWND>,
}

/// Square straight-alpha RGBA pixels for one icon size.
pub struct IconPixels {
    pub rgba: Vec<u8>,
    pub size: u32,
}

/// The window's big icon and a small one rendered at the shell's small-icon size, so
/// wherever Windows draws the small icon it does not shrink the large raster and blur its
/// numerals. Which of the two each shell surface uses is Windows' choice.
struct IconPair {
    big: HICON,
    small: HICON,
}

impl IconPair {
    unsafe fn new(big: &IconPixels, small: &IconPixels) -> Option<Self> {
        let big = create_icon(&big.rgba, big.size)?;
        match create_icon(&small.rgba, small.size) {
            Some(small) => Some(Self { big, small }),
            None => {
                DestroyIcon(big);
                None
            }
        }
    }

    unsafe fn show_on(&self, hwnd: HWND) {
        SendMessageW(hwnd, WM_SETICON, ICON_BIG as WPARAM, self.big as LPARAM);
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL as WPARAM, self.small as LPARAM);
    }
}

impl Drop for IconPair {
    fn drop(&mut self) {
        unsafe {
            DestroyIcon(self.big);
            DestroyIcon(self.small);
        }
    }
}

const RETRY_AFTER: std::time::Duration = std::time::Duration::from_secs(60);

impl<K: PartialEq> WindowIcon<K> {
    pub const fn new() -> Self {
        Self {
            key: None,
            icons: None,
            failed: None,
            windows: Vec::new(),
        }
    }

    /// Rebuilds the icons from `render` (big, small) when `key` differs from the last
    /// one, then shows them on `hwnd`.
    ///
    /// # Safety
    ///
    /// `hwnd` must be null or a live window owned by this thread.
    pub unsafe fn update(
        &mut self,
        hwnd: HWND,
        key: K,
        render: impl FnOnce() -> (IconPixels, IconPixels),
    ) {
        let retry_due = match &self.failed {
            Some((failed, at)) => *failed != key || at.elapsed() >= RETRY_AFTER,
            None => true,
        };
        if self.key.as_ref() != Some(&key) && retry_due {
            let (big, small) = render();
            match IconPair::new(&big, &small) {
                Some(icons) => {
                    self.key = Some(key);
                    self.failed = None;
                    if let Some(old) = self.icons.take() {
                        self.forget_windows_not_showing(old.big);
                        for &window in &self.windows {
                            icons.show_on(window);
                        }
                        // `old` is destroyed here, once no window shows it.
                    }
                    self.icons = Some(icons);
                }
                None => self.failed = Some((key, std::time::Instant::now())),
            }
        }
        self.attach(hwnd);
    }

    /// Shows the current icons on `hwnd` unless that window already shows them. Asking
    /// the window, not comparing handles, also covers a new window that reuses an old
    /// handle.
    ///
    /// # Safety
    ///
    /// As for [`WindowIcon::update`].
    pub unsafe fn attach(&mut self, hwnd: HWND) {
        let Some(icons) = &self.icons else {
            return;
        };
        if hwnd.is_null() {
            return;
        }
        if shown_icon(hwnd) != icons.big {
            icons.show_on(hwnd);
        }
        let thread = GetCurrentThreadId();
        self.windows
            .retain(|&window| window != hwnd && owned_by(window, thread));
        self.windows.push(hwnd);
    }

    /// Keeps only remembered windows that still show `icon`. A handle no longer owned by
    /// this thread (closed, or reused by another window) is dropped before any message
    /// is sent, so a hung window elsewhere can never block this thread.
    fn forget_windows_not_showing(&mut self, icon: HICON) {
        let thread = unsafe { GetCurrentThreadId() };
        self.windows
            .retain(|&window| owned_by(window, thread) && unsafe { shown_icon(window) } == icon);
    }
}

impl<K: PartialEq> Default for WindowIcon<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K> Drop for WindowIcon<K> {
    fn drop(&mut self) {
        let Some(icons) = &self.icons else {
            return;
        };
        // Windows still alive go back to no icon rather than keep destroyed handles.
        let thread = unsafe { GetCurrentThreadId() };
        for &window in &self.windows {
            if owned_by(window, thread) && unsafe { shown_icon(window) } == icons.big {
                unsafe {
                    SendMessageW(window, WM_SETICON, ICON_BIG as WPARAM, 0);
                    SendMessageW(window, WM_SETICON, ICON_SMALL as WPARAM, 0);
                }
            }
        }
    }
}

fn owned_by(hwnd: HWND, thread: u32) -> bool {
    unsafe { GetWindowThreadProcessId(hwnd, std::ptr::null_mut()) == thread }
}

unsafe fn shown_icon(hwnd: HWND) -> HICON {
    SendMessageW(hwnd, WM_GETICON, ICON_BIG as WPARAM, 0) as HICON
}

/// Builds a 32-bit icon with per-pixel alpha from straight-alpha RGBA.
unsafe fn create_icon(rgba: &[u8], size: u32) -> Option<HICON> {
    if size == 0 || rgba.len() != (size * size * 4) as usize {
        return None;
    }
    let bgra: Vec<u8> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[r, g, b, a]| [b, g, r, a])
        .collect();
    let color = Bitmap::from_bgra(size, size, &bgra)?;
    // An all-zero mask: the color bitmap's alpha decides transparency. Monochrome rows
    // are padded to 16 bits.
    let mask_bits = vec![0u8; size.div_ceil(16) as usize * 2 * size as usize];
    let mask = Bitmap(CreateBitmap(
        size as i32,
        size as i32,
        1,
        1,
        mask_bits.as_ptr().cast(),
    ));
    if mask.0.is_null() {
        return None;
    }
    let info = ICONINFO {
        fIcon: 1,
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: mask.0,
        hbmColor: color.0,
    };
    // The icon keeps its own copies of both bitmaps.
    let icon = CreateIconIndirect(&info);
    (!icon.is_null()).then_some(icon)
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

/// Runs an app callback inside the window procedure, where a panic cannot unwind (it would
/// abort the process). A caught panic is written to the debugger output, which DebugView
/// shows, because a GUI build has no console.
fn guarded(callback_name: &str, callback: impl FnOnce()) {
    let Err(payload) = catch_unwind(AssertUnwindSafe(callback)) else {
        return;
    };
    let detail = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("no message");
    let line: Vec<u16> = format!("Lightcal: the {callback_name} callback panicked: {detail}\n")
        .encode_utf16()
        .chain([0])
        .collect();
    unsafe { OutputDebugStringW(line.as_ptr()) };
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
    if changes_display(message) {
        if let Ok(callback) = state.on_display_change.try_borrow() {
            if let Some(callback) = callback.as_ref() {
                guarded("display change", callback);
            }
        }
    }
    #[cfg(debug_assertions)]
    if message == windows_sys::Win32::UI::WindowsAndMessaging::WM_WINDOWPOSCHANGED {
        trace_window_pos(lp);
    }
    match message {
        WM_NCDESTROY => {
            RemoveWindowSubclass(hwnd, Some(hook_proc), SUBCLASS_ID);
            state.attached.set(false);
        }
        WM_SIZE => {
            if leaves_minimized(&state.minimized, wp as u32) {
                guarded("restore", || (state.on_restore)());
            }
        }
        WM_POWERBROADCAST if wp as u32 == PBT_POWERSETTINGCHANGE => {
            if let Some(on) = display_state(lp) {
                crate::platform::windows::shell::set_display_on(on);
                wake(state);
            }
        }
        _ if wakes(message, wp) => wake(state),
        WM_DWMSENDICONICTHUMBNAIL | WM_DWMSENDICONICLIVEPREVIEWBITMAP => {
            guarded("preview", || {
                send_preview(hwnd, message, lp, &state.preview)
            });
            return 0;
        }
        _ => {}
    }
    DefSubclassProc(hwnd, message, wp, lp)
}

/// Debug builds with `LIGHTCAL_TRACE_WINDOW` set print every native move and resize, so a
/// view switch can be counted: each line is one change the user may see.
#[cfg(debug_assertions)]
fn tracing() -> bool {
    thread_local! {
        /// Read once: the window procedure runs this for every move.
        static ENABLED: bool = std::env::var_os("LIGHTCAL_TRACE_WINDOW").is_some();
    }
    ENABLED.with(|enabled| *enabled)
}

#[cfg(debug_assertions)]
fn trace_line(line: &str) {
    thread_local! {
        static START: std::time::Instant = std::time::Instant::now();
    }
    if tracing() {
        let ms = START.with(|start| start.elapsed().as_millis());
        eprintln!("window {ms:>6} ms: {line}");
    }
}

#[cfg(debug_assertions)]
unsafe fn trace_window_pos(lp: LPARAM) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SWP_NOMOVE, SWP_NOSIZE, WINDOWPOS};
    if lp == 0 || !tracing() {
        return;
    }
    let pos = &*(lp as *const WINDOWPOS);
    if pos.flags & SWP_NOMOVE != 0 && pos.flags & SWP_NOSIZE != 0 {
        return;
    }
    trace_line(&format!(
        "x {} y {} {} × {}{}{}",
        pos.x,
        pos.y,
        pos.cx,
        pos.cy,
        if pos.flags & SWP_NOMOVE != 0 {
            " (no move)"
        } else {
            ""
        },
        if pos.flags & SWP_NOSIZE != 0 {
            " (no size)"
        } else {
            ""
        },
    ));
}

/// Activation (the low word of wParam is WA_INACTIVE, 0, on deactivation), any system
/// setting change (the Settings app need not name Animation effects) and a session change
/// such as lock or unlock.
fn wakes(message: u32, wp: WPARAM) -> bool {
    match message {
        WM_ACTIVATE => wp & 0xFFFF != 0,
        WM_SETTINGCHANGE | WM_WTSSESSION_CHANGE => true,
        _ => false,
    }
}

/// A DPI or display change, a system setting change such as the taskbar moving to another
/// monitor, or Explorer restarting with a new taskbar, any of which can change the shell's
/// icon sizes and the taskbar's place.
fn changes_display(message: u32) -> bool {
    matches!(message, WM_DPICHANGED | WM_DISPLAYCHANGE | WM_SETTINGCHANGE)
        || (message != 0 && message == taskbar_created())
}

/// The `TaskbarCreated` message Explorer broadcasts to top-level windows after it restarts,
/// or 0 (`WM_NULL`, which never matches) when it cannot be registered.
fn taskbar_created() -> u32 {
    static MESSAGE: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *MESSAGE.get_or_init(|| {
        let name: Vec<u16> = "TaskbarCreated".encode_utf16().chain([0]).collect();
        // SAFETY: `name` is NUL-terminated and outlives the call.
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW(name.as_ptr())
        }
    })
}

/// The console display state from a `PBT_POWERSETTINGCHANGE` broadcast: `Some(false)` when
/// it turned off, `Some(true)` when on or dimmed, `None` for other power settings.
unsafe fn display_state(lp: LPARAM) -> Option<bool> {
    use windows_sys::Win32::System::Power::POWERBROADCAST_SETTING;
    use windows_sys::Win32::System::SystemServices::GUID_CONSOLE_DISPLAY_STATE;
    let setting = (lp as *const POWERBROADCAST_SETTING).as_ref()?;
    let (a, b) = (setting.PowerSetting, GUID_CONSOLE_DISPLAY_STATE);
    let same = a.data1 == b.data1 && a.data2 == b.data2 && a.data3 == b.data3 && a.data4 == b.data4;
    (same && setting.DataLength >= 1).then(|| setting.Data[0] != 0)
}

fn wake(state: &HookState) {
    if let Ok(on_wake) = state.on_wake.try_borrow() {
        if let Some(on_wake) = on_wake.as_ref() {
            guarded("wake", on_wake);
        }
    }
}

unsafe fn send_preview(hwnd: HWND, message: u32, lp: LPARAM, state: &PreviewState) {
    // A nested DWM request must not borrow the cache recursively.
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
        Self::from_bgra(w, h, &pixels)
    }

    /// A top-down 32-bit DIB section holding `bgra`, which must be `w` × `h` pixels.
    unsafe fn from_bgra(w: u32, h: u32, bgra: &[u8]) -> Option<Self> {
        debug_assert_eq!(bgra.len(), (w * h * 4) as usize);
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
        std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits as *mut u8, bgra.len());
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

    /// The size and first pixel (BGRA) of the color bitmap behind `icon`.
    fn icon_size_and_first_pixel(icon: HICON) -> (i32, [u8; 4]) {
        unsafe {
            let mut info: ICONINFO = std::mem::zeroed();
            assert_ne!(GetIconInfo(icon, &mut info), 0);
            let mut bitmap: BITMAP = std::mem::zeroed();
            GetObjectW(
                info.hbmColor,
                std::mem::size_of::<BITMAP>() as i32,
                &mut bitmap as *mut _ as _,
            );
            assert_eq!(bitmap.bmBitsPixel, 32);
            let size = bitmap.bmWidth;
            let mut header: BITMAPINFO = std::mem::zeroed();
            header.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            header.bmiHeader.biWidth = size;
            header.bmiHeader.biHeight = -size;
            header.bmiHeader.biPlanes = 1;
            header.bmiHeader.biBitCount = 32;
            header.bmiHeader.biCompression = BI_RGB;
            let mut pixels = vec![0u8; (size * size * 4) as usize];
            let dc = GetDC(std::ptr::null_mut());
            let rows = GetDIBits(
                dc,
                info.hbmColor,
                0,
                size as u32,
                pixels.as_mut_ptr().cast(),
                &mut header,
                DIB_RGB_COLORS,
            );
            ReleaseDC(std::ptr::null_mut(), dc);
            DeleteObject(info.hbmColor);
            DeleteObject(info.hbmMask);
            assert_eq!(rows, size);
            (size, [pixels[0], pixels[1], pixels[2], pixels[3]])
        }
    }

    fn solid(rgba: [u8; 4], size: u32) -> IconPixels {
        IconPixels {
            rgba: rgba.repeat((size * size) as usize),
            size,
        }
    }

    #[test]
    fn window_icon_follows_new_pixels_and_new_windows() {
        let first = crate::platform::windows::test_window::TestWindow::new("Lightcal icon test");
        let second = crate::platform::windows::test_window::TestWindow::new("Lightcal icon test");
        let icon_of = |hwnd: HWND, kind: u32| unsafe {
            SendMessageW(hwnd, WM_GETICON, kind as WPARAM, 0) as HICON
        };
        const TEAL: [u8; 4] = [20, 160, 150, 255];
        const CORAL: [u8; 4] = [200, 40, 50, 255];
        let mut icon = WindowIcon::new();
        unsafe { icon.update(first.0, 1, || (solid(TEAL, 64), solid(TEAL, 24))) };
        let shown = icon_of(first.0, ICON_BIG);
        assert!(!shown.is_null());
        assert_eq!(icon_size_and_first_pixel(shown), (64, [150, 160, 20, 255]));
        // The small icon is its own raster at the small size, not the big one shrunk.
        assert_eq!(
            icon_size_and_first_pixel(icon_of(first.0, ICON_SMALL)),
            (24, [150, 160, 20, 255])
        );

        // The same key neither renders nor sends again.
        unsafe { icon.update(first.0, 1, || unreachable!()) };
        assert_eq!(icon_of(first.0, ICON_BIG), shown);

        // A new window shares the icons, so the first window's handles stay alive.
        unsafe { icon.attach(second.0) };
        assert_eq!(icon_of(second.0, ICON_BIG), shown);
        assert_eq!(icon_size_and_first_pixel(icon_of(first.0, ICON_BIG)).0, 64);

        // New pixels reach both windows, so neither keeps the destroyed old handles.
        unsafe { icon.update(second.0, 2, || (solid(CORAL, 64), solid(CORAL, 24))) };
        let coral = icon_of(second.0, ICON_BIG);
        assert_ne!(coral, shown);
        assert_eq!(icon_size_and_first_pixel(coral), (64, [50, 40, 200, 255]));
        assert_eq!(icon_of(first.0, ICON_BIG), coral);

        // A window that lost its icon (as a recreated window reusing a handle would) gets
        // it back even though the handle is the one remembered.
        unsafe {
            SendMessageW(second.0, WM_SETICON, ICON_BIG as WPARAM, 0);
            icon.attach(second.0);
        }
        assert_eq!(icon_of(second.0, ICON_BIG), coral);
        assert_eq!(unsafe { create_icon(&solid(TEAL, 64).rgba, 32) }, None);

        // A failed build keeps the last icon, is not retried at once for the same key,
        // and does not block a build for new inputs.
        let broken = || {
            (
                solid(TEAL, 64),
                IconPixels {
                    rgba: Vec::new(),
                    size: 24,
                },
            )
        };
        unsafe { icon.update(second.0, 3, broken) };
        assert_eq!(icon_of(second.0, ICON_BIG), coral);
        unsafe { icon.update(second.0, 3, || unreachable!()) };
        unsafe { icon.update(second.0, 4, || (solid(TEAL, 64), solid(TEAL, 24))) };
        assert_eq!(
            icon_size_and_first_pixel(icon_of(second.0, ICON_BIG)).1,
            [150, 160, 20, 255]
        );

        // Dropping clears the icons from windows rather than leave destroyed handles.
        drop(icon);
        assert!(icon_of(first.0, ICON_BIG).is_null());
        assert!(icon_of(second.0, ICON_SMALL).is_null());
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
    fn display_state_reads_only_the_console_display_setting() {
        use windows_sys::Win32::System::Power::POWERBROADCAST_SETTING;
        use windows_sys::Win32::System::SystemServices::{
            GUID_CONSOLE_DISPLAY_STATE, GUID_SESSION_DISPLAY_STATUS,
        };
        let setting = |guid, data| POWERBROADCAST_SETTING {
            PowerSetting: guid,
            DataLength: 4,
            Data: [data],
        };
        let read = |s: &POWERBROADCAST_SETTING| unsafe {
            display_state(s as *const POWERBROADCAST_SETTING as LPARAM)
        };
        assert_eq!(read(&setting(GUID_CONSOLE_DISPLAY_STATE, 0)), Some(false));
        assert_eq!(read(&setting(GUID_CONSOLE_DISPLAY_STATE, 1)), Some(true));
        assert_eq!(read(&setting(GUID_CONSOLE_DISPLAY_STATE, 2)), Some(true)); // dimmed
        assert_eq!(read(&setting(GUID_SESSION_DISPLAY_STATUS, 0)), None);
        assert_eq!(unsafe { display_state(0) }, None);
    }
    #[test]
    fn activation_setting_and_session_changes_wake() {
        assert!(wakes(WM_ACTIVATE, 1)); // WA_ACTIVE
        assert!(wakes(WM_ACTIVATE, 2)); // WA_CLICKACTIVE
        assert!(wakes(WM_ACTIVATE, 0x0001_0001)); // WA_ACTIVE, minimized
        assert!(!wakes(WM_ACTIVATE, 0)); // WA_INACTIVE
        assert!(wakes(WM_SETTINGCHANGE, SPI_SETCLIENTAREAANIMATION as usize));
        assert!(wakes(WM_SETTINGCHANGE, 0));
        assert!(wakes(WM_WTSSESSION_CHANGE, 7)); // WTS_SESSION_LOCK
        assert!(!wakes(WM_SIZE, 1));
    }
    #[test]
    fn dpi_display_and_setting_changes_report_a_display_change() {
        let window = crate::platform::windows::test_window::TestWindow::new("Lightcal dpi test");
        let changes = std::rc::Rc::new(Cell::new(0));
        let counter = changes.clone();
        let hooks = unsafe { WindowHooks::install(window.0, || {}, || None) }.unwrap();
        hooks.set_on_display_change(move || counter.set(counter.get() + 1));
        for message in [
            WM_DPICHANGED,
            WM_DISPLAYCHANGE,
            WM_SETTINGCHANGE,
            taskbar_created(),
        ] {
            // A null RECT makes WM_DPICHANGED's default handling a no-op for the test window.
            unsafe { SendMessageW(window.0, message, 0, 0) };
        }
        assert_eq!(changes.get(), 4);
        unsafe { SendMessageW(window.0, WM_SIZE, 0, 0) };
        assert_eq!(changes.get(), 4);
        assert!(!changes_display(WM_ACTIVATE));
        // WM_NULL is never taken for TaskbarCreated, even if registering it failed.
        assert!(!changes_display(0));
    }
    #[test]
    fn hooks_report_native_restores_from_minimized() {
        let window =
            crate::platform::windows::test_window::TestWindow::new("Lightcal restore test");
        let hwnd = window.0;
        let restores = std::rc::Rc::new(Cell::new(0));
        let counter = restores.clone();
        let hooks =
            unsafe { WindowHooks::install(hwnd, move || counter.set(counter.get() + 1), || None) }
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
        let hooks = unsafe { WindowHooks::install(hwnd, || {}, || None) }.unwrap();
        drop(window);
        assert!(!hooks.is_attached_to(hwnd));
    }
    /// Sits below `WindowHooks` and counts the DWM thumbnail requests forwarded to it.
    unsafe extern "system" fn count_forwarded(
        hwnd: HWND,
        message: u32,
        wp: WPARAM,
        lp: LPARAM,
        _id: usize,
        data: usize,
    ) -> LRESULT {
        if message == WM_DWMSENDICONICTHUMBNAIL {
            let forwarded = &*(data as *const Cell<u32>);
            forwarded.set(forwarded.get() + 1);
        }
        DefSubclassProc(hwnd, message, wp, lp)
    }

    fn red_calendar() -> Option<SharedPixelBuffer<Rgba8Pixel>> {
        Some(SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
            &[240u8, 40, 50, 255].repeat(352 * 432),
            352,
            432,
        ))
    }

    #[test]
    fn hooks_answer_dwm_preview_requests_with_fresh_snapshots() {
        const OBSERVER_ID: usize = 0x4c43_5445;
        let window =
            crate::platform::windows::test_window::TestWindow::new("Lightcal preview test");
        let hwnd = window.0;
        let forwarded = Box::new(Cell::new(0u32));
        let observer = &*forwarded as *const Cell<u32> as usize;
        assert_ne!(
            unsafe { SetWindowSubclass(hwnd, Some(count_forwarded), OBSERVER_ID, observer) },
            0
        );
        let snapshots = std::rc::Rc::new(Cell::new(0));
        let counter = snapshots.clone();
        let hooks = unsafe {
            WindowHooks::install(
                hwnd,
                || {},
                move || {
                    counter.set(counter.get() + 1);
                    red_calendar()
                },
            )
        }
        .unwrap();
        // The first snapshot is cached at once, so settings never show an empty preview.
        assert_eq!(snapshots.get(), 1);
        // Every show asks DWM again; repeating the request is harmless.
        hooks.request_previews();
        hooks.request_previews();
        hooks.invalidate_preview();
        assert_eq!(snapshots.get(), 1);

        let thumbnail_size = ((200 << 16) | 150) as LPARAM;
        assert_eq!(
            unsafe { SendMessageW(hwnd, WM_DWMSENDICONICTHUMBNAIL, 0, thumbnail_size) },
            0
        );
        assert_eq!(
            unsafe { SendMessageW(hwnd, WM_DWMSENDICONICLIVEPREVIEWBITMAP, 0, 0) },
            0
        );
        assert_eq!(snapshots.get(), 3);
        // Answered requests are not forwarded to the window's own procedure.
        assert_eq!(forwarded.get(), 0);
        unsafe { RemoveWindowSubclass(hwnd, Some(count_forwarded), OBSERVER_ID) };
    }
    #[test]
    fn panicking_callbacks_are_contained() {
        let window = crate::platform::windows::test_window::TestWindow::new("Lightcal panic test");
        let hwnd = window.0;
        let restores = std::rc::Rc::new(Cell::new(0));
        let snapshots = std::rc::Rc::new(Cell::new(0));
        let (restore_calls, snapshot_calls) = (restores.clone(), snapshots.clone());
        let _hooks = unsafe {
            WindowHooks::install(
                hwnd,
                move || {
                    restore_calls.set(restore_calls.get() + 1);
                    panic!("restore callback")
                },
                move || {
                    snapshot_calls.set(snapshot_calls.get() + 1);
                    if snapshot_calls.get() == 1 {
                        red_calendar()
                    } else {
                        panic!("{}", String::from("snapshot callback"))
                    }
                },
            )
        }
        .unwrap();
        // Without the guard either panic would abort the whole test process.
        unsafe {
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            ShowWindow(hwnd, SW_MINIMIZE);
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        assert_eq!(restores.get(), 1);
        assert_eq!(unsafe { IsIconic(hwnd) }, 0);
        assert_eq!(
            unsafe { SendMessageW(hwnd, WM_DWMSENDICONICTHUMBNAIL, 0, (200 << 16) | 150) },
            0
        );
        assert_eq!(snapshots.get(), 2);
    }
    #[test]
    fn hook_slot_installs_once_per_window_and_follows_a_new_window() {
        let installs = Cell::new(0);
        let install = |hwnd| {
            installs.set(installs.get() + 1);
            unsafe { WindowHooks::install(hwnd, || {}, || None) }
        };
        let mut slot = WindowHookSlot::new();
        unsafe { slot.ensure(std::ptr::null_mut(), install) };
        assert_eq!(installs.get(), 0);
        assert!(!slot.is_attached());

        let first = crate::platform::windows::test_window::TestWindow::new("Lightcal slot test");
        // Repeated shows of the same window keep its hooks and only re-request previews.
        unsafe {
            slot.ensure(first.0, install);
            slot.ensure(first.0, install);
        }
        assert_eq!(installs.get(), 1);
        assert!(slot.is_attached());
        assert!(slot.0.as_ref().unwrap().state.previews_requested.get());
        slot.invalidate_preview();

        // A destroyed window, even if Windows reuses its handle, gets fresh hooks.
        let first_hwnd = first.0;
        drop(first);
        assert!(
            !slot.is_attached(),
            "a destroyed window reports no display changes"
        );
        let second = crate::platform::windows::test_window::TestWindow::new("Lightcal slot test");
        unsafe { slot.ensure(second.0, install) };
        assert_eq!(installs.get(), 2);
        assert!(slot.0.as_ref().unwrap().is_attached_to(second.0));
        if second.0 != first_hwnd {
            assert!(!slot.0.as_ref().unwrap().is_attached_to(first_hwnd));
        }
    }
    #[test]
    fn snapshot_is_taken_only_after_a_successful_attach() {
        let snapshots = std::rc::Rc::new(Cell::new(0));
        let counter = snapshots.clone();
        let snapshot = move || {
            counter.set(counter.get() + 1);
            None
        };
        // The temporary test window is destroyed at the end of this statement, and a
        // destroyed window cannot be subclassed, so the attach fails.
        let gone = crate::platform::windows::test_window::TestWindow::new("Lightcal gone").0;
        assert_eq!(unsafe { IsWindow(gone) }, 0);
        assert!(unsafe { WindowHooks::install(gone, || {}, snapshot) }.is_err());
        assert_eq!(snapshots.get(), 0);
    }
}
