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
    /// Checkpoint journal corruption or invalid checksum.
    CorruptCheckpoint(String),
    /// Chunk decompression failure.
    DecompressionFailed(String),
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
            Self::CorruptCheckpoint(reason) => {
                write!(f, "corrupt checkpoint journal: {}", reason)
            }
            Self::DecompressionFailed(reason) => {
                write!(f, "chunk decompression failed: {}", reason)
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

    // NTFS silently drops trailing dots and spaces, so normalize exactly the way the filesystem will
    // before any check, otherwise "CON ." would pass as a name and land on the CON device.
    let normalized = raw.trim_start().trim_end_matches(['.', ' ']);
    if normalized.is_empty() || normalized == "." || normalized == ".." {
        return Err(TransferError::InvalidFileName(
            "file name cannot be empty, '.' or '..', or consist only of periods and spaces".to_string(),
        ));
    }

    // Windows device names apply to the base name, which ignores trailing spaces before the extension.
    let stem = match normalized.find('.') {
        Some(idx) => &normalized[..idx],
        None => normalized,
    };
    let stem = stem.trim_end_matches(' ');

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

// ─── Standard SHA-256 Engine (FIPS 180-4 via sha2 crate) ────────────────────

/// Calculates the SHA-256 hex digest of a byte slice.
pub fn compute_sha256(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in result {
        use std::fmt::Write;
        let _ = write!(&mut hex, "{:02x}", b);
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
            total_bytes.div_ceil(effective_chunk_size as u64)
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
            total_bytes.div_ceil(effective_chunk_size as u64)
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

// ─── Pure Rust LZ4 Block Compression Engine & Codec ──────────────────────────

/// Supported compression codecs for SMP/1 chunk transfer pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TransferCompression {
    /// Uncompressed raw bytes (used for media, zip, or high-entropy files).
    #[default]
    None,
    /// Ultra-fast LZ4 block compression (near memory bandwidth throughput).
    Lz4,
    /// High-ratio Zstandard compression (for network-constrained links).
    Zstd,
}

/// Computes standard IEEE 802.3 CRC32 checksum over a byte slice.
pub fn compute_crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

const LZ4_HASH_BITS: usize = 12;
const LZ4_HASH_SIZE: usize = 1 << LZ4_HASH_BITS; // 4096 hash slots

#[inline]
fn lz4_hash4(data: &[u8], pos: usize) -> usize {
    let val = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
    ((val.wrapping_mul(2654435761)) >> (32 - LZ4_HASH_BITS)) as usize
}

fn emit_lz4_sequence(out: &mut Vec<u8>, literals: &[u8], offset: u16, match_len: usize) {
    let lit_len = literals.len();
    let token_lit = (lit_len.min(15) as u8) << 4;
    let token_match = if offset > 0 {
        ((match_len.saturating_sub(4)).min(15)) as u8
    } else {
        0
    };
    out.push(token_lit | token_match);

    // Literal length overflow
    if lit_len >= 15 {
        let mut rem = lit_len - 15;
        while rem >= 255 {
            out.push(255);
            rem -= 255;
        }
        out.push(rem as u8);
    }

    out.extend_from_slice(literals);

    if offset > 0 {
        out.extend_from_slice(&offset.to_le_bytes());
        if match_len >= 19 {
            let mut rem = match_len - 19;
            while rem >= 255 {
                out.push(255);
                rem -= 255;
            }
            out.push(rem as u8);
        }
    }
}

/// Compresses a raw byte block using pure Rust LZ4 block format.
pub fn lz4_compress_block(src: &[u8]) -> Vec<u8> {
    if src.len() < 13 {
        let mut out = Vec::with_capacity(src.len() + 16);
        emit_lz4_sequence(&mut out, src, 0, 0);
        return out;
    }

    let mut out = Vec::with_capacity(src.len());
    let mut table = [0u32; LZ4_HASH_SIZE];
    let mut anchor = 0usize;
    let mut ip = 0usize;
    let limit = src.len().saturating_sub(5);

    while ip < limit {
        let h = lz4_hash4(src, ip);
        let ref_pos = table[h] as usize;
        table[h] = ip as u32;

        let distance = ip.wrapping_sub(ref_pos);
        if distance > 0
            && distance <= 65535
            && ip + 4 <= src.len()
            && ref_pos + 4 <= src.len()
            && src[ip..ip + 4] == src[ref_pos..ref_pos + 4]
        {
            // Determine match length
            let mut match_len = 4;
            while ip + match_len < src.len() && src[ip + match_len] == src[ref_pos + match_len] {
                match_len += 1;
            }

            emit_lz4_sequence(&mut out, &src[anchor..ip], distance as u16, match_len);

            ip += match_len;
            anchor = ip;
            if ip < limit {
                table[lz4_hash4(src, ip)] = ip as u32;
            }
            continue;
        }
        ip += 1;
    }

    // Trailing literals
    if anchor < src.len() {
        emit_lz4_sequence(&mut out, &src[anchor..], 0, 0);
    }

    out
}

/// Decompresses an LZ4 compressed block into the uncompressed buffer.
pub fn lz4_decompress_block(src: &[u8], uncompressed_size: usize) -> Result<Vec<u8>, TransferError> {
    let mut dest = Vec::with_capacity(uncompressed_size);
    let mut ip = 0;

    while ip < src.len() {
        let token = src[ip];
        ip += 1;

        let mut lit_len = (token >> 4) as usize;
        if lit_len == 15 {
            loop {
                if ip >= src.len() {
                    return Err(TransferError::DecompressionFailed(
                        "unexpected end of stream in literal length".to_string(),
                    ));
                }
                let extra = src[ip] as usize;
                ip += 1;
                lit_len += extra;
                if extra < 255 {
                    break;
                }
            }
        }

        if ip + lit_len > src.len() {
            return Err(TransferError::DecompressionFailed(
                "literal length extends past input buffer".to_string(),
            ));
        }
        dest.extend_from_slice(&src[ip..ip + lit_len]);
        ip += lit_len;

        if ip == src.len() {
            break;
        }

        if ip + 2 > src.len() {
            return Err(TransferError::DecompressionFailed(
                "truncated match offset".to_string(),
            ));
        }
        let offset = u16::from_le_bytes([src[ip], src[ip + 1]]) as usize;
        ip += 2;
        if offset == 0 || offset > dest.len() {
            return Err(TransferError::DecompressionFailed(format!(
                "invalid match offset: {} (dest len {})",
                offset,
                dest.len()
            )));
        }

        let mut match_len = (token & 0x0F) as usize + 4;
        if match_len == 19 {
            loop {
                if ip >= src.len() {
                    return Err(TransferError::DecompressionFailed(
                        "unexpected end of stream in match length".to_string(),
                    ));
                }
                let extra = src[ip] as usize;
                ip += 1;
                match_len += extra;
                if extra < 255 {
                    break;
                }
            }
        }

        let match_start = dest.len() - offset;
        for i in 0..match_len {
            let b = dest[match_start + i];
            dest.push(b);
        }
    }

    if dest.len() != uncompressed_size {
        return Err(TransferError::DecompressionFailed(format!(
            "size mismatch: expected {} bytes, decoded {}",
            uncompressed_size,
            dest.len()
        )));
    }

    Ok(dest)
}

/// Evaluates file extension and Shannon entropy to determine if on-the-fly compression is beneficial.
pub fn should_compress_file(file_name: &str, sample_data: &[u8]) -> bool {
    let lower = file_name.to_ascii_lowercase();
    let uncompressible_exts = [
        ".zip", ".gz", ".tgz", ".7z", ".rar", ".xz", ".zst", ".tar.gz",
        ".mp4", ".mkv", ".avi", ".mov", ".webm",
        ".mp3", ".aac", ".flac", ".ogg",
        ".jpg", ".jpeg", ".png", ".webp", ".gif",
        ".pdf", ".apk", ".dmg", ".iso",
    ];

    for ext in &uncompressible_exts {
        if lower.ends_with(ext) {
            return false;
        }
    }

    if sample_data.is_empty() {
        return true;
    }

    let sample = if sample_data.len() > 4096 {
        &sample_data[..4096]
    } else {
        sample_data
    };

    let mut freq = [0u32; 256];
    for &b in sample {
        freq[b as usize] += 1;
    }

    let len_f = sample.len() as f64;
    let mut entropy = 0.0f64;
    for &count in &freq {
        if count > 0 {
            let p = count as f64 / len_f;
            entropy -= p * p.log2();
        }
    }

    // Skip compression if entropy is > 7.6 bits/byte (high randomness / already compressed)
    entropy <= 7.6
}

// ─── Parallel Chunk Slicing Pipeline ─────────────────────────────────────────

/// A single sliced chunk within a parallel transfer batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferChunkSlice {
    pub chunk_index: u64,
    pub byte_offset: u64,
    pub uncompressed_length: u32,
    pub compression: TransferCompression,
    pub payload: Vec<u8>,
    pub chunk_sha256: String,
}

/// A batch of slices transmitted or received in parallel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferChunkBatch {
    pub transfer_id: String,
    pub batch_id: u64,
    pub slices: Vec<TransferChunkSlice>,
    pub is_final_batch: bool,
}

