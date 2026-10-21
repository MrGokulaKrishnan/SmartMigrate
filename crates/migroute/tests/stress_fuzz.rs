//! Stress and fuzz tests for the MigRoute engine.
//!
//! These tests hammer the public parsers and the datagram reassembler with
//! random and hostile input, and check invariants the engine promises:
//! no panics, no corrupted payloads, no delivery of stale frames, and
//! Windows-safe file names. Run with:
//!   cargo test -p migroute --test stress_fuzz -- --nocapture

use migroute::message::VideoCodec;
use migroute::pairing::NumericPairingCode;
use migroute::stream::{
    packetize_frame, DatagramFrameReassembler, DatagramHeader, DATAGRAM_MAGIC,
};
use migroute::transfer::{compute_sha256, sanitize_file_name};
use sha2::{Digest, Sha256};
use std::time::Instant;

/// Small deterministic xorshift PRNG so every failure is reproducible from its seed.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.max(1))
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next() % n
        }
    }
    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next() as u8).collect()
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[test]
fn datagram_header_fuzz_never_panics() {
    let mut rng = Rng::new(0xDEAD_BEEF);
    let mut accepted = 0u64;
    for _ in 0..2_000_000 {
        let len = rng.below(80) as usize;
        let mut buf = rng.bytes(len);
        if len >= 4 && rng.below(2) == 0 {
            buf[..4].copy_from_slice(&DATAGRAM_MAGIC);
        }
        if len >= 6 && rng.below(2) == 0 {
            buf[4..6].copy_from_slice(&migroute::PROTOCOL_VERSION.to_be_bytes());
        }
        if DatagramHeader::deserialize(&buf).is_ok() {
            accepted += 1;
        }
    }
    eprintln!("header fuzz: 2,000,000 random inputs, {accepted} accepted as valid headers");
}

#[test]
fn reassembler_delivers_exactly_the_intact_frames_under_loss_and_reordering() {
    let mut rng = Rng::new(42);
    let stream_id = 7u64;
    let mut re = DatagramFrameReassembler::new(stream_id);
    let mut delivered_frames = 0u64;
    let mut intact_frames = 0u64;

    for seq in 1..=3000u64 {
        let size = if rng.below(10) < 6 {
            1 + rng.below(1400) as usize
        } else {
            1 + rng.below(400_000) as usize
        };
        let payload = rng.bytes(size);
        let mut packets = packetize_frame(stream_id, seq, &payload, VideoCodec::H264, seq % 30 == 0, false, seq * 16_666);

        // Reorder packets within the frame.
        for i in (1..packets.len()).rev() {
            let j = rng.below(i as u64 + 1) as usize;
            packets.swap(i, j);
        }

        let mut lost_any = false;
        let mut delivered_this_frame = 0u64;
        for packet in packets {
            if rng.below(100) < 5 {
                lost_any = true;
                continue;
            }
            match re.ingest(&packet) {
                Ok(Some(frame)) => {
                    assert_eq!(frame.frame_seq, seq, "delivered a frame that is not the one being assembled");
                    assert_eq!(frame.payload, payload, "payload corrupted in reassembly for frame {seq}");
                    delivered_this_frame += 1;
                }
                Ok(None) => {}
                Err(e) => panic!("valid packet rejected for frame {seq}: {e:?}"),
            }
        }

        // A frame must be delivered if and only if none of its packets were lost.
        assert_eq!(
            delivered_this_frame,
            if lost_any { 0 } else { 1 },
            "frame {seq}: delivery does not match loss state (lost_any={lost_any})"
        );
        delivered_frames += delivered_this_frame;
        if !lost_any {
            intact_frames += 1;
        }
    }
    assert_eq!(delivered_frames, intact_frames);
    eprintln!("reassembly: 3000 frames, {delivered_frames} intact frames delivered, rest dropped as expected");
}

#[test]
fn reassembler_hostile_packets_never_panic() {
    let mut rng = Rng::new(0xBAD_C0FFEE);
    let mut re = DatagramFrameReassembler::new(1);
    let mut frames = 0u64;
    for _ in 0..1_000_000 {
        let chunk_index = rng.below(65_536) as u16;
        let total_chunks = rng.below(65_536) as u16;
        let frame_seq = rng.below(8);
        let header = DatagramHeader::new(1, frame_seq, chunk_index, total_chunks, 0, VideoCodec::H264, false, false);
        let mut packet = header.serialize().to_vec();
        let body_len = rng.below(200) as usize;
        let body = rng.bytes(body_len);
        packet.extend(body);
        if let Ok(Some(_)) = re.ingest(&packet) {
            frames += 1;
        }
    }
    eprintln!("hostile reassembly: 1,000,000 malformed packets, {frames} frames emitted, no panics");
}

