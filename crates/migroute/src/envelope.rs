//! SMP/1 envelope validation.
//!
//! Every message in the Smart Migrate Protocol is wrapped in an [`Envelope`].
//! The envelope provides session binding, replay protection, expiry, and
//! protocol version checks.  No network I/O occurs here — the adapter layer
//! handles transport; MigRoute validates the policy.
//!
//! # Validation sequence
//!
//! 1. Reject unknown protocol versions.
//! 2. Reject expired envelopes (`expires_at_unix_ms < now`).
//! 3. Reject envelopes with sequence numbers ≤ the last accepted sequence for
//!    the same session (replay protection).
//! 4. Reject mismatched `session_id` when a session binding is expected.
//! 5. Accept and return the typed payload for further processing.

use std::collections::HashMap;

/// The maximum number of bytes allowed in a single envelope payload.
/// This prevents memory exhaustion from oversized messages.
pub const MAX_PAYLOAD_BYTES: usize = 16 * 1024 * 1024; // 16 MiB

/// A raw SMP/1 envelope, mirroring the protobuf schema.
///
/// The payload is opaque bytes here.  Callers decode the payload into a
/// typed [`crate::message::SmpMessage`] *after* the envelope passes validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    /// Must equal [`crate::PROTOCOL_VERSION`].
    pub protocol_version: u16,
    /// Identifies the message type for payload decoding.
    pub message_type: String,
    /// Session context.  Empty string means "no session" (e.g., DEVICE_HELLO).
    pub session_id: String,
    /// Optional correlation ID for request/response pairs.
    pub request_id: String,
    /// Monotonically increasing within a session.  Zero when ordering does not
    /// apply (e.g., PING/PONG outside a session).
    pub sequence: u64,
    /// Unix timestamp in milliseconds at issuance.
    pub issued_at_unix_ms: i64,
    /// Unix timestamp in milliseconds at expiry.  Must be > issued_at_unix_ms.
    pub expires_at_unix_ms: i64,
    /// Serialized payload.  The concrete type is determined by `message_type`.
    pub payload: Vec<u8>,
}

/// Reasons an envelope can be rejected before its payload is processed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeError {
    /// The protocol_version field does not match the supported version.
    UnsupportedProtocolVersion { received: u16 },
    /// The envelope has already expired relative to the provided clock value.
    Expired { expired_at_unix_ms: i64 },
    /// The sequence number is ≤ the last accepted sequence for this session
    /// (replay or reordering attack).
    ReplayDetected { received: u64, last_accepted: u64 },
    /// The session_id in the envelope does not match the expected session.
    SessionMismatch,
    /// The payload exceeds [`MAX_PAYLOAD_BYTES`].
    PayloadTooLarge { bytes: usize },
    /// `expires_at_unix_ms` ≤ `issued_at_unix_ms` — the window is invalid.
    InvalidExpiry,
}

impl std::fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedProtocolVersion { received } => {
                write!(f, "unsupported protocol version {received}; expected {}", crate::PROTOCOL_VERSION)
            }
            Self::Expired { expired_at_unix_ms } => {
                write!(f, "envelope expired at unix_ms {expired_at_unix_ms}")
            }
            Self::ReplayDetected { received, last_accepted } => {
                write!(
                    f,
                    "replay detected: received sequence {received} ≤ last accepted {last_accepted}"
                )
            }
            Self::SessionMismatch => f.write_str("envelope session_id does not match the expected session"),
            Self::PayloadTooLarge { bytes } => {
                write!(f, "payload of {bytes} bytes exceeds the {MAX_PAYLOAD_BYTES}-byte limit")
            }
            Self::InvalidExpiry => {
                f.write_str("expires_at_unix_ms must be strictly greater than issued_at_unix_ms")
            }
        }
    }
}

impl std::error::Error for EnvelopeError {}

/// Sequence tracker for a single session.
///
/// Maintains the highest accepted sequence number so that replayed or
/// reordered envelopes can be rejected.
#[derive(Debug, Default)]
pub struct SequenceTracker {
    /// Maps session_id → last accepted sequence number.
    last: HashMap<String, u64>,
}

impl SequenceTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the last accepted sequence for `session_id`, or `None` if the
    /// session has not been seen yet.
    pub fn last_sequence(&self, session_id: &str) -> Option<u64> {
        self.last.get(session_id).copied()
    }

    /// Records `sequence` as accepted for `session_id`.
    ///
    /// Callers must only call this *after* [`validate`] succeeds.
    pub fn accept(&mut self, session_id: &str, sequence: u64) {
        self.last.insert(session_id.to_owned(), sequence);
    }

    /// Removes tracking state for a session (call on session end or revocation).
    pub fn remove(&mut self, session_id: &str) {
        self.last.remove(session_id);
    }
}

