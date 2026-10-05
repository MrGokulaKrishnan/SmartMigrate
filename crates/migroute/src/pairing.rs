//! Short-lived QR and numeric pairing state machine for Smart Migrate.
//!
//! # Security guarantees
//!
//! - **Single-use 6-digit numeric codes**: constant-time comparison to defeat timing attacks.
//! - **Cryptographic Nonce Token**: unguessable token embedded in QR URI for zero-trust pairing.
//! - **Short TTL**: expires automatically after e.g. 180 seconds.
//! - **Strict attempt limits**: lock out after N (default 3) incorrect attempts to prevent online brute force.
//! - **Explicit host-side authorization**: the host chooses and narrows granted permissions.

use crate::{DeviceId, PairingError, PairingState, SessionPermission};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

/// Formatted 6-digit numeric pairing code (e.g. "492-183" or "492183").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NumericPairingCode(String);

impl NumericPairingCode {
    /// Formats a 6-digit integer in range [0, 999_999] with leading zeros.
    pub fn from_u32(val: u32) -> Self {
        let normalized = val % 1_000_000;
        Self(format!("{:06}", normalized))
    }

    /// Creates code from a string slice after validating it is exactly 6 ASCII digits.
    pub fn try_from_digits(s: &str) -> Result<Self, PairingVerificationError> {
        let cleaned: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        if cleaned.len() != 6 {
            return Err(PairingVerificationError::InvalidCodeFormat);
        }
        Ok(Self(cleaned))
    }

    /// Formatted presentation string with hyphen separator, e.g. "492-183".
    pub fn formatted(&self) -> String {
        format!("{}-{}", &self.0[..3], &self.0[3..])
    }

    /// Raw 6-digit string without separator, e.g. "492183".
    pub fn raw(&self) -> &str {
        &self.0
    }

    /// Constant-time comparison between input and internal code to eliminate timing side channels.
    pub fn constant_time_eq(&self, candidate: &str) -> bool {
        let cleaned: String = candidate.chars().filter(|c| c.is_ascii_digit()).collect();
        let a = self.0.as_bytes();
        let b = cleaned.as_bytes();

        if a.len() != b.len() {
            return false;
        }

        let mut diff = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            diff |= x ^ y;
        }
        diff == 0
    }
}

/// Errors during pairing code or token verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PairingVerificationError {
    Expired,
    TooManyAttempts,
    InvalidCodeFormat,
    CodeMismatch,
    TokenMismatch,
    AlreadyDecided,
}

impl fmt::Display for PairingVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expired => f.write_str("pairing session has expired"),
            Self::TooManyAttempts => f.write_str("maximum pairing verification attempts exceeded"),
            Self::InvalidCodeFormat => f.write_str("pairing code must be exactly 6 numeric digits"),
            Self::CodeMismatch => f.write_str("incorrect pairing code"),
            Self::TokenMismatch => f.write_str("pairing token mismatch"),
            Self::AlreadyDecided => f.write_str("pairing session has already been decided"),
        }
    }
}

impl std::error::Error for PairingVerificationError {}

/// Host-initiated pairing session state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingSession {
    pub session_id: String,
    pub host_id: DeviceId,
    pub host_name: String,
    pub code: NumericPairingCode,
    pub secret_token: String,
    pub created_at_epoch_ms: u64,
    pub expires_at_epoch_ms: u64,
    pub max_attempts: u32,
    pub failed_attempts: u32,
    pub state: PairingState,
    pub pending_requester_id: Option<DeviceId>,
    pub pending_requester_name: Option<String>,
    pub requested_permissions: BTreeSet<SessionPermission>,
}

impl PairingSession {
    /// Default TTL for a pairing session is 3 minutes (180,000 ms).
    pub const DEFAULT_TTL_MS: u64 = 180_000;
    /// Default maximum incorrect code attempts before automatic lockout.
    pub const DEFAULT_MAX_ATTEMPTS: u32 = 3;