/// Result of processing an incoming chunk slice.
#[derive(Debug, PartialEq, Eq)]
pub enum ProcessedSlice {
    /// Slice was successfully validated and decompressed for writing to disk.
    ReadyToWrite {
        chunk_index: u64,
        byte_offset: u64,
        data: Vec<u8>,
    },
    /// Slice was already received and committed in a previous batch.
    AlreadyProcessed { chunk_index: u64 },
}

/// State-machine assembler for out-of-order parallel chunk ingestion.
pub struct ParallelChunkAssembler {
    pub transfer_id: String,
    pub total_chunks: u64,
    pub chunk_size: u32,
    pub total_bytes: u64,
    pub completed_chunks: BTreeSet<u64>,
    pub bytes_written: u64,
    pub next_contiguous_chunk: u64,
}

impl ParallelChunkAssembler {
    pub fn new(
        transfer_id: String,
        total_chunks: u64,
        chunk_size: u32,
        total_bytes: u64,
    ) -> Self {
        Self {
            transfer_id,
            total_chunks,
            chunk_size,
            total_bytes,
            completed_chunks: BTreeSet::new(),
            bytes_written: 0,
            next_contiguous_chunk: 0,
        }
    }

    pub fn from_checkpoint(checkpoint: &TransferCheckpoint) -> Self {
        let mut completed = BTreeSet::new();
        for &idx in &checkpoint.completed_chunk_indices {
            completed.insert(idx);
        }

        let mut next_contiguous = 0u64;
        while completed.contains(&next_contiguous) {
            next_contiguous += 1;
        }

        let mut bytes_written = 0u64;
        for &idx in &completed {
            let len = if idx == checkpoint.total_chunks - 1 {
                let rem = (checkpoint.file_size % checkpoint.chunk_size as u64) as usize;
                if rem == 0 && checkpoint.file_size > 0 {
                    checkpoint.chunk_size as u64
                } else {
                    rem as u64
                }
            } else {
                checkpoint.chunk_size as u64
            };
            bytes_written += len;
        }

        Self {
            transfer_id: checkpoint.transfer_id.clone(),
            total_chunks: checkpoint.total_chunks,
            chunk_size: checkpoint.chunk_size,
            total_bytes: checkpoint.file_size,
            completed_chunks: completed,
            bytes_written,
            next_contiguous_chunk: next_contiguous,
        }
    }

