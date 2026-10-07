//! Strongly-typed SMP/1 file transfer engine and verification primitives.
//!
//! # Architecture & Constraints
//!
//! - **Strict Path Traversal Defense**: File names provided by remote peers are strictly
//!   validated and sanitized. Paths, directories (`/`, `\`), `..`, control characters,
//!   null bytes, and Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, etc.) are
//!   immediately rejected.
//! - **Host Authorization as Source of Truth**: Transfers require explicit host approval
//!   and must match the granted permission (`SendFiles` for HostToClient, `ReceiveFiles`
//!   for ClientToHost).
//! - **Chunk Integrity & Resumability**: Every chunk carries a SHA-256 digest verified
//!   before writing. Resumption allows continuing from the exact chunk index where a
//!   previous connection was interrupted.
//! - **Redaction in Logs**: Debug formatting for transfer sessions redacts raw file paths.

use crate::SessionPermission;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

/// Default chunk size for SMP/1 transfers: 64 KiB.
pub const DEFAULT_CHUNK_SIZE: u32 = 64 * 1024;

/// Maximum allowed chunk size: 1 MiB.
pub const MAX_CHUNK_SIZE: u32 = 1024 * 1024;

/// Maximum allowed single file transfer size: 4 GiB.
pub const MAX_FILE_SIZE: u64 = 4 * 1024 * 1024 * 1024;

/// Maximum allowed file name length in bytes.
pub const MAX_FILE_NAME_LEN: usize = 255;

// ─── Transfer Direction ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferDirection {
    /// Host is sending file to client (requires `SessionPermission::SendFiles`).
    HostToClient,
    /// Client is sending file to host (requires `SessionPermission::ReceiveFiles`).
    ClientToHost,
}

impl TransferDirection {
    /// Returns the required session permission for this transfer direction.
    pub fn required_permission(&self) -> SessionPermission {
        match self {
            Self::HostToClient => SessionPermission::SendFiles,
            Self::ClientToHost => SessionPermission::ReceiveFiles,
        }
    }
}

// ─── Transfer State ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferState {
    /// Transfer has been announced / requested but not yet approved by the host.
    PendingApproval,
    /// Transfer is actively streaming chunks.
    InProgress,
    /// Transfer is paused by either endpoint.
    Paused,
    /// All chunks verified and file integrity successfully matched.
    Completed,
    /// Transfer failed due to an error (integrity mismatch, I/O, etc.).
    Failed,
    /// Transfer was cancelled by either endpoint.
    Cancelled,
}

// ─── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferError {
    /// Filename contains illegal characters, directory separators, or path traversal.
    InvalidFileName(String),
    /// File size exceeds maximum allowed policy limit.
    FileTooLarge { size: u64, max: u64 },
    /// Chunk size is invalid or exceeds maximum chunk limit.
    InvalidChunkSize { size: u32, max: u32 },
    /// Total chunks count does not match total bytes / chunk size calculation.
    ChunkCountMismatch { expected: u64, actual: u64 },
    /// Chunk index is out of bounds for this transfer.
    ChunkIndexOutOfBounds { chunk_index: u64, total_chunks: u64 },
    /// Chunk data size is invalid (unexpected length for chunk).
    InvalidChunkDataLength { expected: usize, actual: usize },
    /// Chunk SHA-256 digest mismatch.
    ChunkDigestMismatch { chunk_index: u64, expected: String, actual: String },
    /// Final file SHA-256 digest mismatch.
    FinalDigestMismatch { expected: String, actual: String },
    /// State machine invalid transition.
    InvalidStateTransition { current: TransferState, attempted: &'static str },
    /// Session permission missing.
    PermissionDenied(SessionPermission),
}

