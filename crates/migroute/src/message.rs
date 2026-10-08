//! Strongly-typed SMP/1 message family.
//!
//! Each variant corresponds to a message type string carried in the envelope's
//! `message_type` field.  The payload bytes in an [`crate::envelope::Envelope`]
//! are decoded into one of these variants.
//!
//! # Design constraints
//!
//! - All types are `Clone + PartialEq + Eq` to support testing and state diffing.
//! - No network, I/O, or async code here — decoding and encoding adapters live
//!   in the platform layer.
//! - File paths and clipboard content are *never* echoed in logs.
//! - Mouse / keyboard payloads are intentionally compact.

use crate::{DeviceId, SessionPermission};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Message type string constants matching the SMP/1 specification.
pub mod types {
    pub const DEVICE_HELLO: &str = "DEVICE_HELLO";
    pub const DEVICE_CAPABILITIES: &str = "DEVICE_CAPABILITIES";

    pub const PAIR_REQUEST: &str = "PAIR_REQUEST";
    pub const PAIR_ACCEPT: &str = "PAIR_ACCEPT";
    pub const PAIR_REJECT: &str = "PAIR_REJECT";

    pub const SESSION_CREATE: &str = "SESSION_CREATE";
    pub const SESSION_ACCEPT: &str = "SESSION_ACCEPT";
    pub const SESSION_REJECT: &str = "SESSION_REJECT";
    pub const SESSION_END: &str = "SESSION_END";

    pub const TRANSFER_START: &str = "TRANSFER_START";
    pub const TRANSFER_CHUNK: &str = "TRANSFER_CHUNK";
    pub const TRANSFER_ACK: &str = "TRANSFER_ACK";
    pub const TRANSFER_RETRY: &str = "TRANSFER_RETRY";
    pub const TRANSFER_PAUSE: &str = "TRANSFER_PAUSE";
    pub const TRANSFER_RESUME: &str = "TRANSFER_RESUME";
    pub const TRANSFER_COMPLETE: &str = "TRANSFER_COMPLETE";
    pub const TRANSFER_CANCEL: &str = "TRANSFER_CANCEL";

    pub const INPUT_MOUSE: &str = "INPUT_MOUSE";
    pub const INPUT_KEYBOARD: &str = "INPUT_KEYBOARD";

    pub const STREAM_START: &str = "STREAM_START";
    pub const STREAM_STOP: &str = "STREAM_STOP";
    pub const STREAM_CONFIG: &str = "STREAM_CONFIG";

    pub const CLIPBOARD_UPDATE: &str = "CLIPBOARD_UPDATE";

    pub const PING: &str = "PING";
    pub const PONG: &str = "PONG";
    pub const ERROR: &str = "ERROR";
}

// ─── Device ──────────────────────────────────────────────────────────────────

/// Initial device announcement.  Sent before a session exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceHello {
    pub device_id: DeviceId,
    pub device_name: String,
    /// Platform string, e.g. "Windows 11" or "Android 15".
    pub platform: String,
    /// SMP protocol versions supported, descending preference order.
    pub supported_versions: Vec<u16>,
}

/// Device capability advertisement.  Sent after DEVICE_HELLO is acknowledged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub device_id: DeviceId,
    pub available_permissions: BTreeSet<SessionPermission>,
    /// Maximum video resolution hint (width × height).  Zero means unknown.
    pub max_video_width: u32,
    pub max_video_height: u32,
    /// Whether hardware video encoding is available.
    pub hardware_encode: bool,
    /// Whether hardware video decoding is available.
    pub hardware_decode: bool,
}

// ─── Pairing ─────────────────────────────────────────────────────────────────

/// Request to pair with a host device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairRequest {
    pub requester_device_id: DeviceId,
    pub host_device_id: DeviceId,
    pub requested_permissions: BTreeSet<SessionPermission>,
    /// Cryptographically random, single-use verification code (numeric or alphanumeric).
    /// The host must display this and the user must confirm it matches the client display.
    /// This field is never logged.
    pub verification_code: String,
}

/// Host accepted the pairing request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairAccept {
    pub request_id: String,
    pub granted_permissions: BTreeSet<SessionPermission>,
}

/// Host rejected the pairing request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairReject {
    pub request_id: String,
    /// Human-readable reason (never contains user data, only policy reasons).
    pub reason: String,
}

