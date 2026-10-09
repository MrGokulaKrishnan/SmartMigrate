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
        dx: i32,
        dy: i32,
        is_relative: bool,
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

        // 3. Monotonic sequence replay protection (enforced when sequence > 0)
        if sequence > 0 {
            let mut tracker = self.sequence_tracker.lock().unwrap();
            if let Some(prev) = tracker.last_sequence(device_id_str) {
                if sequence <= prev {
                    let mut tel = self.telemetry.lock().unwrap();
                    tel.replayed_packets_dropped += 1;
                    return Err("replayed or out-of-order input packet rejected".to_string());
                }
            }
            tracker.accept(device_id_str, sequence);
        }

        // 4. Native Win32 SendInput injection
        #[cfg(target_os = "windows")]
        {
            if is_relative && (dx != 0 || dy != 0) {
                inject_win32_mouse_relative(dx, dy);
            }
            inject_win32_mouse(x, y, is_relative, left_down, left_up, right_down, right_up, middle_down, middle_up, scroll_delta);
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

        // 3. Monotonic sequence replay protection (enforced when sequence > 0)
        if sequence > 0 {
            let mut tracker = self.sequence_tracker.lock().unwrap();
            if let Some(prev) = tracker.last_sequence(device_id_str) {
                if sequence <= prev {
                    let mut tel = self.telemetry.lock().unwrap();
                    tel.replayed_packets_dropped += 1;
                    return Err("replayed or out-of-order keyboard packet rejected".to_string());
                }
            }
            tracker.accept(device_id_str, sequence);
        }

        // 4. Native Win32 SendInput injection
        #[cfg(target_os = "windows")]
        {
            inject_win32_keyboard(vk_code, key_up);
        }

        let mut tel = self.telemetry.lock().unwrap();
        tel.keyboard_events_injected += 1;
        Ok(())
    }

    /// Injects Unicode string text directly to the active Windows window.
    pub fn inject_text(
        &self,
        trust_store: &TrustStore,
        device_id_str: &str,
        text: &str,
        sequence: u64,
    ) -> Result<(), String> {
        let device_id = DeviceId::try_from(device_id_str).map_err(|e| e.to_string())?;

        if !*self.host_keyboard_override.lock().unwrap() {
            let mut tel = self.telemetry.lock().unwrap();
            tel.unauthorized_dropped += 1;
            return Err("host operator has temporarily suspended remote keyboard input".to_string());
        }

        if !trust_store.is_authorized(&device_id, SessionPermission::ControlKeyboard) {
            let mut tel = self.telemetry.lock().unwrap();
            tel.unauthorized_dropped += 1;
            return Err("device is not authorized for CONTROL_KEYBOARD".to_string());
        }

        if sequence > 0 {
            let mut tracker = self.sequence_tracker.lock().unwrap();
            if let Some(prev) = tracker.last_sequence(device_id_str) {
                if sequence <= prev {
                    let mut tel = self.telemetry.lock().unwrap();
                    tel.replayed_packets_dropped += 1;
                    return Err("replayed or out-of-order keyboard packet rejected".to_string());
                }
            }
            tracker.accept(device_id_str, sequence);
        }

        #[cfg(target_os = "windows")]
        {
            inject_win32_unicode_str(text);
        }

        let mut tel = self.telemetry.lock().unwrap();
        tel.keyboard_events_injected += text.chars().count() as u64;
        Ok(())
    }

    /// Injects a system shortcut / hotkey (e.g. "show_desktop", "task_switch", "enter", "backspace").
    pub fn inject_hotkey(
        &self,
        trust_store: &TrustStore,
        device_id_str: &str,
        hotkey: &str,
        sequence: u64,
    ) -> Result<(), String> {
        let device_id = DeviceId::try_from(device_id_str).map_err(|e| e.to_string())?;

        if !*self.host_keyboard_override.lock().unwrap() {
            let mut tel = self.telemetry.lock().unwrap();
            tel.unauthorized_dropped += 1;
            return Err("host operator has temporarily suspended remote keyboard input".to_string());
        }

        if !trust_store.is_authorized(&device_id, SessionPermission::ControlKeyboard) {
            let mut tel = self.telemetry.lock().unwrap();
            tel.unauthorized_dropped += 1;
            return Err("device is not authorized for CONTROL_KEYBOARD".to_string());
        }

        if sequence > 0 {
            let mut tracker = self.sequence_tracker.lock().unwrap();
            if let Some(prev) = tracker.last_sequence(device_id_str) {
                if sequence <= prev {
                    let mut tel = self.telemetry.lock().unwrap();
                    tel.replayed_packets_dropped += 1;
                    return Err("replayed or out-of-order keyboard packet rejected".to_string());
                }
            }
            tracker.accept(device_id_str, sequence);
        }

        #[cfg(target_os = "windows")]
        {
            inject_win32_hotkey(hotkey);
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
    pub const KEYEVENTF_UNICODE: u32 = 0x0004;
}

#[cfg(target_os = "windows")]
fn inject_win32_mouse(
    x: i32,
    y: i32,
    is_relative: bool,
    left_down: bool,
    left_up: bool,
    right_down: bool,
    right_up: bool,
    middle_down: bool,
    middle_up: bool,
    scroll_delta: i32,
) {
    use win32::*;

    let mut flags = 0u32;
    let mut norm_x = 0i32;
    let mut norm_y = 0i32;

    if !is_relative {
        let screen_w = unsafe { GetSystemMetrics(0) };
        let screen_h = unsafe { GetSystemMetrics(1) };
        let sw = if screen_w > 0 { screen_w } else { 1920 };
        let sh = if screen_h > 0 { screen_h } else { 1080 };

        let clamped_x = x.clamp(0, sw);
        let clamped_y = y.clamp(0, sh);

        norm_x = ((clamped_x as f64 / sw as f64) * 65535.0) as i32;
        norm_y = ((clamped_y as f64 / sh as f64) * 65535.0) as i32;
        flags |= MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE;
    }

    if left_down { flags |= MOUSEEVENTF_LEFTDOWN; }
    if left_up { flags |= MOUSEEVENTF_LEFTUP; }
    if right_down { flags |= MOUSEEVENTF_RIGHTDOWN; }
    if right_up { flags |= MOUSEEVENTF_RIGHTUP; }
    if middle_down { flags |= MOUSEEVENTF_MIDDLEDOWN; }
    if middle_up { flags |= MOUSEEVENTF_MIDDLEUP; }
    if scroll_delta != 0 { flags |= MOUSEEVENTF_WHEEL; }

    if flags != 0 {
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
}

#[cfg(target_os = "windows")]
fn inject_win32_mouse_relative(dx: i32, dy: i32) {
    use win32::*;
    let input = INPUT {
        r#type: INPUT_MOUSE,
        u: INPUT_UNION {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouse_data: 0,
                dw_flags: MOUSEEVENTF_MOVE,
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

#[cfg(target_os = "windows")]
fn inject_win32_unicode_str(text: &str) {
    use win32::*;
    for c in text.chars() {
        let mut buf = [0u16; 2];
        let encoded = c.encode_utf16(&mut buf);
        for &mut code_unit in encoded {
            let input_down = INPUT {
                r#type: INPUT_KEYBOARD,
                u: INPUT_UNION {
                    ki: KEYBDINPUT {
                        w_vk: 0,
                        w_scan: code_unit,
                        dw_flags: KEYEVENTF_UNICODE,
                        time: 0,
                        dw_extra_info: 0,
                    },
                },
            };
            let input_up = INPUT {
                r#type: INPUT_KEYBOARD,
                u: INPUT_UNION {
                    ki: KEYBDINPUT {
                        w_vk: 0,
                        w_scan: code_unit,
                        dw_flags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                        time: 0,
                        dw_extra_info: 0,
                    },
                },
            };
            let inputs = [input_down, input_up];
            unsafe {
                SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn inject_win32_hotkey(hotkey: &str) {
    use win32::*;
    let make_key = |vk: u16, up: bool| -> INPUT {
        let flags = if up { KEYEVENTF_KEYUP } else { 0 };
        INPUT {
            r#type: INPUT_KEYBOARD,
            u: INPUT_UNION {
                ki: KEYBDINPUT {
                    w_vk: vk,
                    w_scan: 0,
                    dw_flags: flags,
                    time: 0,
                    dw_extra_info: 0,
                },
            },
        }
    };

    match hotkey.to_lowercase().as_str() {
        "show_desktop" | "desktop" => {
            // Win + D
            let inputs = [
                make_key(0x5B, false),
                make_key(0x44, false),
                make_key(0x44, true),
                make_key(0x5B, true),
            ];
            unsafe { SendInput(4, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32); }
        }
        "task_switch" | "task_switcher" | "alt_tab" => {
            // Alt + Tab
            let inputs = [
                make_key(0x12, false),
                make_key(0x09, false),
                make_key(0x09, true),
                make_key(0x12, true),
            ];
            unsafe { SendInput(4, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32); }
        }
        "enter" => {
            let inputs = [make_key(0x0D, false), make_key(0x0D, true)];
            unsafe { SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32); }
        }
        "backspace" => {
            let inputs = [make_key(0x08, false), make_key(0x08, true)];
            unsafe { SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32); }
        }
        "tab" => {
            let inputs = [make_key(0x09, false), make_key(0x09, true)];
            unsafe { SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32); }
        }
        "escape" | "esc" => {
            let inputs = [make_key(0x1B, false), make_key(0x1B, true)];
            unsafe { SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32); }
        }
        "space" => {
            let inputs = [make_key(0x20, false), make_key(0x20, true)];
            unsafe { SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32); }
        }
        _ => {}
    }
}

#[cfg(not(target_os = "windows"))]
fn inject_win32_mouse(
    _x: i32, _y: i32, _is_relative: bool,
    _left_down: bool, _left_up: bool,
    _right_down: bool, _right_up: bool,
    _middle_down: bool, _middle_up: bool,
    _scroll_delta: i32,
) {}

#[cfg(not(target_os = "windows"))]
fn inject_win32_mouse_relative(_dx: i32, _dy: i32) {}

#[cfg(not(target_os = "windows"))]
fn inject_win32_keyboard(_vk_code: u16, _key_up: bool) {}

#[cfg(not(target_os = "windows"))]
fn inject_win32_unicode_str(_text: &str) {}

#[cfg(not(target_os = "windows"))]
fn inject_win32_hotkey(_hotkey: &str) {}


