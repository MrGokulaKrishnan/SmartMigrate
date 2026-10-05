//! Controlled host-side mouse and keyboard input injection.
//!
//! Enforces:
//! - "Host-side authorization is the source of truth for every requested capability."
//! - "SessionPermission::ControlMouse and SessionPermission::ControlKeyboard must be explicitly granted."
//! - "Coordinates are clamped to valid screen bounds."
//! - "No secret, device-private key, pairing token, screen frame, password, or raw user file path may reach logs."

use migroute::envelope::SequenceTracker;
use migroute::trust::TrustStore;
use migroute::{DeviceId, SessionPermission};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputTelemetry {
    pub mouse_events_injected: u64,
    pub keyboard_events_injected: u64,
    pub unauthorized_dropped: u64,
    pub replayed_packets_dropped: u64,
    pub mouse_control_host_enabled: bool,
    pub keyboard_control_host_enabled: bool,
}

pub struct InputController {
    pub telemetry: Mutex<InputTelemetry>,
    pub sequence_tracker: Mutex<SequenceTracker>,
    pub host_mouse_override: Mutex<bool>,
    pub host_keyboard_override: Mutex<bool>,
}

impl InputController {
    pub fn new() -> Self {
        Self {
            telemetry: Mutex::new(InputTelemetry {
                mouse_control_host_enabled: true,
                keyboard_control_host_enabled: true,
                ..Default::default()
            }),
            sequence_tracker: Mutex::new(SequenceTracker::new()),
            host_mouse_override: Mutex::new(true),
            host_keyboard_override: Mutex::new(true),
        }
    }

    /// Injects a remote mouse input event after verifying authorization, sequence replay, and screen bounds.
    pub fn inject_mouse(
        &self,
        trust_store: &TrustStore,
        device_id_str: &str,
        x: i32,
        y: i32,
        left_down: bool,
        left_up: bool,
        right_down: bool,
        right_up: bool,
        middle_down: bool,
        middle_up: bool,
        scroll_delta: i32,
        sequence: u64,
    ) -> Result<(), String> {
        let device_id = DeviceId::try_from(device_id_str).map_err(|e| e.to_string())?;

        // 1. Host override check
        if !*self.host_mouse_override.lock().unwrap() {
            let mut tel = self.telemetry.lock().unwrap();
            tel.unauthorized_dropped += 1;
            return Err("host operator has temporarily suspended remote mouse input".to_string());
        }

        // 2. Trust store authorization check
        if !trust_store.is_authorized(&device_id, SessionPermission::ControlMouse) {
            let mut tel = self.telemetry.lock().unwrap();
            tel.unauthorized_dropped += 1;
            return Err("device is not authorized for CONTROL_MOUSE".to_string());
        }

        // 3. Monotonic sequence replay protection
        let mut tracker = self.sequence_tracker.lock().unwrap();
        if let Some(prev) = tracker.last_sequence(device_id_str) {
            if sequence <= prev {
                let mut tel = self.telemetry.lock().unwrap();
                tel.replayed_packets_dropped += 1;
                return Err("replayed or out-of-order input packet rejected".to_string());
            }
        }
        tracker.accept(device_id_str, sequence);

        // 4. Native Win32 SendInput injection
        #[cfg(target_os = "windows")]
        {
            inject_win32_mouse(x, y, left_down, left_up, right_down, right_up, middle_down, middle_up, scroll_delta);
        }

        let mut tel = self.telemetry.lock().unwrap();
        tel.mouse_events_injected += 1;
        Ok(())
    }

    /// Injects a remote keyboard event after verifying authorization and sequence replay.
    pub fn inject_keyboard(
        &self,
        trust_store: &TrustStore,
        device_id_str: &str,
        vk_code: u16,
        key_up: bool,
        sequence: u64,
    ) -> Result<(), String> {
        let device_id = DeviceId::try_from(device_id_str).map_err(|e| e.to_string())?;

        // 1. Host override check
        if !*self.host_keyboard_override.lock().unwrap() {
            let mut tel = self.telemetry.lock().unwrap();
            tel.unauthorized_dropped += 1;
            return Err("host operator has temporarily suspended remote keyboard input".to_string());
        }

        // 2. Trust store authorization check
        if !trust_store.is_authorized(&device_id, SessionPermission::ControlKeyboard) {
            let mut tel = self.telemetry.lock().unwrap();
            tel.unauthorized_dropped += 1;
            return Err("device is not authorized for CONTROL_KEYBOARD".to_string());
        }

        // 3. Monotonic sequence replay protection
        let mut tracker = self.sequence_tracker.lock().unwrap();
        if let Some(prev) = tracker.last_sequence(device_id_str) {
            if sequence <= prev {
                let mut tel = self.telemetry.lock().unwrap();
                tel.replayed_packets_dropped += 1;
                return Err("replayed or out-of-order keyboard packet rejected".to_string());
            }
        }
        tracker.accept(device_id_str, sequence);

        // 4. Native Win32 SendInput injection
        #[cfg(target_os = "windows")]
        {
            inject_win32_keyboard(vk_code, key_up);
        }

        let mut tel = self.telemetry.lock().unwrap();
        tel.keyboard_events_injected += 1;
        Ok(())
    }

