mod capture;
mod state;
mod storage;
mod stream;

use capture::{DisplaySource, EncoderCapability};
use migroute::identity::{DeviceIdentity, DevicePlatform};
use migroute::pairing::PairingSession;
use migroute::trust::TrustedDevice;
use migroute::{DeviceId, SessionPermission};
use serde::Serialize;
use state::AppState;
use std::time::{SystemTime, UNIX_EPOCH};
use stream::{StreamSessionState, StreamTelemetry};
use tauri::{State, Window};


fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HostStatus {
    engine: &'static str,
    platform: &'static str,
    profile: String,
    device_id: String,
    fingerprint: String,
    trusted_device_count: usize,
    privileged_features_enabled: bool,
}

#[tauri::command]
fn host_status(state: State<'_, AppState>) -> HostStatus {
    let trust_store = state.trust_store.lock().unwrap();
    let stream = state.active_stream.lock().unwrap();
    HostStatus {
        engine: "MigRoute",
        platform: "Windows host shell",
        profile: state.identity.name.clone(),
        device_id: state.identity.id.to_string(),
        fingerprint: state.identity.fingerprint.clone(),
        trusted_device_count: trust_store.active_count(),
        privileged_features_enabled: stream.is_active,
    }
}


/// Returns the host's persistent, validated device identity.
#[tauri::command]
fn get_device_identity(state: State<'_, AppState>) -> DeviceIdentity {
    state.identity.clone()
}

/// Initiates a new short-lived pairing session with a 6-digit numeric code, nonce, and QR URI.
#[tauri::command]
fn start_pairing_session(state: State<'_, AppState>) -> Result<PairingSession, String> {
    let mut active = state.active_pairing.lock().unwrap();
    let code_int = storage::generate_pairing_code();
    let token = storage::generate_crypto_token();
    let session_id = format!("sm-pair-{}", &token[..8]);

    let session = PairingSession::new(
        session_id,
        state.identity.id.clone(),
        state.identity.name.clone(),
        code_int,
        token,
        now_ms(),
    );

    *active = Some(session.clone());
    Ok(session)
}

/// Checks the currently active pairing session if one exists and hasn't expired.
#[tauri::command]
fn get_active_pairing(state: State<'_, AppState>) -> Option<PairingSession> {
    let mut active = state.active_pairing.lock().unwrap();
    if let Some(ref session) = *active {
        if session.is_expired(now_ms()) {
            *active = None;
            return None;
        }
    }
    active.clone()
}

/// Cancels and destroys the current pending pairing session.
#[tauri::command]
fn cancel_pairing(state: State<'_, AppState>) -> Result<(), String> {
    let mut active = state.active_pairing.lock().unwrap();
    *active = None;
    Ok(())
}

/// Simulates or processes client pairing credential submission (code, name, permissions).
#[tauri::command]
fn submit_client_pairing_code(
    state: State<'_, AppState>,
    requester_id: String,
    requester_name: String,
    code: String,
    token: Option<String>,
    permissions: Vec<String>,
) -> Result<PairingSession, String> {
    let mut active = state.active_pairing.lock().unwrap();
    let session = active.as_mut().ok_or_else(|| "no active pairing session".to_string())?;

    let req_id = DeviceId::try_from(requester_id.as_str()).map_err(|e| e.to_string())?;
    let perms: Vec<SessionPermission> = permissions
        .into_iter()
        .filter_map(|p| match p.as_str() {
            "VIEW_SCREEN" => Some(SessionPermission::ViewScreen),
            "CONTROL_MOUSE" => Some(SessionPermission::ControlMouse),
            "CONTROL_KEYBOARD" => Some(SessionPermission::ControlKeyboard),
            "SEND_FILES" => Some(SessionPermission::SendFiles),
            "RECEIVE_FILES" => Some(SessionPermission::ReceiveFiles),
            "CLIPBOARD" => Some(SessionPermission::Clipboard),
            "AUDIO" => Some(SessionPermission::Audio),
            _ => None,
        })
        .collect();

    session
        .submit_credentials(
            req_id,
            requester_name,
            &code,
            token.as_deref(),
            perms,
            now_ms(),
        )
        .map_err(|e| e.to_string())?;

    Ok(session.clone())
}

/// Host decides and approves granted permissions for the pending device.
#[tauri::command]
fn approve_pairing(
    state: State<'_, AppState>,
    granted_permissions: Vec<String>,
) -> Result<TrustedDevice, String> {
    let mut active = state.active_pairing.lock().unwrap();
    let session = active.as_mut().ok_or_else(|| "no active pairing session to approve".to_string())?;

    let perms: Vec<SessionPermission> = granted_permissions
        .into_iter()
        .filter_map(|p| match p.as_str() {
            "VIEW_SCREEN" => Some(SessionPermission::ViewScreen),
            "CONTROL_MOUSE" => Some(SessionPermission::ControlMouse),
            "CONTROL_KEYBOARD" => Some(SessionPermission::ControlKeyboard),
            "SEND_FILES" => Some(SessionPermission::SendFiles),
            "RECEIVE_FILES" => Some(SessionPermission::ReceiveFiles),
            "CLIPBOARD" => Some(SessionPermission::Clipboard),
            "AUDIO" => Some(SessionPermission::Audio),
            _ => None,
        })
        .collect();

    let granted = session.host_approve(perms).map_err(|e| e.to_string())?;
    let req_id = session.pending_requester_id.clone().ok_or("missing requester ID")?;
    let req_name = session.pending_requester_name.clone().unwrap_or_else(|| "Paired Device".to_string());

    let trusted_device = TrustedDevice::new(
        req_id,
        req_name,
        DevicePlatform::Android,
        format!("FP-{}", &session.secret_token[..8].to_uppercase()),
        now_ms(),
        granted,
    );

    let mut trust_store = state.trust_store.lock().unwrap();
    let _ = trust_store.remove_device(&trusted_device.id); // replace if previously existed
    trust_store.add_device(trusted_device.clone()).map_err(|e| e.to_string())?;
    storage::save_trust_store(&trust_store)?;

    // Clear pairing session after successful approval
    *active = None;
    Ok(trusted_device)
}

