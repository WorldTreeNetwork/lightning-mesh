//! Audio packet loss concealment backends.
//!
//! The trait used here lives in [`mjolnir_media::Recover`] — it is the
//! media-generic decode-and-conceal seam. This module provides the
//! audio-specific impls and a convenience type alias [`PlcBackend`] for
//! `dyn Recover + Send`.
//!
//! Two backends ship in-tree:
//!
//! * [`OpusPlc`] — the CPU default. Uses Opus's built-in decoder PLC
//!   ([`OpusDecoder::decode_lost`]), which draws on recent codec state to
//!   synthesise a smooth fill frame. Microsecond-class on a modern CPU.
//!   Upgrades automatically to neural FARGAN PLC when linked against
//!   libopus 1.5+ built with `--enable-deep-plc`.
//! * [`SilencePlc`] — a baseline that emits zeros on loss. Useful as a
//!   worst-case audibility reference and in tests.
//!
//! Future backends (neural PLC on CPU via [`tract`](https://github.com/sonos/tract),
//! AIE-resident cascade) implement the same [`Recover`] trait. See
//! `docs/architecture/self-healing-jitter-buffer.md` and
//! `docs/research/audio-models-for-neural-plc/synthesis.md`.
//!
//! All backends write into a caller-provided `&mut [i16]` slice and
//! must not allocate on the inference path — see [`Recover`].

use anyhow::Result;
use mjolnir_media::Recover;
use std::sync::Arc;

use crate::codec::OpusDecoder;
use crate::AudioConfig;

/// Audio-side alias for the boxed concealment backend.
///
/// `Box<PlcBackend>` is the storage shape used throughout the audio
/// pipeline. Concrete impls (Opus, silence, tract-hosted neural model)
/// implement [`Recover`](mjolnir_media::Recover).
pub type PlcBackend = dyn Recover + Send;

/// Factory closure type used by [`Mixer`](crate::Mixer) to mint a fresh
/// per-peer backend.
pub type PlcFactory =
    Arc<dyn Fn(&AudioConfig) -> Result<Box<PlcBackend>> + Send + Sync>;

/// Default factory: one [`OpusPlc`] per peer.
pub fn default_plc_factory() -> PlcFactory {
    Arc::new(|cfg| Ok(Box::new(OpusPlc::new(cfg)?) as Box<PlcBackend>))
}

/// Factory that produces [`SilencePlc`] backends. Intended for tests and
/// dropout-audibility demos.
pub fn silence_plc_factory() -> PlcFactory {
    Arc::new(|cfg| Ok(Box::new(SilencePlc::new(cfg)?) as Box<PlcBackend>))
}

/// Opus PLC backend. The CPU default.
pub struct OpusPlc {
    decoder: OpusDecoder,
}

impl OpusPlc {
    pub fn new(config: &AudioConfig) -> Result<Self> {
        Ok(Self {
            decoder: OpusDecoder::new(config)?,
        })
    }
}

impl Recover for OpusPlc {
    fn decode(&mut self, packet: &[u8], out: &mut [i16]) -> Result<()> {
        self.decoder.decode(packet, out)?;
        Ok(())
    }

    fn decode_lost(&mut self, lookahead: Option<&[u8]>, out: &mut [i16]) -> Result<()> {
        // If we have the next packet, use Opus's in-band FEC to
        // reconstruct the lost frame; the lookahead is left in the
        // buffer and decoded normally at its own scheduled slot.
        match lookahead {
            Some(next) => {
                self.decoder.decode_fec(next, out)?;
            }
            None => {
                self.decoder.decode_lost(out)?;
            }
        }
        Ok(())
    }
}

/// Silence-on-loss baseline. Decodes real packets normally; the
/// concealment path returns zeros.
pub struct SilencePlc {
    decoder: OpusDecoder,
}

impl SilencePlc {
    pub fn new(config: &AudioConfig) -> Result<Self> {
        Ok(Self {
            decoder: OpusDecoder::new(config)?,
        })
    }
}

impl Recover for SilencePlc {
    fn decode(&mut self, packet: &[u8], out: &mut [i16]) -> Result<()> {
        self.decoder.decode(packet, out)?;
        Ok(())
    }

    fn decode_lost(&mut self, _lookahead: Option<&[u8]>, out: &mut [i16]) -> Result<()> {
        for s in out.iter_mut() {
            *s = 0;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::OpusEncoder;
    use bytes::Bytes;

    fn make_encoded(config: &AudioConfig, seed: i32) -> Bytes {
        let mut enc = OpusEncoder::new(config).expect("encoder");
        let n = config.frame_size() * config.channels as usize;
        let pcm: Vec<i16> = (0..n)
            .map(|i| ((i as i32 * 7 + seed) % 32_000) as i16)
            .collect();
        enc.encode(&pcm).expect("encode")
    }

    fn frame_buf(config: &AudioConfig) -> Vec<i16> {
        vec![0i16; config.frame_size() * config.channels as usize]
    }

    #[test]
    fn opus_plc_decodes_and_conceals_in_frame_shape() {
        let cfg = AudioConfig::default();
        let mut plc = OpusPlc::new(&cfg).expect("plc");
        let packet = make_encoded(&cfg, 7);
        let mut out = frame_buf(&cfg);
        plc.decode(&packet, &mut out).expect("decode");
        // No lookahead -> codec-native PLC.
        plc.decode_lost(None, &mut out).expect("conceal");
    }

    #[test]
    fn opus_plc_recovers_via_fec_lookahead() {
        let cfg = AudioConfig::default();
        let mut plc = OpusPlc::new(&cfg).expect("plc");
        let mut out = frame_buf(&cfg);
        // Prime the decoder with one frame so internal state is realistic.
        let p0 = make_encoded(&cfg, 1);
        plc.decode(&p0, &mut out).expect("decode");
        // Now simulate loss of seq 1 with seq 2 available as lookahead.
        let p2 = make_encoded(&cfg, 2);
        plc.decode_lost(Some(&p2), &mut out)
            .expect("fec recover");
    }

    #[test]
    fn silence_plc_emits_zeros_on_loss() {
        let cfg = AudioConfig::default();
        let mut plc = SilencePlc::new(&cfg).expect("plc");
        let mut out = vec![42i16; cfg.frame_size() * cfg.channels as usize];
        plc.decode_lost(None, &mut out).expect("conceal");
        assert!(out.iter().all(|&s| s == 0));
        // Lookahead is ignored for silence backend.
        out.fill(99);
        let dummy = make_encoded(&cfg, 9);
        plc.decode_lost(Some(&dummy), &mut out).expect("conceal");
        assert!(out.iter().all(|&s| s == 0));
    }

    #[test]
    fn trait_object_round_trip() {
        let cfg = AudioConfig::default();
        let mut backend: Box<PlcBackend> = Box::new(OpusPlc::new(&cfg).expect("plc"));
        let mut out = frame_buf(&cfg);
        let packet = make_encoded(&cfg, 3);
        backend.decode(&packet, &mut out).expect("decode via trait");
        backend
            .decode_lost(None, &mut out)
            .expect("conceal via trait");
        assert!(!backend.supports_speculation());
    }

    #[test]
    fn default_factory_produces_opus_backend() {
        let cfg = AudioConfig::default();
        let factory = default_plc_factory();
        let mut backend = factory(&cfg).expect("factory");
        let mut out = frame_buf(&cfg);
        let packet = make_encoded(&cfg, 11);
        backend.decode(&packet, &mut out).expect("decode");
    }
}