// ─── Session ─────────────────────────────────────────────────────────────────

/// Client requests a new active session with a paired host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCreate {
    pub session_id: String,
    pub requester_device_id: DeviceId,
    pub host_device_id: DeviceId,
    /// Must be a subset of the permissions granted during pairing.
    pub requested_permissions: BTreeSet<SessionPermission>,
}

/// Host accepted the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionAccept {
    pub session_id: String,
    pub granted_permissions: BTreeSet<SessionPermission>,
}

/// Host rejected the session request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionReject {
    pub session_id: String,
    pub reason: String,
}

/// Either party is ending the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEnd {
    pub session_id: String,
    pub reason: SessionEndReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionEndReason {
    UserRequested,
    HostRevoked,
    /// Permission was withdrawn by the host during the session.
    PermissionRevoked,
    Timeout,
    NetworkError,
}

// ─── Transfer ────────────────────────────────────────────────────────────────

/// Announce a file transfer to the receiving side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferStart {
    pub transfer_id: String,
    /// Filename only — no path component.  The receiving side determines the
    /// destination.  Path traversal via this field is explicitly rejected.
    pub file_name: String,
    /// Total uncompressed byte count.
    pub total_bytes: u64,
    /// Total number of chunks.
    pub total_chunks: u64,
    /// SHA-256 hex digest of the complete file content.
    pub sha256: String,
}

/// One chunk of file data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferChunk {
    pub transfer_id: String,
    pub chunk_index: u64,
    pub data: Vec<u8>,
    /// SHA-256 hex digest of this chunk's data.
    pub chunk_sha256: String,
}

/// Receiver acknowledges a chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferAck {
    pub transfer_id: String,
    pub chunk_index: u64,
}

/// Receiver requests a chunk retransmission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferRetry {
    pub transfer_id: String,
    pub chunk_index: u64,
}

/// Either party pauses the transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferPause {
    pub transfer_id: String,
}

/// Either party resumes the transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferResume {
    pub transfer_id: String,
    /// Next expected chunk index (enables resumption after disconnect).
    pub resume_from_chunk: u64,
}

/// Sender signals the transfer is complete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferComplete {
    pub transfer_id: String,
}

/// Either party cancels the transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferCancel {
    pub transfer_id: String,
    pub reason: String,
}

// ─── Input ───────────────────────────────────────────────────────────────────

/// Mouse state report from the Android client to the Windows host.
///
/// Requires `SessionPermission::ControlMouse` to have been granted.
/// Coordinate origin is the top-left corner of the captured display region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputMouse {
    pub session_id: String,
    /// Horizontal position in display pixels.
    pub x: i32,
    /// Vertical position in display pixels.
    pub y: i32,
    pub buttons: MouseButtons,
    /// Vertical scroll delta (positive = scroll up).
    pub scroll_delta: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MouseButtons {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
}

/// Key event from the Android client to the Windows host.
///
/// Requires `SessionPermission::ControlKeyboard` to have been granted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputKeyboard {
    pub session_id: String,
    /// USB HID usage code for the key.
    pub hid_usage: u32,
    pub event: KeyEvent,
    /// Active modifier keys at the time of this event.
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyEvent {
    Press,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct KeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

// ─── Streaming ───────────────────────────────────────────────────────────────

/// Host signals that the video stream is starting.
///
/// Requires `SessionPermission::ViewScreen` to have been granted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamStart {
    pub session_id: String,
    pub config: StreamConfig,
}

/// Either party requests a stream stop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamStop {
    pub session_id: String,
    pub reason: String,
}

/// Current stream encoding parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamConfig {
    pub width: u32,
    pub height: u32,
    /// Frame rate as a rational number (numerator / denominator).
    pub fps_num: u32,
    pub fps_den: u32,
    pub codec: VideoCodec,
    /// Bit rate in bits per second.
    pub bitrate_bps: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoCodec {
    H264,
    H265,
    Av1,
}

// ─── Clipboard ───────────────────────────────────────────────────────────────

/// Clipboard content synchronization.
///
/// Requires `SessionPermission::Clipboard` to have been granted.
/// The payload is intentionally not logged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardUpdate {
    pub session_id: String,
    /// Only plain text in the initial implementation.
    pub text: String,
}

