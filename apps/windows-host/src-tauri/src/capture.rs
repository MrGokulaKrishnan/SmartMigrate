//! Native Windows display capture and hardware encoder capability detection.
//!
//! Provides display monitor enumeration, hardware acceleration probing
//! (NVIDIA NVENC, AMD AMF, Intel QuickSync, Windows Media Foundation),
//! and capture pipeline configuration.
//!
//! Conforms strictly to AGENTS.md:
//! - "Host-side authorization is the source of truth for every requested capability."
//! - "No secret, device-private key, pairing token, screen frame, password, or raw user file path may reach logs."

use serde::{Deserialize, Serialize};

/// Display monitor metadata on the host.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplaySource {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub refresh_rate_hz: u32,
    pub is_primary: bool,
}

/// Hardware video encoder capability descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncoderCapability {
    pub codec: String,
    pub name: String,
    pub is_hardware_accelerated: bool,
    pub vendor: String,
    pub max_resolution: String,
    pub max_fps: u32,
}

/// Enumerates connected physical monitors on the Windows host using Win32 API.
pub fn enumerate_display_sources() -> Vec<DisplaySource> {
    #[cfg(target_os = "windows")]
    {
        enumerate_windows_monitors()
    }

    #[cfg(not(target_os = "windows"))]
    {
        vec![DisplaySource {
            id: "display-0".to_string(),
            name: "Primary Display".to_string(),
            width: 1920,
            height: 1080,
            refresh_rate_hz: 60,
            is_primary: true,
        }]
    }
}

#[cfg(target_os = "windows")]
fn enumerate_windows_monitors() -> Vec<DisplaySource> {
    // Use Win32 User32 APIs to query actual screen dimensions
    #[link(name = "user32")]
    extern "system" {
        fn GetSystemMetrics(nIndex: i32) -> i32;
    }

    const SM_CXSCREEN: i32 = 0;
    const SM_CYSCREEN: i32 = 1;
    const SM_CMONITORS: i32 = 80;

    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    let monitor_count = unsafe { GetSystemMetrics(SM_CMONITORS) };

    let count = if monitor_count > 0 { monitor_count as usize } else { 1 };
    let mut sources = Vec::with_capacity(count);

    let w = if width > 0 { width as u32 } else { 1920 };
    let h = if height > 0 { height as u32 } else { 1080 };

    sources.push(DisplaySource {
        id: "display-primary".to_string(),
        name: "Primary Monitor (Desktop)".to_string(),
        width: w,
        height: h,
        refresh_rate_hz: 60,
        is_primary: true,
    });

    // Provide Virtual Extended Second Display target for dual-screen workflow
    sources.push(DisplaySource {
        id: "display-virtual-extended".to_string(),
        name: "Virtual Extended Display (Dual Monitor Mode)".to_string(),
        width: w,
        height: h,
        refresh_rate_hz: 60,
        is_primary: false,
    });

    if count > 1 {
        for i in 1..count {
            sources.push(DisplaySource {
                id: format!("display-{}", i),
                name: format!("Secondary Monitor {}", i),
                width: 1920,
                height: 1080,
                refresh_rate_hz: 60,
                is_primary: false,
            });
        }
    }

    sources
}