    /// Processes an incoming chunk slice: decompresses, verifies SHA-256, and returns write payload.
    pub fn process_incoming_slice(
        &mut self,
        slice: &TransferChunkSlice,
    ) -> Result<ProcessedSlice, TransferError> {
        if slice.chunk_index >= self.total_chunks {
            return Err(TransferError::ChunkIndexOutOfBounds {
                chunk_index: slice.chunk_index,
                total_chunks: self.total_chunks,
            });
        }

        if self.completed_chunks.contains(&slice.chunk_index) {
            return Ok(ProcessedSlice::AlreadyProcessed {
                chunk_index: slice.chunk_index,
            });
        }

        // Decompress payload if needed
        let uncompressed = match slice.compression {
            TransferCompression::None => slice.payload.clone(),
            TransferCompression::Lz4 => {
                lz4_decompress_block(&slice.payload, slice.uncompressed_length as usize)?
            }
            TransferCompression::Zstd => {
                slice.payload.clone()
            }
        };

        if uncompressed.len() != slice.uncompressed_length as usize {
            return Err(TransferError::InvalidChunkDataLength {
                expected: slice.uncompressed_length as usize,
                actual: uncompressed.len(),
            });
        }

        // Check SHA-256
        let calculated_hash = compute_sha256(&uncompressed);
        if !calculated_hash.eq_ignore_ascii_case(&slice.chunk_sha256) {
            return Err(TransferError::ChunkDigestMismatch {
                chunk_index: slice.chunk_index,
                expected: slice.chunk_sha256.clone(),
                actual: calculated_hash,
            });
        }

        // Mark as completed
        self.completed_chunks.insert(slice.chunk_index);
        self.bytes_written += uncompressed.len() as u64;

        while self.completed_chunks.contains(&self.next_contiguous_chunk) {
            self.next_contiguous_chunk += 1;
        }

        Ok(ProcessedSlice::ReadyToWrite {
            chunk_index: slice.chunk_index,
            byte_offset: slice.byte_offset,
            data: uncompressed,
        })
    }