/// Validates an [`Envelope`] against policy rules.
///
/// # Parameters
///
/// - `envelope` — the received envelope.
/// - `now_unix_ms` — current wall clock time in milliseconds (Unix epoch).
///   Obtained by the adapter layer; MigRoute does not read the clock itself.
/// - `expected_session_id` — `Some(id)` to enforce session binding; `None` to
///   allow any session ID (use for unauthenticated messages like DEVICE_HELLO).
/// - `tracker` — mutable reference to the session-level sequence tracker.
///   The tracker is updated *only* on success.
///
/// # Returns
///
/// `Ok(())` if all checks pass.  `Err(EnvelopeError)` with the first failing
/// check otherwise.
pub fn validate(
    envelope: &Envelope,
    now_unix_ms: i64,
    expected_session_id: Option<&str>,
    tracker: &mut SequenceTracker,
) -> Result<(), EnvelopeError> {
    // 1. Protocol version.
    if envelope.protocol_version != crate::PROTOCOL_VERSION {
        return Err(EnvelopeError::UnsupportedProtocolVersion {
            received: envelope.protocol_version,
        });
    }

    // 2. Expiry window sanity.
    if envelope.expires_at_unix_ms <= envelope.issued_at_unix_ms {
        return Err(EnvelopeError::InvalidExpiry);
    }

    // 3. Expiry check.
    if envelope.expires_at_unix_ms < now_unix_ms {
        return Err(EnvelopeError::Expired {
            expired_at_unix_ms: envelope.expires_at_unix_ms,
        });
    }

    // 4. Payload size.
    if envelope.payload.len() > MAX_PAYLOAD_BYTES {
        return Err(EnvelopeError::PayloadTooLarge {
            bytes: envelope.payload.len(),
        });
    }

    // 5. Session binding (when a session is expected).
    if let Some(expected) = expected_session_id {
        if envelope.session_id != expected {
            return Err(EnvelopeError::SessionMismatch);
        }
    }

    // 6. Replay / reorder check (only when sequence tracking is active for this session).
    if envelope.sequence > 0 {
        if let Some(last) = tracker.last_sequence(&envelope.session_id) {
            if envelope.sequence <= last {
                return Err(EnvelopeError::ReplayDetected {
                    received: envelope.sequence,
                    last_accepted: last,
                });
            }
        }
        // Accept this sequence.
        tracker.accept(&envelope.session_id, envelope.sequence);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_envelope() -> Envelope {
        Envelope {
            protocol_version: crate::PROTOCOL_VERSION,
            message_type: "PING".to_owned(),
            session_id: "session-abc".to_owned(),
            request_id: String::new(),
            sequence: 1,
            issued_at_unix_ms: 1_000,
            expires_at_unix_ms: 61_000,
            payload: vec![],
        }
    }

    #[test]
    fn valid_envelope_is_accepted() {
        let env = base_envelope();
        let mut tracker = SequenceTracker::new();
        assert!(validate(&env, 30_000, Some("session-abc"), &mut tracker).is_ok());
    }

    #[test]
    fn wrong_protocol_version_is_rejected() {
        let mut env = base_envelope();
        env.protocol_version = 99;
        let mut tracker = SequenceTracker::new();
        assert_eq!(
            validate(&env, 30_000, None, &mut tracker),
            Err(EnvelopeError::UnsupportedProtocolVersion { received: 99 })
        );
    }

    #[test]
    fn expired_envelope_is_rejected() {
        let env = base_envelope(); // expires at 61_000
        let mut tracker = SequenceTracker::new();
        assert_eq!(
            validate(&env, 62_000, None, &mut tracker),
            Err(EnvelopeError::Expired { expired_at_unix_ms: 61_000 })
        );
    }

    #[test]
    fn invalid_expiry_window_is_rejected() {
        let mut env = base_envelope();
        env.expires_at_unix_ms = env.issued_at_unix_ms; // equal, not strictly greater
        let mut tracker = SequenceTracker::new();
        assert_eq!(validate(&env, 500, None, &mut tracker), Err(EnvelopeError::InvalidExpiry));
    }

    #[test]
    fn replayed_sequence_is_rejected() {
        let env = base_envelope(); // sequence = 1
        let mut tracker = SequenceTracker::new();
        validate(&env, 30_000, Some("session-abc"), &mut tracker).unwrap();
        // Replay the same envelope.
        assert_eq!(
            validate(&env, 30_000, Some("session-abc"), &mut tracker),
            Err(EnvelopeError::ReplayDetected { received: 1, last_accepted: 1 })
        );
    }

    #[test]
    fn monotonically_increasing_sequences_are_accepted() {
        let mut tracker = SequenceTracker::new();
        for seq in 1u64..=5 {
            let mut env = base_envelope();
            env.sequence = seq;
            assert!(validate(&env, 30_000, Some("session-abc"), &mut tracker).is_ok());
        }
    }

    #[test]
    fn session_mismatch_is_rejected() {
        let env = base_envelope(); // session_id = "session-abc"
        let mut tracker = SequenceTracker::new();
        assert_eq!(
            validate(&env, 30_000, Some("session-xyz"), &mut tracker),
            Err(EnvelopeError::SessionMismatch)
        );
    }

    #[test]
    fn oversized_payload_is_rejected() {
        let mut env = base_envelope();
        env.payload = vec![0u8; MAX_PAYLOAD_BYTES + 1];
        let mut tracker = SequenceTracker::new();
        assert_eq!(
            validate(&env, 30_000, None, &mut tracker),
            Err(EnvelopeError::PayloadTooLarge { bytes: MAX_PAYLOAD_BYTES + 1 })
        );
    }
}
