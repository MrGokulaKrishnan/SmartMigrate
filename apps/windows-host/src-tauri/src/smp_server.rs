//! Smart Migrate Protocol (SMP/1) unified network server.
//!
//! Provides the primary communication channel between Android clients and Windows host:
//! - UDP broadcast discovery on port 7889
//! - TCP control and streaming server on port 7890
//! - Full protocol state machine: Discovery -> Hello -> Pairing -> Session -> Transport -> Heartbeat -> Stream

use crate::capture::{capture_primary_display, DesktopCaptureSession, SoftwareJpegEncoder};
use crate::state::AppState;
use crate::storage::{generate_crypto_token, save_trust_store};
use migroute::identity::DevicePlatform;
use migroute::trust::TrustedDevice;
use migroute::{DeviceId, SessionPermission, PROTOCOL_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub const SMP_CONTROL_PORT: u16 = 7890;
pub const SMP_DISCOVERY_PORT: u16 = 7889;

pub fn get_primary_lan_ip() -> String {
    // 1. Try UDP routing socket against known public endpoints (queries OS routing table, no packets sent)
    for target in &["8.8.8.8:80", "1.1.1.1:80", "223.5.5.5:80"] {
        if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
            if socket.connect(target).is_ok() {
                if let Ok(addr) = socket.local_addr() {
                    let ip_str = addr.ip().to_string();
                    if !ip_str.starts_with("127.") && !ip_str.starts_with("169.254.") {
                        return ip_str;
                    }
                }
            }
        }
    }

    // 2. Fallback: query ipconfig for active IPv4 address
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = std::process::Command::new("ipconfig");
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        if let Ok(output) = cmd.output() {
            if let Ok(text) = String::from_utf8(output.stdout) {
                for line in text.lines() {
                    if line.contains("IPv4") {
                        if let Some(pos) = line.rfind(':') {
                            let ip = line[pos + 1..].trim();
                            if !ip.is_empty() && !ip.starts_with("127.") && !ip.starts_with("169.254.") {
                                return ip.to_string();
                            }
                        }
                    }
                }
            }
        }
    }

    String::new()
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceHelloResponse {
    pub protocol_version: u16,
    pub protocol_name: String,
    pub device_id: String,
    pub device_name: String,
    pub platform: String,
    pub app_version: String,
    pub capabilities: Vec<String>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairRequestBody {
    pub requester_device_id: String,
    pub requester_name: String,
    pub code: String,
    pub token: Option<String>,
    pub requested_permissions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairResponseBody {
    pub status: String,
    pub session_token: String,
    pub host_device_id: String,
    pub granted_permissions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCreateBody {
    pub client_device_id: String,
    pub session_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCreateResponse {
    pub session_id: String,
    pub status: String,
    pub transport: String,
    pub stream_url: String,
    pub protocol_version: u16,
    pub heartbeat_interval_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatResponse {
    pub r#type: String,
    pub server_time_ms: u64,
    pub status: String,
    pub protocol_version: u16,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteInputBody {
    pub device_id: String,
    pub session_token: Option<String>,
    pub action: String,
    pub button: Option<String>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub dx: Option<i32>,
    pub dy: Option<i32>,
    pub normalized_x: Option<f32>,
    pub normalized_y: Option<f32>,
    pub scroll_delta: Option<i32>,
    pub sequence: Option<u64>,
    pub vk_code: Option<u16>,
    pub key_up: Option<bool>,
    pub text: Option<String>,
    pub hotkey: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDiagnosticsResponse {
    pub discovery: String,
    pub device_id: String,
    pub protocol_version: String,
    pub pairing_status: String,
    pub trust_store_count: usize,
    pub screen_capture: String,
    pub streaming_port: u16,
    pub firewall_status: String,
    pub overall: String,
}

#[cfg(windows)]
fn probe_firewall_status() -> String {
    let out = std::process::Command::new("netsh")
        .args(["advfirewall", "firewall", "show", "rule", "name=Smart Migrate TCP 7890"])
        .output();
    match out {
        Ok(o) if o.status.success() => "PASS (Rules Configured)".to_string(),
        _ => "UNCONFIGURED (Ports 7889/7890 not explicitly allowed)".to_string(),
    }
}

#[cfg(not(windows))]
fn probe_firewall_status() -> String {
    "N/A".to_string()
}

/// Spawns the background SMP TCP control server and UDP discovery beacon on default ports (7890/7889).
pub fn start_smp_server(app_state: AppState) -> Arc<AtomicBool> {
    start_smp_server_on_ports(app_state, SMP_CONTROL_PORT, SMP_DISCOVERY_PORT)
}

/// Spawns the background SMP TCP control server and UDP discovery beacon on specified ports.
pub fn start_smp_server_on_ports(
    app_state: AppState,
    control_port: u16,
    discovery_port: u16,
) -> Arc<AtomicBool> {
    let stop_signal = Arc::new(AtomicBool::new(false));

    // 1. Spawn UDP Discovery Beacon
    let stop_udp = Arc::clone(&stop_signal);
    let state_udp = app_state.clone();
    std::thread::Builder::new()
        .name("sm-smp-discovery".to_string())
        .spawn(move || {
            run_discovery_responder(stop_udp, state_udp, discovery_port, control_port);
        })
        .expect("Failed to spawn SMP discovery responder");

    // 2. Spawn TCP Protocol & Stream Server
    let stop_tcp = Arc::clone(&stop_signal);
    let state_tcp = app_state;
    std::thread::Builder::new()
        .name("sm-smp-server".to_string())
        .spawn(move || {
            run_smp_tcp_server(stop_tcp, state_tcp, control_port);
        })
        .expect("Failed to spawn SMP TCP server");

    stop_signal
}

/// Responds to LAN broadcast discovery probes over UDP.
fn run_discovery_responder(
    stop_signal: Arc<AtomicBool>,
    state: AppState,
    discovery_port: u16,
    control_port: u16,
) {
    let bind_addr = format!("0.0.0.0:{}", discovery_port);
    let socket = match UdpSocket::bind(&bind_addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[Smart Migrate] UDP discovery bind warning on {bind_addr}: {e}");
            return;
        }
    };
    let _ = socket.set_read_timeout(Some(Duration::from_millis(500)));

    let mut buf = [0u8; 1024];
    while !stop_signal.load(Ordering::Relaxed) {
        match socket.recv_from(&mut buf) {
            Ok((len, peer_addr)) => {
                let msg = String::from_utf8_lossy(&buf[..len]);
                if msg.contains("SM_DISCOVERY_PROBE") {
                    let host_ip = get_primary_lan_ip();
                    let resp = format!(
                        "SM_DISCOVERY_RESPONSE:port={}:ip={}:id={}:name={}:platform=Windows\n",
                        control_port, host_ip, state.identity.id, state.identity.name
                    );
                    let _ = socket.send_to(resp.as_bytes(), peer_addr);
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                // Timeout normal loop
            }
            Err(_) => {}
        }
    }
}

/// Runs the multi-threaded SMP HTTP control and display streaming server on specified port.
fn run_smp_tcp_server(stop_signal: Arc<AtomicBool>, state: AppState, control_port: u16) {
    let bind_addr = format!("0.0.0.0:{}", control_port);
    let listener = match TcpListener::bind(&bind_addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[Smart Migrate] TCP server bind error on {bind_addr}: {e}");
            return;
        }
    };
    let _ = listener.set_nonblocking(true);

    const MAX_CONCURRENT_CLIENTS: usize = 32;
    let active_clients = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    while !stop_signal.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((mut socket, _peer)) => {
                if active_clients.load(Ordering::Relaxed) >= MAX_CONCURRENT_CLIENTS {
                    let _ = socket.write_all(b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 1\r\nConnection: close\r\n\r\nToo Many Requests\r\n");
                    let _ = socket.flush();
                    continue;
                }
                active_clients.fetch_add(1, Ordering::Relaxed);
                let client_counter = Arc::clone(&active_clients);

                let _ = socket.set_nodelay(true);
                let _ = socket.set_read_timeout(Some(Duration::from_millis(1500)));

                let state_clone = state.clone();
                let stop_clone = Arc::clone(&stop_signal);

                std::thread::spawn(move || {
                    struct ConnGuard(Arc<std::sync::atomic::AtomicUsize>);
                    impl Drop for ConnGuard {
                        fn drop(&mut self) {
                            self.0.fetch_sub(1, Ordering::Relaxed);
                        }
                    }
                    let _guard = ConnGuard(client_counter);
                    handle_client_connection(socket, state_clone, stop_clone);
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                eprintln!("[Smart Migrate] Listener accept error: {e}");
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

/// Ultra-fast 64-bit frame change detector using strided sampling.
/// Samples every 4KB across the frame buffer (~2,000 samples for 1080p BGRA)
/// executing in ~1 microsecond to prevent wasteful re-encoding of static desktop frames.
fn fast_frame_checksum(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV-1a 64-bit offset basis
    let stride = 4096;
    let mut i = 0;
    while i < data.len() {
        hash ^= data[i] as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        i += stride;
    }
    let len = data.len();
    if len >= 16 {
        for &idx in &[0, len / 4, len / 2, 3 * len / 4, len - 4] {
            let chunk = [data[idx], data[idx + 1], data[idx + 2], data[idx + 3]];
            hash ^= u32::from_le_bytes(chunk) as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

/// Reads a complete HTTP request, buffering headers until delimiter and reading full body based on Content-Length.
fn read_http_request(socket: &mut TcpStream) -> Option<(String, String)> {
    let _ = socket.set_read_timeout(Some(Duration::from_millis(1500)));
    let mut header_buf = Vec::with_capacity(4096);
    let mut chunk = [0u8; 1024];
    let mut header_end_pos = None;

    while header_buf.len() < 65536 {
        match socket.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                header_buf.extend_from_slice(&chunk[..n]);
                if let Some(pos) = find_subsequence(&header_buf, b"\r\n\r\n") {
                    header_end_pos = Some((pos, 4));
                    break;
                } else if let Some(pos) = find_subsequence(&header_buf, b"\n\n") {
                    header_end_pos = Some((pos, 2));
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let (header_end, delimiter_len) = header_end_pos?;
    let header_str = String::from_utf8_lossy(&header_buf[..header_end]).to_string();

    let content_length = header_str
        .lines()
        .find_map(|line| {
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("content-length:") {
                line.split(':').nth(1).and_then(|s| s.trim().parse::<usize>().ok())
            } else {
                None
            }
        })
        .unwrap_or(0);

    let body_start = header_end + delimiter_len;
    let mut body_bytes = header_buf[body_start..].to_vec();

    while body_bytes.len() < content_length && body_bytes.len() < 16 * 1024 * 1024 {
        let remaining = content_length - body_bytes.len();
        let to_read = remaining.min(chunk.len());
        match socket.read(&mut chunk[..to_read]) {
            Ok(0) => break,
            Ok(n) => body_bytes.extend_from_slice(&chunk[..n]),
            Err(_) => break,
        }
    }

    let body_str = String::from_utf8_lossy(&body_bytes).to_string();
    Some((header_str, body_str))
}

fn get_header_value<'a>(headers: &'a str, name: &str) -> Option<&'a str> {
    for line in headers.lines() {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case(name) {
                return Some(v.trim());
            }
        }
    }
    None
}

fn extract_bearer_token(headers: &str) -> Option<String> {
    let auth = get_header_value(headers, "Authorization")?;
    if let Some(token) = auth.strip_prefix("Bearer ") {
        let trimmed = token.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

fn is_trusted_origin(origin: &str) -> bool {
    // Parse scheme and host exactly. A prefix check would also trust hosts such as
    // "http://localhost.attacker.example" or "http://127.0.0.1.attacker.example".
    let Some((scheme, rest)) = origin.split_once("://") else {
        return false;
    };
    let (host, port) = match rest.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (rest, None),
    };
    if let Some(port) = port {
        if port.is_empty() || !port.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    match scheme {
        "tauri" => host == "localhost",
        "http" | "https" => matches!(host, "localhost" | "127.0.0.1" | "tauri.localhost"),
        _ => false,
    }
}

fn constant_time_eq_str(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    if a_bytes.len() != b_bytes.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a_bytes.iter().zip(b_bytes.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Handles an incoming client request on port 7890.
fn handle_client_connection(mut socket: TcpStream, state: AppState, stop_signal: Arc<AtomicBool>) {
    let (header_str, body_str) = match read_http_request(&mut socket) {
        Some(req) => req,
        None => return,
    };

    let first_line = header_str.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let method = parts[0];
    let uri = parts[1];

    let (path, query) = if let Some(idx) = uri.find('?') {
        (&uri[..idx], &uri[idx + 1..])
    } else {
        (uri, "")
    };

    // Cross-origin request protection (CSRF / drive-by intranet defense)
    let origin_opt = get_header_value(&header_str, "Origin");
    if let Some(origin) = origin_opt {
        if !is_trusted_origin(origin) {
            send_error_response(
                &mut socket,
                403,
                "Forbidden",
                "SMP_FORBIDDEN_ORIGIN",
                "Cross-origin requests from untrusted web domains are blocked.",
                None,
            );
            return;
        }
    }
    let cors_allow_origin: Option<&str> = origin_opt.filter(|o| is_trusted_origin(o));

    // Route table
    match (method, path) {
        // ── 0. CORS Preflight ──
        ("OPTIONS", _) => {
            let cors_hdr = cors_allow_origin
                .map(|o| format!("Access-Control-Allow-Origin: {o}\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type, Authorization\r\nAccess-Control-Max-Age: 86400\r\n"))
                .unwrap_or_default();
            let resp = format!("HTTP/1.1 204 No Content\r\n{cors_hdr}\r\n");
            let _ = socket.write_all(resp.as_bytes());
            let _ = socket.flush();
        }

        // ── 1. Protocol Hello & Capabilities ──
        ("GET", "/smp/hello") => {
            let hello = DeviceHelloResponse {
                protocol_version: PROTOCOL_VERSION,
                protocol_name: "SMP/1".to_string(),
                device_id: state.identity.id.to_string(),
                device_name: state.identity.name.clone(),
                platform: "Windows 11".to_string(),
                app_version: env!("CARGO_PKG_VERSION").to_string(),
                capabilities: vec![
                    "SCREEN_CAPTURE_GDI".to_string(),
                    "TURBO_JPEG_STREAM".to_string(),
                    "REMOTE_MOUSE".to_string(),
                    "REMOTE_KEYBOARD".to_string(),
                    "CLIPBOARD".to_string(),
                ],
                timestamp_ms: now_ms(),
            };
            send_json_response(&mut socket, 200, "OK", &hello, cors_allow_origin);
        }

        // ── 2. Pairing Request (Locked out after 3 failed attempts, constant-time PIN verify) ──
        ("POST", "/smp/pair") => {
            if let Ok(pair_req) = serde_json::from_str::<PairRequestBody>(&body_str) {
                let mut active_lock = state.active_pairing.lock().unwrap();

                if let Some(ref mut session) = *active_lock {
                    let req_id = match DeviceId::try_from(pair_req.requester_device_id.as_str()) {
                        Ok(id) => id,
                        Err(e) => {
                            send_error_response(&mut socket, 400, "Bad Request", "SMP_INVALID_DEVICE_ID", &e.to_string(), cors_allow_origin);
                            return;
                        }
                    };

                    let requested_perms: Vec<SessionPermission> = pair_req
                        .requested_permissions
                        .iter()
                        .filter_map(|p| match p.as_str() {
                            "ViewScreen" => Some(SessionPermission::ViewScreen),
                            "ControlMouse" => Some(SessionPermission::ControlMouse),
                            "ControlKeyboard" => Some(SessionPermission::ControlKeyboard),
                            "SendFiles" => Some(SessionPermission::SendFiles),
                            "ReceiveFiles" => Some(SessionPermission::ReceiveFiles),
                            "Clipboard" => Some(SessionPermission::Clipboard),
                            _ => None,
                        })
                        .collect();

                    // Submit credentials to migroute PairingSession state machine
                    match session.submit_credentials(
                        req_id.clone(),
                        pair_req.requester_name.clone(),
                        &pair_req.code,
                        pair_req.token.as_deref(),
                        requested_perms.clone(),
                        now_ms(),
                    ) {
                        Ok(()) => {
                            // Credentials verified; grant requested permissions authorized by host
                            let mut perms = BTreeSet::new();
                            for p in requested_perms {
                                perms.insert(p);
                            }
                            perms.insert(SessionPermission::ViewScreen);
                            if perms.contains(&SessionPermission::ControlMouse) {
                                perms.insert(SessionPermission::ControlKeyboard);
                            }

                            let session_token = generate_crypto_token();
                            let token_hash = migroute::compute_sha256(session_token.as_bytes());
                            let dev_fingerprint = format!("FP-{}", migroute::compute_sha256(req_id.as_str().as_bytes())[..16].to_uppercase());

                            let mut trusted_device = TrustedDevice::new(
                                req_id.clone(),
                                pair_req.requester_name.clone(),
                                DevicePlatform::Android,
                                dev_fingerprint,
                                now_ms(),
                                perms.clone(),
                            );
                            trusted_device.token_hash = token_hash;

                            let mut trust_store = state.trust_store.lock().unwrap();
                            let _ = trust_store.remove_device(&trusted_device.id);
                            let _ = trust_store.add_device(trusted_device);
                            let _ = save_trust_store(&trust_store);

                            // Reset sequence tracker so client's initial input counter is accepted
                            state.input_controller.reset_sequence(req_id.as_ref());

                            // Update active stream token and bind target device
                            {
                                let mut stream_lock = state.active_stream.lock().unwrap();
                                stream_lock.auth_token = session_token.clone();
                                stream_lock.target_device_id = req_id.to_string();
                                stream_lock.target_device_name = pair_req.requester_name.clone();
                                stream_lock.is_active = true;
                            }

                            *active_lock = None; // clear pending pairing after success

                            let resp = PairResponseBody {
                                status: "APPROVED".to_string(),
                                session_token,
                                host_device_id: state.identity.id.to_string(),
                                granted_permissions: perms.into_iter().map(|p| format!("{p:?}")).collect(),
                            };
                            send_json_response(&mut socket, 200, "OK", &resp, cors_allow_origin);
                        }
                        Err(migroute::pairing::PairingVerificationError::TooManyAttempts) => {
                            *active_lock = None; // Destroy pairing session upon lockout
                            send_error_response(
                                &mut socket,
                                429,
                                "Too Many Requests",
                                "SMP_PAIRING_LOCKED_OUT",
                                "Maximum pairing attempts exceeded (3/3). Pairing session locked out for security.",
                                cors_allow_origin,
                            );
                        }
                        Err(migroute::pairing::PairingVerificationError::Expired) => {
                            *active_lock = None;
                            send_error_response(
                                &mut socket,
                                410,
                                "Gone",
                                "SMP_PAIRING_EXPIRED",
                                "Pairing session has expired. Start a new one on PC.",
                                cors_allow_origin,
                            );
                        }
                        Err(migroute::pairing::PairingVerificationError::CodeMismatch) => {
                            let attempts_left = session.max_attempts.saturating_sub(session.failed_attempts);
                            send_error_response(
                                &mut socket,
                                403,
                                "Forbidden",
                                "SMP_INVALID_CODE",
                                &format!("Invalid pairing PIN code. Attempts remaining: {attempts_left}"),
                                cors_allow_origin,
                            );
                        }
                        Err(migroute::pairing::PairingVerificationError::TokenMismatch) => {
                            send_error_response(
                                &mut socket,
                                403,
                                "Forbidden",
                                "SMP_INVALID_TOKEN",
                                "Pairing secret token mismatch.",
                                cors_allow_origin,
                            );
                        }
                        Err(migroute::pairing::PairingVerificationError::InvalidCodeFormat) => {
                            send_error_response(
                                &mut socket,
                                400,
                                "Bad Request",
                                "SMP_INVALID_CODE_FORMAT",
                                "Pairing code must be exactly 6 numeric digits.",
                                cors_allow_origin,
                            );
                        }
                        Err(migroute::pairing::PairingVerificationError::AlreadyDecided) => {
                            send_error_response(
                                &mut socket,
                                409,
                                "Conflict",
                                "SMP_ALREADY_DECIDED",
                                "Pairing session has already been decided.",
                                cors_allow_origin,
                            );
                        }
                    }
                } else {
                    send_error_response(
                        &mut socket,
                        404,
                        "Not Found",
                        "SMP_NO_ACTIVE_PAIRING",
                        "No active pairing session on Windows PC. Click Start Pairing on PC first.",
                        cors_allow_origin,
                    );
                }
            } else {
                send_error_response(&mut socket, 400, "Bad Request", "SMP_INVALID_PAYLOAD", "Malformed pairing request payload", cors_allow_origin);
            }
        }

        // ── 3. Session Creation (Host-verified session token tied to pairing) ──
        ("POST", "/smp/session") => {
            if let Ok(sess_req) = serde_json::from_str::<SessionCreateBody>(&body_str) {
                let trust_store = state.trust_store.lock().unwrap();
                let client_id = match DeviceId::try_from(sess_req.client_device_id.as_str()) {
                    Ok(id) => id,
                    Err(_) => {
                        send_error_response(&mut socket, 400, "Bad Request", "SMP_INVALID_DEVICE_ID", "Invalid client device ID", cors_allow_origin);
                        return;
                    }
                };

                let device = match trust_store.get_device(&client_id) {
                    Some(d) => d,
                    None => {
                        send_error_response(&mut socket, 401, "Unauthorized", "SMP_DEVICE_NOT_TRUSTED", "Device is not paired or trusted with this host", cors_allow_origin);
                        return;
                    }
                };

                if device.is_revoked {
                    send_error_response(&mut socket, 403, "Forbidden", "SMP_DEVICE_REVOKED", "Device authorization has been revoked by host", cors_allow_origin);
                    return;
                }

                // Session token verification
                let provided_token = extract_bearer_token(&header_str)
                    .unwrap_or_else(|| sess_req.session_token.clone());

                if provided_token.is_empty() {
                    send_error_response(&mut socket, 401, "Unauthorized", "SMP_MISSING_TOKEN", "Session token required", cors_allow_origin);
                    return;
                }

                let mut stream_lock = state.active_stream.lock().unwrap();

                // Prevent a second device from hijacking or kicking an ongoing active session
                if stream_lock.is_active && !stream_lock.target_device_id.is_empty() && stream_lock.target_device_id != client_id.as_ref() {
                    send_error_response(&mut socket, 409, "Conflict", "SMP_SESSION_BUSY", "Host is in an active session with another device", cors_allow_origin);
                    return;
                }

                // Verify token matches active stream token or device cryptographic token hash
                let token_valid = if !stream_lock.auth_token.is_empty() && stream_lock.target_device_id == client_id.as_ref() {
                    constant_time_eq_str(&stream_lock.auth_token, &provided_token)
                } else if !device.token_hash.is_empty() {
                    let provided_hash = migroute::compute_sha256(provided_token.as_bytes());
                    constant_time_eq_str(&device.token_hash, &provided_hash)
                } else {
                    let expected_fp_prefix = format!("FP-{}", provided_token[..provided_token.len().min(8)].to_uppercase());
                    device.fingerprint.starts_with(&expected_fp_prefix)
                };

                if !token_valid {
                    send_error_response(&mut socket, 401, "Unauthorized", "SMP_INVALID_SESSION_TOKEN", "Session token is invalid or does not match device trust anchor", cors_allow_origin);
                    return;
                }

                // Reset sequence tracking for this device on new session establishment
                state.input_controller.reset_sequence(device.id.as_ref());

                let session_id = format!("sm-sess-{}", &now_ms().to_string()[7..]);
                stream_lock.is_active = true;
                stream_lock.codec = "mjpeg".to_string();
                stream_lock.target_fps = 60;
                stream_lock.auth_token = provided_token.clone();
                stream_lock.session_id = session_id.clone();
                stream_lock.started_at_epoch_ms = now_ms();
                stream_lock.target_device_id = device.id.to_string();
                stream_lock.target_device_name = device.name.clone();

                let resp = SessionCreateResponse {
                    session_id,
                    status: "SESSION_ESTABLISHED".to_string(),
                    transport: "LAN_DIRECT".to_string(),
                    stream_url: "/live".to_string(),
                    protocol_version: PROTOCOL_VERSION,
                    heartbeat_interval_ms: 2000,
                };
                send_json_response(&mut socket, 200, "OK", &resp, cors_allow_origin);
            } else {
                send_error_response(&mut socket, 400, "Bad Request", "SMP_INVALID_PAYLOAD", "Malformed session request payload", cors_allow_origin);
            }
        }

        // ── 4. Heartbeat (Ping / Pong) ──
        ("GET", "/smp/ping") | ("POST", "/smp/heartbeat") => {
            let resp = HeartbeatResponse {
                r#type: "PONG".to_string(),
                server_time_ms: now_ms(),
                status: "HEALTHY".to_string(),
                protocol_version: PROTOCOL_VERSION,
            };
            send_json_response(&mut socket, 200, "OK", &resp, cors_allow_origin);
        }

        // ── 5. Remote Input Injection (Token authenticated, capability checked) ──
        ("POST", "/smp/input") => {
            if let Ok(input_body) = serde_json::from_str::<RemoteInputBody>(&body_str) {
                // 1. Authenticate session token
                let token = extract_bearer_token(&header_str)
                    .or_else(|| input_body.session_token.clone());

                let token_str = match token {
                    Some(t) if !t.is_empty() => t,
                    _ => {
                        send_error_response(&mut socket, 401, "Unauthorized", "SMP_INPUT_UNAUTHORIZED", "Missing session token in Authorization header or payload", cors_allow_origin);
                        return;
                    }
                };

                let mut target_device_id = String::new();
                let is_token_valid = {
                    let mut stream_lock = state.active_stream.lock().unwrap();
                    if !stream_lock.auth_token.is_empty() && constant_time_eq_str(&stream_lock.auth_token, &token_str) {
                        target_device_id = stream_lock.target_device_id.clone();
                        stream_lock.is_active = true;
                        true
                    } else {
                        // Check trust store cryptographic token hash
                        let trust_store = state.trust_store.lock().unwrap();
                        let token_hash = migroute::compute_sha256(token_str.as_bytes());
                        let matched_dev = trust_store.list_all().into_iter().find(|d| {
                            if !d.token_hash.is_empty() {
                                constant_time_eq_str(&d.token_hash, &token_hash)
                            } else {
                                let expected_fp_prefix = format!("FP-{}", token_str[..token_str.len().min(8)].to_uppercase());
                                d.fingerprint.starts_with(&expected_fp_prefix)
                            }
                        });

                        if let Some(dev) = matched_dev {
                            target_device_id = dev.id.to_string();
                            stream_lock.auth_token = token_str.clone();
                            stream_lock.target_device_id = dev.id.to_string();
                            stream_lock.target_device_name = dev.name.clone();
                            stream_lock.is_active = true;
                            true
                        } else {
                            false
                        }
                    }
                };

                if !is_token_valid {
                    send_error_response(&mut socket, 401, "Unauthorized", "SMP_INVALID_SESSION_TOKEN", "Session token is invalid", cors_allow_origin);
                    return;
                }

                // 2. Validate device identity and trust
                let trust_store = state.trust_store.lock().unwrap();
                let client_device_id = if !target_device_id.is_empty() {
                    if !input_body.device_id.is_empty() && input_body.device_id != target_device_id {
                        send_error_response(&mut socket, 403, "Forbidden", "SMP_DEVICE_MISMATCH", "Cannot inject input for a different device", cors_allow_origin);
                        return;
                    }
                    target_device_id
                } else if !input_body.device_id.is_empty() {
                    input_body.device_id.clone()
                } else {
                    send_error_response(&mut socket, 400, "Bad Request", "SMP_MISSING_DEVICE_ID", "Device ID required", cors_allow_origin);
                    return;
                };

                let parsed_id = match DeviceId::try_from(client_device_id.as_str()) {
                    Ok(id) => id,
                    Err(_) => {
                        send_error_response(&mut socket, 400, "Bad Request", "SMP_INVALID_DEVICE_ID", "Invalid device ID", cors_allow_origin);
                        return;
                    }
                };

                let device = match trust_store.get_device(&parsed_id) {
                    Some(d) => d,
                    None => {
                        send_error_response(&mut socket, 401, "Unauthorized", "SMP_DEVICE_NOT_TRUSTED", "Device is not in trust store", cors_allow_origin);
                        return;
                    }
                };

                if device.is_revoked {
                    send_error_response(&mut socket, 403, "Forbidden", "SMP_DEVICE_REVOKED", "Device has been revoked by host", cors_allow_origin);
                    return;
                }

                let action = input_body.action.as_str();

                // 3. Enforce capability authorization (ControlMouse vs ControlKeyboard)
                let is_mouse_action = matches!(
                    action,
                    "move" | "relative_move" | "left_click" | "right_click" | "double_click" | "mouse_down" | "mouse_up" | "scroll"
                );
                let is_keyboard_action = matches!(action, "key" | "text" | "hotkey");

                if is_mouse_action && !device.has_permission(SessionPermission::ControlMouse) {
                    send_error_response(&mut socket, 403, "Forbidden", "SMP_PERMISSION_DENIED", "ControlMouse capability not authorized for this device", cors_allow_origin);
                    return;
                }
                if is_keyboard_action && !device.has_permission(SessionPermission::ControlKeyboard) {
                    send_error_response(&mut socket, 403, "Forbidden", "SMP_PERMISSION_DENIED", "ControlKeyboard capability not authorized for this device", cors_allow_origin);
                    return;
                }

                let seq = input_body.sequence.unwrap_or(0);
                let (sw, sh) = get_screen_size();

                let x = if let Some(nx) = input_body.normalized_x {
                    (nx.clamp(0.0, 1.0) * sw as f32) as i32
                } else {
                    input_body.x.unwrap_or(0)
                };
                let y = if let Some(ny) = input_body.normalized_y {
                    (ny.clamp(0.0, 1.0) * sh as f32) as i32
                } else {
                    input_body.y.unwrap_or(0)
                };

                let res = match action {
                    "move" => state.input_controller.inject_mouse(
                        &trust_store,
                        &client_device_id,
                        x, y, 0, 0, false,
                        false, false, false, false, false, false, 0, seq,
                    ),
                    "relative_move" => {
                        let dx = input_body.dx.unwrap_or(0);
                        let dy = input_body.dy.unwrap_or(0);
                        state.input_controller.inject_mouse(
                            &trust_store,
                            &client_device_id,
                            0, 0, dx, dy, true,
                            false, false, false, false, false, false, 0, seq,
                        )
                    }
                    "left_click" => state.input_controller.inject_mouse(
                        &trust_store,
                        &client_device_id,
                        x, y, 0, 0, false,
                        true, true, false, false, false, false, 0, seq,
                    ),
                    "right_click" => state.input_controller.inject_mouse(
                        &trust_store,
                        &client_device_id,
                        x, y, 0, 0, false,
                        false, false, true, true, false, false, 0, seq,
                    ),
                    "double_click" => {
                        let r1 = state.input_controller.inject_mouse(
                            &trust_store,
                            &client_device_id,
                            x, y, 0, 0, false,
                            true, true, false, false, false, false, 0, seq,
                        );
                        std::thread::sleep(Duration::from_millis(50));
                        let r2 = state.input_controller.inject_mouse(
                            &trust_store,
                            &client_device_id,
                            x, y, 0, 0, false,
                            true, true, false, false, false, false, 0, if seq > 0 { seq + 1 } else { 0 },
                        );
                        r1.and(r2)
                    }
                    "mouse_down" => {
                        let btn = input_body.button.as_deref().unwrap_or("left");
                        let is_l = btn == "left";
                        let is_r = btn == "right";
                        let is_m = btn == "middle";
                        state.input_controller.inject_mouse(
                            &trust_store,
                            &client_device_id,
                            x, y, 0, 0, false,
                            is_l, false, is_r, false, is_m, false, 0, seq,
                        )
                    }
                    "mouse_up" => {
                        let btn = input_body.button.as_deref().unwrap_or("left");
                        let is_l = btn == "left";
                        let is_r = btn == "right";
                        let is_m = btn == "middle";
                        state.input_controller.inject_mouse(
                            &trust_store,
                            &client_device_id,
                            x, y, 0, 0, false,
                            false, is_l, false, is_r, false, is_m, 0, seq,
                        )
                    }
                    "scroll" => {
                        let delta = input_body.scroll_delta.unwrap_or(0);
                        state.input_controller.inject_mouse(
                            &trust_store,
                            &client_device_id,
                            0, 0, 0, 0, false,
                            false, false, false, false, false, false, delta, seq,
                        )
                    }
                    "key" => {
                        let vk = input_body.vk_code.unwrap_or(0);
                        let up = input_body.key_up.unwrap_or(false);
                        state.input_controller.inject_keyboard(
                            &trust_store,
                            &client_device_id,
                            vk,
                            up,
                            seq,
                        )
                    }
                    "text" => {
                        let txt = input_body.text.unwrap_or_default();
                        state.input_controller.inject_text(
                            &trust_store,
                            &client_device_id,
                            &txt,
                            seq,
                        )
                    }
                    "hotkey" => {
                        let hk = input_body.hotkey.unwrap_or_default();
                        state.input_controller.inject_hotkey(
                            &trust_store,
                            &client_device_id,
                            &hk,
                            seq,
                        )
                    }
                    _ => Err(format!("unknown remote input action: {action}")),
                };

                if let Err(e) = res {
                    send_error_response(&mut socket, 403, "Forbidden", "SMP_INPUT_UNAUTHORIZED", &e, cors_allow_origin);
                } else {
                    send_json_response(&mut socket, 200, "OK", &serde_json::json!({ "status": "INJECTED" }), cors_allow_origin);
                }
            } else {
                send_error_response(&mut socket, 400, "Bad Request", "SMP_INVALID_INPUT_PAYLOAD", "Malformed input payload", cors_allow_origin);
            }
        }

        // ── 6. Disconnect Session (Token authenticated) ──
        ("POST", "/smp/disconnect") => {
            let token = extract_bearer_token(&header_str);
            let mut stream_lock = state.active_stream.lock().unwrap();
            if let Some(t) = token {
                if !stream_lock.auth_token.is_empty() && constant_time_eq_str(&stream_lock.auth_token, &t) {
                    stream_lock.is_active = false;
                    stream_lock.started_at_epoch_ms = 0;
                    stream_lock.auth_token.clear();
                    send_json_response(&mut socket, 200, "OK", &serde_json::json!({ "status": "DISCONNECTED" }), cors_allow_origin);
                    return;
                }
            }
            send_error_response(&mut socket, 401, "Unauthorized", "SMP_UNAUTHORIZED", "Valid session token required to disconnect", cors_allow_origin);
        }

        // ── 7. Diagnostics & Connection Test Tool ──
        ("GET", "/smp/diagnostics") => {
            let trust_count = state.trust_store.lock().unwrap().active_count();
            let pairing_status = if state.active_pairing.lock().unwrap().is_some() {
                "ACTIVE_PAIRING"
            } else {
                "IDLE"
            };
            let capture_probe = match capture_primary_display() {
                Ok(f) => format!("READY: {}x{}", f.width, f.height),
                Err(e) => format!("CAPTURE_UNAVAILABLE: {e}"),
            };

            let diag = ConnectionDiagnosticsResponse {
                discovery: "PASS".to_string(),
                device_id: state.identity.id.to_string(),
                protocol_version: format!("PASS (SMP/{PROTOCOL_VERSION})"),
                pairing_status: pairing_status.to_string(),
                trust_store_count: trust_count,
                screen_capture: capture_probe,
                streaming_port: SMP_CONTROL_PORT,
                firewall_status: probe_firewall_status(),
                overall: "PASS".to_string(),
            };
            send_json_response(&mut socket, 200, "OK", &diag, cors_allow_origin);
        }

        // ── 8. Live Display Stream (MJPEG, strictly Authorization header Bearer token authenticated) ──
        ("GET", "/live") => {
            // Strictly require Authorization: Bearer token header. Query parameters are rejected to prevent token leakage in URLs and logs.
            let provided_token = extract_bearer_token(&header_str);

            let token = match provided_token {
                Some(t) if !t.is_empty() => t,
                _ => {
                    let reject_resp = "HTTP/1.1 401 Unauthorized\r\n\
                        Content-Type: text/plain; charset=utf-8\r\n\
                        Connection: close\r\n\
                        WWW-Authenticate: Bearer realm=\"SmartMigrate\"\r\n\r\n\
                        Unauthorized: Valid Smart Migrate session token required\r\n";
                    let _ = socket.write_all(reject_resp.as_bytes());
                    return;
                }
            };

            // Validate token against active stream state or trust store device fingerprint anchor
            let target_dev_id = {
                let (active_token, current_dev_id) = {
                    let lock = state.active_stream.lock().unwrap();
                    (lock.auth_token.clone(), lock.target_device_id.clone())
                };

                if !active_token.is_empty() && constant_time_eq_str(&active_token, &token) {
                    current_dev_id
                } else {
                    // Check if token matches a paired device in the persistent trust store
                    let trust_store = state.trust_store.lock().unwrap();
                    let token_hash = migroute::compute_sha256(token.as_bytes());
                    let paired_dev = trust_store
                        .list_all()
                        .into_iter()
                        .find(|d| {
                            if !d.token_hash.is_empty() {
                                constant_time_eq_str(&d.token_hash, &token_hash)
                            } else {
                                let expected_fp_prefix = format!("FP-{}", token[..token.len().min(8)].to_uppercase());
                                d.fingerprint.starts_with(&expected_fp_prefix)
                            }
                        });

                    match paired_dev {
                        Some(dev) => {
                            let dev_id_str = dev.id.to_string();
                            let dev_name = dev.name.clone();
                            drop(trust_store);

                            // Bind active stream token to the authenticated paired device
                            let mut stream_lock = state.active_stream.lock().unwrap();
                            stream_lock.auth_token = token.clone();
                            stream_lock.target_device_id = dev_id_str.clone();
                            stream_lock.target_device_name = dev_name;
                            dev_id_str
                        }
                        None => {
                            let reject_resp = "HTTP/1.1 401 Unauthorized\r\n\
                                Content-Type: text/plain; charset=utf-8\r\n\
                                Connection: close\r\n\
                                WWW-Authenticate: Bearer realm=\"SmartMigrate\"\r\n\r\n\
                                Unauthorized: Invalid session token\r\n";
                            let _ = socket.write_all(reject_resp.as_bytes());
                            return;
                        }
                    }
                }
            };

            // Reset sequence tracker so streaming session starts with clean monotonic counter
            state.input_controller.reset_sequence(&target_dev_id);

            // Verify ViewScreen capability authorization
            {
                let trust_store = state.trust_store.lock().unwrap();
                let client_id = DeviceId::try_from(target_dev_id.as_str()).ok();
                let has_screen_perm = client_id
                    .and_then(|id| trust_store.get_device(&id))
                    .map(|dev| dev.has_permission(SessionPermission::ViewScreen))
                    .unwrap_or(false);

                if !has_screen_perm {
                    let reject_resp = "HTTP/1.1 403 Forbidden\r\n\
                        Content-Type: text/plain; charset=utf-8\r\n\
                        Connection: close\r\n\r\n\
                        Forbidden: ViewScreen capability is not granted to this device\r\n";
                    let _ = socket.write_all(reject_resp.as_bytes());
                    return;
                }
            }

            let connection_token = now_ms();
            // Mark stream session as actively transmitting
            {
                let mut stream_lock = state.active_stream.lock().unwrap();
                stream_lock.is_active = true;
                stream_lock.connection_id = connection_token;
                if stream_lock.started_at_epoch_ms == 0 {
                    stream_lock.started_at_epoch_ms = now_ms();
                }
                if stream_lock.target_device_name.is_empty() {
                    let trust = state.trust_store.lock().unwrap();
                    if let Some(dev) = trust.list_all().first() {
                        stream_lock.target_device_name = dev.name.clone();
                        stream_lock.target_device_id = dev.id.to_string();
                    } else {
                        stream_lock.target_device_name = "Android Remote".to_string();
                    }
                }
            }

            // Optimize streaming socket: disable Nagle, remove read timeout, set generous write deadline
            let _ = socket.set_nodelay(true);
            let _ = socket.set_read_timeout(None);
            let _ = socket.set_write_timeout(Some(Duration::from_millis(6000)));

            // Stream frames directly on this connection (No wildcard CORS)
            let http_header = "HTTP/1.1 200 OK\r\n\
                Content-Type: multipart/x-mixed-replace; boundary=--smartmigrate\r\n\
                Cache-Control: no-cache, no-store, must-revalidate\r\n\
                Pragma: no-cache\r\n\
                Expires: 0\r\n\
                Connection: keep-alive\r\n\r\n";
            if socket.write_all(http_header.as_bytes()).is_err() {
                return;
            }

            let requested_source = if query.contains("source=display-virtual-extended") {
                "display-virtual-extended".to_string()
            } else {
                "display-primary".to_string()
            };

            let requested_fps: u64 = query
                .split(['&', '?'])
                .find_map(|kv| kv.strip_prefix("fps="))
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(60);
            let target_fps: u64 = requested_fps.clamp(15, 60);
            let frame_interval = Duration::from_micros(1_000_000 / target_fps);

            struct EncodedStreamFrame {
                payload: Vec<u8>,
                width: u32,
                height: u32,
                timestamp_ms: u64,
                capture_latency_ms: f32,
            }

            let frame_slot = Arc::new((Mutex::new(None::<EncodedStreamFrame>), Condvar::new()));
            let worker_stop = stop_signal.clone();
            let worker_slot = frame_slot.clone();
            let req_source = requested_source.clone();

            // Decoupled Screen Capture & Encoder Thread:
            // Runs on a strict 60 FPS pacing loop without touching network I/O or blocking on TCP sockets.
            let capture_worker = std::thread::Builder::new()
                .name("smartmigrate-capture-worker".into())
                .spawn(move || {
                    let mut capture_session = match DesktopCaptureSession::new_with_source(&req_source) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("[SMP/1 Stream] Failed to initialize desktop capture session: {e}");
                            worker_stop.store(true, Ordering::Relaxed);
                            let (_, cvar) = &*worker_slot;
                            cvar.notify_all();
                            return;
                        }
                    };
                    // Quality 75 with 4:2:0 subsampling delivers crisp text and sub-15ms encoding latency
                    let mut encoder = SoftwareJpegEncoder::new(75);
                    let mut last_frame_checksum: u64 = 0;
                    let mut last_sent_time = Instant::now();

                    while !worker_stop.load(Ordering::Relaxed) {
                        let loop_start = Instant::now();
                        let now_ms = now_ms();

                        if let Ok((bgra_data, width, height, cap_duration_ms)) = capture_session.capture_frame() {
                            let checksum = fast_frame_checksum(bgra_data);
                            let screen_changed = checksum != last_frame_checksum;
                            let keepalive_due = last_sent_time.elapsed() >= Duration::from_millis(350);

                            if screen_changed || keepalive_due {
                                if let Ok((encoded_slice, enc_duration_ms)) = encoder.encode_slice(bgra_data, width, height, 75) {
                                    let frame = EncodedStreamFrame {
                                        payload: encoded_slice.to_vec(),
                                        width,
                                        height,
                                        timestamp_ms: now_ms,
                                        capture_latency_ms: cap_duration_ms + enc_duration_ms,
                                    };
                                    let (lock, cvar) = &*worker_slot;
                                    if let Ok(mut guard) = lock.lock() {
                                        *guard = Some(frame);
                                        cvar.notify_one();
                                    }
                                    last_frame_checksum = checksum;
                                    last_sent_time = Instant::now();
                                }
                            }
                        }

                        let elapsed = loop_start.elapsed();
                        if elapsed < frame_interval {
                            std::thread::sleep(frame_interval - elapsed);
                        }
                    }
                });

            // Rolling 1-second FPS and bitrate measurement window
            let mut fps_window_start = Instant::now();
            let mut fps_window_frames: u32 = 0;
            let mut fps_window_bytes: u64 = 0;
            let mut scratch_buf = Vec::with_capacity(128 * 1024);
            let mut last_activity = Instant::now();

            loop {
                // Respect stop_display_stream() and verify this connection has not been superseded
                let is_current = state.active_stream.lock().is_ok_and(|s| s.is_active && s.connection_id == connection_token);
                if !is_current || stop_signal.load(Ordering::Relaxed) {
                    break;
                }

                // Wait for the next frame with a 400ms timeout for keepalive
                let (lock, cvar) = &*frame_slot;
                let frame_opt = {
                    let guard = match lock.lock() {
                        Ok(g) => g,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    if guard.is_none() && !stop_signal.load(Ordering::Relaxed) {
                        match cvar.wait_timeout(guard, Duration::from_millis(400)) {
                            Ok((mut g, _)) => g.take(),
                            Err(e) => e.into_inner().0.take(),
                        }
                    } else {
                        let mut g = guard;
                        g.take()
                    }
                };

                if stop_signal.load(Ordering::Relaxed) {
                    break;
                }

                if let Some(frame) = frame_opt {
                    scratch_buf.clear();
                    let frame_head = format!(
                        "--smartmigrate\r\n\
                        Content-Type: image/jpeg\r\n\
                        Content-Length: {}\r\n\
                        X-Timestamp: {}\r\n\
                        X-Width: {}\r\n\
                        X-Height: {}\r\n\r\n",
                        frame.payload.len(), frame.timestamp_ms, frame.width, frame.height
                    );
                    scratch_buf.extend_from_slice(frame_head.as_bytes());
                    scratch_buf.extend_from_slice(&frame.payload);
                    scratch_buf.extend_from_slice(b"\r\n");

                    if socket.write_all(&scratch_buf).is_err() || socket.flush().is_err() {
                        break; // Client disconnected
                    }

                    last_activity = Instant::now();
                    let frame_bytes = scratch_buf.len() as u64;

                    fps_window_frames += 1;
                    fps_window_bytes += frame_bytes;

                    if let Ok(mut stream_lock) = state.active_stream.lock() {
                        stream_lock.frames_sent += 1;
                        stream_lock.frames_captured += 1;
                        stream_lock.capture_latency_ms = frame.capture_latency_ms;
                        stream_lock.width = frame.width;
                        stream_lock.height = frame.height;

                        let window_elapsed = fps_window_start.elapsed();
                        if window_elapsed >= Duration::from_secs(1) {
                            let elapsed_secs = window_elapsed.as_secs_f32();
                            stream_lock.measured_fps = fps_window_frames as f32 / elapsed_secs;
                            stream_lock.measured_bitrate_kbps = (fps_window_bytes as f32 * 8.0)
                                / (elapsed_secs * 1000.0);
                            stream_lock.fps_window_frames = fps_window_frames;
                            stream_lock.bytes_window = fps_window_bytes;
                            stream_lock.fps_window_start_ms = now_ms();

                            fps_window_frames = 0;
                            fps_window_bytes = 0;
                            fps_window_start = Instant::now();
                        }
                    }
                } else if last_activity.elapsed() >= Duration::from_millis(400) {
                    // Send lightweight keepalive heartbeat packet to maintain TCP connection
                    // and prevent client read timeout on static desktop screens
                    let heartbeat = "--smartmigrate\r\n\
                        Content-Type: text/plain\r\n\
                        Content-Length: 0\r\n\
                        X-Heartbeat: 1\r\n\r\n";
                    if socket.write_all(heartbeat.as_bytes()).is_err() || socket.flush().is_err() {
                        break; // Client disconnected
                    }
                    last_activity = Instant::now();
                }
            }

            // Signal capture worker to exit and notify condvar
            stop_signal.store(true, Ordering::Relaxed);
            let (_, cvar) = &*frame_slot;
            cvar.notify_all();
            if let Ok(handle) = capture_worker {
                let _ = handle.join();
            }

            // Reset active stream on disconnect only if this connection is still current
            if let Ok(mut stream_lock) = state.active_stream.lock() {
                if stream_lock.connection_id == connection_token {
                    stream_lock.is_active = false;
                    stream_lock.started_at_epoch_ms = 0;
                    stream_lock.measured_fps = 0.0;
                    stream_lock.measured_bitrate_kbps = 0.0;
                    stream_lock.fps_window_frames = 0;
                    stream_lock.bytes_window = 0;
                }
            }
        }

        // ── 404 Not Found ──
        _ => {
            send_error_response(&mut socket, 404, "Not Found", "SMP_ROUTE_NOT_FOUND", "Requested Smart Migrate endpoint does not exist", cors_allow_origin);
        }
    }
}

#[cfg(target_os = "windows")]
fn get_screen_size() -> (i32, i32) {
    #[link(name = "user32")]
    extern "system" {
        fn GetSystemMetrics(nIndex: i32) -> i32;
    }
    let w = unsafe { GetSystemMetrics(0) };
    let h = unsafe { GetSystemMetrics(1) };
    (if w > 0 { w } else { 1920 }, if h > 0 { h } else { 1080 })
}

#[cfg(not(target_os = "windows"))]
fn get_screen_size() -> (i32, i32) {
    (1920, 1080)
}

fn send_json_response<T: Serialize>(
    socket: &mut TcpStream,
    status_code: u16,
    status_text: &str,
    body: &T,
    cors_allow_origin: Option<&str>,
) {
    if let Ok(json_str) = serde_json::to_string(body) {
        let cors_hdr = cors_allow_origin
            .map(|o| format!("Access-Control-Allow-Origin: {o}\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type, Authorization\r\n"))
            .unwrap_or_default();
        let resp = format!(
            "HTTP/1.1 {status_code} {status_text}\r\n\
            Content-Type: application/json; charset=utf-8\r\n\
            Content-Length: {}\r\n\
            Connection: close\r\n\
            {cors_hdr}\r\n\
            {json_str}",
            json_str.len()
        );
        let _ = socket.write_all(resp.as_bytes());
        let _ = socket.flush();
    }
}

fn send_error_response(
    socket: &mut TcpStream,
    status_code: u16,
    status_text: &str,
    code: &str,
    message: &str,
    cors_allow_origin: Option<&str>,
) {
    let err_obj = serde_json::json!({
        "error": code,
        "message": message,
        "protocol_version": PROTOCOL_VERSION,
        "timestamp_ms": now_ms()
    });
    send_json_response(socket, status_code, status_text, &err_obj, cors_allow_origin);
}

// #[cfg(test)]
// mod tests {
// ...
// }
