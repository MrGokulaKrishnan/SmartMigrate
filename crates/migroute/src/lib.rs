//! MigRoute is the Smart Migrate systems-engine boundary.
//! This crate intentionally models authorization and state only; platform I/O
//! belongs behind reviewed adapters in later milestones.
//!
//! # Module layout
//!
//! - [`DeviceId`] — validated, transport-safe device identifier
//! - [`SessionPermission`] — granular host-controlled capabilities
//! - [`PairingRequest`] / [`PairingState`] — pairing lifecycle with permission narrowing
//! - [`envelope`] — SMP/1 envelope validation (version, expiry, replay, session binding)
//! - [`session`] — session lifecycle state machine
//! - [`message`] — strongly-typed SMP/1 message family
//! - [`clipboard`] — clipboard sync policy engine (grant state, direction, size validation)

pub mod clipboard;
pub mod envelope;
pub mod identity;
pub mod message;
pub mod pairing;
pub mod session;
pub mod transfer;
pub mod trust;

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

// Re-export primary types at the crate root for ergonomics.
pub use identity::{DeviceIdentity, DevicePlatform, DeviceRole, IdentityError};
pub use clipboard::{
    validate_clipboard_update, ClipboardDirection, ClipboardError, ClipboardGrant,
    MAX_CLIPBOARD_BYTES,
};
pub use pairing::{NumericPairingCode, PairingSession, PairingVerificationError};
pub use session::{Session, SessionError, SessionState};
pub use transfer::{
    compute_sha256, sanitize_file_name, TransferDirection, TransferError, TransferSession,
    TransferState, DEFAULT_CHUNK_SIZE, MAX_CHUNK_SIZE, MAX_FILE_SIZE,
};
pub use trust::{TrustStore, TrustStoreError, TrustedDevice};

pub const ENGINE_NAME: &str = "MigRoute";
pub const PROTOCOL_VERSION: u16 = 1;

// ─── DeviceId ────────────────────────────────────────────────────────────────

/// A validated, transport-safe device identifier.
///
/// Allowed characters: ASCII alphanumeric plus `-` and `_`.
/// Maximum length: 128 bytes.  Must be non-empty.
///
/// These constraints ensure the ID is safe across all wire formats (JSON, protobuf,
/// URL path segments, log entries) without escaping.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeviceId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceIdError {
    Empty,
    InvalidCharacter,
    TooLong,
}

impl fmt::Display for DeviceIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("device ID cannot be empty"),
            Self::InvalidCharacter => f.write_str("device ID contains an unsupported character"),
            Self::TooLong => f.write_str("device ID is longer than 128 characters"),
        }
    }
}

impl std::error::Error for DeviceIdError {}

impl TryFrom<&str> for DeviceId {
    type Error = DeviceIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(DeviceIdError::Empty);
        }
        if value.len() > 128 {
            return Err(DeviceIdError::TooLong);
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(DeviceIdError::InvalidCharacter);
        }
        Ok(Self(value.to_owned()))
    }
}

impl AsRef<str> for DeviceId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ─── SessionPermission ───────────────────────────────────────────────────────

/// Granular capabilities that a host may grant to an approved client.
///
/// Nothing is implicitly granted.  Each permission must be explicitly requested
/// during pairing and independently approved by the host.  The host may narrow
/// (but never widen) the set of granted permissions at decision time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionPermission {
    ViewScreen,
    ControlMouse,
    ControlKeyboard,
    SendFiles,
    ReceiveFiles,
    Clipboard,
    Audio,
}

impl fmt::Display for SessionPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ViewScreen => "VIEW_SCREEN",
            Self::ControlMouse => "CONTROL_MOUSE",
            Self::ControlKeyboard => "CONTROL_KEYBOARD",
            Self::SendFiles => "SEND_FILES",
            Self::ReceiveFiles => "RECEIVE_FILES",
            Self::Clipboard => "CLIPBOARD",
            Self::Audio => "AUDIO",
        })
    }
}

