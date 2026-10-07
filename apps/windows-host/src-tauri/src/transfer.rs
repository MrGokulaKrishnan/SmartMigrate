//! Native Windows file transfer adapter and session manager for Smart Migrate.
//!
//! Handles chunked reading and writing with SHA-256 verification, staging directory
//! isolation, atomic file finalization, and path-traversal prevention.
//!
//! # Security constraints
//!
//! - Strict adherence to `docs/THREAT-MODEL.md`:
//!   - File destination derived strictly from local policy (Downloads/SmartMigrate).
//!   - Filenames validated and sanitized via `migroute::sanitize_file_name`.
//!   - Remote input can never write outside staging/destination folders.
//!   - Incomplete transfers reside in `<staging>/<id>.part` and are purged on cancel.
//!   - Raw local filesystem paths are **never** echoed in logs or untrusted telemetry.

use migroute::transfer::{
    compute_sha256, sanitize_file_name, TransferSession, TransferState, DEFAULT_CHUNK_SIZE,
    MAX_FILE_SIZE,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// ─── Lightweight Base64 Implementation ───────────────────────────────────────

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn to_base64(data: &[u8]) -> String {
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);

        result.push(BASE64_ALPHABET[((n >> 18) & 63) as usize] as char);
        result.push(BASE64_ALPHABET[((n >> 12) & 63) as usize] as char);

        if chunk.len() > 1 {
            result.push(BASE64_ALPHABET[((n >> 6) & 63) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(BASE64_ALPHABET[(n & 63) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

pub fn from_base64(input: &str) -> Result<Vec<u8>, String> {
    fn decode_char(c: u8) -> Result<u32, String> {
        match c {
            b'A'..=b'Z' => Ok((c - b'A') as u32),
            b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
            b'+' => Ok(62),
            b'/' => Ok(63),
            b'=' => Ok(0),
            _ => Err(format!("invalid base64 character: {}", c as char)),
        }
    }

    let clean: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if clean.len() % 4 != 0 {
        return Err("invalid base64 length".to_string());
    }

    let mut result = Vec::with_capacity(clean.len() / 4 * 3);
    for quad in clean.chunks_exact(4) {
        let c0 = decode_char(quad[0])?;
        let c1 = decode_char(quad[1])?;
        let c2 = decode_char(quad[2])?;
        let c3 = decode_char(quad[3])?;

        let n = (c0 << 18) | (c1 << 12) | (c2 << 6) | c3;

        result.push(((n >> 16) & 0xFF) as u8);
        if quad[2] != b'=' {
            result.push(((n >> 8) & 0xFF) as u8);
        }
        if quad[3] != b'=' {
            result.push((n & 0xFF) as u8);
        }
    }
    Ok(result)
}

// ─── Transfer Manager & Storage Isolation ────────────────────────────────────

pub struct TransferManager {
    sessions: Mutex<HashMap<String, TransferSession>>,
    source_paths: Mutex<HashMap<String, PathBuf>>,
}

impl TransferManager {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            source_paths: Mutex::new(HashMap::new()),
        }
    }

    /// Gets the base destination folder for Smart Migrate transfers:
    /// `%USERPROFILE%/Downloads/SmartMigrate`
    pub fn get_download_dir() -> PathBuf {
        let base = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let dir = base.join("Downloads").join("SmartMigrate");
        let _ = fs::create_dir_all(&dir);
        dir
    }

    /// Gets the staging folder for temporary `.part` files:
    /// `%USERPROFILE%/Downloads/SmartMigrate/.staging`
    pub fn get_staging_dir() -> PathBuf {
        let dir = Self::get_download_dir().join(".staging");
        let _ = fs::create_dir_all(&dir);
        dir
    }

    /// Registers a new outgoing file session (Host to Client).
    pub fn register_outgoing(
        &self,
        transfer_id: String,
        source_path: PathBuf,
    ) -> Result<TransferSessionDto, String> {
        if !source_path.exists() || !source_path.is_file() {
            return Err("source file does not exist or is not a regular file".to_string());
        }

        let raw_filename = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| "invalid source file name".to_string())?;

        let sanitized = sanitize_file_name(raw_filename)
            .map_err(|e| format!("filename validation failed: {}", e))?;

        let metadata = fs::metadata(&source_path)
            .map_err(|e| format!("failed to read file metadata: {}", e))?;

        let file_size = metadata.len();
        if file_size > MAX_FILE_SIZE {
            return Err(format!(
                "file size ({} bytes) exceeds maximum 4 GiB policy limit",
                file_size
            ));
        }

        // Calculate full file SHA-256
        let mut file = File::open(&source_path)
            .map_err(|e| format!("failed to open file for hashing: {}", e))?;
        let mut full_bytes = Vec::with_capacity(file_size as usize);
        file.read_to_end(&mut full_bytes)
            .map_err(|e| format!("failed to read file: {}", e))?;

        let sha256 = compute_sha256(&full_bytes);

        let mut session = TransferSession::new_outgoing(
            transfer_id.clone(),
            &sanitized,
            file_size,
            DEFAULT_CHUNK_SIZE,
            sha256,
        )
        .map_err(|e| format!("session creation failed: {}", e))?;

        session.approve().map_err(|e| format!("failed to approve: {}", e))?;

        let dto = TransferSessionDto::from(&session);

        self.sessions.lock().unwrap().insert(transfer_id.clone(), session);
        self.source_paths.lock().unwrap().insert(transfer_id, source_path);

        Ok(dto)
    }

    /// Registers an incoming file session from client (Client to Host).
    pub fn register_incoming(
        &self,
        transfer_id: String,
        file_name: String,
        total_bytes: u64,
        chunk_size: u32,
        total_chunks: u64,
        expected_sha256: String,
    ) -> Result<TransferSessionDto, String> {
        let mut session = TransferSession::new_incoming(
            transfer_id.clone(),
            &file_name,
            total_bytes,
            chunk_size,
            total_chunks,
            expected_sha256,
        )
        .map_err(|e| format!("invalid incoming transfer request: {}", e))?;

        session.approve().map_err(|e| format!("approval failed: {}", e))?;

        // Prepare the .part staging file
        let staging_file = Self::get_staging_dir().join(format!("{}.part", transfer_id));
        let _ = File::create(&staging_file)
            .map_err(|e| format!("failed to initialize staging file: {}", e))?;

        let dto = TransferSessionDto::from(&session);
        self.sessions.lock().unwrap().insert(transfer_id, session);

        Ok(dto)
    }

    /// Reads a chunk for an outgoing transfer.
    pub fn read_outgoing_chunk(
        &self,
        transfer_id: &str,
        chunk_index: u64,
    ) -> Result<OutgoingChunkDto, String> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(transfer_id)
            .ok_or_else(|| "transfer session not found".to_string())?;

        let source_path = {
            let paths = self.source_paths.lock().unwrap();
            paths
                .get(transfer_id)
                .cloned()
                .ok_or_else(|| "source file path missing".to_string())?
        };

        if chunk_index >= session.total_chunks {
            return Err("chunk index out of bounds".to_string());
        }

        let mut file = File::open(&source_path)
            .map_err(|e| format!("failed to open source file: {}", e))?;

        let offset = chunk_index * (session.chunk_size as u64);
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| format!("seek error: {}", e))?;

        let mut buffer = vec![
            0u8;
            if chunk_index == session.total_chunks - 1 {
                let rem = (session.total_bytes % session.chunk_size as u64) as usize;
                if rem == 0 && session.total_bytes > 0 {
                    session.chunk_size as usize
                } else {
                    rem
                }
            } else {
                session.chunk_size as usize
            }
        ];

        file.read_exact(&mut buffer)
            .map_err(|e| format!("failed to read chunk: {}", e))?;

        let chunk_sha256 = compute_sha256(&buffer);
        let bytes_len = buffer.len();

        session.record_chunk_completed(chunk_index, bytes_len);

        let is_last = chunk_index + 1 == session.total_chunks;
        if is_last {
            session.state = TransferState::Completed;
        }

        Ok(OutgoingChunkDto {
            transfer_id: transfer_id.to_string(),
            chunk_index,
            chunk_sha256,
            data_base64: to_base64(&buffer),
            is_last,
            progress_percent: session.progress_percent(),
        })
    }

    /// Writes an incoming chunk from the client into the staging `.part` file.
    pub fn write_incoming_chunk(
        &self,
        transfer_id: &str,
        chunk_index: u64,
        chunk_sha256: &str,
        data_base64: &str,
    ) -> Result<TransferProgressDto, String> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(transfer_id)
            .ok_or_else(|| "transfer session not found".to_string())?;

        let data = from_base64(data_base64)
            .map_err(|e| format!("chunk base64 decode failed: {}", e))?;

        session
            .validate_chunk(chunk_index, &data, chunk_sha256)
            .map_err(|e| format!("chunk validation failed: {}", e))?;

        let staging_file = Self::get_staging_dir().join(format!("{}.part", transfer_id));
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&staging_file)
            .map_err(|e| format!("failed to open staging file: {}", e))?;

        let offset = chunk_index * (session.chunk_size as u64);
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| format!("seek error: {}", e))?;

        file.write_all(&data)
            .map_err(|e| format!("write error: {}", e))?;

        file.flush()
            .map_err(|e| format!("flush error: {}", e))?;

        session.record_chunk_completed(chunk_index, data.len());

        Ok(TransferProgressDto {
            transfer_id: transfer_id.to_string(),
            chunks_completed: session.chunks_completed,
            total_chunks: session.total_chunks,
            bytes_transferred: session.bytes_transferred,
            total_bytes: session.total_bytes,
            progress_percent: session.progress_percent(),
            state: format!("{:?}", session.state),
            next_expected_chunk: session.next_expected_chunk,
        })
    }

    /// Finalizes an incoming transfer: verifies the overall file digest and moves to destination.
    pub fn finalize_incoming(&self, transfer_id: &str) -> Result<FinalizeResultDto, String> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(transfer_id)
            .ok_or_else(|| "transfer session not found".to_string())?;

        let staging_file = Self::get_staging_dir().join(format!("{}.part", transfer_id));
        if !staging_file.exists() {
            return Err("staging file missing".to_string());
        }

        // Read and hash the entire staged file
        let mut staged_bytes = Vec::with_capacity(session.total_bytes as usize);
        let mut file = File::open(&staging_file)
            .map_err(|e| format!("failed to open staged file: {}", e))?;
        file.read_to_end(&mut staged_bytes)
            .map_err(|e| format!("failed to read staged file: {}", e))?;

        let actual_sha256 = compute_sha256(&staged_bytes);
        session
            .finalize(&actual_sha256)
            .map_err(|e| format!("final integrity check failed: {}", e))?;

        // Determine destination with collision resolution
        let dest_dir = Self::get_download_dir();
        let dest_file = get_unique_destination_path(&dest_dir, &session.file_name);

        fs::rename(&staging_file, &dest_file)
            .or_else(|_| {
                // Fallback to copy and remove if cross-filesystem rename fails
                fs::copy(&staging_file, &dest_file)
                    .and_then(|_| fs::remove_file(&staging_file))
            })
            .map_err(|e| format!("failed to move staged file to destination: {}", e))?;

        Ok(FinalizeResultDto {
            transfer_id: transfer_id.to_string(),
            file_name: session.file_name.clone(),
            total_bytes: session.total_bytes,
            sha256_verified: true,
            saved_to_folder: dest_dir.to_string_lossy().to_string(),
        })
    }

    /// Pauses an active transfer.
    pub fn pause(&self, transfer_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(transfer_id)
            .ok_or_else(|| "transfer not found".to_string())?;
        session.pause().map_err(|e| format!("{}", e))
    }

    /// Resumes a paused transfer.
    pub fn resume(&self, transfer_id: &str, from_chunk: u64) -> Result<u64, String> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(transfer_id)
            .ok_or_else(|| "transfer not found".to_string())?;
        session.resume(from_chunk).map_err(|e| format!("{}", e))?;
        Ok(session.next_expected_chunk)
    }

    /// Cancels a transfer and cleans up any partial staging files.
    pub fn cancel(&self, transfer_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(transfer_id) {
            session.cancel();
        }

        // Clean up partial file
        let staging_file = Self::get_staging_dir().join(format!("{}.part", transfer_id));
        if staging_file.exists() {
            let _ = fs::remove_file(staging_file);
        }

        Ok(())
    }

    /// Lists all current and recent transfers.
    pub fn list_transfers(&self) -> Vec<TransferSessionDto> {
        let sessions = self.sessions.lock().unwrap();
        sessions.values().map(TransferSessionDto::from).collect()
    }
}