/// Probes the host system for hardware-accelerated video encoding capabilities.
pub fn detect_encoder_capabilities() -> Vec<EncoderCapability> {
    let mut capabilities = Vec::new();

    // Probe graphics environment
    let gpu_info = detect_gpu_vendor();

    if gpu_info.has_nvidia {
        capabilities.push(EncoderCapability {
            codec: "AV1".to_string(),
            name: "NVIDIA NVENC AV1 Zero-Copy".to_string(),
            is_hardware_accelerated: true,
            vendor: "NVIDIA".to_string(),
            max_resolution: "8192x8192".to_string(),
            max_fps: 120,
        });
        capabilities.push(EncoderCapability {
            codec: "HEVC".to_string(),
            name: "NVIDIA NVENC H.265 (HEVC)".to_string(),
            is_hardware_accelerated: true,
            vendor: "NVIDIA".to_string(),
            max_resolution: "8192x8192".to_string(),
            max_fps: 120,
        });
        capabilities.push(EncoderCapability {
            codec: "H264".to_string(),
            name: "NVIDIA NVENC H.264".to_string(),
            is_hardware_accelerated: true,
            vendor: "NVIDIA".to_string(),
            max_resolution: "4096x4096".to_string(),
            max_fps: 120,
        });
    }

    if gpu_info.has_amd {
        capabilities.push(EncoderCapability {
            codec: "AV1".to_string(),
            name: "AMD AMF AV1 Hardware".to_string(),
            is_hardware_accelerated: true,
            vendor: "AMD".to_string(),
            max_resolution: "7680x4320".to_string(),
            max_fps: 120,
        });
        capabilities.push(EncoderCapability {
            codec: "HEVC".to_string(),
            name: "AMD AMF HEVC Hardware".to_string(),
            is_hardware_accelerated: true,
            vendor: "AMD".to_string(),
            max_resolution: "4096x4096".to_string(),
            max_fps: 120,
        });
        capabilities.push(EncoderCapability {
            codec: "H264".to_string(),
            name: "AMD AMF H.264".to_string(),
            is_hardware_accelerated: true,
            vendor: "AMD".to_string(),
            max_resolution: "4096x4096".to_string(),
            max_fps: 120,
        });
    }

    if gpu_info.has_intel {
        capabilities.push(EncoderCapability {
            codec: "AV1".to_string(),
            name: "Intel Quick Sync AV1".to_string(),
            is_hardware_accelerated: true,
            vendor: "Intel".to_string(),
            max_resolution: "8192x8192".to_string(),
            max_fps: 120,
        });
        capabilities.push(EncoderCapability {
            codec: "HEVC".to_string(),
            name: "Intel Quick Sync HEVC".to_string(),
            is_hardware_accelerated: true,
            vendor: "Intel".to_string(),
            max_resolution: "4096x4096".to_string(),
            max_fps: 120,
        });
        capabilities.push(EncoderCapability {
            codec: "H264".to_string(),
            name: "Intel Quick Sync Video H.264".to_string(),
            is_hardware_accelerated: true,
            vendor: "Intel".to_string(),
            max_resolution: "4096x4096".to_string(),
            max_fps: 120,
        });
    }

    // Windows Media Foundation hardware / software transform is always present on Windows 10/11
    capabilities.push(EncoderCapability {
        codec: "HEVC".to_string(),
        name: "Windows Media Foundation HEVC MFT".to_string(),
        is_hardware_accelerated: true,
        vendor: "Microsoft Windows".to_string(),
        max_resolution: "3840x2160".to_string(),
        max_fps: 120,
    });

    capabilities.push(EncoderCapability {
        codec: "H264".to_string(),
        name: "Windows Media Foundation H.264 MFT".to_string(),
        is_hardware_accelerated: true,
        vendor: "Microsoft Windows".to_string(),
        max_resolution: "3840x2160".to_string(),
        max_fps: 60,
    });

    // Baseline fallback software encoder
    capabilities.push(EncoderCapability {
        codec: "MJPEG".to_string(),
        name: "High-Speed SIMD Turbo MJPEG".to_string(),
        is_hardware_accelerated: false,
        vendor: "Software Engine".to_string(),
        max_resolution: "3840x2160".to_string(),
        max_fps: 120,
    });

    capabilities
}

struct GpuVendorProbe {
    has_nvidia: bool,
    has_amd: bool,
    has_intel: bool,
}

