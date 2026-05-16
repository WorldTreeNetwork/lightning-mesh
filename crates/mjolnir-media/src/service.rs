//! Composition of [`JitterBuffer`] with a [`Recover`] backend.
//!
//! `SelfHealingBuffer` is the "Redis-server-style" data structure
//! described in mjolnir-mesh's `docs/architecture/self-healing-jitter-buffer.md`:
//! a long-running owner of recent-encoded frames plus a warm
//! decoder/concealer that turns both delivered and missing packets into
//! a coherent stream of decoded media units. The consumer pulls at the
//! playout cadence and never sees the difference between a received
//! frame and a concealed one — but [`PullStatus`] preserves provenance
//! so cross-fade and stats are possible downstream.
//!
//! The pull surface writes decoded PCM into a caller-owned `&mut [i16]`
//! slice. The buffer never allocates on the pull path; the backend
//! ([`Recover`]) is contractually required not to allocate either.

use anyhow::Result;
use bytes::Bytes;

use crate::jitter::{JitterBuffer, Pull, PushOutcome};
use crate::recover::Recover;

/// Outcome of [`SelfHealingBuffer::pull`].
///
/// The decoded PCM is written into the caller's slice; `PullStatus`
/// carries only the metadata about *what kind of frame* was produced,
/// so the mixer can record stats and (in the future) cross-fade between
/// concealed and decoded frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullStatus {
    /// Buffer is warming up to its target depth; no samples were written.
    /// The caller's slice is left untouched.
    Empty,
    /// A received packet was decoded normally into the slice.
    Decoded,
    /// The expected packet was missing; the backend synthesised samples
    /// into the slice. `fec_lookahead` is true when the next-in-sequence
    /// packet was present and handed to the backend as a recovery hint
    /// (codecs that support in-band FEC may have used it to reconstruct
    /// the lost frame from real data rather than pure extrapolation).
    Concealed { fec_lookahead: bool },
}

impl PullStatus {
    pub fn was_concealed(&self) -> bool {
        matches!(self, PullStatus::Concealed { .. })
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, PullStatus::Empty)
    }

    /// True if the buffer produced PCM into the slice (`Decoded` or
    /// `Concealed`). False on `Empty` warm-up ticks.
    pub fn produced(&self) -> bool {
        !self.is_empty()
    }
}

/// Running counts of buffer activity. Useful for "is PLC engaging?"
/// observability without piping events out of the audio thread.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BufferStats {
    /// Packets the buffer accepted via [`SelfHealingBuffer::push`].
    /// Counts every push call regardless of [`PushOutcome`] — i.e. raw
    /// arrival count from the wire. Useful as a liveness signal even
    /// before any pulls have happened.
    pub received: u64,
    /// Frames produced from a received packet.
    pub decoded: u64,
    /// Frames produced from concealment (codec PLC or FEC).
    pub concealed: u64,
    /// Of `concealed`, the count where the buffer had a lookahead
    /// packet to hand to the backend's `decode_lost`. Whether the
    /// backend actually used FEC vs codec-PLC is opaque here — this is
    /// the *opportunity* count, not a usage guarantee.
    pub fec_recovered: u64,
    /// Backend errors during decode or conceal.
    pub errors: u64,
}

pub struct SelfHealingBuffer<R: Recover> {
    jitter: JitterBuffer<Bytes>,
    recover: R,
    stats: BufferStats,
}

impl<R: Recover> SelfHealingBuffer<R> {
    pub fn new(target_depth: usize, capacity: usize, recover: R) -> Self {
        Self {
            jitter: JitterBuffer::new(target_depth, capacity),
            recover,
            stats: BufferStats::default(),
        }
    }

    /// Insert a freshly-arrived encoded packet at sequence `seq`.
    pub fn push(&mut self, seq: u64, packet: Bytes) -> PushOutcome {
        self.stats.received += 1;
        self.jitter.push(seq, packet)
    }

    /// Pull the next decoded frame into `out`.
    ///
    /// On a [`Pull::Gap`], the buffer peeks the next-in-sequence slot
    /// (non-destructively) and hands it to the backend's
    /// [`Recover::decode_lost`] as a lookahead. Codecs supporting FEC
    /// can recover the lost frame from the next packet's FEC payload;
    /// codecs that don't ignore the hint and fall back to codec-native
    /// concealment.
    ///
    /// On [`PullStatus::Empty`] the slice is left untouched. Otherwise
    /// the backend writes exactly `out.len()` samples.
    pub fn pull(&mut self, out: &mut [i16]) -> Result<PullStatus> {
        match self.jitter.pull() {
            Pull::Frame(bytes) => match self.recover.decode(&bytes, out) {
                Ok(()) => {
                    self.stats.decoded += 1;
                    Ok(PullStatus::Decoded)
                }
                Err(e) => {
                    self.stats.errors += 1;
                    Err(e)
                }
            },
            Pull::Gap => {
                let lookahead = self.jitter.peek_next().map(|b| b.as_ref());
                let had_lookahead = lookahead.is_some();
                match self.recover.decode_lost(lookahead, out) {
                    Ok(()) => {
                        self.stats.concealed += 1;
                        if had_lookahead {
                            self.stats.fec_recovered += 1;
                        }
                        Ok(PullStatus::Concealed {
                            fec_lookahead: had_lookahead,
                        })
                    }
                    Err(e) => {
                        self.stats.errors += 1;
                        Err(e)
                    }
                }
            }
            Pull::Empty => Ok(PullStatus::Empty),
        }
    }

