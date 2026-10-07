//! Clipboard synchronization policy engine.
//!
//! # Design constraints
//!
//! - Clipboard sync is **opt-in only** and requires the host to grant
//!   `SessionPermission::Clipboard` explicitly.
//! - Clipboard text is **never echoed in logs** (see `Debug` impl).
//! - The payload size is hard-capped at [`MAX_CLIPBOARD_BYTES`] to prevent
//!   out-of-memory attacks or accidental large-payload forwarding.
//! - Only plain UTF-8 text is accepted in this initial implementation.
//! - Direction flags (`host_to_client`, `client_to_host`) are controlled
//!   by the host — the client cannot self-elevate its clipboard direction.

use crate::SessionPermission;
use std::collections::BTreeSet;

/// Maximum allowed clipboard payload in bytes (64 KiB).
pub const MAX_CLIPBOARD_BYTES: usize = 65_536;

/// Direction of clipboard sync permitted for a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardDirection {
    /// Host → Client only (host clipboard pushed to Android).
    HostToClient,
    /// Client → Host only (Android clipboard pushed to Windows).
    ClientToHost,
    /// Bidirectional sync (both directions).
    Bidirectional,
}

/// Grant state for the clipboard capability within a session.
///
/// The host controls all fields; the client observes the negotiated state.
#[derive(Clone, PartialEq, Eq)]
pub struct ClipboardGrant {
    /// The session ID this grant is bound to.
    pub session_id: String,
    /// Whether clipboard sync is currently active.
    pub active: bool,
    /// Which direction the host has permitted.
    pub direction: ClipboardDirection,
    /// Number of updates the host has pushed to the client.
    pub host_push_count: u64,
    /// Number of updates the host has received from the client.
    pub client_push_count: u64,
}

/// Intentionally suppresses clipboard text from the `Debug` output.
impl std::fmt::Debug for ClipboardGrant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClipboardGrant")
            .field("session_id", &self.session_id)
            .field("active", &self.active)
            .field("direction", &self.direction)
            .field("host_push_count", &self.host_push_count)
            .field("client_push_count", &self.client_push_count)
            .finish()
    }
}

impl ClipboardGrant {
    /// Creates a new active clipboard grant for a session.
    pub fn new(session_id: String, direction: ClipboardDirection) -> Self {
        Self {
            session_id,
            active: true,
            direction,
            host_push_count: 0,
            client_push_count: 0,
        }
    }

    /// Suspends clipboard sync without revoking the grant.
    pub fn suspend(&mut self) {
        self.active = false;
    }

    /// Resumes a suspended clipboard sync.
    pub fn resume(&mut self) {
        self.active = true;
    }

    /// Revokes the clipboard grant entirely (terminal).
    pub fn revoke(mut self) -> RevokedClipboardGrant {
        self.active = false;
        RevokedClipboardGrant {
            session_id: self.session_id,
        }
    }

    /// Returns `true` if the host is allowed to push clipboard to the client.
    pub fn can_host_push(&self) -> bool {
        self.active
            && matches!(
                self.direction,
                ClipboardDirection::HostToClient | ClipboardDirection::Bidirectional
            )
    }

    /// Returns `true` if the client is allowed to push clipboard to the host.
    pub fn can_client_push(&self) -> bool {
        self.active
            && matches!(
                self.direction,
                ClipboardDirection::ClientToHost | ClipboardDirection::Bidirectional
            )
    }
}

/// Marker for a clipboard grant that has been permanently revoked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevokedClipboardGrant {
    pub session_id: String,
}

/// Errors that can occur during clipboard operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardError {
    /// The session does not have `SessionPermission::Clipboard`.
    PermissionNotGranted,
    /// Clipboard sync is suspended or revoked for this session.
    NotActive,
    /// The text payload exceeds [`MAX_CLIPBOARD_BYTES`].
    PayloadTooLarge { actual_bytes: usize },
    /// The text contains invalid UTF-8 or disallowed content.
    InvalidText,
    /// The clipboard direction does not allow this push.
    DirectionNotPermitted,
}

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PermissionNotGranted => write!(f, "session lacks Clipboard permission"),
            Self::NotActive => write!(f, "clipboard sync is not active for this session"),
            Self::PayloadTooLarge { actual_bytes } => {
                write!(
                    f,
                    "clipboard payload is {} bytes, exceeding the {} byte limit",
                    actual_bytes, MAX_CLIPBOARD_BYTES
                )
            }
            Self::InvalidText => write!(f, "clipboard text is invalid"),
            Self::DirectionNotPermitted => {
                write!(f, "clipboard direction not permitted by host grant")
            }
        }
    }
}