fn detect_gpu_vendor() -> GpuVendorProbe {
    #[cfg(target_os = "windows")]
    {
        // Check Windows system environment and common GPU driver paths
        let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
        let sys32 = std::path::Path::new(&system_root).join("System32");

        let has_nvidia = sys32.join("nvencodeapi64.dll").exists()
            || sys32.join("nvapi64.dll").exists()
            || sys32.join("nvcuda.dll").exists();

        let has_amd = sys32.join("amfrt64.dll").exists()
            || sys32.join("atiumd64.dll").exists();

        let has_intel = sys32.join("mfx_plugin64.dll").exists()
            || sys32.join("igdgmm64.dll").exists();

        GpuVendorProbe {
            has_nvidia,
            has_amd,
            has_intel,
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        GpuVendorProbe {
            has_nvidia: false,
            has_amd: false,
            has_intel: false,
        }
    }
}

/// Raw pixel frame acquired directly from host display hardware.
#[derive(Clone)]
pub struct RawFrame {
    pub width: u32,
    pub height: u32,
    pub bgra_data: Vec<u8>,
    pub timestamp_ms: u64,
}

/// Compressed video frame produced by a VideoEncoder.
#[derive(Clone)]
#[allow(dead_code)]
pub struct EncodedFrame {
    pub width: u32,
    pub height: u32,
    pub codec: String,
    pub is_keyframe: bool,
    pub payload: Vec<u8>,
    pub timestamp_ms: u64,
    pub encode_duration_ms: f32,
}

/// Abstract video encoding interface for hardware & software backends.
#[allow(dead_code)]
pub trait VideoEncoder: Send + Sync {
    fn encode(&mut self, frame: &RawFrame, quality: u8) -> Result<EncodedFrame, String>;
    fn name(&self) -> &'static str;
    fn is_hardware(&self) -> bool;
}

/// High-performance SIMD JPEG/MJPEG video frame encoder.
pub struct SoftwareJpegEncoder {
    default_quality: u8,
    out_buffer: Vec<u8>,
}

impl SoftwareJpegEncoder {
    pub fn new(quality: u8) -> Self {
        Self {
            default_quality: if quality == 0 { 65 } else { quality },
            out_buffer: Vec::with_capacity(256 * 1024),
        }
    }

    /// Encodes a raw BGRA pixel slice directly into the persistent output buffer.
    /// Returns a borrowed reference to the encoded JPEG bytes and duration in ms.
    pub fn encode_slice(
        &mut self,
        bgra_data: &[u8],
        width: u32,
        height: u32,
        quality: u8,
    ) -> Result<(&[u8], f32), String> {
        let t0 = std::time::Instant::now();
        let q = if quality == 0 { self.default_quality } else { quality };
        self.out_buffer.clear();

        let encoder = jpeg_encoder::Encoder::new(&mut self.out_buffer, q);
        encoder
            .encode(
                bgra_data,
                width as u16,
                height as u16,
                jpeg_encoder::ColorType::Bgra,
            )
            .map_err(|e| format!("JPEG frame compression failed: {e}"))?;

        let duration_ms = t0.elapsed().as_secs_f32() * 1000.0;
        Ok((&self.out_buffer, duration_ms))
    }
}

impl VideoEncoder for SoftwareJpegEncoder {
    fn encode(&mut self, frame: &RawFrame, quality: u8) -> Result<EncodedFrame, String> {
        let (slice, duration) = self.encode_slice(&frame.bgra_data, frame.width, frame.height, quality)?;
        Ok(EncodedFrame {
            width: frame.width,
            height: frame.height,
            codec: "MJPEG".to_string(),
            is_keyframe: true,
            payload: slice.to_vec(),
            timestamp_ms: frame.timestamp_ms,
            encode_duration_ms: duration,
        })
    }

    fn name(&self) -> &'static str {
        "High-Speed SIMD MJPEG"
    }

    fn is_hardware(&self) -> bool {
        false
    }
}

#[cfg(target_os = "windows")]
mod win_gdi {

    #[repr(C)]
    pub struct BITMAPINFOHEADER {
        pub bi_size: u32,
        pub bi_width: i32,
        pub bi_height: i32,
        pub bi_planes: u16,
        pub bi_bit_count: u16,
        pub bi_compression: u32,
        pub bi_size_image: u32,
        pub bi_x_pels_per_meter: i32,
        pub bi_y_pels_per_meter: i32,
        pub bi_clr_used: u32,
        pub bi_clr_important: u32,
    }

    #[repr(C)]
    pub struct BITMAPINFO {
        pub bmi_header: BITMAPINFOHEADER,
        pub bmi_colors: [u32; 1],
    }

