//! tract-backed PLC backend.
//!
//! Loads an ONNX model via [`tract-onnx`](https://github.com/sonos/tract)
//! and implements the [`Recover`] trait against it. Always compiled in;
//! the model itself is selected per-deployment by passing a path to
//! [`TractPlc::new`].
//!
//! ## Status
//!
//! The model **load** path (parse ONNX, optimise, build runnable plan)
//! is wired and exercised by the unit tests. The model **inference**
//! path for `decode_lost` is intentionally not implemented: every
//! neural PLC model has its own input/output tensor names and recurrent
//! state-management conventions, and we don't yet have a target model
//! selected (per the H1 research finding: `tPLCnet` is the MIT-licensed
//! near-term candidate, pending TFLite→ONNX conversion).
//!
//! Until a model is selected, `decode_lost` returns an explicit error
//! so callers see "no PLC model wired" rather than silent silence.
//! `decode` always passes through to the inner Opus decoder, so this
//! backend is safe to mix into the PLC factory chain (e.g. as a fallback
//! that defers to the codec on losses) without breaking received-frame
//! decode.
//!
//! ## Why tract instead of `ort`?
//!
//! Per `docs/research/audio-models-for-neural-plc/synthesis.md` §7:
//! pure-Rust, no dynamic library, designed by Sonos for on-device
//! real-time audio DSP — friendlier to the audio inference thread's
//! allocation and latency contracts than the C++-runtime-backed `ort`.
//! Trade-off: smaller op coverage (no big transformers). For the
//! small-CNN / GRU-class neural PLC models we're targeting first
//! (tPLCnet, PARCnet-IS2-style), the coverage is sufficient.

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use mjolnir_media::Recover;
use tract_onnx::prelude::*;

use crate::codec::OpusDecoder;
use crate::AudioConfig;

/// A [`Recover`] backend that delegates concealment to an ONNX model
/// loaded via tract.
///
/// Construction loads + optimises the model; the runnable plan is
/// stashed for per-frame inference. `decode` passes through to an
/// inner [`OpusDecoder`] (received frames don't need the neural path);
/// `decode_lost` runs the model.
pub struct TractPlc {
    decoder: OpusDecoder,
    /// Compiled, optimised tract plan. Held for the lifetime of the
    /// backend so inference doesn't pay re-optimisation cost per frame.
    #[allow(dead_code)] // wired by tests; concealment impl pending model selection
    plan: TypedSimplePlan<TypedModel>,
    #[allow(dead_code)]
    frame_samples: usize,
}

impl TractPlc {
    /// Load an ONNX model from `path` and bind it against the audio
    /// config. Fails fast if the file can't be loaded, parsed, or
    /// optimised — surface errors at peer creation, not on the audio
    /// thread.
    pub fn new(model_path: impl AsRef<Path>, config: &AudioConfig) -> Result<Self> {
        let plan = load_plan(model_path.as_ref())?;
        Ok(Self {
            decoder: OpusDecoder::new(config)?,
            plan,
            frame_samples: config.frame_size() * config.channels as usize,
        })
    }
}

/// Load an ONNX file at `path`, run tract's optimiser, and compile to
/// a runnable plan.
fn load_plan(path: &Path) -> Result<TypedSimplePlan<TypedModel>> {
    tract_onnx::onnx()
        .model_for_path(path)
        .with_context(|| format!("load ONNX model from {}", path.display()))?
        .into_optimized()
        .context("tract optimisation pass")?
        .into_runnable()
        .context("tract runnable plan")
}

impl Recover for TractPlc {
    fn decode(&mut self, packet: &[u8], out: &mut [i16]) -> Result<()> {
        self.decoder.decode(packet, out)?;
        Ok(())
    }

    fn decode_lost(&mut self, _lookahead: Option<&[u8]>, _out: &mut [i16]) -> Result<()> {
        // Intentional: no model architecture committed yet. When tPLCnet
        // (or another small recurrent PLC model) is selected, fill this
        // in with: (1) feed last decoded frame into the model's input
        // tensor, (2) read predicted PCM from the model's output tensor,
        // (3) advance the recurrent hidden state held inside `self`.
        Err(anyhow!(
            "TractPlc::decode_lost: no PLC model architecture wired yet; \
             see plc_tract.rs and docs/research/audio-models-for-neural-plc/"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_model_path_errors_with_clear_context() {
        let cfg = AudioConfig::default();
        let err = TractPlc::new("/nonexistent/path/model.onnx", &cfg)
            .err()
            .expect("missing model should error");
        let full = format!("{err:#}");
        assert!(
            full.contains("load ONNX model from /nonexistent/path/model.onnx"),
            "error must point at the offending path. Got: {full}"
        );
    }

    #[test]
    fn decode_lost_errors_until_model_wired() {
        // Construct a TractPlc via the test-only model fixture so we can
        // exercise the trait impl without a real ONNX file on disk.
        let cfg = AudioConfig::default();
        let plc = TractPlc {
            decoder: OpusDecoder::new(&cfg).unwrap(),
            plan: minimal_test_plan(),
            frame_samples: cfg.frame_size() * cfg.channels as usize,
        };
        let mut plc = plc;
        let mut out = vec![0i16; cfg.frame_size() * cfg.channels as usize];
        let err = plc
            .decode_lost(None, &mut out)
            .expect_err("decode_lost is intentionally not wired yet");
        assert!(
            format!("{err}").contains("no PLC model architecture wired"),
            "error must clearly say the impl is not wired"
        );
    }

    /// Build a minimal tract plan in-memory (no ONNX file): a single
    /// input → identity → output graph. Used by tests to construct a
    /// `TractPlc` without an on-disk fixture, proving the runnable-plan
    /// pipeline is integrated correctly.
    fn minimal_test_plan() -> TypedSimplePlan<TypedModel> {
        let mut model = TypedModel::default();
        let input_fact = f32::fact([1usize]);
        let input = model
            .add_source("input", input_fact)
            .expect("add source");
        model.set_output_outlets(&[input]).expect("set output");
        model
            .into_optimized()
            .expect("optimise")
            .into_runnable()
            .expect("runnable")
    }

    #[test]
    fn minimal_plan_runs_identity() {
        // Sanity-check the test fixture itself: feeding [42.0] into the
        // identity plan returns [42.0]. Proves tract is linked and
        // functioning before we plug in a real model.
        let plan = minimal_test_plan();
        let input = tract_onnx::prelude::tract_ndarray::arr1(&[42.0f32]);
        let outputs = plan.run(tvec!(input.into_tensor().into())).expect("run");
        let out = outputs[0]
            .to_array_view::<f32>()
            .expect("output is f32 array");
        assert_eq!(out[[0]], 42.0);
    }
}
