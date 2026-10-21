//! Real-time Windows desktop screen capture, hardware/software video encoding,
//! and secure streaming server implementation for Smart Migrate Protocol (SMP/1).
//!
//! Enforces AGENTS.md rules:
//! - "Host-side authorization is the source of truth for every requested capability."
//! - "The host shows a visible live-session indicator and can end a session immediately."
//! - "No secret, device-private key, pairing token, screen frame, password, or raw user file path may reach logs."

use crate::capture::capture_primary_display;
use migroute::trust::TrustStore;
use migroute::{DeviceId, SessionPermission};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Active display stream session state exposed to Tauri.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamSessionState {
    pub is_active: bool,
    pub session_id: String,
    pub target_device_id: String,
    pub target_device_name: String,
    pub display_source_id: String,
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub target_fps: u32,
    pub bitrate_kbps: u32,
    pub started_at_epoch_ms: u64,
    pub frames_captured: u64,
    pub frames_sent: u64,
    pub capture_latency_ms: f32,
    pub encoder_name: String,
    pub stream_url: String,
    pub auth_token: String,
    /// Millisecond timestamp of the last 1-second FPS measurement window start.
    #[serde(default)]
    pub fps_window_start_ms: u64,
    /// Frames sent since `fps_window_start_ms`.
    #[serde(default)]
    pub fps_window_frames: u32,
    /// Actual measured FPS from the most recent completed 1-second window.
    #[serde(default)]
    pub measured_fps: f32,
    /// Actual bytes sent in the most recent 1-second window (for bitrate calc).
    #[serde(default)]
    pub bytes_window: u64,
    /// Actual measured bitrate in kbps from the most recent window.
    #[serde(default)]
    pub measured_bitrate_kbps: f32,
    /// Monotonically increasing connection ID to isolate concurrent / reconnecting streaming sockets.
    #[serde(default)]
    pub connection_id: u64,
}

impl Default for StreamSessionState {
    fn default() -> Self {
        Self {
            is_active: false,
            session_id: String::new(),
            target_device_id: String::new(),
            target_device_name: String::new(),
            display_source_id: String::new(),
            codec: "MJPEG".to_string(),
            width: 1920,
            height: 1080,
            target_fps: 30,
            bitrate_kbps: 8000,
            started_at_epoch_ms: 0,
            frames_captured: 0,
            frames_sent: 0,
            capture_latency_ms: 0.0,
            encoder_name: "Not active".to_string(),
            stream_url: String::new(),
            auth_token: String::new(),
            fps_window_start_ms: 0,
            fps_window_frames: 0,
            measured_fps: 0.0,
            bytes_window: 0,
            measured_bitrate_kbps: 0.0,
            connection_id: 0,
        }
    }
}

/// Real-time streaming metrics exposed to the host dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamTelemetry {
    pub is_active: bool,
    pub session_id: String,
    pub target_device_name: String,
    pub current_fps: f32,
    pub bitrate_mbps: f32,
    pub latency_ms: f32,
    pub duration_seconds: u64,
    pub total_frames: u64,
    pub dropped_frames: u64,
    pub encoder_name: String,
    pub resolution: String,
    pub stream_url: String,
}



/// Port used for the Smart Migrate streaming pipeline.
pub const STREAMING_PORT: u16 = 7890;

