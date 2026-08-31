use anyhow::{Context, Result, bail};
use rambler::models;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;

const TARGET_RATE: u32 = 16_000;

#[test]
#[ignore = "loads local model weights; run with RAMBLER_E2E_WAV=/path/to/file.wav"]
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

#[test]
#[ignore = "loads local model weights; set RAMBLER_NAME_WAV to a name-and-spelling WAV file"]
fn name_spelling_pipeline() -> Result<()> {
    let path = env::var("RAMBLER_NAME_WAV").unwrap_or_else(|_| "/tmp/rambler-name.wav".to_string());
    let cleaned = submit(&models::start_model_worker(), read_wav(Path::new(&path))?)
        .map_err(anyhow::Error::msg)?;
    println!("name spelling transcript: {cleaned}");
    if !cleaned
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|word| word.eq_ignore_ascii_case("Fanisus"))
    {
        bail!("the LLM did not emit the canonical vocabulary word Fanisus: {cleaned}");
    }
    Ok(())
}

/// Run several ways a person may say a name (plain, hyphenated spelling,
/// letter names, different rates, and a later conversational mention) through
/// one warmed worker. This keeps the regression check close to the real UI
/// path while avoiding a model reload for every recording.
#[test]
#[ignore = "loads local model weights; set RAMBLER_NAME_VARIANTS_DIR to generated WAV fixtures"]
fn name_variant_matrix() -> Result<()> {
    let directory = env::var("RAMBLER_NAME_VARIANTS_DIR")
        .unwrap_or_else(|_| "/tmp/rambler-name-variants".to_string());
    let directory = Path::new(&directory);
    let cases = [
        ("01_plain_phonetic.wav", "plain phonetic name"),
        ("02_plain_canonical.wav", "canonical name"),
        ("03_spelled_hyphen.wav", "hyphenated spelling"),
        ("04_spelled_spaces.wav", "spaced spelling"),
        ("05_spelled_phonetic.wav", "phonetic letters"),
        ("06_fast.wav", "fast spelling"),
        ("07_slow.wav", "slow spelling"),
        ("08_marker_omitted.wav", "omitted marker"),
        ("09_contact.wav", "later contact mention"),
        ("10_full.wav", "full conversation"),
    ];

    let worker = models::start_model_worker();
    println!("name_columns=file|variant|duration_s|latency_ms|status|transcript");
    for (file, variant) in cases {
        let path = directory.join(file);
        let samples = read_wav(&path)?;
        let duration_s = samples.len() as f64 / f64::from(TARGET_RATE);
        let started = Instant::now();
        let transcript = submit(&worker, samples).map_err(anyhow::Error::msg)?;
        let latency_ms = started.elapsed().as_secs_f64() * 1_000.0;
        println!(
            "name|{file}|{variant}|{duration_s:.2}|{latency_ms:.2}|ok|{}",
            transcript.replace('|', "\\|")
        );
        if !transcript
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|word| word.eq_ignore_ascii_case("Fanisus"))
        {
            bail!("{variant} did not resolve to Fanisus: {transcript}");
        }
    }
    Ok(())
}