impl fmt::Display for TransferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFileName(reason) => write!(f, "invalid file name: {}", reason),
            Self::FileTooLarge { size, max } => {
                write!(f, "file size {} exceeds maximum limit {}", size, max)
            }
            Self::InvalidChunkSize { size, max } => {
                write!(f, "chunk size {} exceeds maximum {}", size, max)
            }
            Self::ChunkCountMismatch { expected, actual } => {
                write!(f, "chunk count mismatch: expected {}, got {}", expected, actual)
            }
            Self::ChunkIndexOutOfBounds { chunk_index, total_chunks } => {
                write!(f, "chunk index {} out of bounds (total {})", chunk_index, total_chunks)
            }
            Self::InvalidChunkDataLength { expected, actual } => {
                write!(f, "invalid chunk data length: expected {}, got {}", expected, actual)
            }
            Self::ChunkDigestMismatch { chunk_index, expected, actual } => {
                write!(
                    f,
                    "chunk {} checksum mismatch: expected {}, calculated {}",
                    chunk_index, expected, actual
                )
            }
            Self::FinalDigestMismatch { expected, actual } => {
                write!(
                    f,
                    "final file checksum mismatch: expected {}, calculated {}",
                    expected, actual
                )
            }
            Self::InvalidStateTransition { current, attempted } => {
                write!(f, "invalid transfer state transition: cannot {} while {:?}", attempted, current)
            }
            Self::PermissionDenied(perm) => {
                write!(f, "host permission denied: requires {:?}", perm)
            }
        }
    }
}

impl std::error::Error for TransferError {}

// ─── Filename Sanitization & Path Traversal Prevention ───────────────────────

/// Sanitizes and validates a peer-provided file name.
///
/// Rules:
/// - Rejects directory separators (`/`, `\`).
/// - Rejects path traversal (`.`, `..`).
/// - Rejects null bytes and ASCII control characters (0x00..=0x1F, 0x7F).
/// - Rejects Windows reserved device names: CON, PRN, AUX, NUL, COM1-9, LPT1-9
///   (case-insensitive, even with arbitrary file extensions).
/// - Trims leading/trailing whitespace and trailing dots (forbidden by NTFS).
/// - Enforces max length <= 255 bytes.
pub fn sanitize_file_name(raw: &str) -> Result<String, TransferError> {
    if raw.is_empty() {
        return Err(TransferError::InvalidFileName("empty file name".to_string()));
    }

    if raw.len() > MAX_FILE_NAME_LEN {
        return Err(TransferError::InvalidFileName(format!(
            "file name length {} exceeds maximum {}",
            raw.len(),
            MAX_FILE_NAME_LEN
        )));
    }

    // Check for directory traversal / separators
    if raw.contains('/') || raw.contains('\\') {
        return Err(TransferError::InvalidFileName(
            "path separators are strictly prohibited".to_string(),
        ));
    }

    // Check for null bytes and control chars
    for ch in raw.chars() {
        if ch == '\0' {
            return Err(TransferError::InvalidFileName(
                "null bytes are strictly prohibited".to_string(),
            ));
        }
        if (ch as u32) < 0x20 || ch as u32 == 0x7F {
            return Err(TransferError::InvalidFileName(
                "control characters are strictly prohibited".to_string(),
            ));
        }
        // Illegal Windows file characters
        if matches!(ch, ':' | '*' | '?' | '"' | '<' | '>' | '|') {
            return Err(TransferError::InvalidFileName(format!(
                "character '{}' is prohibited in file names",
                ch
            )));
        }
    }

    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        return Err(TransferError::InvalidFileName(
            "file name cannot be '.' or '..' or whitespace only".to_string(),
        ));
    }

    // Reject trailing dots or spaces which Windows NTFS strips or treats ambiguously
    let normalized = trimmed.trim_end_matches('.');
    if normalized.is_empty() {
        return Err(TransferError::InvalidFileName(
            "file name cannot consist solely of periods".to_string(),
        ));
    }

    // Check Windows reserved device names
    let stem = match normalized.find('.') {
        Some(idx) => &normalized[..idx],
        None => normalized,
    };

    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    for res in &reserved {
        if stem.eq_ignore_ascii_case(res) {
            return Err(TransferError::InvalidFileName(format!(
                "reserved device name '{}' is prohibited",
                res
            )));
        }
    }

    Ok(normalized.to_string())
}

// ─── Pure SHA-256 Engine (FIPS 180-4) ────────────────────────────────────────