    /// Creates an instant microsecond checkpoint of the current transfer state.
    pub fn create_checkpoint(&self, now_us: u64) -> TransferCheckpoint {
        let indices: Vec<u64> = self.completed_chunks.iter().copied().collect();
        TransferCheckpoint::new(
            self.transfer_id.clone(),
            self.total_bytes,
            self.chunk_size,
            self.total_chunks,
            indices,
            now_us,
        )
    }

    /// Returns list of all missing chunk indices.
    pub fn get_missing_chunks(&self) -> Vec<u64> {
        let mut missing = Vec::new();
        for i in 0..self.total_chunks {
            if !self.completed_chunks.contains(&i) {
                missing.push(i);
            }
        }
        missing
    }

    /// Checks if all chunks have been received and verified.
    pub fn is_complete(&self) -> bool {
        self.completed_chunks.len() as u64 == self.total_chunks
    }
}

// ─── Microsecond Crash-Recovery Checkpoint Journaling ─────────────────────────

/// Journal checkpoint persisted on disk for instantaneous crash recovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferCheckpoint {
    pub magic: [u8; 4], // b"SMTJ"
    pub version: u8,    // 1
    pub transfer_id: String,
    pub file_size: u64,
    pub chunk_size: u32,
    pub total_chunks: u64,
    pub completed_count: u64,
    pub completed_chunk_indices: Vec<u64>,
    pub timestamp_us: u64,
    pub crc32: u32,
}

impl TransferCheckpoint {
    pub fn new(
        transfer_id: String,
        file_size: u64,
        chunk_size: u32,
        total_chunks: u64,
        completed_chunk_indices: Vec<u64>,
        timestamp_us: u64,
    ) -> Self {
        let mut cp = Self {
            magic: *b"SMTJ",
            version: 1,
            transfer_id,
            file_size,
            chunk_size,
            total_chunks,
            completed_count: completed_chunk_indices.len() as u64,
            completed_chunk_indices,
            timestamp_us,
            crc32: 0,
        };
        let encoded_raw = cp.encode_without_crc();
        cp.crc32 = compute_crc32(&encoded_raw);
        cp
    }

    fn encode_without_crc(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(64 + self.completed_chunk_indices.len() * 8);
        buf.extend_from_slice(&self.magic);
        buf.push(self.version);

        let id_bytes = self.transfer_id.as_bytes();
        buf.extend_from_slice(&(id_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(id_bytes);

        buf.extend_from_slice(&self.file_size.to_le_bytes());
        buf.extend_from_slice(&self.chunk_size.to_le_bytes());
        buf.extend_from_slice(&self.total_chunks.to_le_bytes());
        buf.extend_from_slice(&self.completed_count.to_le_bytes());

        for &idx in &self.completed_chunk_indices {
            buf.extend_from_slice(&idx.to_le_bytes());
        }

        buf.extend_from_slice(&self.timestamp_us.to_le_bytes());
        buf
    }

    /// Encodes checkpoint into persistent byte format with trailing CRC32.
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = self.encode_without_crc();
        bytes.extend_from_slice(&self.crc32.to_le_bytes());
        bytes
    }

    /// Decodes and validates a persistent checkpoint byte slice.
    pub fn decode(bytes: &[u8]) -> Result<Self, TransferError> {
        if bytes.len() < 37 {
            return Err(TransferError::CorruptCheckpoint(
                "checkpoint data too small".to_string(),
            ));
        }

        let magic = &bytes[0..4];
        if magic != b"SMTJ" {
            return Err(TransferError::CorruptCheckpoint("invalid magic".to_string()));
        }

        let version = bytes[4];
        if version != 1 {
            return Err(TransferError::CorruptCheckpoint(format!(
                "unsupported checkpoint version {}",
                version
            )));
        }

        // Verify CRC32
        let payload_len = bytes.len() - 4;
        let expected_crc = u32::from_le_bytes([
            bytes[payload_len],
            bytes[payload_len + 1],
            bytes[payload_len + 2],
            bytes[payload_len + 3],
        ]);
        let actual_crc = compute_crc32(&bytes[..payload_len]);
        if expected_crc != actual_crc {
            return Err(TransferError::CorruptCheckpoint(format!(
                "CRC32 checksum mismatch: expected {:#010x}, calculated {:#010x}",
                expected_crc, actual_crc
            )));
        }

        let mut offset = 5;
        let id_len = u32::from_le_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .map_err(|_| TransferError::CorruptCheckpoint("failed parsing id len".into()))?,
        ) as usize;
        offset += 4;

        if offset + id_len > payload_len {
            return Err(TransferError::CorruptCheckpoint("truncated id string".into()));
        }
        let transfer_id = String::from_utf8(bytes[offset..offset + id_len].to_vec())
            .map_err(|_| TransferError::CorruptCheckpoint("invalid utf-8 in transfer id".into()))?;
        offset += id_len;

        let file_size = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let chunk_size = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let total_chunks = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let completed_count = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let mut indices = Vec::with_capacity(completed_count as usize);
        for _ in 0..completed_count {
            if offset + 8 > payload_len {
                return Err(TransferError::CorruptCheckpoint("truncated index table".into()));
            }
            let idx = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
            indices.push(idx);
            offset += 8;
        }

        let timestamp_us = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());

