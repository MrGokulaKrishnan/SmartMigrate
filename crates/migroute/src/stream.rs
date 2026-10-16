//! High-performance display stream protocol, datagram framing, and adaptive rate control.
//!
//! Enforces Smart Migrate Protocol (SMP/1) rules:
//! - "Host-side authorization is the source of truth for every requested capability."
//! - "No secret, device-private key, pairing token, screen frame, password, or raw user file path may reach logs."
//! - "Never represent unimplemented remote-control, screen-capture, encryption, transfer, or pairing work as functional."
//!
//! Provides:
//! - [`DatagramHeader`] / [`DatagramChunk`]: Low-latency UDP packet framing for video frames with zero head-of-line blocking.
//! - [`DatagramFrameReassembler`]: Reassembles chunks into full frames and immediately drops stale frames to preserve sub-frame latency.
//! - [`AdaptiveBitrateController`]: Dynamic bandwidth estimation, packet loss mitigation, and 120 FPS rate scaling.

use crate::message::VideoCodec;
use crate::PROTOCOL_VERSION;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 4-byte magic signature at the start of every Smart Migrate Datagram packet: "SMDT".
pub const DATAGRAM_MAGIC: [u8; 4] = [b'S', b'M', b'D', b'T'];

/// Maximum safe UDP payload size to prevent IP-level fragmentation across Wi-Fi networks (MTU ~1400 bytes).
pub const MAX_DATAGRAM_PAYLOAD_SIZE: usize = 1380;

/// Header size in bytes:
/// Magic (4) + Version (2) + StreamID (8) + FrameSeq (8) + ChunkIndex (2) + TotalChunks (2) +
/// TimestampUs (8) + CodecId (1) + Flags (1) = 36 bytes.
pub const DATAGRAM_HEADER_SIZE: usize = 36;

/// Datagram flag bits.
pub mod flags {
    pub const KEYFRAME: u8 = 1 << 0;
    pub const REFRESH_120HZ: u8 = 1 << 1;
    pub const DISCARDABLE: u8 = 1 << 2;
}

/// Binary datagram header for ultra-low-latency display streaming over UDP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatagramHeader {
    pub version: u16,
    pub stream_id: u64,
    pub frame_seq: u64,
    pub chunk_index: u16,
    pub total_chunks: u16,
    pub timestamp_us: u64,
    pub codec: VideoCodec,
    pub flags: u8,
}

impl DatagramHeader {
    pub fn new(
        stream_id: u64,
        frame_seq: u64,
        chunk_index: u16,
        total_chunks: u16,
        timestamp_us: u64,
        codec: VideoCodec,
        is_keyframe: bool,
        is_120hz: bool,
    ) -> Self {
        let mut flags = 0u8;
        if is_keyframe {
            flags |= flags::KEYFRAME;
        }
        if is_120hz {
            flags |= flags::REFRESH_120HZ;
        }

        Self {
            version: PROTOCOL_VERSION,
            stream_id,
            frame_seq,
            chunk_index,
            total_chunks,
            timestamp_us,
            codec,
            flags,
        }
    }

    /// Serializes this header into fixed 36 bytes.
    pub fn serialize(&self) -> [u8; DATAGRAM_HEADER_SIZE] {
        let mut buf = [0u8; DATAGRAM_HEADER_SIZE];
        buf[0..4].copy_from_slice(&DATAGRAM_MAGIC);
        buf[4..6].copy_from_slice(&self.version.to_be_bytes());
        buf[6..14].copy_from_slice(&self.stream_id.to_be_bytes());
        buf[14..22].copy_from_slice(&self.frame_seq.to_be_bytes());
        buf[22..24].copy_from_slice(&self.chunk_index.to_be_bytes());
        buf[24..26].copy_from_slice(&self.total_chunks.to_be_bytes());
        buf[26..34].copy_from_slice(&self.timestamp_us.to_be_bytes());

        let codec_byte = match self.codec {
            VideoCodec::H264 => 1u8,
            VideoCodec::H265 => 2u8,
            VideoCodec::Av1 => 3u8,
        };
        buf[34] = codec_byte;
        buf[35] = self.flags;

        buf
    }