/// Calculates the SHA-256 hex digest of a byte slice.
pub fn compute_sha256(data: &[u8]) -> String {
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let total_len = data.len();
    let mut buffer = Vec::with_capacity(total_len + 64);
    buffer.extend_from_slice(data);

    // Padding: append 0x80, then 0x00 until len % 64 == 56
    buffer.push(0x80);
    while (buffer.len() % 64) != 56 {
        buffer.push(0x00);
    }

    // Append 64-bit length in bits (big-endian)
    let bit_len = (total_len as u64).wrapping_mul(8);
    buffer.extend_from_slice(&bit_len.to_be_bytes());

    // Process blocks
    for chunk in buffer.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut hex = String::with_capacity(64);
    for word in state {
        hex.push_str(&format!("{:08x}", word));
    }
    hex
}

// ─── Transfer Session Model ──────────────────────────────────────────────────

/// Active or completed transfer session tracked by MigRoute.
#[derive(Clone, Serialize, Deserialize)]
pub struct TransferSession {
    pub transfer_id: String,
    pub file_name: String,
    pub total_bytes: u64,
    pub chunk_size: u32,
    pub total_chunks: u64,
    pub expected_sha256: String,
    pub direction: TransferDirection,
    pub state: TransferState,
    pub bytes_transferred: u64,
    pub chunks_completed: u64,
    pub next_expected_chunk: u64,
    #[serde(skip)]
    pub acknowledged_chunks: BTreeSet<u64>,
}

impl fmt::Debug for TransferSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Redact file paths in compliance with repository rules
        f.debug_struct("TransferSession")
            .field("transfer_id", &self.transfer_id)
            .field("file_name", &"[REDACTED_FILE_NAME]")
            .field("total_bytes", &self.total_bytes)
            .field("chunk_size", &self.chunk_size)
            .field("total_chunks", &self.total_chunks)
            .field("direction", &self.direction)
            .field("state", &self.state)
            .field("bytes_transferred", &self.bytes_transferred)
            .field("chunks_completed", &self.chunks_completed)
            .field("next_expected_chunk", &self.next_expected_chunk)
            .finish()
    }
}

impl TransferSession {
    /// Creates a new outgoing file transfer session (Host to Client).
    pub fn new_outgoing(
        transfer_id: String,
        file_name: &str,
        total_bytes: u64,
        chunk_size: u32,
        expected_sha256: String,
    ) -> Result<Self, TransferError> {
        let sanitized_name = sanitize_file_name(file_name)?;

        if total_bytes > MAX_FILE_SIZE {
            return Err(TransferError::FileTooLarge {
                size: total_bytes,
                max: MAX_FILE_SIZE,
            });
        }

        let effective_chunk_size = if chunk_size == 0 {
            DEFAULT_CHUNK_SIZE
        } else if chunk_size > MAX_CHUNK_SIZE {
            return Err(TransferError::InvalidChunkSize {
                size: chunk_size,
                max: MAX_CHUNK_SIZE,
            });
        } else {
            chunk_size
        };

        let total_chunks = if total_bytes == 0 {
            1
        } else {
            (total_bytes + effective_chunk_size as u64 - 1) / effective_chunk_size as u64
        };

        Ok(Self {
            transfer_id,
            file_name: sanitized_name,
            total_bytes,
            chunk_size: effective_chunk_size,
            total_chunks,
            expected_sha256: expected_sha256.to_ascii_lowercase(),
            direction: TransferDirection::HostToClient,
            state: TransferState::PendingApproval,
            bytes_transferred: 0,
            chunks_completed: 0,
            next_expected_chunk: 0,
            acknowledged_chunks: BTreeSet::new(),
        })
    }