// ─── Collision Resolution ───────────────────────────────────────────────────

fn get_unique_destination_path(dir: &Path, file_name: &str) -> PathBuf {
    let target = dir.join(file_name);
    if !target.exists() {
        return target;
    }

    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();

    for i in 1..1000 {
        let candidate = dir.join(format!("{} ({}){}", stem, i, ext));
        if !candidate.exists() {
            return candidate;
        }
    }

    dir.join(format!("{}_{}", stem, std::time::SystemTime::now().elapsed().unwrap_or_default().as_millis()))
}

// ─── DTOs for Tauri IPC ──────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TransferSessionDto {
    pub transfer_id: String,
    pub file_name: String,
    pub total_bytes: u64,
    pub chunk_size: u32,
    pub total_chunks: u64,
    pub expected_sha256: String,
    pub direction: String,
    pub state: String,
    pub bytes_transferred: u64,
    pub chunks_completed: u64,
    pub progress_percent: f32,
    pub next_expected_chunk: u64,
}

impl From<&TransferSession> for TransferSessionDto {
    fn from(s: &TransferSession) -> Self {
        Self {
            transfer_id: s.transfer_id.clone(),
            file_name: s.file_name.clone(),
            total_bytes: s.total_bytes,
            chunk_size: s.chunk_size,
            total_chunks: s.total_chunks,
            expected_sha256: s.expected_sha256.clone(),
            direction: format!("{:?}", s.direction),
            state: format!("{:?}", s.state),
            bytes_transferred: s.bytes_transferred,
            chunks_completed: s.chunks_completed,
            progress_percent: s.progress_percent(),
            next_expected_chunk: s.next_expected_chunk,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OutgoingChunkDto {
    pub transfer_id: String,
    pub chunk_index: u64,
    pub chunk_sha256: String,
    pub data_base64: String,
    pub is_last: bool,
    pub progress_percent: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgressDto {
    pub transfer_id: String,
    pub chunks_completed: u64,
    pub total_chunks: u64,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub progress_percent: f32,
    pub state: String,
    pub next_expected_chunk: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FinalizeResultDto {
    pub transfer_id: String,
    pub file_name: String,
    pub total_bytes: u64,
    pub sha256_verified: bool,
    pub saved_to_folder: String,
}