    /// Deserializes a datagram header from raw slice.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, DatagramError> {
        if bytes.len() < DATAGRAM_HEADER_SIZE {
            return Err(DatagramError::HeaderTooShort(bytes.len()));
        }

        if bytes[0..4] != DATAGRAM_MAGIC {
            return Err(DatagramError::InvalidMagic);
        }

        let version = u16::from_be_bytes([bytes[4], bytes[5]]);
        if version != PROTOCOL_VERSION {
            return Err(DatagramError::UnsupportedVersion(version));
        }

        let stream_id = u64::from_be_bytes(bytes[6..14].try_into().unwrap());
        let frame_seq = u64::from_be_bytes(bytes[14..22].try_into().unwrap());
        let chunk_index = u16::from_be_bytes([bytes[22], bytes[23]]);
        let total_chunks = u16::from_be_bytes([bytes[24], bytes[25]]);
        let timestamp_us = u64::from_be_bytes(bytes[26..34].try_into().unwrap());

        if total_chunks == 0 || chunk_index >= total_chunks {
            return Err(DatagramError::InvalidChunkIndex {
                chunk_index,
                total_chunks,
            });
        }

        let codec = match bytes[34] {
            1 => VideoCodec::H264,
            2 => VideoCodec::H265,
            3 => VideoCodec::Av1,
            other => return Err(DatagramError::UnknownCodec(other)),
        };

        let flags = bytes[35];

        Ok(Self {
            version,
            stream_id,
            frame_seq,
            chunk_index,
            total_chunks,
            timestamp_us,
            codec,
            flags,
        })
    }

    pub fn is_keyframe(&self) -> bool {
        (self.flags & flags::KEYFRAME) != 0
    }

    pub fn is_120hz(&self) -> bool {
        (self.flags & flags::REFRESH_120HZ) != 0
    }
}

/// Errors occurring during datagram parsing or reassembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatagramError {
    HeaderTooShort(usize),
    InvalidMagic,
    UnsupportedVersion(u16),
    InvalidChunkIndex { chunk_index: u16, total_chunks: u16 },
    UnknownCodec(u8),
    OversizedChunk(usize),
}

impl std::fmt::Display for DatagramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HeaderTooShort(s) => write!(f, "datagram header too short ({s} bytes)"),
            Self::InvalidMagic => write!(f, "invalid datagram magic signature"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported protocol version {v}"),
            Self::InvalidChunkIndex { chunk_index, total_chunks } => {
                write!(f, "invalid chunk index {chunk_index} of {total_chunks}")
            }
            Self::UnknownCodec(c) => write!(f, "unknown video codec id {c}"),
            Self::OversizedChunk(s) => write!(f, "datagram chunk oversized ({s} bytes)"),
        }
    }
}

impl std::error::Error for DatagramError {}

/// Slices an encoded video frame into MTU-safe datagram chunks ready for zero-latency UDP transmission.
pub fn packetize_frame(
    stream_id: u64,
    frame_seq: u64,
    payload: &[u8],
    codec: VideoCodec,
    is_keyframe: bool,
    is_120hz: bool,
    timestamp_us: u64,
) -> Vec<Vec<u8>> {
    if payload.is_empty() {
        return Vec::new();
    }

    let chunk_payload_size = MAX_DATAGRAM_PAYLOAD_SIZE;
    let total_chunks = ((payload.len() + chunk_payload_size - 1) / chunk_payload_size) as u16;
    let mut packets = Vec::with_capacity(total_chunks as usize);

    for (idx, slice) in payload.chunks(chunk_payload_size).enumerate() {
        let header = DatagramHeader::new(
            stream_id,
            frame_seq,
            idx as u16,
            total_chunks,
            timestamp_us,
            codec,
            is_keyframe,
            is_120hz,
        );

        let mut packet = Vec::with_capacity(DATAGRAM_HEADER_SIZE + slice.len());
        packet.extend_from_slice(&header.serialize());
        packet.extend_from_slice(slice);
        packets.push(packet);
    }

    packets
}

