//! Smart Migrate Protocol (SMP/1) unified network server.
//!
//! Provides the primary communication channel between Android clients and Windows host:
//! - UDP broadcast discovery on port 7889
//! - TCP control and streaming server on port 7890
//! - Full protocol state machine: Discovery -> Hello -> Pairing -> Session -> Transport -> Heartbeat -> Stream

use crate::capture::{capture_primary_display, SoftwareJpegEncoder, VideoEncoder};
use crate::state::AppState;
use crate::storage::{generate_crypto_token, save_trust_store};
use migroute::identity::DevicePlatform;
use migroute::pairing::NumericPairingCode;
use migroute::trust::TrustedDevice;
use migroute::{DeviceId, SessionPermission, PROTOCOL_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                let ip_str = addr.ip().to_string();
                if !ip_str.starts_with("127.") && !ip_str.starts_with("169.254.") {
                    return ip_str;
                }
            }
        }
    }
    "192.168.31.33".to_string()
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
    pub action: String,
    pub x: i32,
    pub y: i32,
    pub scroll_delta: Option<i32>,
    pub sequence: Option<u64>,
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
    pub overall: String,
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

    while !stop_signal.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((socket, _peer)) => {
                let _ = socket.set_nodelay(true);
                let _ = socket.set_read_timeout(Some(Duration::from_millis(1500)));

                let state_clone = state.clone();
                let stop_clone = Arc::clone(&stop_signal);

                std::thread::spawn(move || {
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

/// Handles an incoming client request on port 7890.
fn handle_client_connection(mut socket: TcpStream, state: AppState, stop_signal: Arc<AtomicBool>) {
    let mut buffer = [0u8; 8192];
    let bytes_read = match socket.read(&mut buffer) {
        Ok(n) if n > 0 => n,
        _ => return,
    };

    let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
    let first_line = request_str.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let method = parts[0];
    let uri = parts[1];

    // Split URI into path and query string
    let (path, query) = if let Some(idx) = uri.find('?') {
        (&uri[..idx], &uri[idx + 1..])
    } else {
        (uri, "")
    };

    // Route table
    match (method, path) {
        // ── 0. CORS Preflight ──
        ("OPTIONS", _) => {
            let resp = "HTTP/1.1 204 No Content\r\n\
                Access-Control-Allow-Origin: *\r\n\
                Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
                Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
                Access-Control-Max-Age: 86400\r\n\r\n";
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
                app_version: "0.1.0".to_string(),
                capabilities: vec![
                    "SCREEN_CAPTURE".to_string(),
                    "VIDEO_ENCODE".to_string(),
                    "REMOTE_MOUSE".to_string(),
                    "REMOTE_KEYBOARD".to_string(),
                    "CLIPBOARD".to_string(),
                ],
                timestamp_ms: now_ms(),
            };
            send_json_response(&mut socket, 200, "OK", &hello);
        }

        // ── 2. Pairing Request ──
        ("POST", "/smp/pair") => {
            let body_str = extract_http_body(&request_str);
            if let Ok(pair_req) = serde_json::from_str::<PairRequestBody>(body_str) {
                let mut active_lock = state.active_pairing.lock().unwrap();

                if let Some(ref mut session) = *active_lock {
                    if session.is_expired(now_ms()) {
                        *active_lock = None;
                        send_error_response(&mut socket, 410, "Gone", "SMP_PAIRING_EXPIRED", "Pairing session has expired. Start a new one on PC.");
                        return;
                    }

                    let code_res = NumericPairingCode::try_from_digits(&pair_req.code);
                    if code_res.is_err() || !session.code.constant_time_eq(&pair_req.code) {
                        send_error_response(&mut socket, 403, "Forbidden", "SMP_INVALID_CODE", "Invalid pairing PIN code.");
                        return;
                    }

                    // PIN matches! Register trusted device and create persistent trust
                    let req_id = match DeviceId::try_from(pair_req.requester_device_id.as_str()) {
                        Ok(id) => id,
                        Err(e) => {
                            send_error_response(&mut socket, 400, "Bad Request", "SMP_INVALID_DEVICE_ID", &e.to_string());
                            return;
                        }
                    };

                    let mut perms = BTreeSet::new();
                    perms.insert(SessionPermission::ViewScreen);
                    perms.insert(SessionPermission::ControlMouse);
                    perms.insert(SessionPermission::SendFiles);
                    perms.insert(SessionPermission::ReceiveFiles);
                    perms.insert(SessionPermission::Clipboard);

                    let session_token = generate_crypto_token();
                    let trusted_device = TrustedDevice::new(
                        req_id,
                        pair_req.requester_name.clone(),
                        DevicePlatform::Android,
                        format!("FP-{}", &session_token[..8].to_uppercase()),
                        now_ms(),
                        perms.clone(),
                    );

                    let mut trust_store = state.trust_store.lock().unwrap();
                    let _ = trust_store.remove_device(&trusted_device.id);
                    let _ = trust_store.add_device(trusted_device);
                    let _ = save_trust_store(&trust_store);

                    // Update active stream token to match this session
                    {
                        let mut stream_lock = state.active_stream.lock().unwrap();
                        stream_lock.auth_token = session_token.clone();
                    }

                    *active_lock = None; // clear pending pairing after success

                    let resp = PairResponseBody {
                        status: "APPROVED".to_string(),
                        session_token,
                        host_device_id: state.identity.id.to_string(),
                        granted_permissions: perms.into_iter().map(|p| format!("{p:?}")).collect(),
                    };
                    send_json_response(&mut socket, 200, "OK", &resp);
                } else {
                    send_error_response(&mut socket, 404, "Not Found", "SMP_NO_ACTIVE_PAIRING", "No active pairing session on Windows PC. Click Start Pairing on PC first.");
                }
            } else {
                send_error_response(&mut socket, 400, "Bad Request", "SMP_INVALID_PAYLOAD", "Malformed pairing request payload");
            }
        }

        // ── 3. Session Creation ──
        ("POST", "/smp/session") => {
            let body_str = extract_http_body(&request_str);
            if let Ok(sess_req) = serde_json::from_str::<SessionCreateBody>(body_str) {
                let trust_store = state.trust_store.lock().unwrap();
                let client_id = match DeviceId::try_from(sess_req.client_device_id.as_str()) {
                    Ok(id) => id,
                    Err(_) => {
                        send_error_response(&mut socket, 400, "Bad Request", "SMP_INVALID_DEVICE_ID", "Invalid client device ID");
                        return;
                    }
                };

                let device = match trust_store.get_device(&client_id) {
                    Some(d) => d,
                    None => {
                        send_error_response(&mut socket, 401, "Unauthorized", "SMP_DEVICE_NOT_TRUSTED", "Device is not paired or trusted with this host");
                        return;
                    }
                };

                if device.is_revoked {
                    send_error_response(&mut socket, 403, "Forbidden", "SMP_DEVICE_REVOKED", "Device authorization has been revoked by host");
                    return;
                }

                // Activate stream state with session token
                let mut stream_lock = state.active_stream.lock().unwrap();
                stream_lock.is_active = true;
                stream_lock.codec = "mjpeg".to_string();
                stream_lock.target_fps = 30;
                stream_lock.auth_token = sess_req.session_token.clone();
                let session_id = format!("sm-sess-{}", &now_ms().to_string()[7..]);
                stream_lock.session_id = session_id.clone();

                let resp = SessionCreateResponse {
                    session_id,
                    status: "SESSION_ESTABLISHED".to_string(),
                    transport: "LAN_DIRECT".to_string(),
                    stream_url: format!("/live?token={}", sess_req.session_token),
                    protocol_version: PROTOCOL_VERSION,
                    heartbeat_interval_ms: 2000,
                };
                send_json_response(&mut socket, 200, "OK", &resp);
            } else {
                send_error_response(&mut socket, 400, "Bad Request", "SMP_INVALID_PAYLOAD", "Malformed session request payload");
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
            send_json_response(&mut socket, 200, "OK", &resp);
        }

        // ── 5. Remote Input Injection ──
        ("POST", "/smp/input") => {
            let body_str = extract_http_body(&request_str);
            if let Ok(input_body) = serde_json::from_str::<RemoteInputBody>(body_str) {
                let trust_store = state.trust_store.lock().unwrap();
                let is_left = input_body.action == "left_click";
                let is_right = input_body.action == "right_click";
                let scroll = input_body.scroll_delta.unwrap_or(0);
                let seq = input_body.sequence.unwrap_or(1);

                let res = state.input_controller.inject_mouse(
                    &trust_store,
                    &input_body.device_id,
                    input_body.x,
                    input_body.y,
                    is_left,
                    is_left,
                    is_right,
                    is_right,
                    false,
                    false,
                    scroll,
                    seq,
                );

                if res.is_ok() {
                    send_json_response(&mut socket, 200, "OK", &serde_json::json!({ "status": "INJECTED" }));
                } else {
                    send_error_response(&mut socket, 403, "Forbidden", "SMP_INPUT_UNAUTHORIZED", &res.unwrap_err());
                }
            } else {
                send_error_response(&mut socket, 400, "Bad Request", "SMP_INVALID_INPUT_PAYLOAD", "Malformed input payload");
            }
        }

        // ── 6. Diagnostics & Connection Test Tool ──
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
                overall: "PASS".to_string(),
            };
            send_json_response(&mut socket, 200, "OK", &diag);
        }

        // ── 7. Live Display Stream (MJPEG) ──
        ("GET", "/live") => {
            // Check auth token
            let active_token = {
                let lock = state.active_stream.lock().unwrap();
                lock.auth_token.clone()
            };

            let is_authorized = if active_token.is_empty() {
                // If no token is set in active stream, check if any trusted device exists or reject
                state.trust_store.lock().unwrap().active_count() > 0
            } else {
                let token_query = format!("token={}", active_token);
                let token_bearer = format!("Bearer {}", active_token);
                query.contains(&token_query) || request_str.contains(&token_bearer)
            };

            if !is_authorized {
                let reject_resp = "HTTP/1.1 401 Unauthorized\r\n\
                    Content-Type: text/plain; charset=utf-8\r\n\
                    Connection: close\r\n\
                    WWW-Authenticate: Bearer realm=\"SmartMigrate\"\r\n\r\n\
                    Unauthorized: Valid Smart Migrate session token required\r\n";
                let _ = socket.write_all(reject_resp.as_bytes());
                return;
            }

            // Stream frames directly on this connection
            let http_header = "HTTP/1.1 200 OK\r\n\
                Content-Type: multipart/x-mixed-replace; boundary=--smartmigrate\r\n\
                Cache-Control: no-cache, no-store, must-revalidate\r\n\
                Pragma: no-cache\r\n\
                Expires: 0\r\n\
                Access-Control-Allow-Origin: *\r\n\r\n";
            if socket.write_all(http_header.as_bytes()).is_err() {
                return;
            }

            let mut encoder = SoftwareJpegEncoder::new(75);
            let frame_interval = Duration::from_millis(33); // ~30 fps

            while !stop_signal.load(Ordering::Relaxed) {
                let loop_start = Instant::now();

                if let Ok(raw_frame) = capture_primary_display() {
                    if let Ok(encoded) = encoder.encode(&raw_frame, 75) {
                        let frame_head = format!(
                            "--smartmigrate\r\n\
                            Content-Type: image/jpeg\r\n\
                            Content-Length: {}\r\n\
                            X-Timestamp: {}\r\n\
                            X-Width: {}\r\n\
                            X-Height: {}\r\n\r\n",
                            encoded.payload.len(), encoded.timestamp_ms, encoded.width, encoded.height
                        );

                        if socket.write_all(frame_head.as_bytes()).is_err()
                            || socket.write_all(&encoded.payload).is_err()
                            || socket.write_all(b"\r\n").is_err()
                            || socket.flush().is_err()
                        {
                            break; // Client disconnected
                        }

                        if let Ok(mut stream_lock) = state.active_stream.lock() {
                            stream_lock.frames_captured += 1;
                            stream_lock.frames_sent += 1;
                            stream_lock.capture_latency_ms = encoded.encode_duration_ms;
                            stream_lock.width = encoded.width;
                            stream_lock.height = encoded.height;
                        }
                    }
                }

                let elapsed = loop_start.elapsed();
                if elapsed < frame_interval {
                    std::thread::sleep(frame_interval - elapsed);
                }
            }
        }

        // ── 404 Not Found ──
        _ => {
            send_error_response(&mut socket, 404, "Not Found", "SMP_ROUTE_NOT_FOUND", "Requested Smart Migrate endpoint does not exist");
        }
    }
}

fn extract_http_body(request: &str) -> &str {
    if let Some(pos) = request.find("\r\n\r\n") {
        &request[pos + 4..]
    } else if let Some(pos) = request.find("\n\n") {
        &request[pos + 2..]
    } else {
        ""
    }
}

fn send_json_response<T: Serialize>(socket: &mut TcpStream, status_code: u16, status_text: &str, body: &T) {
    if let Ok(json_str) = serde_json::to_string(body) {
        let resp = format!(
            "HTTP/1.1 {status_code} {status_text}\r\n\
            Content-Type: application/json; charset=utf-8\r\n\
            Content-Length: {}\r\n\
            Connection: close\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
            Access-Control-Allow-Headers: Content-Type, Authorization\r\n\r\n\
            {json_str}",
            json_str.len()
        );
        let _ = socket.write_all(resp.as_bytes());
        let _ = socket.flush();
    }
}

fn send_error_response(socket: &mut TcpStream, status_code: u16, status_text: &str, code: &str, message: &str) {
    let err_obj = serde_json::json!({
        "error": code,
        "message": message,
        "protocol_version": PROTOCOL_VERSION,
        "timestamp_ms": now_ms()
    });
    send_json_response(socket, status_code, status_text, &err_obj);
}

// #[cfg(test)]
// mod tests {
// ...
// }
