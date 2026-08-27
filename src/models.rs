use std::fs::File;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use anyhow::{Context, Result, anyhow, bail};
use candle::quantized::gguf_file;
use candle::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::quantized_qwen3::ModelWeights as Qwen3;
use tokenizers::Tokenizer;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const SYSTEM_PROMPT: &str = "You are a text normalizer for speech-to-text transcripts. The input begins with a control line specifying the styling, structure, and context settings; clean the transcript to match those settings and output only the cleaned text.";
const CONTROL_LINE: &str = "[Styling: semi-formal] [Structure: prose] [Context: general]";

const MODEL_DIR: &str = "assets/models";
const DEFAULT_WHISPER_FILENAME: &str = "ggml-base.en-q5_1.bin";
const S1_FILENAME: &str = "s1-mini-q4_k_m.gguf";
const TOKENIZER_FILENAME: &str = "s1-mini-tokenizer.json";

pub struct InferenceJob {
    pub samples: Vec<f32>,
    pub reply: Sender<Result<String, String>>,
}

pub fn start_model_worker() -> Sender<InferenceJob> {
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new()
        .name("rambler-models".to_string())
        .spawn(move || model_worker(receiver))
        .expect("failed to start model worker");
    sender
}

fn model_worker(receiver: Receiver<InferenceJob>) {
    // Load once in the background as soon as the app starts. A normal user
    // spends this time reading the UI or speaking, so the first stop click
    // does not also pay the model initialization cost.
    let mut runtime = ModelRuntime::load().map_err(format_error);
    for job in receiver {
        let result = match &mut runtime {
            Ok(runtime) => runtime.process(&job.samples).map_err(format_error),
            Err(message) => Err(message.clone()),
        };
        let _ = job.reply.send(result);
    }
}

struct ModelRuntime {
    whisper: WhisperContext,
    normalizer: Qwen3,
    tokenizer: Tokenizer,
    device: Device,
}

impl ModelRuntime {
    fn load() -> Result<Self> {
        let load_started = Instant::now();
        let whisper_filename = std::env::var("RAMBLER_WHISPER_MODEL")
            .ok()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WHISPER_FILENAME.to_string());
        let model_dir = resolve_model_dir(&whisper_filename)?;
        let whisper_path = model_dir.join(&whisper_filename);
        let s1_path = model_dir.join(S1_FILENAME);
        let tokenizer_path = model_dir.join(TOKENIZER_FILENAME);

        let whisper =
            WhisperContext::new_with_params(&whisper_path, WhisperContextParameters::default())
                .map_err(|error| {
                    anyhow!(
                        "could not load Whisper from {}: {error}",
                        whisper_path.display()
                    )
                })?;

        let device = Device::Cpu;
        let mut model = File::open(&s1_path)
            .with_context(|| format!("could not open S1-mini model at {}", s1_path.display()))?;
        let content = gguf_file::Content::read(&mut model)
            .with_context(|| format!("could not parse S1-mini model at {}", s1_path.display()))?;
        let normalizer = Qwen3::from_gguf(content, &mut model, &device)
            .with_context(|| format!("could not load S1-mini model at {}", s1_path.display()))?;
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(anyhow::Error::msg)
            .with_context(|| {
                format!(
                    "could not load S1-mini tokenizer at {}",
                    tokenizer_path.display()
                )
            })?;

