//! Native Windows clipboard read/write adapter for Smart Migrate.
//!
//! This module wraps the Win32 clipboard API to provide:
//! - Reading the current host clipboard text.
//! - Writing validated clipboard text to the host clipboard.
//!
//! # Security constraints
//!
//! - Clipboard text is **never** written to any log or telemetry record.
//! - Writing to the clipboard requires an explicit host operator grant.
//! - The payload is validated through `migroute::validate_clipboard_update`
//!   before any Win32 call is issued.
//! - Only CF_UNICODETEXT is handled; binary clipboard formats are ignored.

use migroute::{
    validate_clipboard_update, ClipboardDirection, ClipboardGrant, SessionPermission,
    MAX_CLIPBOARD_BYTES,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::sync::Mutex;

// ─── State ───────────────────────────────────────────────────────────────────

/// Holds the live clipboard sync grant for the current active session.
pub struct ClipboardState {
    pub grant: Mutex<Option<ClipboardGrant>>,
}

impl ClipboardState {
    pub fn new() -> Self {
        Self {
            grant: Mutex::new(None),
        }
    }

    /// Creates and stores a new clipboard grant for the given session.
    pub fn activate(
        &self,
        session_id: String,
        direction: ClipboardDirection,
    ) -> ClipboardGrant {
        let grant = ClipboardGrant::new(session_id, direction);
        *self.grant.lock().unwrap() = Some(grant.clone());
        grant
    }

    /// Suspends clipboard sync without destroying the grant.
    pub fn suspend(&self) {
        if let Some(g) = self.grant.lock().unwrap().as_mut() {
            g.suspend();
        }
    }

    /// Resumes a previously suspended clipboard grant.
    pub fn resume(&self) {
        if let Some(g) = self.grant.lock().unwrap().as_mut() {
            g.resume();
        }
    }

    /// Clears the clipboard grant (session end, revocation).
    pub fn clear(&self) {
        *self.grant.lock().unwrap() = None;
    }
}

// ─── Serialized telemetry ─────────────────────────────────────────────────────

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardStatus {
    pub active: bool,
    pub session_id: String,
    pub direction: String,
    pub host_push_count: u64,
    pub client_push_count: u64,
}

// ─── Win32 clipboard primitives ───────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod win32 {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    /// Win32 CF_UNICODETEXT clipboard format identifier.
    const CF_UNICODETEXT: u32 = 13;

    extern "system" {
        fn OpenClipboard(hwnd: *mut std::ffi::c_void) -> i32;
        fn CloseClipboard() -> i32;
        fn EmptyClipboard() -> i32;
        fn GetClipboardData(u_format: u32) -> *mut std::ffi::c_void;
        fn SetClipboardData(u_format: u32, h_mem: *mut std::ffi::c_void)
            -> *mut std::ffi::c_void;
        fn GlobalAlloc(u_flags: u32, dw_bytes: usize) -> *mut std::ffi::c_void;
        fn GlobalLock(h_mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        fn GlobalUnlock(h_mem: *mut std::ffi::c_void) -> i32;
        fn GlobalSize(h_mem: *mut std::ffi::c_void) -> usize;
    }

    /// GMEM_MOVEABLE flag for GlobalAlloc.
    const GMEM_MOVEABLE: u32 = 0x0002;

    /// Reads the current Windows clipboard as a UTF-8 string.
    ///
    /// Returns `None` if the clipboard is empty or contains no text.
    /// The caller must ensure this is called from a context that can own
    /// the clipboard (not inside another clipboard-open scope).
    pub fn read_clipboard_text() -> Option<String> {
        // SAFETY: Win32 clipboard API. All handles are checked for null before use.
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return None;
            }
            let h_data = GetClipboardData(CF_UNICODETEXT);
            if h_data.is_null() {
                CloseClipboard();
                return None;
            }
            let ptr = GlobalLock(h_data);
            if ptr.is_null() {
                CloseClipboard();
                return None;
            }
            let size = GlobalSize(h_data);
            // Each UTF-16 code unit is 2 bytes; compute the number of u16 elements.
            let wchar_count = size / 2;
            let slice = std::slice::from_raw_parts(ptr as *const u16, wchar_count);
            // Find the null terminator.
            let len = slice.iter().position(|&c| c == 0).unwrap_or(wchar_count);
            let text = OsString::from_wide(&slice[..len])
                .into_string()
                .ok();
            GlobalUnlock(h_data);
            CloseClipboard();
            text
        }
    }

    /// Writes UTF-8 text to the Windows clipboard as CF_UNICODETEXT.
    ///
    /// Returns `Ok(())` on success or an error string describing the failure.
    pub fn write_clipboard_text(text: &str) -> Result<(), String> {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = std::ffi::OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0u16))
            .collect();
        let byte_size = wide.len() * 2;

        // SAFETY: Win32 clipboard API. Memory is allocated via GlobalAlloc and
        // ownership is transferred to the clipboard via SetClipboardData.
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return Err("OpenClipboard failed".to_string());
            }
            EmptyClipboard();
            let h_mem = GlobalAlloc(GMEM_MOVEABLE, byte_size);
            if h_mem.is_null() {
                CloseClipboard();
                return Err("GlobalAlloc failed".to_string());
            }
            let ptr = GlobalLock(h_mem);
            if ptr.is_null() {
                CloseClipboard();
                return Err("GlobalLock failed".to_string());
            }
            std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr as *mut u16, wide.len());
            GlobalUnlock(h_mem);
            let result = SetClipboardData(CF_UNICODETEXT, h_mem);
            CloseClipboard();
            if result.is_null() {
                return Err("SetClipboardData failed".to_string());
            }
            Ok(())
        }
    }
}