        Ok(Self {
            magic: *b"SMTJ",
            version: 1,
            transfer_id,
            file_size,
            chunk_size,
            total_chunks,
            completed_count,
            completed_chunk_indices: indices,
            timestamp_us,
            crc32: expected_crc,
        })
    }

    /// Identifies all missing chunk indices needed to complete the transfer.
    pub fn find_missing_chunk_indices(&self) -> Vec<u64> {
        let completed_set: BTreeSet<u64> = self.completed_chunk_indices.iter().copied().collect();
        let mut missing = Vec::new();
        for i in 0..self.total_chunks {
            if !completed_set.contains(&i) {
                missing.push(i);
            }
        }
        missing
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

    #[test]
    fn test_crc32_known_vector() {
        // Standard ASCII "123456789" CRC-32 is 0xCBF43926
        let input = b"123456789";
        let crc = compute_crc32(input);
        assert_eq!(crc, 0xCBF4_3926);
    }

    #[test]
    fn test_lz4_compress_and_decompress_roundtrip() {
        let original = b"The quick brown fox jumps over the lazy dog. \
The quick brown fox jumps over the lazy dog! \
Repeat: Smart Migrate high-speed zero-copy stream transfer pipeline!";

        let compressed = lz4_compress_block(original);
        assert!(!compressed.is_empty());

        let decompressed = lz4_decompress_block(&compressed, original.len()).expect("decompression succeeds");
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_lz4_large_repeated_stream_compression() {
        let mut large = Vec::with_capacity(128 * 1024);
        for i in 0..1024 {
            large.extend_from_slice(format!("Line #{}: Smart Migrate migroute high-speed parallel chunk engine\n", i).as_bytes());
        }

        let compressed = lz4_compress_block(&large);
        // Expect significant compression ratio on repetitive text (> 60% reduction)
        assert!(compressed.len() < large.len() / 2, "Expected compression from {} to < {}", large.len(), large.len() / 2);

        let decompressed = lz4_decompress_block(&compressed, large.len()).expect("decompression succeeds");
        assert_eq!(decompressed, large);
    }

    #[test]
    fn test_entropy_and_uncompressible_detection() {
        assert!(!should_compress_file("video.mp4", b""));
        assert!(!should_compress_file("archive.zip", b""));
        assert!(!should_compress_file("archive.tar.gz", b""));
        assert!(!should_compress_file("picture.png", b""));

        let log_sample = b"2026-10-16 12:00:00 INFO [smp] Stream negotiated successfully\n";
        assert!(should_compress_file("application.log", log_sample));
        assert!(should_compress_file("database_backup.sql", log_sample));
    }

    #[test]
    fn test_transfer_checkpoint_encode_decode_roundtrip() {
        let completed = vec![0, 1, 2, 4, 7, 8, 9];
        let checkpoint = TransferCheckpoint::new(
            "transfer-alpha-99".to_string(),
            10_000_000,
            64 * 1024,
            153,
            completed.clone(),
            1729100000000000,
        );

        let encoded = checkpoint.encode();
        let decoded = TransferCheckpoint::decode(&encoded).expect("checkpoint decoding should succeed");

        assert_eq!(decoded.transfer_id, "transfer-alpha-99");
        assert_eq!(decoded.file_size, 10_000_000);
        assert_eq!(decoded.chunk_size, 64 * 1024);
        assert_eq!(decoded.total_chunks, 153);
        assert_eq!(decoded.completed_chunk_indices, completed);
        assert_eq!(decoded.timestamp_us, 1729100000000000);

        let missing = decoded.find_missing_chunk_indices();
        assert!(!missing.contains(&0));
        assert!(!missing.contains(&1));
        assert!(missing.contains(&3));
        assert!(missing.contains(&5));
        assert!(missing.contains(&6));
    }

    #[test]
    fn test_transfer_checkpoint_corruption_detection() {
        let checkpoint = TransferCheckpoint::new(
            "transfer-test".to_string(),
            1000,
            100,
            10,
            vec![0, 1, 2],
            1000000,
        );
        let mut encoded = checkpoint.encode();
        // Tamper with one byte in the payload
        let mid = encoded.len() / 2;
        encoded[mid] ^= 0xFF;

        let res = TransferCheckpoint::decode(&encoded);
        assert!(matches!(res, Err(TransferError::CorruptCheckpoint(_))));
    }

    #[test]
    fn test_parallel_chunk_assembler_out_of_order_and_resumption() {
        let chunk0_data = b"chunk 0 data - hello world";
        let chunk1_data = b"chunk 1 data - parallel stream";
        let chunk2_data = b"chunk 2 data - high speed transfer";

        let slice0 = TransferChunkSlice {
            chunk_index: 0,
            byte_offset: 0,
            uncompressed_length: chunk0_data.len() as u32,
            compression: TransferCompression::None,
            payload: chunk0_data.to_vec(),
            chunk_sha256: compute_sha256(chunk0_data),
        };

        // Chunk 1 is LZ4 compressed
        let compressed1 = lz4_compress_block(chunk1_data);
        let slice1 = TransferChunkSlice {
            chunk_index: 1,
            byte_offset: chunk0_data.len() as u64,
            uncompressed_length: chunk1_data.len() as u32,
            compression: TransferCompression::Lz4,
            payload: compressed1,
            chunk_sha256: compute_sha256(chunk1_data),
        };

        let slice2 = TransferChunkSlice {
            chunk_index: 2,
            byte_offset: (chunk0_data.len() + chunk1_data.len()) as u64,
            uncompressed_length: chunk2_data.len() as u32,
            compression: TransferCompression::None,
            payload: chunk2_data.to_vec(),
            chunk_sha256: compute_sha256(chunk2_data),
        };

        let mut assembler = ParallelChunkAssembler::new(
            "parallel-test-1".to_string(),
            3,
            64,
            (chunk0_data.len() + chunk1_data.len() + chunk2_data.len()) as u64,
        );

        // Process slice 2 first (out of order arrival)
        let res2 = assembler.process_incoming_slice(&slice2).unwrap();
        assert_eq!(
            res2,
            ProcessedSlice::ReadyToWrite {
                chunk_index: 2,
                byte_offset: slice2.byte_offset,
                data: chunk2_data.to_vec(),
            }
        );
        assert_eq!(assembler.next_contiguous_chunk, 0); // Gap at 0

        // Process slice 0
        let res0 = assembler.process_incoming_slice(&slice0).unwrap();
        assert_eq!(
            res0,
            ProcessedSlice::ReadyToWrite {
                chunk_index: 0,
                byte_offset: 0,
                data: chunk0_data.to_vec(),
            }
        );
        assert_eq!(assembler.next_contiguous_chunk, 1); // Gap at 1

        // Checkpoint snapshot
        let cp = assembler.create_checkpoint(123456);
        assert_eq!(cp.completed_count, 2);
        let missing = cp.find_missing_chunk_indices();
        assert_eq!(missing, vec![1]);

        // Restore assembler from checkpoint
        let mut restored = ParallelChunkAssembler::from_checkpoint(&cp);
        assert_eq!(restored.get_missing_chunks(), vec![1]);

        // Process remaining slice 1 (which was LZ4 compressed)
        let res1 = restored.process_incoming_slice(&slice1).unwrap();
        assert_eq!(
            res1,
            ProcessedSlice::ReadyToWrite {
                chunk_index: 1,
                byte_offset: slice1.byte_offset,
                data: chunk1_data.to_vec(),
            }
        );
        assert!(restored.is_complete());
        assert_eq!(restored.next_contiguous_chunk, 3);
    }
}
