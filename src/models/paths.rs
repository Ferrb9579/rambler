use std::path::PathBuf;

use anyhow::{Result, bail};

use super::config::{CLEANER_MODEL_FILENAME, CLEANER_TOKENIZER_FILENAME};

const MODEL_DIR: &str = "assets/models";

fn candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    // `cargo run` starts at the project root. A copied target binary can also
    // find the project assets by walking upward from its own path.
    if let Ok(executable) = std::env::current_exe() {
        for parent in executable.ancestors() {
            candidates.push(parent.join(MODEL_DIR));
        }
    }
    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join(MODEL_DIR));
    }
    candidates
}

pub fn locate_assets_dir() -> Result<PathBuf> {
    let candidates = candidates();
    if let Some(directory) = candidates.iter().find(|candidate| candidate.is_dir()) {
        return Ok(directory.clone());
    }

    let searched = candidates
        .iter()
        .map(|path| format!("  {}", path.display()))
        .collect::<Vec<_>>()
        .join("\n");
    bail!("The assets/models directory is missing. Searched:\n{searched}")
}

pub fn resolve_assets_dir() -> Result<PathBuf> {
    let candidates = candidates();

    for candidate in &candidates {
        if candidate.join(CLEANER_MODEL_FILENAME).is_file()
            && candidate.join(CLEANER_TOKENIZER_FILENAME).is_file()
        {
            return Ok(candidate.clone());
        }
    }

    let searched = candidates
        .iter()
        .map(|path| format!("  {}", path.display()))
        .collect::<Vec<_>>()
        .join("\n");
    bail!(
        "Local cleaner assets are missing. Put {CLEANER_MODEL_FILENAME} and \
         {CLEANER_TOKENIZER_FILENAME} in assets/models/. Searched:\n{searched}"
    )
}