// ─── Non-Windows stubs (build guard) ─────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
mod win32 {
    pub fn read_clipboard_text() -> Option<String> {
        None
    }
    pub fn write_clipboard_text(_text: &str) -> Result<(), String> {
        Err("clipboard write is only supported on Windows".to_string())
    }
}

// ─── Public adapter functions (called by Tauri commands in lib.rs) ───────────

/// Reads the current Windows host clipboard text.
///
/// Returns `None` if the clipboard is empty, not text, or cannot be opened.
/// **The returned text is never written to logs.**
pub fn read_host_clipboard() -> Option<String> {
    let text = win32::read_clipboard_text()?;
    // Reject if oversized (paranoia guard — we don't log the content).
    if text.len() > MAX_CLIPBOARD_BYTES {
        return None;
    }
    Some(text)
}

/// Writes validated text to the Windows host clipboard.
///
/// Before writing, validates through the MigRoute clipboard policy engine.
/// **The text content is never written to logs.**
///
/// # Arguments
///
/// - `text` — Candidate clipboard text from the remote client.
/// - `state` — The live clipboard grant state.
/// - `granted_permissions` — The full session permission set.
///
/// Returns `Ok(())` if the clipboard was updated, or an error string.
pub fn write_host_clipboard(
    text: &str,
    grant: &ClipboardGrant,
    granted_permissions: &BTreeSet<SessionPermission>,
) -> Result<(), String> {
    validate_clipboard_update(text, grant, granted_permissions, false)
        .map_err(|e| e.to_string())?;
    win32::write_clipboard_text(text)
}

/// Gets the current clipboard grant status for telemetry reporting.
pub fn get_clipboard_status(state: &ClipboardState) -> ClipboardStatus {
    let lock = state.grant.lock().unwrap();
    match lock.as_ref() {
        None => ClipboardStatus {
            active: false,
            session_id: String::new(),
            direction: "none".to_owned(),
            host_push_count: 0,
            client_push_count: 0,
        },
        Some(g) => ClipboardStatus {
            active: g.active,
            session_id: g.session_id.clone(),
            direction: match g.direction {
                ClipboardDirection::HostToClient => "host_to_client",
                ClipboardDirection::ClientToHost => "client_to_host",
                ClipboardDirection::Bidirectional => "bidirectional",
            }
            .to_owned(),
            host_push_count: g.host_push_count,
            client_push_count: g.client_push_count,
        },
    }
}
