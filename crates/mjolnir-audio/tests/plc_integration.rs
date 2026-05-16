//! End-to-end integration tests for the audio PLC pipeline.
//!
//! These tests exercise the *real* Opus encoder/decoder and the real
//! `SelfHealingBuffer<Box<PlcBackend>>` composition with `OpusPlc` as
//! the backend. No cpal involvement — the buffer is driven directly
//! by handing it encoded packets in various network-failure patterns
//! and writing decoded output into a caller-owned scratch slice.
//!
//! The cases below cover the two activation paths the design doc
//! promises:
//!
//! 1. **Reorder / out-of-order arrival** — `reorder_within_window`,
//!    `single_loss_with_lookahead`, `burst_loss_partially_recovered`,
//!    `late_arrival_after_conceal_is_dropped`.
//! 2. **Drain / no transmitted packets in flight** —
//!    `buffer_drain_streams_replacement_audio`.

use bytes::Bytes;
use mjolnir_audio::codec::OpusEncoder;
use mjolnir_audio::{AudioConfig, OpusPlc, PlcBackend};
use mjolnir_media::{PullStatus, PushOutcome, SelfHealingBuffer};

const TARGET_DEPTH: usize = 2;
const CAPACITY: usize = 16;

fn fresh_buffer() -> SelfHealingBuffer<Box<PlcBackend>> {
    let cfg = AudioConfig::default();
    let plc: Box<PlcBackend> = Box::new(OpusPlc::new(&cfg).expect("plc"));
    SelfHealingBuffer::new(TARGET_DEPTH, CAPACITY, plc)
}

fn encoded_frames(count: usize) -> Vec<Bytes> {
    let cfg = AudioConfig::default();
    let mut enc = OpusEncoder::new(&cfg).expect("encoder");
    let frame_samples = cfg.frame_size() * cfg.channels as usize;
    (0..count)
        .map(|i| {
            let offset = i * frame_samples;
            let pcm: Vec<i16> = (0..frame_samples)
                .map(|j| {
                    let t = (offset + j) as f64 / cfg.sample_rate as f64;
                    (f64::sin(t * 440.0 * 2.0 * std::f64::consts::PI) * 16000.0) as i16
                })
                .collect();
            enc.encode(&pcm).expect("encode")
        })
        .collect()
}

fn frame_buf() -> Vec<i16> {
    let cfg = AudioConfig::default();
    vec![0i16; cfg.frame_size() * cfg.channels as usize]
}

#[test]
fn in_order_baseline_no_plc() {
    let frames = encoded_frames(10);
    let mut buf = fresh_buffer();
    let mut out = frame_buf();
    for (i, f) in frames.iter().enumerate() {
        buf.push(i as u64, f.clone());
    }
    for _ in 0..10 {
        let status = buf.pull(&mut out).expect("pull");
        assert_eq!(status, PullStatus::Decoded);
    }
    let stats = buf.stats();
    assert_eq!(stats.decoded, 10);
    assert_eq!(stats.concealed, 0);
    assert_eq!(stats.errors, 0);
}

#[test]
fn reorder_within_window_does_not_engage_plc() {
    // Datagram transport CAN deliver packets out of order. As long as
    // the late one arrives before its slot is pulled, the buffer
    // reorders it transparently — no PLC.
    let frames = encoded_frames(5);
    let mut buf = fresh_buffer();
    let mut out = frame_buf();
    let arrival = [0u64, 2, 4, 1, 3];
    for &seq in &arrival {
        buf.push(seq, frames[seq as usize].clone());
    }
    for _ in 0..5 {
        assert_eq!(buf.pull(&mut out).expect("pull"), PullStatus::Decoded);
    }
    let stats = buf.stats();
    assert_eq!(stats.decoded, 5);
    assert_eq!(stats.concealed, 0);
}

#[test]
fn single_loss_with_lookahead_engages_plc_with_fec_hint() {
    // The canonical PLC activation: packet 3 is lost, packet 4 has
    // arrived. The buffer sees a gap at slot 3 and hands packet 4 to
    // `decode_lost` as a recovery hint — the FEC plumbing path.
    let frames = encoded_frames(10);
    let mut buf = fresh_buffer();
    let mut out = frame_buf();
    let expected_samples = out.len();
    for (i, f) in frames.iter().enumerate() {
        if i == 3 {
            continue;
        }
        buf.push(i as u64, f.clone());
    }
    let mut decoded = 0;
    let mut concealed = 0;
    for slot in 0..10u64 {
        let status = buf.pull(&mut out).expect("pull");
        match status {
            PullStatus::Decoded => {
                decoded += 1;
                assert_eq!(out.len(), expected_samples);
            }
            PullStatus::Concealed { fec_lookahead } => {
                concealed += 1;
                assert_eq!(slot, 3, "only seq 3 should conceal");
                assert!(fec_lookahead, "seq 4 was available as lookahead");
                assert!(
                    out.iter().any(|&s| s != 0),
                    "concealed audio must not be pure silence"
                );
            }
            PullStatus::Empty => panic!("buffer should be warm by this point"),
        }
    }
    assert_eq!(decoded, 9);
    assert_eq!(concealed, 1);
    let stats = buf.stats();
    assert_eq!(stats.fec_recovered, 1, "lookahead was available at seq 3");
}