    pub fn new(
        session_id: impl Into<String>,
        host_id: DeviceId,
        host_name: impl Into<String>,
        code_int: u32,
        secret_token: impl Into<String>,
        now_epoch_ms: u64,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            host_id,
            host_name: host_name.into(),
            code: NumericPairingCode::from_u32(code_int),
            secret_token: secret_token.into(),
            created_at_epoch_ms: now_epoch_ms,
            expires_at_epoch_ms: now_epoch_ms + Self::DEFAULT_TTL_MS,
            max_attempts: Self::DEFAULT_MAX_ATTEMPTS,
            failed_attempts: 0,
            state: PairingState::Requested,
            pending_requester_id: None,
            pending_requester_name: None,
            requested_permissions: BTreeSet::new(),
        }
    }

    /// Generates canonical URI for encoding in a QR code.
    ///
    /// Scheme: `smp://pair?v=1&host=<id>&name=<encoded>&code=<code>&token=<token>&exp=<timestamp>`
    pub fn qr_payload(&self) -> String {
        format!(
            "smp://pair?v=1&host={}&name={}&code={}&token={}&exp={}",
            self.host_id,
            urlencoding_simple(&self.host_name),
            self.code.raw(),
            self.secret_token,
            self.expires_at_epoch_ms
        )
    }

    /// Whether this session has expired based on current wall-clock timestamp.
    pub fn is_expired(&self, now_epoch_ms: u64) -> bool {
        now_epoch_ms >= self.expires_at_epoch_ms
    }

    /// Client submits a pairing code and optional token to pair with this host.
    pub fn submit_credentials(
        &mut self,
        requester_id: DeviceId,
        requester_name: impl Into<String>,
        code: &str,
        token: Option<&str>,
        requested_permissions: impl IntoIterator<Item = SessionPermission>,
        now_epoch_ms: u64,
    ) -> Result<(), PairingVerificationError> {
        if self.state != PairingState::Requested {
            return Err(PairingVerificationError::AlreadyDecided);
        }

        if self.is_expired(now_epoch_ms) {
            self.state = PairingState::Expired;
            return Err(PairingVerificationError::Expired);
        }

        if self.failed_attempts >= self.max_attempts {
            self.state = PairingState::Rejected;
            return Err(PairingVerificationError::TooManyAttempts);
        }

        // Verify secret token if provided (e.g. from QR scan)
        if let Some(t) = token {
            if t != self.secret_token {
                self.failed_attempts += 1;
                return Err(PairingVerificationError::TokenMismatch);
            }
        }

        // Verify numeric code in constant time
        if !self.code.constant_time_eq(code) {
            self.failed_attempts += 1;
            if self.failed_attempts >= self.max_attempts {
                self.state = PairingState::Rejected;
                return Err(PairingVerificationError::TooManyAttempts);
            }
            return Err(PairingVerificationError::CodeMismatch);
        }

        // Client credentials verified; transition to AwaitingHostApproval
        self.pending_requester_id = Some(requester_id);
        self.pending_requester_name = Some(requester_name.into());
        self.requested_permissions = requested_permissions.into_iter().collect();
        self.state = PairingState::AwaitingHostApproval;
        Ok(())
    }

    /// Host grants or narrows requested permissions.
    pub fn host_approve(
        &mut self,
        granted_permissions: impl IntoIterator<Item = SessionPermission>,
    ) -> Result<BTreeSet<SessionPermission>, PairingError> {
        if self.state != PairingState::AwaitingHostApproval {
            return Err(PairingError::NotAwaitingApproval);
        }

        let granted = granted_permissions
            .into_iter()
            .filter(|p| self.requested_permissions.contains(p))
            .collect::<BTreeSet<_>>();

        self.state = PairingState::Approved;
        Ok(granted)
    }

    /// Host rejects pairing request.
    pub fn host_reject(&mut self) -> Result<(), PairingError> {
        if self.state != PairingState::AwaitingHostApproval && self.state != PairingState::Requested {
            return Err(PairingError::NotAwaitingApproval);
        }

        self.state = PairingState::Rejected;
        Ok(())
    }
}