    /// Creates a new incoming file transfer session (Client to Host).
    pub fn new_incoming(
        transfer_id: String,
        file_name: &str,
        total_bytes: u64,
        chunk_size: u32,
        total_chunks: u64,
        expected_sha256: String,
    ) -> Result<Self, TransferError> {
        let sanitized_name = sanitize_file_name(file_name)?;

        if total_bytes > MAX_FILE_SIZE {
            return Err(TransferError::FileTooLarge {
                size: total_bytes,
                max: MAX_FILE_SIZE,
            });
        }

        let effective_chunk_size = if chunk_size == 0 {
            DEFAULT_CHUNK_SIZE
        } else if chunk_size > MAX_CHUNK_SIZE {
            return Err(TransferError::InvalidChunkSize {
                size: chunk_size,
                max: MAX_CHUNK_SIZE,
            });
        } else {
            chunk_size
        };

        let calculated_chunks = if total_bytes == 0 {
            1
        } else {
            (total_bytes + effective_chunk_size as u64 - 1) / effective_chunk_size as u64
        };

        if total_chunks != 0 && total_chunks != calculated_chunks {
            return Err(TransferError::ChunkCountMismatch {
                expected: calculated_chunks,
                actual: total_chunks,
            });
        }

        Ok(Self {
            transfer_id,
            file_name: sanitized_name,
            total_bytes,
            chunk_size: effective_chunk_size,
            total_chunks: calculated_chunks,
            expected_sha256: expected_sha256.to_ascii_lowercase(),
            direction: TransferDirection::ClientToHost,
            state: TransferState::PendingApproval,
            bytes_transferred: 0,
            chunks_completed: 0,
            next_expected_chunk: 0,
            acknowledged_chunks: BTreeSet::new(),
        })
    }

    /// Approves the transfer session and moves it to `InProgress`.
    pub fn approve(&mut self) -> Result<(), TransferError> {
        if self.state != TransferState::PendingApproval {
            return Err(TransferError::InvalidStateTransition {
                current: self.state,
                attempted: "approve",
            });
        }
        self.state = TransferState::InProgress;
        Ok(())
    }

    /// Validates an incoming chunk's index, data length, and SHA-256 digest.
    pub fn validate_chunk(
        &self,
        chunk_index: u64,
        data: &[u8],
        chunk_sha256: &str,
    ) -> Result<(), TransferError> {
        if self.state != TransferState::InProgress {
            return Err(TransferError::InvalidStateTransition {
                current: self.state,
                attempted: "validate_chunk",
            });
        }

        if chunk_index >= self.total_chunks {
            return Err(TransferError::ChunkIndexOutOfBounds {
                chunk_index,
                total_chunks: self.total_chunks,
            });
        }

        // Expected length for this chunk
        let expected_len = if chunk_index == self.total_chunks - 1 {
            let remainder = (self.total_bytes % self.chunk_size as u64) as usize;
            if remainder == 0 && self.total_bytes > 0 {
                self.chunk_size as usize
            } else {
                remainder
            }
        } else {
            self.chunk_size as usize
        };

        if data.len() != expected_len {
            return Err(TransferError::InvalidChunkDataLength {
                expected: expected_len,
                actual: data.len(),
            });
        }

        // Verify SHA-256
        let calculated_hash = compute_sha256(data);
        if !calculated_hash.eq_ignore_ascii_case(chunk_sha256) {
            return Err(TransferError::ChunkDigestMismatch {
                chunk_index,
                expected: chunk_sha256.to_string(),
                actual: calculated_hash,
            });
        }

        Ok(())
    }

    /// Records that a chunk has been successfully written and acknowledged.
    pub fn record_chunk_completed(&mut self, chunk_index: u64, bytes: usize) {
        if !self.acknowledged_chunks.contains(&chunk_index) {
            self.acknowledged_chunks.insert(chunk_index);
            self.chunks_completed += 1;
            self.bytes_transferred += bytes as u64;

            // Advance monotonic next expected chunk if contiguous
            while self.acknowledged_chunks.contains(&self.next_expected_chunk) {
                self.next_expected_chunk += 1;
            }
        }
    }

    /// Pauses the transfer.
    pub fn pause(&mut self) -> Result<(), TransferError> {
        if self.state != TransferState::InProgress {
            return Err(TransferError::InvalidStateTransition {
                current: self.state,
                attempted: "pause",
            });
        }
        self.state = TransferState::Paused;
        Ok(())
    }

    /// Resumes the transfer from the specified chunk index.
    pub fn resume(&mut self, from_chunk: u64) -> Result<(), TransferError> {
        if self.state != TransferState::Paused {
            return Err(TransferError::InvalidStateTransition {
                current: self.state,
                attempted: "resume",
            });
        }
        if from_chunk >= self.total_chunks {
            return Err(TransferError::ChunkIndexOutOfBounds {
                chunk_index: from_chunk,
                total_chunks: self.total_chunks,
            });
        }
        self.next_expected_chunk = from_chunk;
        self.state = TransferState::InProgress;
        Ok(())
    }