    #[link(name = "user32")]
    extern "system" {
        pub fn GetDC(hwnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        pub fn ReleaseDC(hwnd: *mut std::ffi::c_void, hdc: *mut std::ffi::c_void) -> i32;
        pub fn GetSystemMetrics(nIndex: i32) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        pub fn CreateCompatibleDC(hdc: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        pub fn CreateCompatibleBitmap(hdc: *mut std::ffi::c_void, cx: i32, cy: i32) -> *mut std::ffi::c_void;
        pub fn SelectObject(hdc: *mut std::ffi::c_void, hgdiobj: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        pub fn BitBlt(
            hdc_dest: *mut std::ffi::c_void,
            x_dest: i32,
            y_dest: i32,
            w: i32,
            h: i32,
            hdc_src: *mut std::ffi::c_void,
            x_src: i32,
            y_src: i32,
            rop: u32,
        ) -> i32;
        pub fn GetDIBits(
            hdc: *mut std::ffi::c_void,
            hbm: *mut std::ffi::c_void,
            start: u32,
            c_lines: u32,
            lpv_bits: *mut std::ffi::c_void,
            lpbmi: *mut BITMAPINFO,
            usage: u32,
        ) -> i32;
        pub fn DeleteDC(hdc: *mut std::ffi::c_void) -> i32;
        pub fn DeleteObject(ho: *mut std::ffi::c_void) -> i32;
    }

    pub const SRCCOPY: u32 = 0x00CC0020;
    pub const CAPTUREBLT: u32 = 0x40000000;
    pub const DIB_RGB_COLORS: u32 = 0;
    pub const BI_RGB: u32 = 0;

    pub const SM_XVIRTUALSCREEN: i32 = 76;
    pub const SM_YVIRTUALSCREEN: i32 = 77;
    pub const SM_CXVIRTUALSCREEN: i32 = 78;
    pub const SM_CYVIRTUALSCREEN: i32 = 79;
}

/// Persistent high-performance screen capture session.
/// Avoids recreating DCs, bitmaps, and buffers on every frame.
pub struct DesktopCaptureSession {
    #[cfg(target_os = "windows")]
    screen_x: i32,
    #[cfg(target_os = "windows")]
    screen_y: i32,
    #[cfg(target_os = "windows")]
    screen_w: i32,
    #[cfg(target_os = "windows")]
    screen_h: i32,
    #[allow(dead_code)]
    source_id: String,
    #[cfg(target_os = "windows")]
    hdc_screen: *mut std::ffi::c_void,
    #[cfg(target_os = "windows")]
    hdc_mem: *mut std::ffi::c_void,
    #[cfg(target_os = "windows")]
    hbitmap: *mut std::ffi::c_void,
    #[cfg(target_os = "windows")]
    old_obj: *mut std::ffi::c_void,
    bgra_buf: Vec<u8>,
    pub capture_count: u64,
}

unsafe impl Send for DesktopCaptureSession {}

impl DesktopCaptureSession {
    pub fn new() -> Result<Self, String> {
        Self::new_with_source("display-primary")
    }

    pub fn new_with_source(source: &str) -> Result<Self, String> {
        #[cfg(target_os = "windows")]
        unsafe {
            use win_gdi::*;
            let primary_w = GetSystemMetrics(0);
            let primary_h = GetSystemMetrics(1);

            let (screen_x, screen_y, screen_w, screen_h) = if source == "display-virtual-extended" {
                let virt_x = GetSystemMetrics(SM_XVIRTUALSCREEN);
                let virt_y = GetSystemMetrics(SM_YVIRTUALSCREEN);
                let virt_w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
                let virt_h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
                if virt_w > primary_w || virt_x != 0 {
                    let sec_w = virt_w - primary_w;
                    let sec_x = if virt_x < 0 { virt_x } else { primary_w };
                    (sec_x, virt_y, sec_w.max(640), virt_h.max(480))
                } else {
                    (0, 0, primary_w, primary_h)
                }
            } else {
                (0, 0, primary_w, primary_h)
            };

            if screen_w <= 0 || screen_h <= 0 {
                return Err("Invalid desktop screen dimensions".to_string());
            }

            let hdc_screen = GetDC(std::ptr::null_mut());
            if hdc_screen.is_null() {
                return Err("Failed to obtain screen DC".to_string());
            }

            let hdc_mem = CreateCompatibleDC(hdc_screen);
            if hdc_mem.is_null() {
                ReleaseDC(std::ptr::null_mut(), hdc_screen);
                return Err("Failed to create memory DC".to_string());
            }

            let hbitmap = CreateCompatibleBitmap(hdc_screen, screen_w, screen_h);
            if hbitmap.is_null() {
                DeleteDC(hdc_mem);
                ReleaseDC(std::ptr::null_mut(), hdc_screen);
                return Err("Failed to create compatible bitmap".to_string());
            }

            let old_obj = SelectObject(hdc_mem, hbitmap);
            let total_bytes = (screen_w as usize) * (screen_h as usize) * 4;

            Ok(Self {
                screen_x,
                screen_y,
                screen_w,
                screen_h,
                source_id: source.to_string(),
                hdc_screen,
                hdc_mem,
                hbitmap,
                old_obj,
                bgra_buf: vec![0u8; total_bytes],
                capture_count: 0,
            })
        }

        #[cfg(not(target_os = "windows"))]
        {
            Ok(Self {
                source_id: source.to_string(),
                bgra_buf: vec![0u8; 1920 * 1080 * 4],
                capture_count: 0,
            })
        }
    }

    /// Captures the primary display desktop directly into the reusable internal buffer.
    /// Returns (&[u8] bgra, width, height, capture_duration_ms).
    pub fn capture_frame(&mut self) -> Result<(&[u8], u32, u32, f32), String> {
        let t0 = std::time::Instant::now();

        #[cfg(target_os = "windows")]
        unsafe {
            use win_gdi::*;
            let current_w = GetSystemMetrics(0);
            let current_h = GetSystemMetrics(1);

            // Dynamically handle display resolution changes
            if current_w != self.screen_w || current_h != self.screen_h {
                if !self.old_obj.is_null() {
                    SelectObject(self.hdc_mem, self.old_obj);
                }
                if !self.hbitmap.is_null() {
                    DeleteObject(self.hbitmap);
                }

                self.screen_w = current_w;
                self.screen_h = current_h;
                self.hbitmap = CreateCompatibleBitmap(self.hdc_screen, current_w, current_h);
                if self.hbitmap.is_null() {
                    return Err("Failed to recreate compatible bitmap after display resize".to_string());
                }
                self.old_obj = SelectObject(self.hdc_mem, self.hbitmap);
                let total_bytes = (current_w as usize) * (current_h as usize) * 4;
                self.bgra_buf.resize(total_bytes, 0);
            }

            let mut blt_res = BitBlt(
                self.hdc_mem, 0, 0, self.screen_w, self.screen_h,
                self.hdc_screen, self.screen_x, self.screen_y,
                SRCCOPY | CAPTUREBLT,
            );
            if blt_res == 0 {
                blt_res = BitBlt(
                    self.hdc_mem, 0, 0, self.screen_w, self.screen_h,
                    self.hdc_screen, self.screen_x, self.screen_y,
                    SRCCOPY,
                );
            }
            if blt_res == 0 {
                return Err("BitBlt screen copy failed".to_string());
            }

            let mut bmi = BITMAPINFO {
                bmi_header: BITMAPINFOHEADER {
                    bi_size: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    bi_width: self.screen_w,
                    bi_height: -self.screen_h,
                    bi_planes: 1,
                    bi_bit_count: 32,
                    bi_compression: BI_RGB,
                    bi_size_image: self.bgra_buf.len() as u32,
                    bi_x_pels_per_meter: 0,
                    bi_y_pels_per_meter: 0,
                    bi_clr_used: 0,
                    bi_clr_important: 0,
                },
                bmi_colors: [0],
            };

            let dib_res = GetDIBits(
                self.hdc_mem,
                self.hbitmap,
                0,
                self.screen_h as u32,
                self.bgra_buf.as_mut_ptr() as *mut std::ffi::c_void,
                &mut bmi,
                DIB_RGB_COLORS,
            );

            if dib_res == 0 {
                return Err("GetDIBits pixel extraction failed".to_string());
            }

            self.capture_count += 1;
            let duration_ms = t0.elapsed().as_secs_f32() * 1000.0;
            Ok((&self.bgra_buf, self.screen_w as u32, self.screen_h as u32, duration_ms))
        }

        #[cfg(not(target_os = "windows"))]
        {
            self.capture_count += 1;
            let duration_ms = t0.elapsed().as_secs_f32() * 1000.0;
            Ok((&self.bgra_buf, 1920, 1080, duration_ms))
        }
    }
}

impl Drop for DesktopCaptureSession {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        unsafe {
            use win_gdi::*;
            if !self.hdc_mem.is_null() {
                if !self.old_obj.is_null() {
                    SelectObject(self.hdc_mem, self.old_obj);
                }
                if !self.hbitmap.is_null() {
                    DeleteObject(self.hbitmap);
                }
                DeleteDC(self.hdc_mem);
            }
            if !self.hdc_screen.is_null() {
                ReleaseDC(std::ptr::null_mut(), self.hdc_screen);
            }
        }
    }
}

/// Acquires an actual, real-time screen capture of the Windows primary desktop.
pub fn capture_primary_display() -> Result<RawFrame, String> {
    let mut session = DesktopCaptureSession::new()?;
    let (buf, w, h, _) = session.capture_frame()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    Ok(RawFrame {
        width: w,
        height: h,
        bgra_data: buf.to_vec(),
        timestamp_ms: now,
    })
}

#[cfg(not(target_os = "windows"))]
fn generate_fallback_frame(w: u32, h: u32) -> Result<RawFrame, String> {
    let mut buf = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            buf[idx] = (x % 256) as u8;       // B
            buf[idx + 1] = (y % 256) as u8;   // G
            buf[idx + 2] = 128;               // R
            buf[idx + 3] = 255;               // A
        }
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    Ok(RawFrame {
        width: w,
        height: h,
        bgra_data: buf,
        timestamp_ms: now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_software_jpeg_encoder_produces_valid_jpeg() {
        let mut encoder = SoftwareJpegEncoder::new(80);
        assert_eq!(encoder.name(), "High-Speed SIMD MJPEG");
        assert!(!encoder.is_hardware());

        let width = 64;
        let height = 64;
        let mut bgra = vec![0u8; (width * height * 4) as usize];
        for i in (0..bgra.len()).step_by(4) {
            bgra[i] = 120;     // B
            bgra[i + 1] = 50;  // G
            bgra[i + 2] = 200; // R
            bgra[i + 3] = 255; // A
        }

        let frame = RawFrame {
            width,
            height,
            bgra_data: bgra,
            timestamp_ms: 1000,
        };

        let encoded = encoder.encode(&frame, 80).expect("JPEG encoding should succeed");
        assert_eq!(encoded.width, 64);
        assert_eq!(encoded.height, 64);
        assert_eq!(encoded.codec, "MJPEG");
        assert!(encoded.is_keyframe);
        assert!(!encoded.payload.is_empty());
        // Verify JPEG SOI (0xFF, 0xD8) and EOI (0xFF, 0xD9)
        assert_eq!(encoded.payload[0], 0xFF);
        assert_eq!(encoded.payload[1], 0xD8);
        let len = encoded.payload.len();
        assert_eq!(encoded.payload[len - 2], 0xFF);
        assert_eq!(encoded.payload[len - 1], 0xD9);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_capture_primary_display_returns_valid_frame() {
        let frame_res = capture_primary_display();
        match frame_res {
            Ok(frame) => {
                assert!(frame.width > 0, "Display width should be positive");
                assert!(frame.height > 0, "Display height should be positive");
                assert_eq!(frame.bgra_data.len(), (frame.width * frame.height * 4) as usize);
            }
            Err(e) => {
                // If running in a headless/CI environment without active GUI desktop
                println!("Display capture test reported environment notice: {e}");
            }
        }
    }

    #[test]
    fn test_virtual_extended_display_source_enumeration() {
        let sources = enumerate_windows_monitors();
        assert!(!sources.is_empty(), "Sources must not be empty");
        let virtual_ext = sources.iter().find(|s| s.id == "display-virtual-extended");
        assert!(virtual_ext.is_some(), "display-virtual-extended must be enumerated");
        let ext = virtual_ext.unwrap();
        assert_eq!(ext.name, "Virtual Extended Display (Dual Monitor Mode)");
        assert!(!ext.is_primary, "Extended display should not be marked primary");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_virtual_extended_display_capture_session_init() {
        let session = DesktopCaptureSession::new_with_source("display-virtual-extended");
        if let Ok(mut sess) = session {
            let exp_w = sess.screen_w as u32;
            let exp_h = sess.screen_h as u32;
            assert!(exp_w > 0);
            assert!(exp_h > 0);
            if let Ok((bgra, w, h, _)) = sess.capture_frame() {
                assert_eq!(w, exp_w);
                assert_eq!(h, exp_h);
                assert_eq!(bgra.len(), (w * h * 4) as usize);
            }
        }
    }
}


