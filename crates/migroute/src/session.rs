//! Session state machine and authorization tracking for SMP/1.
//!
//! A [`Session`] represents an active, authenticated association between a
//! host [`DeviceId`] and a client [`DeviceId`].
//!
//! # Invariants
//!
//! 1. A session is only active if its state is [`SessionState::Active`] AND
//!    the current timestamp has not exceeded `expires_at_unix_ms`.
//! 2. Capabilities must be explicitly authorized in `granted_permissions`.
//! 3. Host can revoke individual permissions or terminate the entire session
//!    at any time.
//! 4. Once terminated or revoked, a session can never return to an active state.

use crate::envelope::{Envelope, EnvelopeError, SequenceTracker};
use crate::message::{SessionEndReason, SmpMessage};
use crate::{DeviceId, SessionPermission};
use std::collections::BTreeSet;
use std::fmt;

/// State of an established or terminated session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Active and authorized for permitted operations.
    Active,
    /// Terminated gracefully or due to timeout/error.
    Ended(SessionEndReason),
    /// Forcibly revoked by host policy or trust revocation.
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    /// The session is not in the [`SessionState::Active`] state.
    SessionNotActive,
    /// The session's time window has expired.
    SessionExpired { expired_at_unix_ms: i64 },
    /// The requested operation requires a permission not granted to this session.
    PermissionDenied(SessionPermission),
    /// Host or client identity did not match the session.
    DeviceMismatch,
    /// Envelope validation failed against this session.
    EnvelopeValidation(EnvelopeError),
    /// Invalid creation parameters.
    InvalidParameters(&'static str),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SessionNotActive => f.write_str("session is not active"),
            Self::SessionExpired { expired_at_unix_ms } => {
                write!(f, "session expired at unix_ms {expired_at_unix_ms}")
            }
            Self::PermissionDenied(perm) => {
                write!(f, "permission denied: required capability {perm} is not granted")
            }
            Self::DeviceMismatch => f.write_str("device identity does not match session"),
            Self::EnvelopeValidation(err) => write!(f, "envelope validation failed: {err}"),
            Self::InvalidParameters(msg) => write!(f, "invalid session parameters: {msg}"),
        }
    }
}

impl std::error::Error for SessionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::EnvelopeValidation(err) => Some(err),
            _ => None,
        }
    }
}

/// An established Smart Migrate session.
#[derive(Debug)]
pub struct Session {
    pub id: String,
    pub host: DeviceId,
    pub client: DeviceId,
    pub granted_permissions: BTreeSet<SessionPermission>,
    pub state: SessionState,
    pub created_at_unix_ms: i64,
    pub expires_at_unix_ms: i64,
    pub sequence_tracker: SequenceTracker,
}

