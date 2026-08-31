/// Change this one value to select the local ASR checkpoint.
///
/// Qwen3-ASR checkpoints share the Qwen adapter. A model family with a
/// different runtime (such as ZipFormer or Parakeet) needs one implementation
/// of `SpeechRecognizer` and one factory arm in `asr.rs`.
pub const ASR_MODEL_NAME: &str = "qwen3-asr-0.6b";

pub const CLEANER_MODEL_NAME: &str = "s1-mini-q4_k_m";
pub const CLEANER_MODEL_FILENAME: &str = "s1-mini-q4_k_m.gguf";
pub const CLEANER_TOKENIZER_FILENAME: &str = "s1-mini-tokenizer.json";
pub const VOCABULARY_FILENAME: &str = "personal_dictionary.tsv";

pub const ASR_LANGUAGE: &str = "English";

// qwen-asr supports at most 16 kernel workers. This laptop has 16 physical
// cores, so use them all while still adapting safely to smaller CPUs.
pub const ASR_CPU_THREADS: usize = 16;

pub fn asr_cpu_threads() -> usize {
    std::thread::available_parallelism()
        .map_or(1, |count| count.get())
        .min(ASR_CPU_THREADS)
}