// ─── PairingRequest ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairingState {
    Requested,
    AwaitingHostApproval,
    Approved,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingRequest {
    pub requester: DeviceId,
    pub host: DeviceId,
    pub requested_permissions: BTreeSet<SessionPermission>,
    pub state: PairingState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairingError {
    SelfPairing,
    NoPermissionsRequested,
    NotAwaitingApproval,
}

impl fmt::Display for PairingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelfPairing => f.write_str("a device cannot pair with itself"),
            Self::NoPermissionsRequested => {
                f.write_str("a pairing request needs at least one permission")
            }
            Self::NotAwaitingApproval => {
                f.write_str("only pending pairing requests can be decided")
            }
        }
    }
}

impl std::error::Error for PairingError {}

impl PairingRequest {
    pub fn create(
        requester: DeviceId,
        host: DeviceId,
        requested_permissions: impl IntoIterator<Item = SessionPermission>,
    ) -> Result<Self, PairingError> {
        if requester == host {
            return Err(PairingError::SelfPairing);
        }
        let requested_permissions = requested_permissions.into_iter().collect::<BTreeSet<_>>();
        if requested_permissions.is_empty() {
            return Err(PairingError::NoPermissionsRequested);
        }
        Ok(Self {
            requester,
            host,
            requested_permissions,
            state: PairingState::AwaitingHostApproval,
        })
    }

    /// A host can only narrow the requested scope, never add authority.
    pub fn decide(
        &mut self,
        granted_permissions: impl IntoIterator<Item = SessionPermission>,
        approve: bool,
    ) -> Result<BTreeSet<SessionPermission>, PairingError> {
        if self.state != PairingState::AwaitingHostApproval {
            return Err(PairingError::NotAwaitingApproval);
        }

        if !approve {
            self.state = PairingState::Rejected;
            return Ok(BTreeSet::new());
        }

        let granted = granted_permissions
            .into_iter()
            .filter(|permission| self.requested_permissions.contains(permission))
            .collect::<BTreeSet<_>>();
        self.state = PairingState::Approved;
        Ok(granted)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> DeviceId {
        DeviceId::try_from(value).expect("test device ID should be valid")
    }

    #[test]
    fn ids_are_constrained_to_safe_transport_characters() {
        assert!(DeviceId::try_from("host-PC_01").is_ok());
        assert_eq!(
            DeviceId::try_from("host/../pc"),
            Err(DeviceIdError::InvalidCharacter)
        );
    }

    #[test]
    fn id_cannot_be_empty() {
        assert_eq!(DeviceId::try_from(""), Err(DeviceIdError::Empty));
    }

    #[test]
    fn id_cannot_exceed_128_chars() {
        let long = "a".repeat(129);
        assert_eq!(DeviceId::try_from(long.as_str()), Err(DeviceIdError::TooLong));
        let max = "a".repeat(128);
        assert!(DeviceId::try_from(max.as_str()).is_ok());
    }

    #[test]
    fn host_can_only_grant_requested_permissions() {
        let mut request = PairingRequest::create(
            id("android-client"),
            id("windows-host"),
            [SessionPermission::ViewScreen],
        )
        .unwrap();

        let granted = request
            .decide(
                [SessionPermission::ViewScreen, SessionPermission::ControlKeyboard],
                true,
            )
            .unwrap();

        assert_eq!(granted, BTreeSet::from([SessionPermission::ViewScreen]));
        assert_eq!(request.state, PairingState::Approved);
    }

    #[test]
    fn rejected_requests_grant_nothing() {
        let mut request = PairingRequest::create(
            id("android-client"),
            id("windows-host"),
            [SessionPermission::ViewScreen],
        )
        .unwrap();

        assert!(request.decide([], false).unwrap().is_empty());
        assert_eq!(request.state, PairingState::Rejected);
    }

    #[test]
    fn self_pairing_is_rejected() {
        let result =
            PairingRequest::create(id("same-device"), id("same-device"), [SessionPermission::ViewScreen]);
        assert_eq!(result, Err(PairingError::SelfPairing));
    }

    #[test]
    fn pairing_with_no_permissions_is_rejected() {
        let result =
            PairingRequest::create(id("android-client"), id("windows-host"), []);
        assert_eq!(result, Err(PairingError::NoPermissionsRequested));
    }

    #[test]
    fn double_decide_is_rejected() {
        let mut request = PairingRequest::create(
            id("android-client"),
            id("windows-host"),
            [SessionPermission::ViewScreen],
        )
        .unwrap();
        request.decide([], false).unwrap();
        assert_eq!(
            request.decide([], true),
            Err(PairingError::NotAwaitingApproval)
        );
    }
}
