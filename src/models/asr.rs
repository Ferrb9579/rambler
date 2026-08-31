use std::path::Path;

use anyhow::{Result, anyhow, bail};
use qwen_asr::{context::QwenCtx, transcribe};

use super::config::{ASR_LANGUAGE, ASR_MODEL_NAME, asr_cpu_threads};
use super::profiling_enabled;
use super::vocabulary::Vocabulary;

pub trait SpeechRecognizer {
    fn name(&self) -> &str;
    fn set_vocabulary(&mut self, vocabulary: &Vocabulary) -> Result<()>;
    fn transcribe(&mut self, samples: &[f32]) -> Result<String>;
}

pub fn load_recognizer(
    assets_dir: &Path,
    vocabulary: &Vocabulary,
) -> Result<Box<dyn SpeechRecognizer>> {
    match ASR_MODEL_NAME {
        name if name.starts_with("qwen3-asr-") => {
            Ok(Box::new(QwenRecognizer::load(assets_dir, vocabulary)?))
        }
        name => bail!(
            "Unsupported ASR model {name:?}. Add its SpeechRecognizer adapter in src/models/asr.rs"
        ),
    }
}

struct QwenRecognizer {
    context: QwenCtx,
}

impl QwenRecognizer {
    fn load(assets_dir: &Path, vocabulary: &Vocabulary) -> Result<Self> {
        configure_blas();
        qwen_asr::kernels::set_threads(asr_cpu_threads());
        if profiling_enabled() {
            qwen_asr::kernels::set_profile(true);
        }

        let model_dir = assets_dir.join(ASR_MODEL_NAME);
        if !model_dir.is_dir() {
            bail!(
                "ASR model {ASR_MODEL_NAME:?} is missing at {}",
                model_dir.display()
            );
        }
        let mut context = QwenCtx::load(&model_dir.to_string_lossy()).ok_or_else(|| {
            anyhow!(
                "could not load {ASR_MODEL_NAME} from {}. The directory must contain model*.safetensors, vocab.json, and merges.txt",
                model_dir.display()
            )
        })?;
        context.set_force_language(ASR_LANGUAGE).map_err(|_| {
            anyhow!("{ASR_MODEL_NAME} does not recognize the {ASR_LANGUAGE} language setting")
        })?;
        if let Some(prompt) = vocabulary.asr_prompt() {
            context
                .set_prompt(&prompt)
                .map_err(|_| anyhow!("{ASR_MODEL_NAME} could not encode the vocabulary prompt"))?;
        }

        Ok(Self { context })
    }
}

// qwen-asr divides large GEMMs across its own worker pool. Letting every one
// of those workers start another full OpenBLAS pool causes severe nested
// oversubscription on Linux, so each BLAS call must remain single-threaded.
#[cfg(all(feature = "qwen-blas", target_os = "linux"))]
fn configure_blas() {
    unsafe extern "C" {
        fn openblas_set_num_threads(num_threads: i32);
    }
    unsafe { openblas_set_num_threads(1) };
}

#[cfg(not(all(feature = "qwen-blas", target_os = "linux")))]
fn configure_blas() {}

impl SpeechRecognizer for QwenRecognizer {
    fn name(&self) -> &str {
        ASR_MODEL_NAME
    }

    fn set_vocabulary(&mut self, vocabulary: &Vocabulary) -> Result<()> {
        let prompt = vocabulary.asr_prompt().unwrap_or_default();
        self.context
            .set_prompt(&prompt)
            .map_err(|_| anyhow!("{ASR_MODEL_NAME} could not encode the vocabulary prompt"))
    }

    fn transcribe(&mut self, samples: &[f32]) -> Result<String> {
        // QwenCtx defaults to one direct full-audio pass. Rambler deliberately
        // does not run VAD, silence compaction, or custom segmentation.
        let transcript = transcribe::transcribe_audio(&mut self.context, samples)
            .ok_or_else(|| anyhow!("{ASR_MODEL_NAME} could not transcribe the audio"))?;
        if profiling_enabled() {
            qwen_asr::kernels::profile_report();
        }
        if std::env::var_os("RAMBLER_DEBUG_TRANSCRIPT").is_some() {
            eprintln!("rambler debug: {ASR_MODEL_NAME} raw = {transcript:?}");
        }
        Ok(transcript)
    }
}