    /// Cancels the transfer with a reason.
    pub fn cancel(&mut self) {
        self.state = TransferState::Cancelled;
    }

    /// Finalizes the transfer after verifying the full file's SHA-256 digest.
    pub fn finalize(&mut self, final_sha256: &str) -> Result<(), TransferError> {
        if self.state != TransferState::InProgress {
            return Err(TransferError::InvalidStateTransition {
                current: self.state,
                attempted: "finalize",
            });
        }

        if !self.expected_sha256.eq_ignore_ascii_case(final_sha256) {
            self.state = TransferState::Failed;
            return Err(TransferError::FinalDigestMismatch {
                expected: self.expected_sha256.clone(),
                actual: final_sha256.to_string(),
            });
        }

        self.state = TransferState::Completed;
        Ok(())
    }

    /// Returns the progress ratio as a percentage (0.0 to 100.0).
    pub fn progress_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            if self.state == TransferState::Completed {
                100.0
            } else {
                0.0
            }
        } else {
            let pct = (self.bytes_transferred as f64 / self.total_bytes as f64) * 100.0;
            pct.min(100.0) as f32
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_nist_vectors() {
        // NIST test vector: "" (empty string)
        assert_eq!(
            compute_sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        // NIST test vector: "abc"
        assert_eq!(
            compute_sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );

        // NIST test vector: "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
        assert_eq!(
            compute_sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn test_filename_sanitization_accepts_valid_names() {
        assert_eq!(
            sanitize_file_name("document.pdf").unwrap(),
            "document.pdf"
        );
        assert_eq!(
            sanitize_file_name("Photo_2026-10-05.jpg").unwrap(),
            "Photo_2026-10-05.jpg"
        );
        assert_eq!(
            sanitize_file_name("archive.tar.gz").unwrap(),
            "archive.tar.gz"
        );
        assert_eq!(
            sanitize_file_name(" spaces trimmed .txt ").unwrap(),
            "spaces trimmed .txt"
        );
    }

    #[test]
    fn test_filename_sanitization_rejects_path_traversal() {
        assert!(matches!(
            sanitize_file_name("../secret.txt"),
            Err(TransferError::InvalidFileName(_))
        ));
        assert!(matches!(
            sanitize_file_name("..\\secret.txt"),
            Err(TransferError::InvalidFileName(_))
        ));
        assert!(matches!(
            sanitize_file_name("/etc/passwd"),
            Err(TransferError::InvalidFileName(_))
        ));
        assert!(matches!(
            sanitize_file_name("C:\\Windows\\system32\\cmd.exe"),
            Err(TransferError::InvalidFileName(_))
        ));
        assert!(matches!(
            sanitize_file_name("subdir/file.txt"),
            Err(TransferError::InvalidFileName(_))
        ));
    }

    #[test]
    fn test_filename_sanitization_rejects_null_bytes_and_control_chars() {
        assert!(matches!(
            sanitize_file_name("file\0name.txt"),
            Err(TransferError::InvalidFileName(_))
        ));
        assert!(matches!(
            sanitize_file_name("file\x1bname.txt"),
            Err(TransferError::InvalidFileName(_))
        ));
        assert!(matches!(
            sanitize_file_name("file\r\nname.txt"),
            Err(TransferError::InvalidFileName(_))
        ));
    }

    #[test]
    fn test_filename_sanitization_rejects_windows_reserved_names() {
        assert!(matches!(
            sanitize_file_name("CON"),
            Err(TransferError::InvalidFileName(_))
        ));
        assert!(matches!(
            sanitize_file_name("con.txt"),
            Err(TransferError::InvalidFileName(_))
        ));
        assert!(matches!(
            sanitize_file_name("PRN.pdf"),
            Err(TransferError::InvalidFileName(_))
        ));
        assert!(matches!(
            sanitize_file_name("aux.log"),
            Err(TransferError::InvalidFileName(_))
        ));
        assert!(matches!(
            sanitize_file_name("NUL"),
            Err(TransferError::InvalidFileName(_))
        ));
        assert!(matches!(
            sanitize_file_name("COM1.dat"),
            Err(TransferError::InvalidFileName(_))
        ));
        assert!(matches!(
            sanitize_file_name("LPT3"),
            Err(TransferError::InvalidFileName(_))
        ));
    }

    #[test]
    fn test_filename_sanitization_rejects_invalid_ntfs_chars() {
        assert!(matches!(
            sanitize_file_name("file:stream.txt"),
            Err(TransferError::InvalidFileName(_))
        ));
        assert!(matches!(
            sanitize_file_name("wildcard*file.txt"),
            Err(TransferError::InvalidFileName(_))
        ));
        assert!(matches!(
            sanitize_file_name("pipe|name.txt"),
            Err(TransferError::InvalidFileName(_))
        ));
    }

    #[test]
    fn test_transfer_lifecycle_success() {
        let chunk0 = b"0123456789abcdef0123456789abcdef"; // 32 bytes
        let chunk1 = b"fedcba9876543210fedcba9876543210"; // 32 bytes
        let mut full_data = Vec::new();
        full_data.extend_from_slice(chunk0);
        full_data.extend_from_slice(chunk1);

        let chunk0_sha256 = compute_sha256(chunk0);
        let chunk1_sha256 = compute_sha256(chunk1);
        let full_sha256 = compute_sha256(&full_data);

        let mut session = TransferSession::new_incoming(
            "transfer-123".to_string(),
            "sample.txt",
            full_data.len() as u64,
            chunk0.len() as u32,
            2,
            full_sha256.clone(),
        )
        .unwrap();

        assert_eq!(session.state, TransferState::PendingApproval);
        assert_eq!(session.total_chunks, 2);

        // Approve
        session.approve().unwrap();
        assert_eq!(session.state, TransferState::InProgress);

        // Validate chunk 0
        session.validate_chunk(0, chunk0, &chunk0_sha256).unwrap();
        session.record_chunk_completed(0, chunk0.len());
        assert_eq!(session.chunks_completed, 1);
        assert_eq!(session.next_expected_chunk, 1);

        // Validate chunk 1
        session.validate_chunk(1, chunk1, &chunk1_sha256).unwrap();
        session.record_chunk_completed(1, chunk1.len());
        assert_eq!(session.chunks_completed, 2);
        assert_eq!(session.next_expected_chunk, 2);
        assert_eq!(session.progress_percent(), 100.0);

        // Finalize
        session.finalize(&full_sha256).unwrap();
        assert_eq!(session.state, TransferState::Completed);
    }

    #[test]
    fn test_chunk_corrupt_digest_is_rejected() {
        let chunk = b"valid data";
        let mut session = TransferSession::new_incoming(
            "t1".to_string(),
            "data.bin",
            chunk.len() as u64,
            chunk.len() as u32,
            1,
            compute_sha256(chunk),
        )
        .unwrap();

        session.approve().unwrap();

        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let res = session.validate_chunk(0, chunk, wrong_hash);
        assert!(matches!(res, Err(TransferError::ChunkDigestMismatch { .. })));
    }

    #[test]
    fn test_pause_and_resume_lifecycle() {
        let mut session = TransferSession::new_outgoing(
            "t2".to_string(),
            "large.iso",
            100_000,
            10_000,
            "dummy_hash".to_string(),
        )
        .unwrap();

        session.approve().unwrap();
        assert_eq!(session.state, TransferState::InProgress);

        // Record chunks 0, 1, 2
        session.record_chunk_completed(0, 10_000);
        session.record_chunk_completed(1, 10_000);
        session.record_chunk_completed(2, 10_000);
        assert_eq!(session.next_expected_chunk, 3);

        // Pause
        session.pause().unwrap();
        assert_eq!(session.state, TransferState::Paused);

        // Resume from chunk 3
        session.resume(3).unwrap();
        assert_eq!(session.state, TransferState::InProgress);
        assert_eq!(session.next_expected_chunk, 3);
    }

    #[test]
    fn test_file_too_large_is_rejected() {
        let res = TransferSession::new_outgoing(
            "t3".to_string(),
            "huge.bin",
            MAX_FILE_SIZE + 1,
            DEFAULT_CHUNK_SIZE,
            "hash".to_string(),
        );
        assert!(matches!(res, Err(TransferError::FileTooLarge { .. })));
    }
}