    pub fn stats(&self) -> BufferStats {
        self.stats
    }

    pub fn len(&self) -> usize {
        self.jitter.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jitter.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.jitter.capacity()
    }

    /// Clear all state. Stats are preserved (they're observational).
    pub fn reset(&mut self) {
        self.jitter.reset();
    }

    /// Reset stats counters to zero. Does not clear buffered packets.
    pub fn reset_stats(&mut self) {
        self.stats = BufferStats::default();
    }

    pub fn recover(&self) -> &R {
        &self.recover
    }

    pub fn recover_mut(&mut self) -> &mut R {
        &mut self.recover
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Counting {
        decoded: u32,
        concealed: u32,
        last_lookahead: Option<Vec<u8>>,
        marker: i16,
    }

    impl Counting {
        fn new() -> Self {
            Self {
                decoded: 0,
                concealed: 0,
                last_lookahead: None,
                marker: 0,
            }
        }
    }

    impl Recover for Counting {
        fn decode(&mut self, packet: &[u8], out: &mut [i16]) -> Result<()> {
            self.decoded += 1;
            self.marker = packet.first().copied().unwrap_or(0) as i16;
            for s in out.iter_mut() {
                *s = self.marker;
            }
            Ok(())
        }
        fn decode_lost(&mut self, lookahead: Option<&[u8]>, out: &mut [i16]) -> Result<()> {
            self.concealed += 1;
            self.last_lookahead = lookahead.map(|s| s.to_vec());
            for s in out.iter_mut() {
                *s = -1;
            }
            Ok(())
        }
    }

    fn pkt(b: u8) -> Bytes {
        Bytes::copy_from_slice(&[b])
    }

    #[test]
    fn warmup_returns_empty_until_target_depth() {
        let mut buf = SelfHealingBuffer::new(2, 4, Counting::new());
        let mut out = [0i16; 4];
        assert_eq!(buf.pull(&mut out).unwrap(), PullStatus::Empty);
        buf.push(0, pkt(5));
        assert_eq!(buf.pull(&mut out).unwrap(), PullStatus::Empty);
        buf.push(1, pkt(7));
        let status = buf.pull(&mut out).unwrap();
        assert_eq!(status, PullStatus::Decoded);
        assert_eq!(&out, &[5, 5, 5, 5]);
    }

    #[test]
    fn gap_routes_through_decode_lost_with_lookahead() {
        let mut buf = SelfHealingBuffer::new(1, 4, Counting::new());
        let mut out = [0i16; 4];
        buf.push(0, pkt(10));
        buf.push(2, pkt(20)); // seq 1 skipped, seq 2 sits in the ring
        assert_eq!(buf.pull(&mut out).unwrap(), PullStatus::Decoded); // seq 0
        let status = buf.pull(&mut out).unwrap();
        assert_eq!(
            status,
            PullStatus::Concealed {
                fec_lookahead: true
            }
        );
        // The lookahead handed to the backend should be the seq 2 bytes.
        assert_eq!(buf.recover().last_lookahead.as_deref(), Some(&[20u8][..]));
        // Seq 2 is still in the ring — should now decode normally.
        assert_eq!(buf.pull(&mut out).unwrap(), PullStatus::Decoded);
        assert_eq!(buf.recover().decoded, 2);
        assert_eq!(buf.recover().concealed, 1);
    }

    #[test]
    fn gap_without_lookahead_passes_none() {
        let mut buf = SelfHealingBuffer::new(1, 4, Counting::new());
        let mut out = [0i16; 4];
        buf.push(0, pkt(1));
        assert_eq!(buf.pull(&mut out).unwrap(), PullStatus::Decoded); // seq 0
        // Nothing else in the ring; pull at seq 1 → gap, no lookahead.
        let status = buf.pull(&mut out).unwrap();
        assert_eq!(
            status,
            PullStatus::Concealed {
                fec_lookahead: false
            }
        );
        assert!(buf.recover().last_lookahead.is_none());
    }

    #[test]
    fn stats_accumulate() {
        let mut buf = SelfHealingBuffer::new(1, 4, Counting::new());
        let mut out = [0i16; 4];
        buf.push(0, pkt(0));
        buf.push(2, pkt(2));
        let _ = buf.pull(&mut out); // decoded seq 0
        let _ = buf.pull(&mut out); // concealed seq 1 (with lookahead)
        let _ = buf.pull(&mut out); // decoded seq 2
        let stats = buf.stats();
        assert_eq!(stats.received, 2);
        assert_eq!(stats.decoded, 2);
        assert_eq!(stats.concealed, 1);
        assert_eq!(stats.fec_recovered, 1, "lookahead was present");
        assert_eq!(stats.errors, 0);
    }

    #[test]
    fn boxed_backend_satisfies_recover_via_blanket_impl() {
        let mut buf: SelfHealingBuffer<Box<dyn Recover>> =
            SelfHealingBuffer::new(1, 4, Box::new(Counting::new()));
        let mut out = [0i16; 4];
        buf.push(0, pkt(0));
        assert_eq!(buf.pull(&mut out).unwrap(), PullStatus::Decoded);
    }

    #[test]
    fn empty_pull_leaves_slice_untouched() {
        let mut buf = SelfHealingBuffer::new(2, 4, Counting::new());
        let mut out = [42i16; 4];
        assert_eq!(buf.pull(&mut out).unwrap(), PullStatus::Empty);
        assert_eq!(&out, &[42, 42, 42, 42], "Empty must not mutate the slice");
    }
}