#[test]
fn burst_loss_partially_recovered_via_lookahead() {
    // Lose seq 3, 4, 5 in a row. Frames 3 and 4 conceal *without*
    // lookahead (their next-in-sequence is also missing). Frame 5
    // conceals *with* lookahead (seq 6 is present), so its concealment
    // can use FEC.
    let frames = encoded_frames(10);
    let mut buf = fresh_buffer();
    let mut out = frame_buf();
    for (i, f) in frames.iter().enumerate() {
        if (3..=5).contains(&i) {
            continue;
        }
        buf.push(i as u64, f.clone());
    }
    let mut decoded = 0;
    let mut concealed_slots: Vec<u64> = Vec::new();
    for slot in 0..10u64 {
        match buf.pull(&mut out).expect("pull") {
            PullStatus::Decoded => decoded += 1,
            PullStatus::Concealed { .. } => concealed_slots.push(slot),
            PullStatus::Empty => panic!("buffer should be warm"),
        }
    }
    assert_eq!(decoded, 7);
    assert_eq!(concealed_slots, vec![3, 4, 5]);
    let stats = buf.stats();
    assert_eq!(stats.concealed, 3);
    assert_eq!(
        stats.fec_recovered, 1,
        "only seq 5 had a lookahead (seq 6 present)"
    );
}

#[test]
fn buffer_drain_streams_replacement_audio() {
    // The "no transmitted packets in flight" case. Push a small batch,
    // then keep pulling — the buffer must continue to produce frames
    // (concealment) rather than going silent or returning Empty.
    //
    // This is the contract the doc promises: when the network goes
    // quiet, playback keeps flowing, filled with synthesised audio.
    let frames = encoded_frames(5);
    let mut buf = fresh_buffer();
    let mut out = frame_buf();
    let expected_samples = out.len();
    for (i, f) in frames.iter().enumerate() {
        buf.push(i as u64, f.clone());
    }
    let total_pulls = 20;
    let mut decoded = 0;
    let mut concealed = 0;
    for _ in 0..total_pulls {
        match buf.pull(&mut out).expect("pull") {
            PullStatus::Decoded => decoded += 1,
            PullStatus::Concealed { .. } => {
                concealed += 1;
                assert_eq!(out.len(), expected_samples);
            }
            PullStatus::Empty => panic!(
                "drain test: buffer must keep streaming, not return Empty after warmup"
            ),
        }
    }
    assert_eq!(decoded, 5, "all pushed frames decoded");
    assert_eq!(concealed, total_pulls - 5, "all subsequent slots concealed");
    let stats = buf.stats();
    assert_eq!(stats.fec_recovered, 0, "no lookaheads during drain");
}

#[test]
fn late_arrival_after_conceal_is_dropped() {
    // Seq 1 is "lost," buffer conceals at slot 1 with seq 2 as
    // lookahead, then seq 1 finally arrives — too late. It must be
    // dropped (the playout cursor has moved past it) and subsequent
    // pulls must produce the still-buffered later frames normally.
    let frames = encoded_frames(5);
    let mut buf = fresh_buffer();
    let mut out = frame_buf();
    buf.push(0, frames[0].clone());
    buf.push(2, frames[2].clone());
    buf.push(3, frames[3].clone());
    buf.push(4, frames[4].clone());

    assert_eq!(buf.pull(&mut out).expect("pull"), PullStatus::Decoded); // seq 0
    assert!(matches!(
        buf.pull(&mut out).expect("pull"),
        PullStatus::Concealed { .. }
    )); // seq 1

    // Now seq 1 arrives late.
    let outcome = buf.push(1, frames[1].clone());
    assert_eq!(outcome, PushOutcome::DroppedLate);

    // The rest play through normally.
    assert_eq!(buf.pull(&mut out).expect("pull"), PullStatus::Decoded); // seq 2
    assert_eq!(buf.pull(&mut out).expect("pull"), PullStatus::Decoded); // seq 3
    assert_eq!(buf.pull(&mut out).expect("pull"), PullStatus::Decoded); // seq 4

    let stats = buf.stats();
    assert_eq!(stats.decoded, 4);
    assert_eq!(stats.concealed, 1);
    assert_eq!(stats.fec_recovered, 1);
}