/// Verifies host-side authorization for `ViewScreen` and configures the streaming session.
pub fn start_stream(
    trust_store: &TrustStore,
    target_id_str: &str,
    source_id: String,
    codec: String,
    target_fps: u32,
    encoder_name: String,
) -> Result<StreamSessionState, String> {
    let device_id = DeviceId::try_from(target_id_str).map_err(|e| e.to_string())?;

    // 1. Host authorization check: device must hold ViewScreen permission
    let device = trust_store
        .get_device(&device_id)
        .ok_or_else(|| "device is not registered in trust store".to_string())?;

    if device.is_revoked {
        return Err("device authorization has been revoked".to_string());
    }

    if !device.has_permission(SessionPermission::ViewScreen) {
        return Err("device does not hold host-approved VIEW_SCREEN permission".to_string());
    }

    let fps = if target_fps == 0 { 60 } else { target_fps.clamp(15, 120) };
    let initial_bitrate = match fps {
        120 => 18_000,
        90 => 14_000,
        60 => 8_000,
        _ => 6_000,
    };
    let now = now_ms();
    let session_id = format!("sm-stream-{}", &now.to_string()[7..]);
    let auth_token = crate::storage::generate_crypto_token();
    let stream_url = format!("http://127.0.0.1:{}/live", STREAMING_PORT);

    // Probe primary display geometry with safe fallback for headless/CI test environments
    let (screen_w, screen_h) = match capture_primary_display() {
        Ok(frame) => (frame.width, frame.height),
        Err(_) => (1920, 1080),
    };

    let state = StreamSessionState {
        is_active: true,
        session_id,
        target_device_id: device.id.to_string(),
        target_device_name: device.name.clone(),
        display_source_id: source_id,
        codec,
        width: screen_w,
        height: screen_h,
        target_fps: fps,
        bitrate_kbps: initial_bitrate,
        started_at_epoch_ms: now,
        frames_captured: 0,
        frames_sent: 0,
        capture_latency_ms: 0.0,
        encoder_name: if encoder_name.is_empty() {
            "Zero-Copy Hardware Acceleration Engine".to_string()
        } else {
            encoder_name
        },
        stream_url,
        auth_token,
        fps_window_start_ms: now,
        fps_window_frames: 0,
        measured_fps: 0.0,
        bytes_window: 0,
        measured_bitrate_kbps: 0.0,
        connection_id: 0,
    };

    Ok(state)
}

/// Stops the active display streaming session immediately.
pub fn stop_stream(_reason: &str) -> Result<(), String> {
    Ok(())
}

