use anyhow::{Context, Result};
use moq_lite::TrackConsumer;
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::codec::OpusDecoder;
use crate::AudioConfig;

/// Subscribes to a remote audio track, decodes Opus, sends PCM to playback.
pub async fn run_subscribe(
    config: &AudioConfig,
    mut track: TrackConsumer,
    playback_tx: mpsc::Sender<Vec<i16>>,
) -> Result<()> {
    let mut decoder = OpusDecoder::new(config)?;

    info!("audio subscribe pipeline started");

    while let Some(mut group) = track.next_group().await.context("track next_group failed")? {
        while let Some(frame) = group.read_frame().await.context("group read_frame failed")? {
            let pcm = decoder.decode(&frame)?;
            let pcm_vec = pcm.to_vec();
            debug!(len = pcm_vec.len(), "decoded audio frame");
            if playback_tx.send(pcm_vec).await.is_err() {
                info!("playback channel closed, stopping subscribe");
                return Ok(());
            }
        }
    }

    info!("track finished, stopping subscribe");
    Ok(())
}