fn urlencoding_simple(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host_id() -> DeviceId {
        DeviceId::try_from("host-workstation-01").unwrap()
    }

    fn client_id() -> DeviceId {
        DeviceId::try_from("pixel-tablet-01").unwrap()
    }

    #[test]
    fn numeric_code_formatting() {
        let code = NumericPairingCode::from_u32(492183);
        assert_eq!(code.raw(), "492183");
        assert_eq!(code.formatted(), "492-183");

        let leading_zero = NumericPairingCode::from_u32(1234);
        assert_eq!(leading_zero.raw(), "001234");
        assert_eq!(leading_zero.formatted(), "001-234");
    }

    #[test]
    fn constant_time_code_verification() {
        let code = NumericPairingCode::from_u32(765432);
        assert!(code.constant_time_eq("765432"));
        assert!(code.constant_time_eq("765-432"));
        assert!(!code.constant_time_eq("765431"));
        assert!(!code.constant_time_eq("000000"));
    }

    #[test]
    fn full_successful_pairing_flow() {
        let now = 1_000_000;
        let mut session = PairingSession::new(
            "pair-sess-001",
            host_id(),
            "Gokul PC",
            582910,
            "secret-token-nonce-123",
            now,
        );

        assert_eq!(session.state, PairingState::Requested);
        assert!(session.qr_payload().contains("code=582910"));
        assert!(session.qr_payload().contains("host=host-workstation-01"));

        // Client submits matching credentials
        let res = session.submit_credentials(
            client_id(),
            "Pixel Tablet",
            "582-910",
            Some("secret-token-nonce-123"),
            [SessionPermission::ViewScreen, SessionPermission::SendFiles],
            now + 5000,
        );
        assert!(res.is_ok());
        assert_eq!(session.state, PairingState::AwaitingHostApproval);

        // Host narrows permissions (only ViewScreen granted, SendFiles denied)
        let granted = session.host_approve([SessionPermission::ViewScreen]).unwrap();
        assert_eq!(granted, BTreeSet::from([SessionPermission::ViewScreen]));
        assert_eq!(session.state, PairingState::Approved);
    }

    #[test]
    fn pairing_fails_on_expiry() {
        let now = 1_000_000;
        let mut session = PairingSession::new(
            "pair-sess-002",
            host_id(),
            "Gokul PC",
            123456,
            "tok",
            now,
        );

        let res = session.submit_credentials(
            client_id(),
            "Android Phone",
            "123456",
            Some("tok"),
            [SessionPermission::ViewScreen],
            now + PairingSession::DEFAULT_TTL_MS + 1,
        );

        assert_eq!(res, Err(PairingVerificationError::Expired));
        assert_eq!(session.state, PairingState::Expired);
    }

    #[test]
    fn brute_force_rate_limit_lockout() {
        let now = 1_000_000;
        let mut session = PairingSession::new(
            "pair-sess-003",
            host_id(),
            "Gokul PC",
            999888,
            "tok",
            now,
        );

        // 1st wrong attempt
        assert_eq!(
            session.submit_credentials(client_id(), "Hacker", "000000", None, [SessionPermission::ViewScreen], now),
            Err(PairingVerificationError::CodeMismatch)
        );
        // 2nd wrong attempt
        assert_eq!(
            session.submit_credentials(client_id(), "Hacker", "111111", None, [SessionPermission::ViewScreen], now),
            Err(PairingVerificationError::CodeMismatch)
        );
        // 3rd wrong attempt => lockout
        assert_eq!(
            session.submit_credentials(client_id(), "Hacker", "222222", None, [SessionPermission::ViewScreen], now),
            Err(PairingVerificationError::TooManyAttempts)
        );
        assert_eq!(session.state, PairingState::Rejected);

        // Even if correct code entered now, it is locked out
        assert_eq!(
            session.submit_credentials(client_id(), "Hacker", "999888", None, [SessionPermission::ViewScreen], now),
            Err(PairingVerificationError::AlreadyDecided)
        );
    }
}