/// Fully reassembled video frame delivered to the hardware decoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReassembledFrame {
    pub stream_id: u64,
    pub frame_seq: u64,
    pub codec: VideoCodec,
    pub is_keyframe: bool,
    pub is_120hz: bool,
    pub timestamp_us: u64,
    pub payload: Vec<u8>,
}

/// Zero-Head-of-Line-Blocking frame reassembler.
///
/// If packets for frame N+1 arrive while frame N is still missing chunks,
/// frame N is immediately discarded to prevent rendering stalls and keep latency < 6ms.
pub struct DatagramFrameReassembler {
    current_stream_id: u64,
    latest_frame_seq: u64,
    pending_chunks: BTreeMap<u16, Vec<u8>>,
    pending_total_chunks: u16,
    pending_header: Option<DatagramHeader>,
    pub frames_completed: u64,
    pub frames_dropped: u64,
}

impl DatagramFrameReassembler {
    pub fn new(stream_id: u64) -> Self {
        Self {
            current_stream_id: stream_id,
            latest_frame_seq: 0,
            pending_chunks: BTreeMap::new(),
            pending_total_chunks: 0,
            pending_header: None,
            frames_completed: 0,
            frames_dropped: 0,
        }
    }

    /// Ingests a raw datagram packet. If the packet completes a frame, returns `Some(ReassembledFrame)`.
    pub fn ingest(&mut self, packet_bytes: &[u8]) -> Result<Option<ReassembledFrame>, DatagramError> {
        let header = DatagramHeader::deserialize(packet_bytes)?;
        let chunk_data = &packet_bytes[DATAGRAM_HEADER_SIZE..];

        if header.stream_id != self.current_stream_id {
            // Ignore packets from unrelated streams
            return Ok(None);
        }

        // 1. Obsolete frame: older than what we've already rendered
        if header.frame_seq < self.latest_frame_seq {
            self.frames_dropped += 1;
            return Ok(None);
        }

        // 2. Newer frame has arrived before pending frame completed -> discard pending frame (Zero Head-of-Line Blocking)
        if header.frame_seq > self.latest_frame_seq {
            if !self.pending_chunks.is_empty() {
                self.frames_dropped += 1;
                self.pending_chunks.clear();
            }
            self.latest_frame_seq = header.frame_seq;
            self.pending_total_chunks = header.total_chunks;
            self.pending_header = Some(header.clone());
        }

        // Single-chunk frame fast path
        if header.total_chunks == 1 {
            self.frames_completed += 1;
            self.pending_chunks.clear();
            self.pending_header = None;

            return Ok(Some(ReassembledFrame {
                stream_id: header.stream_id,
                frame_seq: header.frame_seq,
                codec: header.codec,
                is_keyframe: header.is_keyframe(),
                is_120hz: header.is_120hz(),
                timestamp_us: header.timestamp_us,
                payload: chunk_data.to_vec(),
            }));
        }

        // Multi-chunk assembly
        self.pending_chunks.insert(header.chunk_index, chunk_data.to_vec());

        if self.pending_chunks.len() == (self.pending_total_chunks as usize) {
            // All chunks received! Assemble in order
            let mut assembled_payload = Vec::new();
            for i in 0..self.pending_total_chunks {
                if let Some(chunk) = self.pending_chunks.get(&i) {
                    assembled_payload.extend_from_slice(chunk);
                } else {
                    // Inconsistency guard
                    self.frames_dropped += 1;
                    self.pending_chunks.clear();
                    return Ok(None);
                }
            }

            let hdr = self.pending_header.take().unwrap_or(header);
            self.pending_chunks.clear();
            self.frames_completed += 1;

            Ok(Some(ReassembledFrame {
                stream_id: hdr.stream_id,
                frame_seq: hdr.frame_seq,
                codec: hdr.codec,
                is_keyframe: hdr.is_keyframe(),
                is_120hz: hdr.is_120hz(),
                timestamp_us: hdr.timestamp_us,
                payload: assembled_payload,
            }))
        } else {
            // Awaiting remaining chunks
            Ok(None)
        }
    }
}

