mod asr;
mod cleaner;
mod config;
mod paths;
mod vocabulary;

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use anyhow::{Result, bail};

use self::asr::{SpeechRecognizer, load_recognizer};
use self::cleaner::S1Cleaner;
use self::paths::{locate_assets_dir, resolve_assets_dir};
use self::vocabulary::Vocabulary;

pub use self::config::{ASR_MODEL_NAME, CLEANER_MODEL_NAME};

pub fn assets_dir() -> Result<PathBuf> {
    locate_assets_dir()
}

pub fn load_vocabulary_source() -> Result<String> {
    Vocabulary::load_source(&locate_assets_dir()?)
}

pub fn save_vocabulary_source(contents: &str) -> Result<()> {
    Vocabulary::save_source(&locate_assets_dir()?, contents)
}

pub struct InferenceJob {
    pub samples: Vec<f32>,
    pub reply: Sender<Result<String, String>>,
}

/// Starts the single long-lived worker used by both the UI and end-to-end tests.
/// Models are loaded once on that thread and reused for every recording.
pub fn start_model_worker() -> Sender<InferenceJob> {
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new()
        .name("rambler-models".to_owned())
        .spawn(move || model_worker(receiver))
        .expect("failed to start model worker");
    sender
}

fn model_worker(receiver: Receiver<InferenceJob>) {
    let mut pipeline = ModelPipeline::load().map_err(format_error);
    for job in receiver {
        let result = match &mut pipeline {
            Ok(pipeline) => pipeline.run(&job.samples).map_err(format_error),
            Err(message) => Err(message.clone()),
        };
        let _ = job.reply.send(result);
    }
}

/// The complete application pipeline: audio -> ASR -> cleaner -> output.
struct ModelPipeline {
    assets_dir: PathBuf,
    recognizer: Box<dyn SpeechRecognizer>,
    cleaner: S1Cleaner,
}

impl ModelPipeline {
    fn load() -> Result<Self> {
        let started = Instant::now();
        let assets_dir = resolve_assets_dir()?;
        let vocabulary = Vocabulary::load(&assets_dir)?;
        let recognizer = load_recognizer(&assets_dir, &vocabulary)?;
        let cleaner = S1Cleaner::load(&assets_dir, vocabulary)?;

        let pipeline = Self {
            assets_dir,
            recognizer,
            cleaner,
        };
        if profiling_enabled() {
            eprintln!(
                "rambler profile: {} + {} model load = {:.2?}",
                pipeline.recognizer.name(),
                pipeline.cleaner.name(),
                started.elapsed()
            );
        }
        Ok(pipeline)
    }

    fn run(&mut self, samples: &[f32]) -> Result<String> {
        let pipeline_started = Instant::now();

        let vocabulary = Vocabulary::load(&self.assets_dir)?;
        self.recognizer.set_vocabulary(&vocabulary)?;
        self.cleaner.set_vocabulary(vocabulary);

        let asr_started = Instant::now();
        let transcript = self.recognizer.transcribe(samples)?;
        if profiling_enabled() {
            eprintln!(
                "rambler profile: {} ASR = {:.2?} ({} samples)",
                self.recognizer.name(),
                asr_started.elapsed(),
                samples.len()
            );
        }
        let transcript = transcript.trim();
        if transcript.is_empty() {
            bail!("No speech was detected. Try speaking closer to the microphone.");
        }

        let cleaner_started = Instant::now();
        let output = self.cleaner.clean(transcript)?;
        if profiling_enabled() {
            eprintln!(
                "rambler profile: {} cleaner = {:.2?}; total = {:.2?}",
                self.cleaner.name(),
                cleaner_started.elapsed(),
                pipeline_started.elapsed()
            );
        }
        let output = output.trim();
        if output.is_empty() {
            bail!("The cleaner returned no text.");
        }
        Ok(output.to_owned())
    }
}

pub(crate) fn profiling_enabled() -> bool {
    std::env::var_os("RAMBLER_PROFILE").is_some()
}

fn format_error(error: anyhow::Error) -> String {
    format!("{error:#}")
}