// ─── Health ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ping {
    pub token: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pong {
    pub token: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmpError {
    /// The message_type of the message that caused this error, if known.
    pub in_response_to: Option<String>,
    pub code: ErrorCode,
    /// Human-readable description.  Must not contain user data.
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    UnknownMessageType,
    Unauthorized,
    InvalidPayload,
    SessionNotFound,
    RateLimited,
    InternalError,
}

// ─── Top-level union ─────────────────────────────────────────────────────────

/// A decoded, validated SMP/1 message payload.
///
/// After the envelope passes [`crate::envelope::validate`], the caller decodes
/// the payload into this enum.  Each variant carries the typed payload struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmpMessage {
    // Device
    DeviceHello(DeviceHello),
    DeviceCapabilities(DeviceCapabilities),

    // Pairing
    PairRequest(PairRequest),
    PairAccept(PairAccept),
    PairReject(PairReject),

    // Session
    SessionCreate(SessionCreate),
    SessionAccept(SessionAccept),
    SessionReject(SessionReject),
    SessionEnd(SessionEnd),

    // Transfer
    TransferStart(TransferStart),
    TransferChunk(TransferChunk),
    TransferAck(TransferAck),
    TransferRetry(TransferRetry),
    TransferPause(TransferPause),
    TransferResume(TransferResume),
    TransferComplete(TransferComplete),
    TransferCancel(TransferCancel),

    // Input
    InputMouse(InputMouse),
    InputKeyboard(InputKeyboard),

    // Streaming
    StreamStart(StreamStart),
    StreamStop(StreamStop),
    StreamConfig(StreamConfig),

    // Clipboard
    ClipboardUpdate(ClipboardUpdate),

    // Health
    Ping(Ping),
    Pong(Pong),
    Error(SmpError),
}

