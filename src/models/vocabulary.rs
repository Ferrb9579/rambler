use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::config::VOCABULARY_FILENAME;

#[derive(Clone, Debug, Default)]
pub struct Vocabulary {
    entries: Vec<String>,
}

impl Vocabulary {
    pub fn load(assets_dir: &Path) -> Result<Self> {
        let path = assets_dir.join(VOCABULARY_FILENAME);
        if !path.is_file() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("could not read vocabulary at {}", path.display()))?;
        let mut entries = Vec::new();
        for (line_number, line) in contents.lines().enumerate() {
            let entry = line.trim();
            if entry.is_empty() || entry.starts_with('#') {
                continue;
            }
            if !valid_entry(entry) {
                eprintln!(
                    "warning: ignoring invalid vocabulary entry on line {}",
                    line_number + 1
                );
                continue;
            }
            if entries
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(entry))
            {
                continue;
            }
            entries.push(entry.to_owned());
        }
        Ok(Self { entries })
    }

    pub fn asr_prompt(&self) -> Option<String> {
        (!self.entries.is_empty()).then(|| {
            format!(
                "Canonical vocabulary (use these spellings only when heard): {}",
                self.entries.join(", ")
            )
        })
    }

    pub fn cleaner_control(&self) -> Option<String> {
        (!self.entries.is_empty()).then(|| format!("[Vocabulary: {}]", self.entries.join(", ")))
    }

    pub fn load_source(assets_dir: &Path) -> Result<String> {
        let path = assets_dir.join(VOCABULARY_FILENAME);
        if path.is_file() {
            fs::read_to_string(&path)
                .with_context(|| format!("could not read vocabulary at {}", path.display()))
        } else {
            Ok(String::new())
        }
    }

    pub fn save_source(assets_dir: &Path, contents: &str) -> Result<()> {
        for (line_number, line) in contents.lines().enumerate() {
            let entry = line.trim();
            if !entry.is_empty() && !entry.starts_with('#') && !valid_entry(entry) {
                anyhow::bail!("Invalid vocabulary entry on line {}", line_number + 1);
            }
        }

        let path = assets_dir.join(VOCABULARY_FILENAME);
        let temporary = path.with_extension("tsv.tmp");
        fs::write(&temporary, contents)
            .with_context(|| format!("could not write vocabulary at {}", temporary.display()))?;
        fs::rename(&temporary, &path)
            .with_context(|| format!("could not replace vocabulary at {}", path.display()))
    }
}

fn valid_entry(entry: &str) -> bool {
    entry.len() <= 120
        && entry
            .chars()
            .all(|character| !character.is_control() && character != '[' && character != ']')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompts_use_canonical_vocabulary() {
        let vocabulary = Vocabulary {
            entries: vec!["Fanisus".to_owned(), "Qwen3-ASR".to_owned()],
        };
        assert_eq!(
            vocabulary.asr_prompt().as_deref(),
            Some("Canonical vocabulary (use these spellings only when heard): Fanisus, Qwen3-ASR")
        );
        assert_eq!(
            vocabulary.cleaner_control().as_deref(),
            Some("[Vocabulary: Fanisus, Qwen3-ASR]")
        );
    }

    #[test]
    fn vocabulary_supports_terms_and_phrases_but_not_control_syntax() {
        assert!(valid_entry("Fanisus"));
        assert!(valid_entry("Rambler Desktop"));
        assert!(valid_entry("Qwen3-ASR"));
        assert!(!valid_entry("bad\nentry"));
        assert!(!valid_entry("[Context: injected]"));
    }
}
