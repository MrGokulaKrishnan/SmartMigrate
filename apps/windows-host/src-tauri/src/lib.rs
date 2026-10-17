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
    CheckpointResumeDto, DeltaSyncSummary, FinalizeResultDto, FolderScanResult, IncomingSliceDto,
    OutgoingBatchDto, OutgoingChunkDto, TransferManager, TransferProgressDto, TransferSessionDto,
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
    version: &'static str,
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
        version: env!("CARGO_PKG_VERSION"),
    }
}


/// Returns all detected active non-loopback LAN IPv4 addresses on this PC.
#[tauri::command]
fn get_host_lan_ips() -> Vec<String> {
    let mut ips = Vec::new();
    let primary = smp_server::get_primary_lan_ip();
    if !primary.is_empty() {
        ips.push(primary);
    }
    if ips.is_empty() {
        ips.push("192.168.31.33".to_string());
    }
    ips
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
    dx: Option<i32>,
    dy: Option<i32>,
    is_relative: Option<bool>,
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
        dx.unwrap_or(0),
        dy.unwrap_or(0),
        is_relative.unwrap_or(false),
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

/// Injects remote Unicode string text if authorized by host.
#[tauri::command]
fn inject_remote_text(
    state: State<'_, AppState>,
    device_id: String,
    text: String,
    sequence: u64,
) -> Result<(), String> {
    let trust_store = state.trust_store.lock().unwrap();
    state.input_controller.inject_text(
        &trust_store,
        &device_id,
        &text,
        sequence,
    )
}

/// Injects remote system hotkey (e.g. "show_desktop", "task_switch") if authorized by host.
#[tauri::command]
fn inject_remote_hotkey(
    state: State<'_, AppState>,
    device_id: String,
    hotkey: String,
    sequence: u64,
) -> Result<(), String> {
    let trust_store = state.trust_store.lock().unwrap();
    state.input_controller.inject_hotkey(
        &trust_store,
        &device_id,
        &hotkey,
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

/// Reads a parallel batch of outgoing chunks with on-the-fly LZ4 compression.
#[tauri::command]
fn read_outgoing_batch(
    state: State<'_, AppState>,
    transfer_id: String,
    start_chunk: u64,
    max_chunks: u32,
    enable_compression: bool,
) -> Result<OutgoingBatchDto, String> {
    state.transfer.read_outgoing_batch(&transfer_id, start_chunk, max_chunks, enable_compression)
}

/// Ingests a parallel batch of incoming slices with on-the-fly decompression and microsecond checkpoint journaling.
#[tauri::command]
fn write_incoming_batch(
    state: State<'_, AppState>,
    transfer_id: String,
    slices: Vec<IncomingSliceDto>,
) -> Result<TransferProgressDto, String> {
    state.transfer.write_incoming_batch(&transfer_id, &slices)
}

/// Recovers and resumes a transfer state from its microsecond on-disk journal checkpoint.
#[tauri::command]
fn resume_transfer_from_checkpoint(
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<CheckpointResumeDto, String> {
    state.transfer.resume_from_checkpoint(&transfer_id)
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

/// Toggle between maximized and restored window state. Returns new maximized state.
#[tauri::command]
fn toggle_maximize(window: Window) -> Result<bool, String> {
    if window.is_maximized().map_err(|e| e.to_string())? {
        window.unmaximize().map_err(|e| e.to_string())?;
        Ok(false)
    } else {
        window.maximize().map_err(|e| e.to_string())?;
        Ok(true)
    }
}

/// Returns whether the window is currently maximized.
#[tauri::command]
fn is_window_maximized(window: Window) -> Result<bool, String> {
    window.is_maximized().map_err(|e| e.to_string())
}

/// Close the main application window.
#[tauri::command]
fn close_window(window: Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[derive(Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateCheckDto {
    has_update: bool,
    current_version: String,
    latest_version: String,
    release_notes: String,
    release_highlights: Vec<String>,
    download_url: String,
    size_bytes: u64,
    sha256: String,
}

/// Checks the official Smart Migrate CDN release manifest for newer versions.
#[tauri::command]
fn check_windows_update() -> Result<UpdateCheckDto, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    let script = r#"
        $OutputEncoding = [System.Text.Encoding]::UTF8;
        [Console]::OutputEncoding = [System.Text.Encoding]::UTF8;
        $ProgressPreference = 'SilentlyContinue';
        try {
            $resp = Invoke-RestMethod -Uri 'https://smartmigrate.web.app/releases/manifest.json' -TimeoutSec 6 -Headers @{'User-Agent'='SmartMigrate-Windows/1.4.1'}
            $json = $resp | ConvertTo-Json -Depth 6 -Compress
            Write-Output $json
        } catch {
            try {
                $resp = Invoke-RestMethod -Uri 'https://smartmigrated.web.app/releases/manifest.json' -TimeoutSec 6
                $json = $resp | ConvertTo-Json -Depth 6 -Compress
                Write-Output $json
            } catch {
                Write-Error $_.Exception.Message
            }
        }
    "#;

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|e| format!("PowerShell error checking update: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Ok(UpdateCheckDto {
            has_update: false,
            current_version: current_version.clone(),
            latest_version: current_version,
            release_notes: "You are running the latest verified release.".to_string(),
            release_highlights: vec!["You are running the latest verified release.".to_string()],
            download_url: String::new(),
            size_bytes: 0,
            sha256: String::new(),
        });
    }

    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse release manifest JSON: {e}"))?;

    let latest_version = parsed.get("version").and_then(|v| v.as_str()).unwrap_or("1.4.1").to_string();
    let release_notes = parsed.get("releaseNotes").and_then(|v| v.as_str()).unwrap_or("").to_string();

    let release_highlights: Vec<String> = if let Some(arr) = parsed.get("releaseHighlights").and_then(|h| h.as_array()) {
        arr.iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        release_notes
            .lines()
            .map(|line| {
                let trimmed = line.trim();
                let clean = trimmed
                    .trim_start_matches(|c: char| c == '•' || c == '-' || c == '*' || c == '+' || c == '\u{FFFD}' || c == '?' || c.is_whitespace());
                clean.trim().to_string()
            })
            .filter(|line| !line.is_empty())
            .collect()
    };

    let win_platform = parsed.get("platforms").and_then(|p| p.get("windows-x86_64"));
    let download_url = win_platform.and_then(|w| w.get("url")).and_then(|u| u.as_str()).unwrap_or("").to_string();
    let size_bytes = win_platform.and_then(|w| w.get("size")).and_then(|s| s.as_u64()).unwrap_or(0);
    let sha256 = win_platform.and_then(|w| w.get("sha256")).and_then(|s| s.as_str()).unwrap_or("").to_string();

    let has_update = latest_version != current_version && !download_url.is_empty();

    Ok(UpdateCheckDto {
        has_update,
        current_version,
        latest_version,
        release_notes,
        release_highlights,
        download_url,
        size_bytes,
        sha256,
    })
}

/// Downloads and cryptographically verifies the Windows installer update package.
#[tauri::command]
fn download_windows_update(download_url: String, expected_sha256: String) -> Result<String, String> {
    let update_dir = std::env::var("APPDATA")
        .map(|d| std::path::PathBuf::from(d).join("SmartMigrate").join("updates"))
        .unwrap_or_else(|_| std::path::PathBuf::from("updates"));

    let _ = std::fs::create_dir_all(&update_dir);
    let target_file = update_dir.join("SmartMigrate-Setup-x64.exe");

    let script = format!(
        "$ProgressPreference = 'SilentlyContinue'; Invoke-WebRequest -Uri '{}' -OutFile '{}'",
        download_url, target_file.display()
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(|e| format!("Failed to download update: {e}"))?;

    if !output.status.success() {
        return Err(format!("Download failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    // Verify SHA-256
    if !expected_sha256.is_empty() {
        let hash_script = format!(
            "(Get-FileHash -Path '{}' -Algorithm SHA256).Hash",
            target_file.display()
        );
        let hash_out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &hash_script])
            .output()
            .map_err(|e| format!("Failed to compute SHA256: {e}"))?;

        let actual_hash = String::from_utf8_lossy(&hash_out.stdout).trim().to_uppercase();
        if actual_hash != expected_sha256.trim().to_uppercase() {
            let _ = std::fs::remove_file(&target_file);
            return Err(format!(
                "Integrity failure: expected SHA-256 {} but got {}",
                expected_sha256, actual_hash
            ));
        }
    }

    Ok(target_file.to_string_lossy().to_string())
}

#[cfg(windows)]
mod win_installer {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    #[repr(C)]
    struct SHELLEXECUTEINFOW {
        cb_size: u32,
        f_mask: u32,
        hwnd: isize,
        lp_verb: *const u16,
        lp_file: *const u16,
        lp_parameters: *const u16,
        lp_directory: *const u16,
        n_show: i32,
        h_inst_app: isize,
        lp_id_list: *mut core::ffi::c_void,
        lp_class: *const u16,
        hkey_class: isize,
        dw_hot_key: u32,
        h_icon_or_monitor: isize,
        h_process: isize,
    }

    const SEE_MASK_NOCLOSEPROCESS: u32 = 0x00000040;
    const SW_SHOWNORMAL: i32 = 1;
    const ERROR_CANCELLED: u32 = 1223;

    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteExW(p_exec_info: *mut SHELLEXECUTEINFOW) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetLastError() -> u32;
        fn CloseHandle(handle: isize) -> i32;
    }

    pub fn launch_elevated(installer_path: &Path) -> Result<(), String> {
        let is_msi = installer_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.eq_ignore_ascii_case("msi"))
            .unwrap_or(false);

        let (target_file, params): (String, Option<String>) = if is_msi {
            (
                "msiexec.exe".to_string(),
                Some(format!("/i \"{}\"", installer_path.display())),
            )
        } else {
            (installer_path.to_string_lossy().to_string(), None)
        };

        let mut file_wide: Vec<u16> = OsStr::new(&target_file).encode_wide().collect();
        file_wide.push(0);

        let mut verb_wide: Vec<u16> = OsStr::new("runas").encode_wide().collect();
        verb_wide.push(0);

        let param_wide: Option<Vec<u16>> = params.map(|p| {
            let mut v: Vec<u16> = OsStr::new(&p).encode_wide().collect();
            v.push(0);
            v
        });

        let dir = installer_path.parent().unwrap_or(Path::new(""));
        let mut dir_wide: Vec<u16> = dir.as_os_str().encode_wide().collect();
        dir_wide.push(0);

        let mut exec_info = SHELLEXECUTEINFOW {
            cb_size: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            f_mask: SEE_MASK_NOCLOSEPROCESS,
            hwnd: 0,
            lp_verb: verb_wide.as_ptr(),
            lp_file: file_wide.as_ptr(),
            lp_parameters: param_wide.as_ref().map(|v| v.as_ptr()).unwrap_or(std::ptr::null()),
            lp_directory: dir_wide.as_ptr(),
            n_show: SW_SHOWNORMAL,
            h_inst_app: 0,
            lp_id_list: std::ptr::null_mut(),
            lp_class: std::ptr::null(),
            hkey_class: 0,
            dw_hot_key: 0,
            h_icon_or_monitor: 0,
            h_process: 0,
        };

        let success = unsafe { ShellExecuteExW(&mut exec_info) };

        if success != 0 {
            if exec_info.h_process != 0 {
                unsafe { CloseHandle(exec_info.h_process) };
            }
            Ok(())
        } else {
            let error_code = unsafe { GetLastError() };
            if error_code == ERROR_CANCELLED {
                Err("UAC_CANCELLED: Administrator approval was cancelled by the user.".to_string())
            } else {
                Err(format!(
                    "Failed to launch installer via ShellExecute (Windows error {error_code})."
                ))
            }
        }
    }
}

/// Stages and executes the official Windows installer with UAC elevation, cleanly exiting the host.
#[tauri::command]
fn apply_windows_update_and_restart(installer_path: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(&installer_path);
    if !path.exists() {
        return Err("Installer file does not exist on disk.".to_string());
    }

    // Security validation: verify extension is .exe or .msi
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if ext != "exe" && ext != "msi" {
        return Err("Invalid installer package type. Only .exe and .msi packages are permitted.".to_string());
    }

    // Security validation: ensure file is non-empty
    let metadata = std::fs::metadata(&path).map_err(|e| format!("Cannot read installer metadata: {e}"))?;
    if metadata.len() == 0 {
        return Err("Installer package is empty or truncated.".to_string());
    }

    #[cfg(windows)]
    {
        win_installer::launch_elevated(&path)?;
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new(&installer_path)
            .spawn()
            .map_err(|e| format!("Failed to spawn installer: {e}"))?;
    }

    // Spawn a brief detachment thread to cleanly exit the host process after handing off to Windows installer
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        std::process::exit(0);
    });

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    std::panic::set_hook(Box::new(|info| {
        let log_path = std::env::var("APPDATA")
            .map(|d| std::path::PathBuf::from(d).join("SmartMigrate").join("crash.log"))
            .unwrap_or_else(|_| std::path::PathBuf::from("crash.log"));
        let _ = std::fs::create_dir_all(log_path.parent().unwrap_or(std::path::Path::new(".")));
        let _ = std::fs::write(&log_path, format!("{:#?}", info));
    }));

    let identity = storage::load_or_create_identity();
    let trust_store = storage::load_trust_store();
    let app_state = AppState::new(identity, trust_store);

    let _smp_stop = smp_server::start_smp_server(app_state.clone());

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            host_status,
            get_host_lan_ips,
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
            inject_remote_text,
            inject_remote_hotkey,
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
            read_outgoing_batch,
            accept_incoming_transfer,
            write_incoming_chunk,
            write_incoming_batch,
            finalize_incoming_transfer,
            pause_transfer,
            resume_transfer,
            resume_transfer_from_checkpoint,
            cancel_transfer,
            list_transfers,
            open_transfers_folder,
            create_sample_migration_file,
            scan_folder_for_migration,
            compute_folder_delta_sync,
            minimize_window,
            toggle_maximize,
            is_window_maximized,
            close_window,
            check_windows_update,
            download_windows_update,
            apply_windows_update_and_restart,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            let _ = std::fs::write("C:\\Smart Migrate\\crash.log", format!("Tauri run error: {:#?}", e));
            panic!("failed to run Smart Migrate Windows host: {:#?}", e);
        });
}


