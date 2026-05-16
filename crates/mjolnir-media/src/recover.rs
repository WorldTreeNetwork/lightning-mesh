//! Decode-and-conceal seam for the audio PLC pipeline.
//!
//! `Recover` turns received wire bytes into decoded PCM *and* synthesises
//! a fill frame when a packet is missing. Both responsibilities live on
//! one trait because codec-native PLC (Opus's `decode(None, ..)`) depends
//! on state that the same backend's `decode` populates; splitting them
//! would force expensive state mirroring.
//!
//! Output is written into a caller-owned `&mut [i16]` slice — backends
//! must not allocate on the inference path. The slice is sized for one
//! frame of PCM at the configured sample rate × channels × frame duration.

use anyhow::Result;

pub trait Recover: Send {
    /// Decode a freshly-arrived encoded packet, writing PCM into `out`.
    ///
    /// `out.len()` is the per-frame sample count (e.g. 960 at 48 kHz mono,
    /// 20 ms). Backends must fill exactly that many samples; underfill or
    /// overfill is a bug.
    fn decode(&mut self, packet: &[u8], out: &mut [i16]) -> Result<()>;

    /// Synthesise output for a missing packet into `out`.
    ///
    /// `lookahead`, when present, is the next-in-sequence packet that has
    /// already arrived. Codecs supporting forward error correction
    /// (Opus's in-band FEC, redundant video slices) can decode the lost
    /// frame from the lookahead's FEC payload. Backends that don't
    /// support FEC should ignore the hint and fall back to codec-native
    /// concealment.
    ///
    /// The hint is non-destructive: the lookahead packet is left in the
    /// buffer and will be returned by the next `decode` call.
    fn decode_lost(&mut self, lookahead: Option<&[u8]>, out: &mut [i16]) -> Result<()>;

    /// Whether this backend benefits from pre-emptive prediction.
    ///
    /// Backends that can predict for free (e.g. NPU-resident cascades
    /// running every cycle anyway) return `true`. The service may then
    /// speculate ahead, discarding the prediction on successful arrival.
    fn supports_speculation(&self) -> bool {
        false
    }
}

/// Blanket impl so `Box<dyn Recover>` itself satisfies `Recover`,
/// which lets [`SelfHealingBuffer`](crate::SelfHealingBuffer) be
/// parameterised over a boxed trait object without an extra wrapper.
impl<R: ?Sized + Recover> Recover for Box<R> {
    fn decode(&mut self, packet: &[u8], out: &mut [i16]) -> Result<()> {
        (**self).decode(packet, out)
    }

    fn decode_lost(&mut self, lookahead: Option<&[u8]>, out: &mut [i16]) -> Result<()> {
        (**self).decode_lost(lookahead, out)
    }

    fn supports_speculation(&self) -> bool {
        (**self).supports_speculation()
    }
}
