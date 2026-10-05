use serde::Serialize;
use tauri::{Manager, Window};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HostStatus {
    engine: &'static str,
    platform: &'static str,
    profile: &'static str,
    privileged_features_enabled: bool,
}

/// This command deliberately exposes only non-sensitive shell state.
/// Pairing, capture, transport, input, and transfer adapters are added only
/// after their milestone acceptance criteria and security reviews are complete.
#[tauri::command]
fn host_status() -> HostStatus {
    HostStatus {
        engine: "MigRoute",
        platform: "Windows host shell",
        profile: "Local design preview",
        privileged_features_enabled: false,
    }
}

/// Minimize the main application window.
/// Non-privileged: controls only the window frame, no sensitive operations.
#[tauri::command]
fn minimize_window(window: Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

/// Toggle between maximized and restored window state.
/// Called both by the maximize button and by double-clicking the title bar.
#[tauri::command]
fn toggle_maximize(window: Window) -> Result<(), String> {
    if window.is_maximized().map_err(|e| e.to_string())? {
        window.unmaximize().map_err(|e| e.to_string())
    } else {
        window.maximize().map_err(|e| e.to_string())
    }
}

/// Close the main application window (equivalent to Alt+F4).
/// Non-privileged: no data access, no network, no capture.
#[tauri::command]
fn close_window(window: Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            host_status,
            minimize_window,
            toggle_maximize,
            close_window,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Smart Migrate Windows host");
}

