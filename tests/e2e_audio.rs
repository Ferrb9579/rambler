#[allow(dead_code)]
#[path = "../src/models.rs"]
mod models;

use anyhow::{Context, Result, bail};
use std::env;
use std::fs;
use std::path::Path;
use std::sync::mpsc;

const TARGET_RATE: u32 = 16_000;

#[test]
#[ignore = "loads roughly 1 GB of local model weights; run with RAMBLER_E2E_WAV=/path/to/file.wav"]
fn speech_pipeline() -> Result<()> {
    let path = env::var("RAMBLER_E2E_WAV")
        .context("set RAMBLER_E2E_WAV to a speech WAV file before running this test")?;
    let samples = read_wav(Path::new(&path))?;
    // Exercise the same worker/channel path used by the Dioxus button after
    // it stops recording, rather than calling the runtime directly.
    let worker = models::start_model_worker();
    let (reply_tx, reply_rx) = mpsc::channel();
    worker
        .send(models::InferenceJob {
            samples,
            reply: reply_tx,
        })
        .context("could not send audio to the model worker")?;
    let cleaned = reply_rx
        .recv()
        .context("the model worker did not reply")?
        .map_err(anyhow::Error::msg)?;

    println!("cleaned transcript: {cleaned}");
    if cleaned.trim().is_empty() {
        bail!("the local ASR/normalizer pipeline returned empty text");
    }
    Ok(())
}

fn read_wav(path: &Path) -> Result<Vec<f32>> {
    let bytes = fs::read(path).with_context(|| format!("could not read {}", path.display()))?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        bail!("{} is not a RIFF/WAVE file", path.display());
    }

    let mut offset = 12usize;
    let mut audio_format = None;
    let mut channels = None;
    let mut sample_rate = None;
    let mut bits_per_sample = None;
    let mut data = None;

    while offset.checked_add(8).is_some_and(|end| end <= bytes.len()) {
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into()?) as usize;
        offset += 8;
        let end = offset
            .checked_add(chunk_size)
            .context("WAV chunk length overflow")?;
        if end > bytes.len() {
            bail!("WAV chunk extends beyond the file");
        }

        match chunk_id {
            b"fmt " if chunk_size >= 16 => {
                audio_format = Some(u16::from_le_bytes(bytes[offset..offset + 2].try_into()?));
                channels = Some(u16::from_le_bytes(
                    bytes[offset + 2..offset + 4].try_into()?,
                ));
                sample_rate = Some(u32::from_le_bytes(
                    bytes[offset + 4..offset + 8].try_into()?,
                ));
                bits_per_sample = Some(u16::from_le_bytes(
                    bytes[offset + 14..offset + 16].try_into()?,
                ));
            }
            b"data" => data = Some(&bytes[offset..end]),
            _ => {}
        }
        offset = end + (chunk_size & 1);
    }

    let audio_format = audio_format.context("WAV is missing its fmt chunk")?;
    let channels = channels.context("WAV is missing its channel count")? as usize;
    let sample_rate = sample_rate.context("WAV is missing its sample rate")?;
    let bits_per_sample = bits_per_sample.context("WAV is missing its bit depth")?;
    let data = data.context("WAV is missing its data chunk")?;

    if audio_format != 1 || channels == 0 || bits_per_sample != 16 {
        bail!("the E2E reader expects mono/stereo 16-bit PCM WAV audio");
    }
    let frame_bytes = channels * 2;
    if data.len() < frame_bytes {
        bail!("WAV contains no complete audio frame");
    }

    let mut mono = Vec::with_capacity(data.len() / frame_bytes);
    for frame in data.chunks_exact(frame_bytes) {
        let mut sum = 0.0f32;
        for channel in 0..channels {
            let start = channel * 2;
            let value = i16::from_le_bytes(frame[start..start + 2].try_into()?);
            sum += f32::from(value) / 32_768.0;
        }
        mono.push(sum / channels as f32);
    }

    Ok(resample(&mono, sample_rate))
}

fn resample(input: &[f32], source_rate: u32) -> Vec<f32> {
    if source_rate == TARGET_RATE || input.is_empty() {
        return input.to_vec();
    }

    let output_len = ((input.len() as u64 * TARGET_RATE as u64 + source_rate as u64 / 2)
        / source_rate as u64) as usize;
    let ratio = source_rate as f64 / TARGET_RATE as f64;
    (0..output_len)
        .map(|index| {
            let position = index as f64 * ratio;
            let left = position.floor() as usize;
            let right = (left + 1).min(input.len() - 1);
            let fraction = (position - left as f64) as f32;
            input[left.min(input.len() - 1)] * (1.0 - fraction) + input[right] * fraction
        })
        .collect()
}