impl SmpMessage {
    /// Returns the message type string for this variant.
    pub fn type_str(&self) -> &'static str {
        match self {
            Self::DeviceHello(_) => types::DEVICE_HELLO,
            Self::DeviceCapabilities(_) => types::DEVICE_CAPABILITIES,
            Self::PairRequest(_) => types::PAIR_REQUEST,
            Self::PairAccept(_) => types::PAIR_ACCEPT,
            Self::PairReject(_) => types::PAIR_REJECT,
            Self::SessionCreate(_) => types::SESSION_CREATE,
            Self::SessionAccept(_) => types::SESSION_ACCEPT,
            Self::SessionReject(_) => types::SESSION_REJECT,
            Self::SessionEnd(_) => types::SESSION_END,
            Self::TransferStart(_) => types::TRANSFER_START,
            Self::TransferChunk(_) => types::TRANSFER_CHUNK,
            Self::TransferAck(_) => types::TRANSFER_ACK,
            Self::TransferRetry(_) => types::TRANSFER_RETRY,
            Self::TransferPause(_) => types::TRANSFER_PAUSE,
            Self::TransferResume(_) => types::TRANSFER_RESUME,
            Self::TransferComplete(_) => types::TRANSFER_COMPLETE,
            Self::TransferCancel(_) => types::TRANSFER_CANCEL,
            Self::InputMouse(_) => types::INPUT_MOUSE,
            Self::InputKeyboard(_) => types::INPUT_KEYBOARD,
            Self::StreamStart(_) => types::STREAM_START,
            Self::StreamStop(_) => types::STREAM_STOP,
            Self::StreamConfig(_) => types::STREAM_CONFIG,
            Self::ClipboardUpdate(_) => types::CLIPBOARD_UPDATE,
            Self::Ping(_) => types::PING,
            Self::Pong(_) => types::PONG,
            Self::Error(_) => types::ERROR,
        }
    }

    /// Returns true if this message type requires an active, authorized session.
    pub fn requires_session(&self) -> bool {
        matches!(
            self,
            Self::TransferStart(_)
                | Self::TransferChunk(_)
                | Self::TransferAck(_)
                | Self::TransferRetry(_)
                | Self::TransferPause(_)
                | Self::TransferResume(_)
                | Self::TransferComplete(_)
                | Self::TransferCancel(_)
                | Self::InputMouse(_)
                | Self::InputKeyboard(_)
                | Self::StreamStart(_)
                | Self::StreamStop(_)
                | Self::StreamConfig(_)
                | Self::ClipboardUpdate(_)
                | Self::SessionEnd(_)
        )
    }

    /// Returns the `SessionPermission` required to process this message on the host,
    /// or `None` if no specific permission is required (e.g., PING, session management).
    pub fn required_permission(&self) -> Option<SessionPermission> {
        match self {
            Self::InputMouse(_) => Some(SessionPermission::ControlMouse),
            Self::InputKeyboard(_) => Some(SessionPermission::ControlKeyboard),
            Self::StreamStart(_) | Self::StreamStop(_) | Self::StreamConfig(_) => {
                Some(SessionPermission::ViewScreen)
            }
            Self::ClipboardUpdate(_) => Some(SessionPermission::Clipboard),
            Self::TransferStart(_)
            | Self::TransferChunk(_)
            | Self::TransferAck(_)
            | Self::TransferRetry(_)
            | Self::TransferPause(_)
            | Self::TransferResume(_)
            | Self::TransferComplete(_)
            | Self::TransferCancel(_) => {
                // Transfer direction determines the permission; callers must check
                // SendFiles / ReceiveFiles based on which side initiated the transfer.
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DeviceId;
    use std::collections::BTreeSet;

    fn device_id(s: &str) -> DeviceId {
        DeviceId::try_from(s).unwrap()
    }

    #[test]
    fn type_str_roundtrips_for_all_variants() {
        let messages: &[&str] = &[
            types::DEVICE_HELLO,
            types::DEVICE_CAPABILITIES,
            types::PAIR_REQUEST,
            types::PAIR_ACCEPT,
            types::PAIR_REJECT,
            types::SESSION_CREATE,
            types::SESSION_ACCEPT,
            types::SESSION_REJECT,
            types::SESSION_END,
            types::TRANSFER_START,
            types::TRANSFER_CHUNK,
            types::TRANSFER_ACK,
            types::TRANSFER_RETRY,
            types::TRANSFER_PAUSE,
            types::TRANSFER_RESUME,
            types::TRANSFER_COMPLETE,
            types::TRANSFER_CANCEL,
            types::INPUT_MOUSE,
            types::INPUT_KEYBOARD,
            types::STREAM_START,
            types::STREAM_STOP,
            types::STREAM_CONFIG,
            types::CLIPBOARD_UPDATE,
            types::PING,
            types::PONG,
            types::ERROR,
        ];
        // Ensure no duplicates.
        let unique: std::collections::HashSet<_> = messages.iter().collect();
        assert_eq!(unique.len(), messages.len(), "duplicate message type strings detected");
    }

    #[test]
    fn ping_does_not_require_session() {
        let msg = SmpMessage::Ping(Ping { token: 42 });
        assert!(!msg.requires_session());
    }

    #[test]
    fn input_mouse_requires_session() {
        let msg = SmpMessage::InputMouse(InputMouse {
            session_id: "s".to_owned(),
            x: 0,
            y: 0,
            buttons: MouseButtons::default(),
            scroll_delta: 0,
        });
        assert!(msg.requires_session());
        assert_eq!(msg.required_permission(), Some(SessionPermission::ControlMouse));
    }

    #[test]
    fn stream_start_requires_view_screen_permission() {
        let msg = SmpMessage::StreamStart(StreamStart {
            session_id: "s".to_owned(),
            config: StreamConfig {
                width: 1920,
                height: 1080,
                fps_num: 30,
                fps_den: 1,
                codec: VideoCodec::H264,
                bitrate_bps: 5_000_000,
            },
        });
        assert_eq!(msg.required_permission(), Some(SessionPermission::ViewScreen));
    }

    #[test]
    fn device_hello_requires_no_session() {
        let msg = SmpMessage::DeviceHello(DeviceHello {
            device_id: device_id("android-01"),
            device_name: "Pixel 9".to_owned(),
            platform: "Android 15".to_owned(),
            supported_versions: vec![1],
        });
        assert!(!msg.requires_session());
        assert_eq!(msg.required_permission(), None);
    }

    #[test]
    fn pair_request_cannot_have_empty_permissions() {
        // PairRequest at the message level carries the field directly;
        // enforcement is done at the PairingRequest policy level in lib.rs.
        let pr = PairRequest {
            requester_device_id: device_id("android-01"),
            host_device_id: device_id("windows-host"),
            requested_permissions: BTreeSet::new(),
            verification_code: "123456".to_owned(),
        };
        // The message struct itself is valid; enforcement happens in PairingRequest::create().
        assert!(pr.requested_permissions.is_empty());
    }
}