/// Run a broad local-audio matrix through the exact worker/channel path used by
/// the desktop record button. The fixture directory is intentionally supplied
/// by the caller so the application does not need a test-data dependency.
#[test]
#[ignore = "loads local model weights; set RAMBLER_E2E_MATRIX_DIR to the generated WAV fixtures"]
fn speech_matrix() -> Result<()> {
    let directory = env::var("RAMBLER_E2E_MATRIX_DIR")
        .unwrap_or_else(|_| "/tmp/rambler-e2e-matrix".to_string());
    let directory = Path::new(&directory);
    let cases = [
        MatrixCase {
            id: "01",
            name: "short",
            file: "01_short.wav",
            expected: "Hello, world.",
            expects_error: false,
        },
        MatrixCase {
            id: "02",
            name: "medium",
            file: "02_medium.wav",
            expected: "Please send the revised project plan to the team before lunch.",
            expects_error: false,
        },
        MatrixCase {
            id: "03",
            name: "long context",
            file: "03_long.wav",
            expected: "Today I want to test a complete dictation workflow from microphone capture to transcription and cleanup. The application should preserve the meaning of every sentence, add useful punctuation, and produce text that is easy to copy into another document. This longer example gives the recognizer more context and more opportunities to make a mistake.",
            expects_error: false,
        },
        MatrixCase {
            id: "04",
            name: "punctuation",
            file: "04_punctuation.wav",
            expected: "First, pack the charger; second, lock the door; finally, call me when you arrive.",
            expects_error: false,
        },
        MatrixCase {
            id: "05",
            name: "numbers and currency",
            file: "05_numbers.wav",
            // S1-mini is expected to normalize spoken quantities/currency.
            expected: "The order contains 42 notebooks, 3 blue pens, and $120.",
            expects_error: false,
        },
        MatrixCase {
            id: "06",
            name: "date and time",
            file: "06_date_time.wav",
            // S1-mini is expected to normalize the date and time.
            expected: "Let's meet on Wednesday, October 21, at 9:45 in the morning.",
            expects_error: false,
        },
        MatrixCase {
            id: "07",
            name: "spoken correction",
            file: "07_correction.wav",
            // The earlier quantity is intentionally superseded by the spoken correction.
            expected: "Book a table for six people, at 7:30.",
            expects_error: false,
        },
        MatrixCase {
            id: "08",
            name: "email and URL",
            file: "08_email.wav",
            // S1-mini is expected to join spoken email/URL components.
            expected: "My email address is fanisus@example.com, and the website is example.org.",
            expects_error: false,
        },
        MatrixCase {
            id: "09",
            name: "proper names",
            file: "09_names.wav",
            expected: "Ada Lovelace and Alan Turing discussed the analytical engine in London.",
            expects_error: false,
        },
        MatrixCase {
            id: "10",
            name: "technical vocabulary",
            file: "10_rust.wav",
            expected: "Run cargo test release with four build jobs, then inspect the application logs.",
            expects_error: false,
        },
        MatrixCase {
            id: "11",
            name: "fast speech",
            file: "11_fast.wav",
            expected: "The quick brown fox jumps over the lazy dog near the river.",
            expects_error: false,
        },
        MatrixCase {
            id: "12",
            name: "long-form dictation",
            file: "12_long_form.wav",
            expected: "When we ship a local voice tool, reliability matters more than flashy features. The recorder should start immediately, stop cleanly, and never lose the captured samples. The recognizer should be fast enough for everyday notes, while the cleanup model should correct grammar without inventing details. Finally, the finished sentence should be visible, selectable, and ready for the clipboard.",
            expects_error: false,
        },
    ];

    let worker = models::start_model_worker();
    // Pay the asynchronous model startup cost before timing the warm matrix.
    // This keeps the table useful for comparing audio length and recognition
    // latency rather than mixing model initialization into the first row.
    let warmup_started = Instant::now();
    let warmup = submit(&worker, read_wav(&directory.join(cases[0].file))?);
    let warmup_ms = warmup_started.elapsed().as_secs_f64() * 1_000.0;
    warmup.map_err(|error| anyhow::anyhow!("warmup request failed: {error}"))?;

    println!("matrix_warmup_ms={warmup_ms:.2}");
    println!("matrix_columns=id|name|duration_s|latency_ms|rtf|wer|status|transcript");
    for case in cases {
        let samples = read_wav(&directory.join(case.file))?;
        let duration_s = samples.len() as f64 / f64::from(TARGET_RATE);
        let started = Instant::now();
        let result = submit(&worker, samples);
        let latency_ms = started.elapsed().as_secs_f64() * 1_000.0;
        let rtf = latency_ms / 1_000.0 / duration_s.max(0.001);

        match (case.expects_error, result) {
            (false, Ok(transcript)) => {
                let wer = word_error_rate(case.expected, &transcript);
                println!(
                    "matrix|{}|{}|{duration_s:.2}|{latency_ms:.2}|{rtf:.3}|{wer:.3}|ok|{}",
                    case.id,
                    case.name,
                    transcript.replace('|', "\\|")
                );
            }
            (true, Err(error)) => {
                println!(
                    "matrix|{}|{}|{duration_s:.2}|{latency_ms:.2}|{rtf:.3}|n/a|expected-error|{}",
                    case.id,
                    case.name,
                    error.replace('|', "\\|")
                );
            }
            (false, Err(error)) => bail!("case {} ({}) failed: {error}", case.id, case.name),
            (true, Ok(transcript)) => {
                bail!(
                    "case {} ({}) unexpectedly succeeded with: {transcript}",
                    case.id,
                    case.name
                )
            }
        }
    }

    Ok(())
}

struct MatrixCase {
    id: &'static str,
    name: &'static str,
    file: &'static str,
    expected: &'static str,
    expects_error: bool,
}

fn submit(
    worker: &mpsc::Sender<models::InferenceJob>,
    samples: Vec<f32>,
) -> Result<String, String> {
    let (reply_tx, reply_rx) = mpsc::channel();
    worker
        .send(models::InferenceJob {
            samples,
            reply: reply_tx,
        })
        .map_err(|error| format!("could not send audio to the model worker: {error}"))?;
    reply_rx
        .recv()
        .map_err(|error| format!("the model worker did not reply: {error}"))?
}

fn word_error_rate(expected: &str, actual: &str) -> f64 {
    let expected = score_words(expected);
    let actual = score_words(actual);
    if expected.is_empty() {
        return f64::from(!actual.is_empty());
    }

    let mut previous: Vec<usize> = (0..=actual.len()).collect();
    for (row, expected_word) in expected.iter().enumerate() {
        let mut current = vec![row + 1; actual.len() + 1];
        for (column, actual_word) in actual.iter().enumerate() {
            let substitution = previous[column] + usize::from(expected_word != actual_word);
            let insertion = current[column] + 1;
            let deletion = previous[column + 1] + 1;
            current[column + 1] = substitution.min(insertion).min(deletion);
        }
        previous = current;
    }
    previous[actual.len()] as f64 / expected.len() as f64
}

fn score_words(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| word.to_ascii_lowercase())
        .collect()
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