/// Dynamic bandwidth, frame pacing, and rate controller.
#[derive(Debug, Clone)]
pub struct AdaptiveBitrateController {
    pub min_bitrate_kbps: u32,
    pub max_bitrate_kbps: u32,
    pub current_bitrate_kbps: u32,
    pub target_fps: u32,
    pub rtt_ms: f32,
    pub loss_ratio: f32,
    pub supports_120fps: bool,
}

impl AdaptiveBitrateController {
    pub fn new(supports_120fps: bool) -> Self {
        let initial_fps = if supports_120fps { 120 } else { 60 };
        let initial_bitrate = if supports_120fps { 18_000 } else { 10_000 };

        Self {
            min_bitrate_kbps: 2_500,
            max_bitrate_kbps: 45_000,
            current_bitrate_kbps: initial_bitrate,
            target_fps: initial_fps,
            rtt_ms: 1.0,
            loss_ratio: 0.0,
            supports_120fps,
        }
    }

    /// Updates network conditions and calculates the new optimal bitrate & frame rate.
    pub fn update_network_stats(&mut self, measured_rtt_ms: f32, measured_loss_ratio: f32) {
        self.rtt_ms = (self.rtt_ms * 0.7) + (measured_rtt_ms * 0.3);
        self.loss_ratio = (self.loss_ratio * 0.7) + (measured_loss_ratio * 0.3);

        let is_congested = measured_loss_ratio > 0.05
            || self.loss_ratio > 0.04
            || measured_rtt_ms > 35.0
            || self.rtt_ms > 30.0;

        if is_congested {
            // Network congested: back off bitrate by 20%
            self.current_bitrate_kbps = ((self.current_bitrate_kbps as f32 * 0.8) as u32)
                .clamp(self.min_bitrate_kbps, self.max_bitrate_kbps);

            // Step down FPS if loss is severe
            if (measured_loss_ratio > 0.10 || self.loss_ratio > 0.08) && self.target_fps > 30 {
                self.target_fps = 30;
            } else if (measured_loss_ratio > 0.05 || self.loss_ratio > 0.04) && self.target_fps > 60 {
                self.target_fps = 60;
            }
        } else if self.loss_ratio < 0.01 && self.rtt_ms < 12.0 {
            // Clean link: probe bandwidth upwards by 8%
            self.current_bitrate_kbps = ((self.current_bitrate_kbps as f32 * 1.08) as u32)
                .clamp(self.min_bitrate_kbps, self.max_bitrate_kbps);

            // Restore high frame rate if supported
            if self.supports_120fps && self.target_fps < 120 {
                self.target_fps = 120;
            } else if !self.supports_120fps && self.target_fps < 60 {
                self.target_fps = 60;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datagram_header_serialization_roundtrip() {
        let header = DatagramHeader::new(
            0x1122334455667788,
            42,
            3,
            10,
            1_700_000_000_123_456,
            VideoCodec::Av1,
            true,
            true,
        );

        let bytes = header.serialize();
        assert_eq!(bytes.len(), DATAGRAM_HEADER_SIZE);

        let parsed = DatagramHeader::deserialize(&bytes).expect("Deserialization must succeed");
        assert_eq!(parsed.version, PROTOCOL_VERSION);
        assert_eq!(parsed.stream_id, 0x1122334455667788);
        assert_eq!(parsed.frame_seq, 42);
        assert_eq!(parsed.chunk_index, 3);
        assert_eq!(parsed.total_chunks, 10);
        assert_eq!(parsed.timestamp_us, 1_700_000_000_123_456);
        assert_eq!(parsed.codec, VideoCodec::Av1);
        assert!(parsed.is_keyframe());
        assert!(parsed.is_120hz());
    }

    #[test]
    fn test_datagram_packetize_and_reassembly_single_chunk() {
        let mut reassembler = DatagramFrameReassembler::new(1001);
        let payload = b"small-frame-test-payload";

        let packets = packetize_frame(
            1001,
            1,
            payload,
            VideoCodec::H265,
            true,
            false,
            500_000,
        );
        assert_eq!(packets.len(), 1);

        let result = reassembler.ingest(&packets[0]).expect("Ingest should succeed");
        assert!(result.is_some());
        let frame = result.unwrap();
        assert_eq!(frame.stream_id, 1001);
        assert_eq!(frame.frame_seq, 1);
        assert_eq!(frame.codec, VideoCodec::H265);
        assert_eq!(&frame.payload, payload);
        assert_eq!(reassembler.frames_completed, 1);
        assert_eq!(reassembler.frames_dropped, 0);
    }

    #[test]
    fn test_datagram_reassembly_multi_chunk_out_of_order() {
        let mut reassembler = DatagramFrameReassembler::new(2002);
        // Create 3500 bytes payload (spans 3 chunks)
        let payload: Vec<u8> = (0..3500).map(|i| (i % 256) as u8).collect();

        let packets = packetize_frame(
            2002,
            10,
            &payload,
            VideoCodec::Av1,
            false,
            true,
            600_000,
        );
        assert_eq!(packets.len(), 3);

        // Ingest chunk 1, then chunk 0, then chunk 2 (out of order delivery)
        assert!(reassembler.ingest(&packets[1]).unwrap().is_none());
        assert!(reassembler.ingest(&packets[0]).unwrap().is_none());
        let final_frame = reassembler.ingest(&packets[2]).unwrap();

        assert!(final_frame.is_some());
        let frame = final_frame.unwrap();
        assert_eq!(frame.frame_seq, 10);
        assert_eq!(frame.payload.len(), 3500);
        assert_eq!(frame.payload, payload);
        assert_eq!(reassembler.frames_completed, 1);
    }

    #[test]
    fn test_stale_frame_discard_zero_head_of_line_blocking() {
        let mut reassembler = DatagramFrameReassembler::new(3003);
        let payload: Vec<u8> = vec![0xAA; 3000]; // 3 chunks

        let frame_1_packets = packetize_frame(3003, 1, &payload, VideoCodec::H264, true, false, 100);
        let frame_2_packets = packetize_frame(3003, 2, b"instant-next-frame", VideoCodec::H264, false, false, 200);

        // Ingest only chunk 0 of frame 1 (chunk 1 and 2 lost on network)
        assert!(reassembler.ingest(&frame_1_packets[0]).unwrap().is_none());

        // Frame 2 arrives: reassembler must immediately drop incomplete frame 1 and yield frame 2
        let f2 = reassembler.ingest(&frame_2_packets[0]).unwrap();
        assert!(f2.is_some());
        assert_eq!(f2.unwrap().frame_seq, 2);
        assert_eq!(reassembler.frames_completed, 1);
        assert_eq!(reassembler.frames_dropped, 1);
    }

    #[test]
    fn test_adaptive_bitrate_controller_congestion_and_recovery() {
        let mut controller = AdaptiveBitrateController::new(true);
        assert_eq!(controller.target_fps, 120);
        let initial_bitrate = controller.current_bitrate_kbps;

        // Simulate high packet loss (congestion)
        controller.update_network_stats(50.0, 0.15);
        assert!(controller.current_bitrate_kbps < initial_bitrate);
        assert!(controller.target_fps <= 60);

        // Simulate clean link (recovery)
        for _ in 0..10 {
            controller.update_network_stats(3.0, 0.0);
        }
        assert_eq!(controller.target_fps, 120);
    }
}