/// Validates that a clipboard update payload is safe to propagate.
///
/// # Arguments
///
/// - `text` — The candidate clipboard text.
/// - `grant` — The active clipboard grant for this session.
/// - `granted_permissions` — The full permission set for the session.
/// - `is_host_push` — `true` if the host is pushing to client; `false` if client pushing to host.
///
/// Returns the validated text (unchanged) or a [`ClipboardError`].
pub fn validate_clipboard_update<'a>(
    text: &'a str,
    grant: &ClipboardGrant,
    granted_permissions: &BTreeSet<SessionPermission>,
    is_host_push: bool,
) -> Result<&'a str, ClipboardError> {
    if !granted_permissions.contains(&SessionPermission::Clipboard) {
        return Err(ClipboardError::PermissionNotGranted);
    }
    if !grant.active {
        return Err(ClipboardError::NotActive);
    }
    let byte_len = text.len();
    if byte_len > MAX_CLIPBOARD_BYTES {
        return Err(ClipboardError::PayloadTooLarge {
            actual_bytes: byte_len,
        });
    }
    // Reject text containing embedded null bytes (potential injection vector).
    if text.contains('\0') {
        return Err(ClipboardError::InvalidText);
    }
    if is_host_push && !grant.can_host_push() {
        return Err(ClipboardError::DirectionNotPermitted);
    }
    if !is_host_push && !grant.can_client_push() {
        return Err(ClipboardError::DirectionNotPermitted);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn perms_with_clipboard() -> BTreeSet<SessionPermission> {
        let mut s = BTreeSet::new();
        s.insert(SessionPermission::Clipboard);
        s
    }

    fn empty_perms() -> BTreeSet<SessionPermission> {
        BTreeSet::new()
    }

    #[test]
    fn host_push_allowed_when_bidirectional() {
        let grant = ClipboardGrant::new("s1".to_owned(), ClipboardDirection::Bidirectional);
        assert!(grant.can_host_push());
        assert!(grant.can_client_push());
    }

    #[test]
    fn host_push_allowed_when_host_to_client() {
        let grant = ClipboardGrant::new("s1".to_owned(), ClipboardDirection::HostToClient);
        assert!(grant.can_host_push());
        assert!(!grant.can_client_push());
    }

    #[test]
    fn client_push_allowed_when_client_to_host() {
        let grant = ClipboardGrant::new("s1".to_owned(), ClipboardDirection::ClientToHost);
        assert!(!grant.can_host_push());
        assert!(grant.can_client_push());
    }

    #[test]
    fn suspended_grant_blocks_both_directions() {
        let mut grant = ClipboardGrant::new("s1".to_owned(), ClipboardDirection::Bidirectional);
        grant.suspend();
        assert!(!grant.can_host_push());
        assert!(!grant.can_client_push());
    }

    #[test]
    fn resumed_grant_re_enables_push() {
        let mut grant = ClipboardGrant::new("s1".to_owned(), ClipboardDirection::Bidirectional);
        grant.suspend();
        grant.resume();
        assert!(grant.can_host_push());
    }

    #[test]
    fn validate_rejects_missing_permission() {
        let grant = ClipboardGrant::new("s1".to_owned(), ClipboardDirection::Bidirectional);
        let result = validate_clipboard_update("hello", &grant, &empty_perms(), true);
        assert_eq!(result, Err(ClipboardError::PermissionNotGranted));
    }

    #[test]
    fn validate_rejects_oversized_payload() {
        let grant = ClipboardGrant::new("s1".to_owned(), ClipboardDirection::Bidirectional);
        let big = "x".repeat(MAX_CLIPBOARD_BYTES + 1);
        let result = validate_clipboard_update(&big, &grant, &perms_with_clipboard(), true);
        assert_eq!(
            result,
            Err(ClipboardError::PayloadTooLarge {
                actual_bytes: MAX_CLIPBOARD_BYTES + 1
            })
        );
    }

    #[test]
    fn validate_rejects_null_bytes() {
        let grant = ClipboardGrant::new("s1".to_owned(), ClipboardDirection::Bidirectional);
        let result =
            validate_clipboard_update("hello\0world", &grant, &perms_with_clipboard(), true);
        assert_eq!(result, Err(ClipboardError::InvalidText));
    }

    #[test]
    fn validate_rejects_wrong_direction_host_only() {
        let grant = ClipboardGrant::new("s1".to_owned(), ClipboardDirection::HostToClient);
        let result = validate_clipboard_update("hi", &grant, &perms_with_clipboard(), false);
        assert_eq!(result, Err(ClipboardError::DirectionNotPermitted));
    }

    #[test]
    fn validate_accepts_valid_host_push() {
        let grant = ClipboardGrant::new("s1".to_owned(), ClipboardDirection::HostToClient);
        let result = validate_clipboard_update("hello", &grant, &perms_with_clipboard(), true);
        assert_eq!(result, Ok("hello"));
    }

    #[test]
    fn validate_accepts_valid_client_push() {
        let grant = ClipboardGrant::new("s1".to_owned(), ClipboardDirection::Bidirectional);
        let result = validate_clipboard_update("world", &grant, &perms_with_clipboard(), false);
        assert_eq!(result, Ok("world"));
    }

    #[test]
    fn validate_rejects_inactive_grant() {
        let mut grant = ClipboardGrant::new("s1".to_owned(), ClipboardDirection::Bidirectional);
        grant.suspend();
        let result = validate_clipboard_update("text", &grant, &perms_with_clipboard(), true);
        assert_eq!(result, Err(ClipboardError::NotActive));
    }

    #[test]
    fn revoke_produces_revoked_marker() {
        let grant = ClipboardGrant::new("sess-abc".to_owned(), ClipboardDirection::Bidirectional);
        let revoked = grant.revoke();
        assert_eq!(revoked.session_id, "sess-abc");
    }

    #[test]
    fn max_size_payload_is_accepted() {
        let grant = ClipboardGrant::new("s1".to_owned(), ClipboardDirection::Bidirectional);
        let exact = "a".repeat(MAX_CLIPBOARD_BYTES);
        let result = validate_clipboard_update(&exact, &grant, &perms_with_clipboard(), true);
        assert!(result.is_ok());
    }
}