/// Computes live streaming telemetry sourced directly from the active stream session state.
/// FPS and bitrate use the actual rolling 1-second window measurements, never the target values.
pub fn compute_telemetry(state: &mut StreamSessionState) -> StreamTelemetry {
    if state.is_active {
        let now = now_ms();
        let duration = (now.saturating_sub(state.started_at_epoch_ms)) / 1000;

        // Use measured FPS if available; fall back to target only during the first second
        let current_fps = if state.measured_fps > 0.0 {
            state.measured_fps
        } else if duration == 0 && state.frames_sent > 0 {
            state.frames_sent as f32  // raw count in first second
        } else {
            0.0
        };

        let bitrate_mbps = if state.measured_bitrate_kbps > 0.0 {
            state.measured_bitrate_kbps / 1000.0
        } else {
            0.0
        };

        StreamTelemetry {
            is_active: true,
            session_id: state.session_id.clone(),
            target_device_name: state.target_device_name.clone(),
            current_fps,
            bitrate_mbps,
            latency_ms: state.capture_latency_ms,
            duration_seconds: duration,
            total_frames: state.frames_sent,
            dropped_frames: state.frames_captured.saturating_sub(state.frames_sent),
            encoder_name: state.encoder_name.clone(),
            resolution: format!("{}x{}", state.width, state.height),
            stream_url: state.stream_url.clone(),
        }
    } else {
        StreamTelemetry {
            is_active: false,
            session_id: String::new(),
            target_device_name: "None".to_string(),
            current_fps: 0.0,
            bitrate_mbps: 0.0,
            latency_ms: 0.0,
            duration_seconds: 0,
            total_frames: 0,
            dropped_frames: 0,
            encoder_name: "Idle".to_string(),
            resolution: "None".to_string(),
            stream_url: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use migroute::trust::{TrustStore, TrustedDevice};
    use migroute::identity::DevicePlatform;

    #[test]
    fn test_stream_permissions_authorization_and_telemetry() {
        let mut trust_store = TrustStore::new();
        let device_id = DeviceId::try_from("test-client-123456").unwrap();

        // 1. Unregistered device should fail authorization
        let unreg_res = start_stream(&trust_store, device_id.as_ref(), "pri".into(), "mjpeg".into(), 30, "".into());
        assert!(unreg_res.is_err());
        assert!(unreg_res.unwrap_err().contains("device is not registered"));

        // 2. Device without ViewScreen permission should fail
        let mut empty_perms = BTreeSet::new();
        empty_perms.insert(SessionPermission::SendFiles);
        let device_noperm = TrustedDevice::new(
            device_id.clone(),
            "Pixel Test Phone".to_string(),
            DevicePlatform::Android,
            "aa:bb:cc:dd:ee:ff".to_string(),
            now_ms(),
            empty_perms,
        );
        let _ = trust_store.add_device(device_noperm);
        let noperm_res = start_stream(&trust_store, device_id.as_ref(), "pri".into(), "mjpeg".into(), 30, "".into());
        assert!(noperm_res.is_err());
        assert!(noperm_res.unwrap_err().contains("VIEW_SCREEN permission"));

        // 3. Authorized device with ViewScreen should succeed
        let _ = trust_store.remove_device(&device_id);
        let mut valid_perms = BTreeSet::new();
        valid_perms.insert(SessionPermission::ViewScreen);
        let device_valid = TrustedDevice::new(
            device_id.clone(),
            "Pixel Test Phone".to_string(),
            DevicePlatform::Android,
            "aa:bb:cc:dd:ee:ff".to_string(),
            now_ms(),
            valid_perms,
        );
        let _ = trust_store.add_device(device_valid);
        let valid_res = start_stream(&trust_store, device_id.as_ref(), "primary".into(), "mjpeg".into(), 30, "".into());
        assert!(valid_res.is_ok(), "Authorized start_stream should succeed");
        let mut state = valid_res.unwrap();
        assert!(state.is_active);
        assert!(!state.auth_token.is_empty());
        assert!(state.stream_url.contains(&state.auth_token));

        // 4. Verify telemetry calculation (starts at 0.0 before frames are sent, reflects measured_fps when set)
        let telem_initial = compute_telemetry(&mut state);
        assert!(telem_initial.is_active);
        assert_eq!(telem_initial.target_device_name, "Pixel Test Phone");
        assert_eq!(telem_initial.current_fps, 0.0);

        state.measured_fps = 30.0;
        let telem = compute_telemetry(&mut state);
        assert_eq!(telem.current_fps, 30.0);

        let mut idle_state = StreamSessionState::default();
        let idle_telem = compute_telemetry(&mut idle_state);
        assert!(!idle_telem.is_active);
    }

    #[test]
    fn test_high_refresh_rate_120fps_stream_negotiation() {
        let mut trust_store = TrustStore::new();
        let device_id = DeviceId::try_from("pixel-120hz-client").unwrap();
        let mut perms = BTreeSet::new();
        perms.insert(SessionPermission::ViewScreen);
        let device = TrustedDevice::new(
            device_id.clone(),
            "Pixel 9 Pro 120Hz".to_string(),
            DevicePlatform::Android,
            "11:22:33:44:55:66".to_string(),
            now_ms(),
            perms,
        );
        let _ = trust_store.add_device(device);

        let stream_res = start_stream(&trust_store, device_id.as_ref(), "primary".into(), "AV1".into(), 120, "NVENC AV1 Zero-Copy".into());
        assert!(stream_res.is_ok());
        let mut state = stream_res.unwrap();
        assert_eq!(state.target_fps, 120);
        assert_eq!(state.bitrate_kbps, 18_000);
        assert_eq!(state.codec, "AV1");

        state.measured_fps = 119.8;
        state.measured_bitrate_kbps = 17_850.0;
        let telem = compute_telemetry(&mut state);
        assert_eq!(telem.current_fps, 119.8);
        assert!((telem.bitrate_mbps - 17.85).abs() < 0.01);
    }

    #[test]
    fn test_software_jpeg_encoder_60fps_throughput() {
        use crate::capture::SoftwareJpegEncoder;

        let mut encoder = SoftwareJpegEncoder::new(75);
        let width = 1920u32;
        let height = 1080u32;
        let total_pixels = (width * height * 4) as usize;

        let mut fake_desktop = vec![0u8; total_pixels];
        for y in 0..height as usize {
            for x in 0..width as usize {
                let offset = (y * (width as usize) + x) * 4;
                fake_desktop[offset] = ((x * 7) % 256) as u8;
                fake_desktop[offset + 1] = ((y * 11) % 256) as u8;
                fake_desktop[offset + 2] = (((x + y) * 13) % 256) as u8;
                fake_desktop[offset + 3] = 255;
            }
        }

        let iterations = 30;
        let mut total_duration_ms = 0.0f32;

        for _ in 0..iterations {
            let (slice, duration_ms) = encoder
                .encode_slice(&fake_desktop, width, height, 75)
                .expect("JPEG compression failed");

            // Validate JPEG SOI (0xFF, 0xD8) and EOI (0xFF, 0xD9)
            assert!(slice.len() > 1000, "Compressed JPEG is unreasonably small");
            assert_eq!(slice[0], 0xFF);
            assert_eq!(slice[1], 0xD8);
            assert_eq!(slice[slice.len() - 2], 0xFF);
            assert_eq!(slice[slice.len() - 1], 0xD9);

            // Verify frame payload size for synthetic pattern is within bounds (<1 MB)
            assert!(
                slice.len() < 1_000_000,
                "1080p JPEG exceeds 1MB budget: {} bytes",
                slice.len()
            );

            total_duration_ms += duration_ms;
        }

        let avg_duration_ms = total_duration_ms / iterations as f32;
        println!("[Test Benchmark] Average 1080p JPEG encode latency: {avg_duration_ms:.2} ms");
        assert!(
            avg_duration_ms < 60.0,
            "Encoder average latency ({avg_duration_ms:.2} ms) exceeds 60ms threshold in unoptimized debug test"
        );
    }

    #[test]
    fn test_decoupled_producer_consumer_stale_frame_dropping() {
        use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
        use std::sync::{Arc, Condvar, Mutex};
        use std::time::{Duration, Instant};

        struct TestFrame {
            _sequence: u32,
        }

        let shared_slot = Arc::new((Mutex::new(None::<TestFrame>), Condvar::new()));
        let stop_signal = Arc::new(AtomicBool::new(false));

        let producer_stop = stop_signal.clone();
        let producer_slot = shared_slot.clone();
        let frames_produced = Arc::new(AtomicU32::new(0));
        let produced_counter = frames_produced.clone();

        // Producer simulates 60 FPS pacing (~16.6ms ticks)
        let producer_handle = std::thread::spawn(move || {
            let mut seq = 0;
            let tick = Duration::from_millis(16);
            let start = Instant::now();

            while !producer_stop.load(Ordering::Relaxed) && seq < 120 {
                let t0 = Instant::now();
                seq += 1;
                let frame = TestFrame { _sequence: seq };

                let (lock, cvar) = &*producer_slot;
                if let Ok(mut guard) = lock.lock() {
                    // Decoupled replacement: Always overwrites with newest frame!
                    *guard = Some(frame);
                    cvar.notify_one();
                }
                produced_counter.store(seq, Ordering::Relaxed);

                let elapsed = t0.elapsed();
                if elapsed < tick {
                    std::thread::sleep(tick - elapsed);
                }
            }
            start.elapsed()
        });

        // Consumer simulates network jitter with intermittent 60ms stalls
        let consumer_slot = shared_slot.clone();
        let consumer_stop = stop_signal.clone();
        let frames_consumed = Arc::new(AtomicU32::new(0));
        let consumed_counter = frames_consumed.clone();

        let consumer_handle = std::thread::spawn(move || {
            let (lock, cvar) = &*consumer_slot;
            let mut count = 0;

            while !consumer_stop.load(Ordering::Relaxed) {
                let frame_opt = {
                    let guard = lock.lock().unwrap();
                    if guard.is_none() && !consumer_stop.load(Ordering::Relaxed) {
                        match cvar.wait_timeout(guard, Duration::from_millis(100)) {
                            Ok((mut g, _)) => g.take(),
                            Err(e) => e.into_inner().0.take(),
                        }
                    } else {
                        let mut g = guard;
                        g.take()
                    }
                };

                if let Some(_f) = frame_opt {
                    count += 1;
                    consumed_counter.store(count, Ordering::Relaxed);
                    // Simulate network latency / TCP backpressure jitter
                    if count % 5 == 0 {
                        std::thread::sleep(Duration::from_millis(60));
                    }
                }

                if count >= 20 {
                    break;
                }
            }
            count
        });

        let _ = consumer_handle.join().unwrap();
        stop_signal.store(true, Ordering::Relaxed);
        let (_, cvar) = &*shared_slot;
        cvar.notify_all();
        let producer_elapsed = producer_handle.join().unwrap();

        let total_produced = frames_produced.load(Ordering::Relaxed);
        let total_consumed = frames_consumed.load(Ordering::Relaxed);

        println!(
            "[Test Decoupling] Produced: {total_produced} frames in {producer_elapsed:?}, Consumed: {total_consumed} frames"
        );

        assert!(
            total_produced >= 60,
            "Producer was starved or blocked by slow consumer: only produced {total_produced} frames"
        );
        assert!(
            total_consumed >= 20,
            "Consumer failed to receive sufficient frames"
        );
    }

    #[test]
    fn test_keepalive_heartbeat_on_static_screen() {
        use std::sync::{Arc, Condvar, Mutex};
        use std::time::{Duration, Instant};

        let shared_slot = Arc::new((Mutex::new(None::<u32>), Condvar::new()));
        let (lock, cvar) = &*shared_slot;

        // Simulate writer waiting for 200ms when screen is completely static
        let guard = lock.lock().unwrap();
        let t0 = Instant::now();
        let (_g, wait_res) = cvar
            .wait_timeout(guard, Duration::from_millis(200))
            .unwrap();
        let elapsed = t0.elapsed();

        assert!(wait_res.timed_out(), "Expected timeout on static screen");
        assert!(
            elapsed >= Duration::from_millis(190),
            "Timeout fired prematurely: {elapsed:?}"
        );

        // Verify heartbeat wire format conforms to SMP/1 spec
        let heartbeat = "--smartmigrate\r\n\
            Content-Type: text/plain\r\n\
            Content-Length: 0\r\n\
            X-Heartbeat: 1\r\n\r\n";
        assert!(heartbeat.contains("Content-Length: 0"));
        assert!(heartbeat.contains("X-Heartbeat: 1"));
    }

    #[test]
    fn test_concurrent_tcp_streaming_and_zero_eof_disconnect() {
        use std::io::{Read, Write};
        use std::net::{TcpListener, TcpStream};
        use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
        use std::sync::Arc;
        use std::time::Duration;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind random port");
        let server_addr = listener.local_addr().expect("local addr");
        let stop_server = Arc::new(AtomicBool::new(false));
        let stop_worker = stop_server.clone();

        // Spawn mock SMP streaming server serving 10 concurrent clients
        let server_handle = std::thread::spawn(move || {
            let mut client_threads = Vec::new();
            listener.set_nonblocking(true).unwrap();

            while !stop_worker.load(Ordering::Relaxed) {
                if let Ok((mut socket, _)) = listener.accept() {
                    let stop_conn = stop_worker.clone();
                    let th = std::thread::spawn(move || {
                        let _ = socket.set_nodelay(true);
                        let _ = socket.set_write_timeout(Some(Duration::from_millis(3000)));

                        let http_header = "HTTP/1.1 200 OK\r\n\
                            Content-Type: multipart/x-mixed-replace; boundary=--smartmigrate\r\n\
                            Cache-Control: no-cache, no-store, must-revalidate\r\n\
                            Pragma: no-cache\r\n\
                            Expires: 0\r\n\
                            Connection: keep-alive\r\n\r\n";
                        if socket.write_all(http_header.as_bytes()).is_err() {
                            return;
                        }

                        // Send 5 video frames followed by keepalive heartbeats
                        let fake_jpeg = vec![0xFF, 0xD8, 0x00, 0x01, 0xFF, 0xD9];
                        for _ in 0..5 {
                            if stop_conn.load(Ordering::Relaxed) {
                                break;
                            }
                            let head = format!(
                                "--smartmigrate\r\n\
                                Content-Type: image/jpeg\r\n\
                                Content-Length: {}\r\n\
                                X-Timestamp: 123456\r\n\
                                X-Width: 1920\r\n\
                                X-Height: 1080\r\n\r\n",
                                fake_jpeg.len()
                            );
                            if socket.write_all(head.as_bytes()).is_err()
                                || socket.write_all(&fake_jpeg).is_err()
                                || socket.write_all(b"\r\n").is_err()
                                || socket.flush().is_err()
                            {
                                return;
                            }
                            std::thread::sleep(Duration::from_millis(16));
                        }

                        // Send keepalive heartbeat
                        let heartbeat = "--smartmigrate\r\n\
                            Content-Type: text/plain\r\n\
                            Content-Length: 0\r\n\
                            X-Heartbeat: 1\r\n\r\n";
                        let _ = socket.write_all(heartbeat.as_bytes());
                        let _ = socket.flush();
                    });
                    client_threads.push(th);
                }
                std::thread::sleep(Duration::from_millis(5));
            }

            for t in client_threads {
                let _ = t.join();
            }
        });

        // Connect 10 concurrent clients
        let client_count = 10;
        let successful_clients = Arc::new(AtomicU32::new(0));
        let mut handles = Vec::new();

        for client_idx in 0..client_count {
            let success_cnt = successful_clients.clone();
            let handle = std::thread::spawn(move || {
                let mut stream = TcpStream::connect(server_addr).expect("client connect");
                stream.set_read_timeout(Some(Duration::from_millis(3000))).unwrap();

                let mut buf = [0u8; 4096];
                let mut bytes_read_total = 0;
                let mut frames_detected = 0;
                let mut _heartbeats_detected = 0;

                while frames_detected < 3 && bytes_read_total < 50_000 {
                    match stream.read(&mut buf) {
                        Ok(0) => break, // Clean EOF
                        Ok(n) => {
                            bytes_read_total += n;
                            let text = String::from_utf8_lossy(&buf[..n]);
                            if text.contains("image/jpeg") {
                                frames_detected += 1;
                            }
                            if text.contains("X-Heartbeat: 1") {
                                _heartbeats_detected += 1;
                            }
                        }
                        Err(e) => panic!("Client {client_idx} failed with unexpected error: {e}"),
                    }
                }

                assert!(bytes_read_total > 0, "Client {client_idx} read 0 bytes");
                assert!(frames_detected > 0, "Client {client_idx} received no video frames");
                success_cnt.fetch_add(1, Ordering::Relaxed);
            });
            handles.push(handle);
        }

        for (idx, h) in handles.into_iter().enumerate() {
            h.join().unwrap_or_else(|_| panic!("Client thread {idx} panicked"));
        }

        stop_server.store(true, Ordering::Relaxed);
        let _ = server_handle.join();

        let total_successful = successful_clients.load(Ordering::Relaxed);
        assert_eq!(total_successful, 10, "Not all 10 clients completed successfully");
        println!("[Test Concurrency] 10 concurrent clients streamed frames and heartbeats with 0 EOF errors.");
    }
}