#[test]
fn sanitize_file_name_upholds_windows_safety_under_fuzzing() {
    let alphabet: Vec<char> = "abcXYZ019 ._-/\\:*?\"<>|\0\n\t\r \u{00E9}\u{6F22}\u{1F642}".chars().chain("CONPRNAUXNULCOM1LPT1".chars()).collect();
    let mut rng = Rng::new(7);
    let mut accepted = 0u64;
    for _ in 0..300_000 {
        let n = rng.below(40) as usize;
        let s: String = (0..n).map(|_| alphabet[rng.below(alphabet.len() as u64) as usize]).collect();
        if let Ok(name) = sanitize_file_name(&s) {
            accepted += 1;
            assert!(!name.is_empty(), "accepted empty name from {s:?}");
            assert!(name != "." && name != "..", "accepted traversal name from {s:?}");
            assert!(!name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0']), "unsafe char in {name:?}");
            assert!(!name.chars().any(|c| c.is_control()), "control char in {name:?}");
            assert!(!name.ends_with('.') && !name.ends_with(' '), "trailing dot/space in {name:?}");
            let stem = name.split('.').next().unwrap_or("").trim_end_matches(' ').to_ascii_uppercase();
            let reserved = ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
                || (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT")) && stem.as_bytes()[3].is_ascii_digit());
            assert!(!reserved, "Windows reserved device name accepted: {name:?}");
        }
    }
    eprintln!("file names: 300,000 hostile inputs, {accepted} accepted, all Windows-safe");
}

#[test]
fn sha256_matches_reference_implementation() {
    let mut rng = Rng::new(99);
    for i in 0..20_000usize {
        let len = if i < 300 { i } else { rng.below(20_000) as usize };
        let data = rng.bytes(len);
        let ours = compute_sha256(&data);
        let reference = hex(&Sha256::digest(&data));
        assert_eq!(ours, reference, "SHA-256 mismatch at input length {len}");
    }
    eprintln!("sha256: 20,000 random inputs (0..20 KB) match the reference implementation");
}

#[test]
fn pairing_code_parser_accepts_only_six_digits() {
    let alphabet: Vec<char> = "0123456789 -abc\u{0661}\u{0662}\u{0663}\u{0664}\u{0665}\u{0666}".chars().collect();
    let mut rng = Rng::new(3);
    let mut accepted = 0u64;
    for _ in 0..500_000 {
        let n = rng.below(14) as usize;
        let s: String = (0..n).map(|_| alphabet[rng.below(alphabet.len() as u64) as usize]).collect();
        if let Ok(code) = NumericPairingCode::try_from_digits(&s) {
            accepted += 1;
            assert_eq!(code.raw().len(), 6);
            assert!(code.raw().chars().all(|c| c.is_ascii_digit()));
        }
    }
    eprintln!("pairing parser: 500,000 inputs, {accepted} accepted, all exactly six ASCII digits");
}

#[test]
fn throughput_of_1080p_sized_frames_stays_lossless() {
    // 600 frames of 200 KB: roughly 10 seconds of a 60 fps stream at high quality.
    let stream_id = 9u64;
    let mut rng = Rng::new(1234);
    let mut re = DatagramFrameReassembler::new(stream_id);
    let start = Instant::now();
    let mut bytes_moved = 0usize;
    let mut packets_total = 0usize;
    for seq in 1..=600u64 {
        let payload = rng.bytes(200_000);
        let packets = packetize_frame(stream_id, seq, &payload, VideoCodec::H265, true, true, seq * 16_666);
        packets_total += packets.len();
        bytes_moved += payload.len();
        let mut out = None;
        for p in &packets {
            if let Some(frame) = re.ingest(p).expect("valid packet") {
                out = Some(frame);
            }
        }
        let frame = out.expect("intact frame must be delivered");
        assert_eq!(frame.payload, payload);
    }
    let secs = start.elapsed().as_secs_f64();
    eprintln!(
        "throughput: 600 frames, {packets_total} datagrams, {:.1} MB in {:.2}s ({:.0} fps packetize+reassemble, {} build)",
        bytes_moved as f64 / 1e6,
        secs,
        600.0 / secs,
        if cfg!(debug_assertions) { "debug" } else { "release" }
    );
    assert_eq!(re.frames_completed, 600);
}
