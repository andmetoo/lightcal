//! Plain-text clipboard writes for the agenda Copy action. Nothing is read back.

/// Places `text` on the Windows clipboard as Unicode text.
#[cfg(windows)]
pub fn copy_text(text: &str) -> Result<(), String> {
    use windows_sys::Win32::Foundation::GlobalFree;
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows_sys::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE,
    };
    const CF_UNICODETEXT: u32 = 13;

    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = wide.len() * std::mem::size_of::<u16>();
    unsafe {
        // Lightcal's own window owns the data: with a NULL owner, EmptyClipboard leaves no
        // owner and SetClipboardData may fail. One attempt; never sleep on the UI thread.
        let owner = super::shell::find_lightcal_hwnd();
        if OpenClipboard(owner) == 0 {
            return Err("The clipboard is busy".into());
        }
        let result = (|| {
            if EmptyClipboard() == 0 {
                return Err("Could not clear the clipboard".to_string());
            }
            let memory = GlobalAlloc(GMEM_MOVEABLE, bytes);
            if memory.is_null() {
                return Err("Could not allocate clipboard memory".to_string());
            }
            let target = GlobalLock(memory) as *mut u16;
            if target.is_null() {
                GlobalFree(memory);
                return Err("Could not lock clipboard memory".to_string());
            }
            std::ptr::copy_nonoverlapping(wide.as_ptr(), target, wide.len());
            GlobalUnlock(memory);
            if SetClipboardData(CF_UNICODETEXT, memory).is_null() {
                GlobalFree(memory);
                return Err("Could not set clipboard text".to_string());
            }
            // The clipboard owns the memory after a successful SetClipboardData.
            Ok(())
        })();
        CloseClipboard();
        result
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn copy_text(_text: &str) -> Result<(), String> {
    Err("Clipboard access is only implemented on Windows".into())
}

#[cfg(target_os = "macos")]
pub fn copy_text(text: &str) -> Result<(), String> {
    crate::platform::macos::copy_text(text)
}