/// Host rejects the pending pairing request.
#[tauri::command]
fn reject_pairing(state: State<'_, AppState>) -> Result<(), String> {
    let mut active = state.active_pairing.lock().unwrap();
    if let Some(ref mut session) = *active {
        let _ = session.host_reject();
    }
    *active = None;
    Ok(())
}

/// Lists all trusted devices registered on this host.
#[tauri::command]
fn get_trusted_devices(state: State<'_, AppState>) -> Vec<TrustedDevice> {
    let store = state.trust_store.lock().unwrap();
    store.list_all().into_iter().cloned().collect()
}

/// Revokes authorization for a device.
#[tauri::command]
fn revoke_trusted_device(state: State<'_, AppState>, device_id: String) -> Result<(), String> {
    let id = DeviceId::try_from(device_id.as_str()).map_err(|e| e.to_string())?;
    let mut store = state.trust_store.lock().unwrap();
    store.revoke_device(&id).map_err(|e| e.to_string())?;
    storage::save_trust_store(&store)?;
    Ok(())
}

/// Removes a device from the trust store.
#[tauri::command]
fn remove_trusted_device(state: State<'_, AppState>, device_id: String) -> Result<(), String> {
    let id = DeviceId::try_from(device_id.as_str()).map_err(|e| e.to_string())?;
    let mut store = state.trust_store.lock().unwrap();
    let _ = store.remove_device(&id).map_err(|e| e.to_string())?;
    storage::save_trust_store(&store)?;
    Ok(())
}

/// Enumerate available display sources for screen capture.
#[tauri::command]
fn get_display_sources() -> Vec<DisplaySource> {
    capture::enumerate_display_sources()
}

/// Detect host GPU hardware video encoder capabilities.
#[tauri::command]
fn detect_hardware_encoders() -> Vec<EncoderCapability> {
    capture::detect_encoder_capabilities()
}

/// Starts an authorized display streaming session to a trusted client device.
/// Strictly enforces that the target device has the VIEW_SCREEN permission.
#[tauri::command]
fn start_display_stream(
    state: State<'_, AppState>,
    target_device_id: String,
    source_id: String,
    codec: String,
    target_fps: u32,
    encoder_name: String,
) -> Result<StreamSessionState, String> {
    let trust_store = state.trust_store.lock().unwrap();
    let stream_state = stream::start_stream(
        &trust_store,
        &target_device_id,
        source_id,
        codec,
        target_fps,
        encoder_name,
    )?;

    let mut active = state.active_stream.lock().unwrap();
    *active = stream_state.clone();
    Ok(stream_state)
}

/// Stops the active display streaming session immediately.
#[tauri::command]
fn stop_display_stream(state: State<'_, AppState>, _reason: String) -> Result<(), String> {
    let mut active = state.active_stream.lock().unwrap();
    *active = StreamSessionState::default();
    Ok(())
}

/// Computes live streaming telemetry (FPS, latency, bitrate, frames).
#[tauri::command]
fn get_stream_telemetry(state: State<'_, AppState>) -> StreamTelemetry {
    let mut active = state.active_stream.lock().unwrap();
    stream::compute_telemetry(&mut active)
}

/// Minimize the main application window.
#[tauri::command]
fn minimize_window(window: Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

/// Toggle between maximized and restored window state.
#[tauri::command]
fn toggle_maximize(window: Window) -> Result<(), String> {
    if window.is_maximized().map_err(|e| e.to_string())? {
        window.unmaximize().map_err(|e| e.to_string())
    } else {
        window.maximize().map_err(|e| e.to_string())
    }
}

/// Close the main application window.
#[tauri::command]
fn close_window(window: Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let identity = storage::load_or_create_identity();
    let trust_store = storage::load_trust_store();
    let app_state = AppState::new(identity, trust_store);

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            host_status,
            get_device_identity,
            start_pairing_session,
            get_active_pairing,
            cancel_pairing,
            submit_client_pairing_code,
            approve_pairing,
            reject_pairing,
            get_trusted_devices,
            revoke_trusted_device,
            remove_trusted_device,
            get_display_sources,
            detect_hardware_encoders,
            start_display_stream,
            stop_display_stream,
            get_stream_telemetry,
            minimize_window,
            toggle_maximize,
            close_window,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Smart Migrate Windows host");
}

