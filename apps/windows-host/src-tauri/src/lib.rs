mod capture;
mod clipboard;
mod input;
mod resilience;
mod smp_server;
mod state;
mod storage;
mod stream;
mod transfer;

use capture::{DisplaySource, EncoderCapability};
use clipboard::ClipboardStatus;
use input::InputTelemetry;
use migroute::identity::{DeviceIdentity, DevicePlatform};
use migroute::pairing::PairingSession;
use migroute::trust::TrustedDevice;
use migroute::{ClipboardDirection, DeviceId, SessionPermission};
use resilience::ResilienceStatus;
use serde::Serialize;
use state::AppState;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use stream::{StreamSessionState, StreamTelemetry};
use tauri::{State, Window};
use transfer::{
    DeltaSyncSummary, FinalizeResultDto, FolderScanResult, OutgoingChunkDto, TransferManager,
    TransferProgressDto, TransferSessionDto,
};



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
fn stop_display_stream(state: State<'_, AppState>, reason: String) -> Result<(), String> {
    let _ = stream::stop_stream(&reason);
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

/// Injects remote mouse movement and button clicks if authorized by host.
#[tauri::command]
fn inject_remote_mouse(
    state: State<'_, AppState>,
    device_id: String,
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
    let trust_store = state.trust_store.lock().unwrap();
    state.input_controller.inject_mouse(
        &trust_store,
        &device_id,
        x,
        y,
        left_down,
        left_up,
        right_down,
        right_up,
        middle_down,
        middle_up,
        scroll_delta,
        sequence,
    )
}

/// Injects remote keyboard keystroke if authorized by host.
#[tauri::command]
fn inject_remote_keyboard(
    state: State<'_, AppState>,
    device_id: String,
    vk_code: u16,
    key_up: bool,
    sequence: u64,
) -> Result<(), String> {
    let trust_store = state.trust_store.lock().unwrap();
    state.input_controller.inject_keyboard(
        &trust_store,
        &device_id,
        vk_code,
        key_up,
        sequence,
    )
}

/// Retrieves input telemetry (injection counts, authorization drops, replay rejects).
#[tauri::command]
fn get_input_telemetry(state: State<'_, AppState>) -> InputTelemetry {
    state.input_controller.get_telemetry()
}

/// Host override to temporarily suspend or resume remote mouse/keyboard control.
#[tauri::command]
fn set_input_override(
    state: State<'_, AppState>,
    mouse_enabled: bool,
    keyboard_enabled: bool,
) -> Result<(), String> {
    state.input_controller.set_mouse_override(mouse_enabled);
    state.input_controller.set_keyboard_override(keyboard_enabled);
    Ok(())
}

/// Retrieves transport resilience metrics (RTT, packet loss, heartbeat status).
#[tauri::command]
fn get_resilience_status(state: State<'_, AppState>) -> ResilienceStatus {
    state.watchdog.get_status()
}

// ─── Clipboard commands ───────────────────────────────────────────────────────

/// Activates clipboard sync for a session with the specified direction.
///
/// Direction values: "host_to_client", "client_to_host", "bidirectional".
/// Requires the target device to have `SessionPermission::Clipboard` granted.
#[tauri::command]
fn activate_clipboard_sync(
    state: State<'_, AppState>,
    device_id: String,
    session_id: String,
    direction: String,
) -> Result<ClipboardStatus, String> {
    let trust_store = state.trust_store.lock().unwrap();
    let id = DeviceId::try_from(device_id.as_str()).map_err(|e| e.to_string())?;
    let device = trust_store
        .get_device(&id)
        .ok_or_else(|| "device not in trust store".to_string())?;
    if device.is_revoked {
        return Err("device is revoked".to_string());
    }
    let perms: BTreeSet<SessionPermission> = device
        .granted_permissions
        .iter()
        .cloned()
        .collect();
    if !perms.contains(&SessionPermission::Clipboard) {
        return Err("device does not have Clipboard permission".to_string());
    }
    let dir = match direction.as_str() {
        "host_to_client" => ClipboardDirection::HostToClient,
        "client_to_host" => ClipboardDirection::ClientToHost,
        "bidirectional" => ClipboardDirection::Bidirectional,
        other => return Err(format!("unknown clipboard direction: {other}")),
    };
    state.clipboard.activate(session_id, dir);
    Ok(clipboard::get_clipboard_status(&state.clipboard))
}

/// Suspends clipboard sync for the active session without revoking the grant.
#[tauri::command]
fn suspend_clipboard_sync(state: State<'_, AppState>) -> Result<(), String> {
    state.clipboard.suspend();
    Ok(())
}

/// Resumes a previously suspended clipboard sync.
#[tauri::command]
fn resume_clipboard_sync(state: State<'_, AppState>) -> Result<(), String> {
    state.clipboard.resume();
    Ok(())
}

/// Reads the current Windows host clipboard text and returns it to the UI.
///
/// The returned string is sent only to the local Tauri frontend — it is never
/// logged. Returns `None` when the clipboard is empty or not text.
#[tauri::command]
fn read_host_clipboard_text(state: State<'_, AppState>) -> Option<String> {
    let lock = state.clipboard.grant.lock().unwrap();
    // Only return clipboard content when sync is active.
    if lock.as_ref().map_or(false, |g| g.can_host_push()) {
        drop(lock);
        clipboard::read_host_clipboard()
    } else {
        None
    }
}

/// Receives clipboard text from the remote client and writes it to the host clipboard.
///
/// Enforces the MigRoute clipboard policy (permission, direction, size, null-byte guard).
/// **The text content is never written to logs.**
#[tauri::command]
fn receive_remote_clipboard(
    state: State<'_, AppState>,
    device_id: String,
    text: String,
) -> Result<(), String> {
    let trust_store = state.trust_store.lock().unwrap();
    let id = DeviceId::try_from(device_id.as_str()).map_err(|e| e.to_string())?;
    let device = trust_store
        .get_device(&id)
        .ok_or_else(|| "device not in trust store".to_string())?;
    if device.is_revoked {
        return Err("device is revoked".to_string());
    }
    let perms: BTreeSet<SessionPermission> = device
        .granted_permissions
        .iter()
        .cloned()
        .collect();
    let mut grant_lock = state.clipboard.grant.lock().unwrap();
    let grant = grant_lock
        .as_mut()
        .ok_or_else(|| "no active clipboard grant".to_string())?;
    clipboard::write_host_clipboard(&text, grant, &perms)?;
    grant.client_push_count += 1;
    Ok(())
}

/// Returns the current clipboard sync status (active, direction, counters).
#[tauri::command]
fn get_clipboard_status(state: State<'_, AppState>) -> ClipboardStatus {
    clipboard::get_clipboard_status(&state.clipboard)
}

/// Deactivates and clears the clipboard grant (call on session end).
#[tauri::command]
fn deactivate_clipboard_sync(state: State<'_, AppState>) -> Result<(), String> {
    state.clipboard.clear();
    Ok(())
}

// ─── File Transfer / Migration Commands ──────────────────────────────────────

/// Prepares an outgoing file migration to a trusted remote peer.
#[tauri::command]
fn prepare_outgoing_transfer(
    state: State<'_, AppState>,
    device_id: String,
    file_path: String,
) -> Result<TransferSessionDto, String> {
    let trust_store = state.trust_store.lock().unwrap();
    let target_dev_id = DeviceId::try_from(device_id.as_str()).map_err(|e| e.to_string())?;
    let device = trust_store
        .get_device(&target_dev_id)
        .ok_or_else(|| "device not found in trust store".to_string())?;
    if device.is_revoked {
        return Err("device authorization is revoked".to_string());
    }
    if !device.has_permission(SessionPermission::SendFiles) {
        return Err("device lacks SEND_FILES capability".to_string());
    }
    let transfer_id = format!("tx-{}", now_ms());
    let path = PathBuf::from(&file_path);
    state.transfer.register_outgoing(transfer_id, path)
}

/// Reads a specific chunk for an outgoing file migration.
#[tauri::command]
fn read_outgoing_chunk(
    state: State<'_, AppState>,
    transfer_id: String,
    chunk_index: u64,
) -> Result<OutgoingChunkDto, String> {
    state.transfer.read_outgoing_chunk(&transfer_id, chunk_index)
}

/// Host operator authorizes an incoming file offer from a trusted peer.
#[tauri::command]
fn accept_incoming_transfer(
    state: State<'_, AppState>,
    device_id: String,
    transfer_id: String,
    file_name: String,
    total_bytes: u64,
    chunk_size: u32,
    total_chunks: u64,
    expected_sha256: String,
) -> Result<TransferSessionDto, String> {
    let trust_store = state.trust_store.lock().unwrap();
    let target_dev_id = DeviceId::try_from(device_id.as_str()).map_err(|e| e.to_string())?;
    let device = trust_store
        .get_device(&target_dev_id)
        .ok_or_else(|| "device not found in trust store".to_string())?;
    if device.is_revoked {
        return Err("device authorization is revoked".to_string());
    }
    if !device.has_permission(SessionPermission::ReceiveFiles) {
        return Err("device lacks RECEIVE_FILES capability".to_string());
    }
    state.transfer.register_incoming(
        transfer_id,
        file_name,
        total_bytes,
        chunk_size,
        total_chunks,
        expected_sha256,
    )
}

/// Ingests, validates SHA-256, and writes an incoming chunk to the staging file.
#[tauri::command]
fn write_incoming_chunk(
    state: State<'_, AppState>,
    transfer_id: String,
    chunk_index: u64,
    chunk_sha256: String,
    data_base64: String,
) -> Result<TransferProgressDto, String> {
    state.transfer.write_incoming_chunk(&transfer_id, chunk_index, &chunk_sha256, &data_base64)
}

/// Finalizes an incoming transfer by verifying the overall file SHA-256 and moving from staging.
#[tauri::command]
fn finalize_incoming_transfer(
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<FinalizeResultDto, String> {
    state.transfer.finalize_incoming(&transfer_id)
}

/// Pauses an active transfer.
#[tauri::command]
fn pause_transfer(state: State<'_, AppState>, transfer_id: String) -> Result<(), String> {
    state.transfer.pause(&transfer_id)
}

/// Resumes a paused transfer from a specific chunk index.
#[tauri::command]
fn resume_transfer(
    state: State<'_, AppState>,
    transfer_id: String,
    from_chunk: u64,
) -> Result<u64, String> {
    state.transfer.resume(&transfer_id, from_chunk)
}

/// Cancels a transfer and deletes any partial staging data.
#[tauri::command]
fn cancel_transfer(state: State<'_, AppState>, transfer_id: String) -> Result<(), String> {
    state.transfer.cancel(&transfer_id)
}

/// Lists all active and recent transfers.
#[tauri::command]
fn list_transfers(state: State<'_, AppState>) -> Vec<TransferSessionDto> {
    state.transfer.list_transfers()
}

/// Opens the verified Downloads/SmartMigrate folder in Windows Explorer.
#[tauri::command]
fn open_transfers_folder() -> Result<String, String> {
    let dir = TransferManager::get_download_dir();
    let dir_str = dir.to_string_lossy().to_string();
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(&dir).spawn();
    }
    Ok(dir_str)
}

/// Creates a sample test migration file in Downloads/SmartMigrate/Samples for testing.
#[tauri::command]
fn create_sample_migration_file(name: String, size_kb: u32) -> Result<String, String> {
    let dir = TransferManager::get_download_dir().join("Samples");
    let _ = std::fs::create_dir_all(&dir);
    let sanitized = migroute::sanitize_file_name(&name).map_err(|e| e.to_string())?;
    let path = dir.join(&sanitized);
    let mut file = File::create(&path).map_err(|e| e.to_string())?;
    let chunk = vec![b'S'; 1024];
    for _ in 0..size_kb {
        let _ = file.write_all(&chunk);
    }
    Ok(path.to_string_lossy().to_string())
}

/// Recursively scans a local folder directory and generates an authoritative SMP/1 migration manifest.
#[tauri::command]
fn scan_folder_for_migration(
    state: State<'_, AppState>,
    folder_path: String,
) -> Result<FolderScanResult, String> {
    let path = PathBuf::from(&folder_path);
    state.transfer.scan_folder_recursive(&path)
}

/// Computes delta sync requirements between a folder and remote target hashes.
#[tauri::command]
fn compute_folder_delta_sync(
    state: State<'_, AppState>,
    folder_path: String,
    target_hashes: std::collections::HashMap<String, String>,
) -> Result<DeltaSyncSummary, String> {
    let path = PathBuf::from(&folder_path);
    let scan = state.transfer.scan_folder_recursive(&path)?;
    Ok(state.transfer.compute_delta_sync(scan, &target_hashes))
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
    std::panic::set_hook(Box::new(|info| {
        let _ = std::fs::write("C:\\Smart Migrate\\crash.log", format!("{:#?}", info));
    }));

    let identity = storage::load_or_create_identity();
    let trust_store = storage::load_trust_store();
    let app_state = AppState::new(identity, trust_store);

    let _smp_stop = smp_server::start_smp_server(app_state.clone());

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
            inject_remote_mouse,
            inject_remote_keyboard,
            get_input_telemetry,
            set_input_override,
            get_resilience_status,
            activate_clipboard_sync,
            suspend_clipboard_sync,
            resume_clipboard_sync,
            read_host_clipboard_text,
            receive_remote_clipboard,
            get_clipboard_status,
            deactivate_clipboard_sync,
            prepare_outgoing_transfer,
            read_outgoing_chunk,
            accept_incoming_transfer,
            write_incoming_chunk,
            finalize_incoming_transfer,
            pause_transfer,
            resume_transfer,
            cancel_transfer,
            list_transfers,
            open_transfers_folder,
            create_sample_migration_file,
            scan_folder_for_migration,
            compute_folder_delta_sync,
            minimize_window,
            toggle_maximize,
            close_window,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            let _ = std::fs::write("C:\\Smart Migrate\\crash.log", format!("Tauri run error: {:#?}", e));
            panic!("failed to run Smart Migrate Windows host: {:#?}", e);
        });
}