    pub fn get_telemetry(&self) -> InputTelemetry {
        let tel = self.telemetry.lock().unwrap();
        InputTelemetry {
            mouse_control_host_enabled: *self.host_mouse_override.lock().unwrap(),
            keyboard_control_host_enabled: *self.host_keyboard_override.lock().unwrap(),
            ..*tel
        }
    }

    pub fn set_mouse_override(&self, enabled: bool) {
        *self.host_mouse_override.lock().unwrap() = enabled;
    }

    pub fn set_keyboard_override(&self, enabled: bool) {
        *self.host_keyboard_override.lock().unwrap() = enabled;
    }
}

#[cfg(target_os = "windows")]
mod win32 {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct MOUSEINPUT {
        pub dx: i32,
        pub dy: i32,
        pub mouse_data: u32,
        pub dw_flags: u32,
        pub time: u32,
        pub dw_extra_info: usize,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct KEYBDINPUT {
        pub w_vk: u16,
        pub w_scan: u16,
        pub dw_flags: u32,
        pub time: u32,
        pub dw_extra_info: usize,
    }

    #[repr(C)]
    pub union INPUT_UNION {
        pub mi: MOUSEINPUT,
        pub ki: KEYBDINPUT,
    }

    #[repr(C)]
    pub struct INPUT {
        pub r#type: u32,
        pub u: INPUT_UNION,
    }

    #[link(name = "user32")]
    extern "system" {
        pub fn SendInput(cInputs: u32, pInputs: *const INPUT, cbSize: i32) -> u32;
        pub fn GetSystemMetrics(nIndex: i32) -> i32;
    }

    pub const INPUT_MOUSE: u32 = 0;
    pub const INPUT_KEYBOARD: u32 = 1;
    pub const MOUSEEVENTF_MOVE: u32 = 0x0001;
    pub const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
    pub const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
    pub const MOUSEEVENTF_RIGHTDOWN: u32 = 0x0008;
    pub const MOUSEEVENTF_RIGHTUP: u32 = 0x0010;
    pub const MOUSEEVENTF_MIDDLEDOWN: u32 = 0x0020;
    pub const MOUSEEVENTF_MIDDLEUP: u32 = 0x0040;
    pub const MOUSEEVENTF_WHEEL: u32 = 0x0800;
    pub const MOUSEEVENTF_ABSOLUTE: u32 = 0x8000;
    pub const KEYEVENTF_KEYUP: u32 = 0x0002;
}

#[cfg(target_os = "windows")]
fn inject_win32_mouse(
    x: i32,
    y: i32,
    left_down: bool,
    left_up: bool,
    right_down: bool,
    right_up: bool,
    middle_down: bool,
    middle_up: bool,
    scroll_delta: i32,
) {
    use win32::*;

    let screen_w = unsafe { GetSystemMetrics(0) };
    let screen_h = unsafe { GetSystemMetrics(1) };

    let sw = if screen_w > 0 { screen_w } else { 1920 };
    let sh = if screen_h > 0 { screen_h } else { 1080 };

    // Clamp coordinates strictly to screen
    let clamped_x = x.clamp(0, sw);
    let clamped_y = y.clamp(0, sh);

    // Normalize to 0..65535 for MOUSEEVENTF_ABSOLUTE
    let norm_x = ((clamped_x as f64 / sw as f64) * 65535.0) as i32;
    let norm_y = ((clamped_y as f64 / sh as f64) * 65535.0) as i32;

    let mut flags = MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE;
    if left_down { flags |= MOUSEEVENTF_LEFTDOWN; }
    if left_up { flags |= MOUSEEVENTF_LEFTUP; }
    if right_down { flags |= MOUSEEVENTF_RIGHTDOWN; }
    if right_up { flags |= MOUSEEVENTF_RIGHTUP; }
    if middle_down { flags |= MOUSEEVENTF_MIDDLEDOWN; }
    if middle_up { flags |= MOUSEEVENTF_MIDDLEUP; }
    if scroll_delta != 0 { flags |= MOUSEEVENTF_WHEEL; }

    let input = INPUT {
        r#type: INPUT_MOUSE,
        u: INPUT_UNION {
            mi: MOUSEINPUT {
                dx: norm_x,
                dy: norm_y,
                mouse_data: scroll_delta as u32,
                dw_flags: flags,
                time: 0,
                dw_extra_info: 0,
            },
        },
    };

    unsafe {
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(target_os = "windows")]
fn inject_win32_keyboard(vk_code: u16, key_up: bool) {
    use win32::*;

    let mut flags = 0u32;
    if key_up {
        flags |= KEYEVENTF_KEYUP;
    }

    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        u: INPUT_UNION {
            ki: KEYBDINPUT {
                w_vk: vk_code,
                w_scan: 0,
                dw_flags: flags,
                time: 0,
                dw_extra_info: 0,
            },
        },
    };

    unsafe {
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
}

