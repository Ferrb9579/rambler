use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

const TARGET_SAMPLE_RATE: u32 = 16_000;

struct RecordingSession {
    _stream: Stream,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

thread_local! {
    static SESSION: RefCell<Option<RecordingSession>> = const { RefCell::new(None) };
}

pub fn start_recording() -> Result<(), String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "No microphone was found.".to_string())?;
    let supported = device
        .default_input_config()
        .map_err(|error| format!("Could not read the microphone settings: {error}"))?;
    let sample_format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let sample_rate = config.sample_rate;
    let channels = usize::from(config.channels);
    let samples = Arc::new(Mutex::new(Vec::with_capacity(sample_rate as usize * 30)));
    let captured = Arc::clone(&samples);
    let on_error = |error| eprintln!("microphone stream error: {error}");

    let stream = match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            config,
            move |data: &[f32], _| append_f32(&captured, data, channels),
            on_error,
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            config,
            move |data: &[i16], _| append_i16(&captured, data, channels),
            on_error,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            config,
            move |data: &[u16], _| append_u16(&captured, data, channels),
            on_error,
            None,
        ),
        other => {
            return Err(format!(
                "The microphone sample format {other:?} is not supported."
            ));
        }
    }
    .map_err(|error| format!("Could not open the microphone: {error}"))?;

    stream
        .play()
        .map_err(|error| format!("Could not start the microphone: {error}"))?;
    SESSION.with(|slot| {
        *slot.borrow_mut() = Some(RecordingSession {
            _stream: stream,
            samples,
            sample_rate,
        });
    });
    Ok(())
}

pub fn stop_recording() -> Result<Vec<f32>, String> {
    let session = SESSION
        .with(|slot| slot.borrow_mut().take())
        .ok_or_else(|| "No recording is active.".to_string())?;
    let RecordingSession {
        _stream,
        samples,
        sample_rate,
    } = session;
    drop(_stream);
    let samples = Arc::try_unwrap(samples)
        .map_err(|_| "The microphone was still finishing its last audio frame.".to_string())?
        .into_inner()
        .map_err(|_| "The recorded audio could not be read.".to_string())?;
    Ok(resample_linear(&samples, sample_rate, TARGET_SAMPLE_RATE))
}

fn append_f32(output: &Arc<Mutex<Vec<f32>>>, input: &[f32], channels: usize) {
    append_mono(
        output,
        input
            .chunks(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32),
    );
}

fn append_i16(output: &Arc<Mutex<Vec<f32>>>, input: &[i16], channels: usize) {
    append_mono(
        output,
        input.chunks(channels).map(|frame| {
            frame
                .iter()
                .map(|sample| *sample as f32 / i16::MAX as f32)
                .sum::<f32>()
                / channels as f32
        }),
    );
}

fn append_u16(output: &Arc<Mutex<Vec<f32>>>, input: &[u16], channels: usize) {
    append_mono(
        output,
        input.chunks(channels).map(|frame| {
            frame
                .iter()
                .map(|sample| *sample as f32 / 32_768.0 - 1.0)
                .sum::<f32>()
                / channels as f32
        }),
    );
}

fn append_mono(output: &Arc<Mutex<Vec<f32>>>, samples: impl Iterator<Item = f32>) {
    if let Ok(mut output) = output.lock() {
        output.extend(samples);
    }
}

fn resample_linear(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if input.is_empty() || from_rate == to_rate {
        return input.to_vec();
    }
    let output_len = input.len() * to_rate as usize / from_rate as usize;
    let ratio = from_rate as f64 / to_rate as f64;
    (0..output_len)
        .map(|index| {
            let position = index as f64 * ratio;
            let left = position.floor() as usize;
            let right = (left + 1).min(input.len() - 1);
            let fraction = (position - left as f64) as f32;
            input[left] + (input[right] - input[left]) * fraction
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resampling_preserves_duration() {
        let input = vec![0.25; 48_000];
        let output = resample_linear(&input, 48_000, 16_000);
        assert_eq!(output.len(), 16_000);
        assert!(
            output
                .iter()
                .all(|sample| (*sample - 0.25).abs() < f32::EPSILON)
        );
    }
}