        let runtime = Self {
            whisper,
            normalizer,
            tokenizer,
            device,
        };
        if std::env::var_os("RAMBLER_PROFILE").is_some() {
            eprintln!(
                "rambler profile: ASR={} model load = {:.2?}",
                whisper_filename,
                load_started.elapsed()
            );
        }
        Ok(runtime)
    }

    fn process(&mut self, samples: &[f32]) -> Result<String> {
        let process_started = Instant::now();
        let transcribe_started = Instant::now();
        let raw = self.transcribe(samples)?;
        if std::env::var_os("RAMBLER_PROFILE").is_some() {
            eprintln!(
                "rambler profile: Whisper = {:.2?} ({} samples)",
                transcribe_started.elapsed(),
                samples.len()
            );
        }
        if raw.trim().is_empty() {
            bail!("No speech was detected. Try speaking closer to the microphone.");
        }
        let normalize_started = Instant::now();
        let clean = self.normalize(raw.trim())?;
        if std::env::var_os("RAMBLER_PROFILE").is_some() {
            eprintln!(
                "rambler profile: S1-mini = {:.2?}; total = {:.2?}",
                normalize_started.elapsed(),
                process_started.elapsed()
            );
        }
        if clean.trim().is_empty() {
            bail!("Only silence or filler words were detected.");
        }
        Ok(clean.trim().to_string())
    }

    fn transcribe(&self, samples: &[f32]) -> Result<String> {
        let mut state = self
            .whisper
            .create_state()
            .map_err(|error| anyhow!("could not create Whisper state: {error}"))?;
        let sampling = if std::env::var_os("RAMBLER_WHISPER_GREEDY").is_some() {
            SamplingStrategy::Greedy { best_of: 1 }
        } else {
            SamplingStrategy::BeamSearch {
                beam_size: 5,
                patience: -1.0,
            }
        };
        let mut params = FullParams::new(sampling);
        params.set_language(Some("en"));
        params.set_translate(false);
        params.set_no_context(true);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        // Whisper is the latency-sensitive stage. Eight workers is a good
        // CPU/memory balance on desktop hardware; allow a deployment to tune
        // it without recompiling (for example, set it to 4 on a smaller
        // machine).
        let available_threads = thread::available_parallelism().map_or(4, |count| count.get());
        let threads = std::env::var("RAMBLER_WHISPER_THREADS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(8)
            .min(available_threads) as i32;
        params.set_n_threads(threads);
        state
            .full(params, samples)
            .map_err(|error| anyhow!("Whisper transcription failed: {error}"))?;
        Ok(state
            .as_iter()
            .map(|segment| segment.to_string())
            .collect::<String>())
    }

    fn normalize(&mut self, transcript: &str) -> Result<String> {
        // Exact S1-mini prefix for enable_thinking=false, per its model card.
        let prompt = format!(
            "<|im_start|>system\n{SYSTEM_PROMPT}<|im_end|>\n<|im_start|>user\n{CONTROL_LINE}\n{transcript}<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
        );
        let encoding = self
            .tokenizer
            .encode(prompt, true)
            .map_err(anyhow::Error::msg)?;
        let prompt_tokens = encoding.get_ids();
        if prompt_tokens.len() > 1_000 {
            bail!("The recording is too long to clean up in one pass.");
        }

        self.normalizer.clear_kv_cache();
        let input = Tensor::new(prompt_tokens, &self.device)?.unsqueeze(0)?;
        let logits = self.normalizer.forward(&input, 0)?.squeeze(0)?;
        let mut logits_processor = LogitsProcessor::from_sampling(0, Sampling::ArgMax);
        let mut next_token = logits_processor.sample(&logits)?;
        let mut generated = Vec::with_capacity(256);
        let eos_token = *self
            .tokenizer
            .get_vocab(true)
            .get("<|im_end|>")
            .ok_or_else(|| anyhow!("S1-mini tokenizer is missing its end token"))?;

        for index in 0..1024 {
            if next_token == eos_token {
                break;
            }
            generated.push(next_token);
            let input = Tensor::new(&[next_token], &self.device)?.unsqueeze(0)?;
            let logits = self
                .normalizer
                .forward(&input, prompt_tokens.len() + index)?
                .squeeze(0)?;
            next_token = logits_processor.sample(&logits)?;
        }

        self.tokenizer
            .decode(&generated, true)
            .map_err(anyhow::Error::msg)
    }
}

fn resolve_model_dir(whisper_filename: &str) -> Result<PathBuf> {
    let mut candidates = Vec::new();

    // `cargo run` starts in the project directory. For a copied release
    // binary, also walk its parent directories so `target/release/rambler`
    // can find the project-level assets/models directory.
    if let Ok(executable) = std::env::current_exe() {
        for parent in executable.ancestors() {
            candidates.push(parent.join(MODEL_DIR));
        }
    }
    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join(MODEL_DIR));
    }

    let required = [whisper_filename, S1_FILENAME, TOKENIZER_FILENAME];
    for candidate in &candidates {
        if required.iter().all(|name| candidate.join(name).is_file()) {
            return Ok(candidate.clone());
        }
    }

    let searched = candidates
        .iter()
        .map(|path| format!("  {}", path.display()))
        .collect::<Vec<_>>()
        .join("\n");
    bail!(
        "Local model files are missing. Put {whisper_filename}, {S1_FILENAME}, and {TOKENIZER_FILENAME} in assets/models/. Searched:\n{searched}"
    )
}

fn format_error(error: anyhow::Error) -> String {
    format!("{error:#}")
}
