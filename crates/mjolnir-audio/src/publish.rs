use anyhow::{Context, Result};
use moq_lite::{BroadcastProducer, Track};
use tracing::{debug, info};

use crate::capture::AudioCapture;
use crate::codec::OpusEncoder;
use crate::AudioConfig;

/// Track name used for audio publishing.
pub const AUDIO_TRACK_NAME: &str = "audio";

/// Captures audio from mic, encodes to Opus, publishes as moq-lite track frames.
pub async fn run_publish(
    config: &AudioConfig,
    broadcast: &mut BroadcastProducer,
) -> Result<()> {
    let mut track = broadcast
        .create_track(Track::new(AUDIO_TRACK_NAME))
        .context("failed to create audio track")?;

    let mut encoder = OpusEncoder::new(config)?;

    let (_capture, mut rx) = AudioCapture::start(config)?;
    info!("audio publish pipeline started");

    while let Some(pcm) = rx.recv().await {
        let opus_bytes = encoder.encode(&pcm)?;
        track
            .write_frame(opus_bytes)
            .context("failed to write frame to track")?;
        debug!(len = pcm.len(), "published audio frame");
    }

    info!("audio capture channel closed, stopping publish");
    Ok(())
}