impl Session {
    /// Creates a new active session.
    ///
    /// # Errors
    ///
    /// Fails with [`SessionError::InvalidParameters`] if:
    /// - `session_id` is empty
    /// - `host` and `client` are the same device
    /// - `expires_at_unix_ms <= created_at_unix_ms`
    pub fn new(
        id: impl Into<String>,
        host: DeviceId,
        client: DeviceId,
        granted_permissions: impl IntoIterator<Item = SessionPermission>,
        created_at_unix_ms: i64,
        expires_at_unix_ms: i64,
    ) -> Result<Self, SessionError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(SessionError::InvalidParameters("session ID cannot be empty"));
        }
        if host == client {
            return Err(SessionError::InvalidParameters("host and client cannot be the same device"));
        }
        if expires_at_unix_ms <= created_at_unix_ms {
            return Err(SessionError::InvalidParameters(
                "expiry must be strictly greater than creation timestamp",
            ));
        }

        Ok(Self {
            id,
            host,
            client,
            granted_permissions: granted_permissions.into_iter().collect(),
            state: SessionState::Active,
            created_at_unix_ms,
            expires_at_unix_ms,
            sequence_tracker: SequenceTracker::new(),
        })
    }

    /// Checks if the session is currently active at the given timestamp.
    pub fn is_active(&self, now_unix_ms: i64) -> bool {
        self.state == SessionState::Active && now_unix_ms < self.expires_at_unix_ms
    }

    /// Checks whether the session holds a specific permission.
    pub fn has_permission(&self, permission: SessionPermission) -> bool {
        self.granted_permissions.contains(&permission)
    }

    /// Authorizes a received [`SmpMessage`] within this session.
    ///
    /// Validates:
    /// 1. Session is active and unexpired.
    /// 2. If the message requires a session permission, ensures it is granted.
    pub fn authorize_message(&self, message: &SmpMessage, now_unix_ms: i64) -> Result<(), SessionError> {
        if !self.is_active(now_unix_ms) {
            if self.state != SessionState::Active {
                return Err(SessionError::SessionNotActive);
            }
            return Err(SessionError::SessionExpired {
                expired_at_unix_ms: self.expires_at_unix_ms,
            });
        }

        if let Some(required) = message.required_permission() {
            if !self.has_permission(required) {
                return Err(SessionError::PermissionDenied(required));
            }
        }

        Ok(())
    }

    /// Validates an incoming wire [`Envelope`] destined for this session.
    ///
    /// Validates envelope version, expiration, session matching, and sequence monotonically.
    pub fn validate_envelope(&mut self, envelope: &Envelope, now_unix_ms: i64) -> Result<(), SessionError> {
        if !self.is_active(now_unix_ms) {
            if self.state != SessionState::Active {
                return Err(SessionError::SessionNotActive);
            }
            return Err(SessionError::SessionExpired {
                expired_at_unix_ms: self.expires_at_unix_ms,
            });
        }

        crate::envelope::validate(envelope, now_unix_ms, Some(&self.id), &mut self.sequence_tracker)
            .map_err(SessionError::EnvelopeValidation)
    }

    /// Revokes an individual permission dynamically during an ongoing session.
    pub fn revoke_permission(&mut self, permission: SessionPermission) {
        self.granted_permissions.remove(&permission);
    }

    /// Ends the session with a specific reason.
    pub fn end(&mut self, reason: SessionEndReason) {
        self.state = SessionState::Ended(reason);
        self.sequence_tracker.remove(&self.id);
    }

    /// Revokes the session immediately (host security revocation).
    pub fn revoke(&mut self) {
        self.state = SessionState::Revoked;
        self.sequence_tracker.remove(&self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{InputMouse, MouseButtons};

    fn test_ids() -> (DeviceId, DeviceId) {
        (
            DeviceId::try_from("host-pc").unwrap(),
            DeviceId::try_from("client-android").unwrap(),
        )
    }

    #[test]
    fn session_creation_valid() {
        let (host, client) = test_ids();
        let session = Session::new(
            "sess-123",
            host,
            client,
            [SessionPermission::ViewScreen],
            1000,
            5000,
        )
        .unwrap();

        assert_eq!(session.id, "sess-123");
        assert!(session.is_active(2000));
        assert!(session.has_permission(SessionPermission::ViewScreen));
        assert!(!session.has_permission(SessionPermission::ControlMouse));
    }

    #[test]
    fn session_creation_invalid_params() {
        let (host, client) = test_ids();
        // empty id
        assert!(Session::new("", host.clone(), client.clone(), [], 1000, 5000).is_err());
        // same host and client
        assert!(Session::new("sess-1", host.clone(), host.clone(), [], 1000, 5000).is_err());
        // invalid time window
        assert!(Session::new("sess-1", host, client, [], 5000, 1000).is_err());
    }

    #[test]
    fn session_expires_at_timestamp() {
        let (host, client) = test_ids();
        let session = Session::new("sess-1", host, client, [], 1000, 2000).unwrap();
        assert!(session.is_active(1500));
        assert!(!session.is_active(2000));
        assert!(!session.is_active(2500));
    }

    #[test]
    fn session_permission_authorization() {
        let (host, client) = test_ids();
        let session = Session::new(
            "sess-1",
            host,
            client,
            [SessionPermission::ViewScreen],
            1000,
            5000,
        )
        .unwrap();

        let mouse_msg = SmpMessage::InputMouse(InputMouse {
            session_id: "sess-1".to_string(),
            x: 100,
            y: 100,
            buttons: MouseButtons::default(),
            scroll_delta: 0,
        });

        // Mouse control was not granted
        assert_eq!(
            session.authorize_message(&mouse_msg, 2000),
            Err(SessionError::PermissionDenied(SessionPermission::ControlMouse))
        );
    }

    #[test]
    fn session_revoke_and_end() {
        let (host, client) = test_ids();
        let mut session = Session::new("sess-1", host, client, [], 1000, 5000).unwrap();
        assert!(session.is_active(2000));

        session.revoke();
        assert_eq!(session.state, SessionState::Revoked);
        assert!(!session.is_active(2000));

        let mut session2 = Session::new("sess-2", test_ids().0, test_ids().1, [], 1000, 5000).unwrap();
        session2.end(SessionEndReason::UserRequested);
        assert_eq!(session2.state, SessionState::Ended(SessionEndReason::UserRequested));
        assert!(!session2.is_active(2000));
    }

    #[test]
    fn dynamic_permission_revocation() {
        let (host, client) = test_ids();
        let mut session = Session::new(
            "sess-1",
            host,
            client,
            [SessionPermission::ViewScreen, SessionPermission::ControlMouse],
            1000,
            5000,
        )
        .unwrap();

        assert!(session.has_permission(SessionPermission::ControlMouse));
        session.revoke_permission(SessionPermission::ControlMouse);
        assert!(!session.has_permission(SessionPermission::ControlMouse));
        assert!(session.has_permission(SessionPermission::ViewScreen));
    }
}
