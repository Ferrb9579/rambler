use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use anyhow::{Context, Result, bail};

use rambler::models::ASR_MODEL_NAME;

const QWEN_REPOSITORY: &str = "Qwen/Qwen3-ASR-0.6B";
const CLEANER_GGUF_REPOSITORY: &str = "superwhisper/s1-mini-GGUF";
const CLEANER_REPOSITORY: &str = "superwhisper/s1-mini";

const QWEN_FILES: &[&str] = &[
    "config.json",
    "generation_config.json",
    "merges.txt",
    "model.safetensors",
    "preprocessor_config.json",
    "tokenizer_config.json",
    "vocab.json",
];

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ModelStatus {
    pub(crate) installed_files: usize,
    pub(crate) total_files: usize,
}

impl Default for ModelStatus {
    fn default() -> Self {
        Self {
            installed_files: 0,
            total_files: 9,
        }
    }
}

impl ModelStatus {
    pub(crate) fn installed(&self) -> bool {
        self.installed_files == self.total_files
    }
}

#[derive(Debug)]
pub(crate) enum DownloadEvent {
    Progress(String),
    Complete,
    Error(String),
}

struct DownloadFile {
    repository: &'static str,
    source_name: &'static str,
    destination: PathBuf,
}

pub(crate) fn status() -> Result<ModelStatus> {
    let files = manifest()?;
    let installed_files = files
        .iter()
        .filter(|file| valid_file(&file.destination))
        .count();
    Ok(ModelStatus {
        installed_files,
        total_files: files.len(),
    })
}

pub(crate) fn start() -> Receiver<DownloadEvent> {
    let (events, receiver) = mpsc::channel();
    std::thread::Builder::new()
        .name("model-download".to_string())
        .spawn(move || {
            if let Err(error) = download_missing(&events) {
                let _ = events.send(DownloadEvent::Error(format!("{error:#}")));
            }
        })
        .expect("failed to start model downloader");
    receiver
}

fn manifest() -> Result<Vec<DownloadFile>> {
    if ASR_MODEL_NAME != "qwen3-asr-0.6b" {
        bail!("No download manifest is defined for {ASR_MODEL_NAME}");
    }

    let assets = rambler::models::assets_dir()?;
    let asr_directory = assets.join(ASR_MODEL_NAME);
    let mut files = QWEN_FILES
        .iter()
        .map(|name| DownloadFile {
            repository: QWEN_REPOSITORY,
            source_name: name,
            destination: asr_directory.join(name),
        })
        .collect::<Vec<_>>();
    files.push(DownloadFile {
        repository: CLEANER_GGUF_REPOSITORY,
        source_name: "s1-mini-q4_k_m.gguf",
        destination: assets.join("s1-mini-q4_k_m.gguf"),
    });
    files.push(DownloadFile {
        repository: CLEANER_REPOSITORY,
        source_name: "tokenizer.json",
        destination: assets.join("s1-mini-tokenizer.json"),
    });
    Ok(files)
}

fn download_missing(events: &mpsc::Sender<DownloadEvent>) -> Result<()> {
    let files = manifest()?;
    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("rambler/{}", env!("CARGO_PKG_VERSION")))
        .build()?;

    for file in files.iter().filter(|file| !valid_file(&file.destination)) {
        if let Some(parent) = file.destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let _ = events.send(DownloadEvent::Progress(format!(
            "Downloading {}",
            file.source_name
        )));
        let url = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            file.repository, file.source_name
        );
        let mut response = client
            .get(url)
            .send()
            .with_context(|| format!("could not download {}", file.source_name))?
            .error_for_status()
            .with_context(|| format!("download failed for {}", file.source_name))?;
        let temporary = file.destination.with_extension("rambler-download");
        let mut output = File::create(&temporary)?;
        if let Err(error) = io::copy(&mut response, &mut output) {
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }
        fs::rename(&temporary, &file.destination)?;
    }

    let _ = events.send(DownloadEvent::Complete);
    Ok(())
}

fn valid_file(path: &Path) -> bool {
    path.metadata().is_ok_and(|metadata| metadata.len() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_model_has_a_complete_download_manifest() {
        let manifest = manifest().unwrap();
        assert_eq!(manifest.len(), 9);
        assert!(
            manifest
                .iter()
                .any(|file| file.source_name == "model.safetensors")
        );
        assert!(
            manifest
                .iter()
                .any(|file| file.source_name == "s1-mini-q4_k_m.gguf")
        );
    }
}
