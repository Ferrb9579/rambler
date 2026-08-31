use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePreference {
    pub(crate) fn class_name(self) -> &'static str {
        match self {
            Self::System => "theme-system",
            Self::Light => "theme-light",
            Self::Dark => "theme-dark",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Preferences {
    pub(crate) theme: ThemePreference,
    pub(crate) shortcut_enabled: bool,
    pub(crate) always_on_top: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: ThemePreference::System,
            shortcut_enabled: true,
            always_on_top: true,
        }
    }
}

impl Preferences {
    pub(crate) fn load() -> Self {
        Self::try_load().unwrap_or_else(|error| {
            eprintln!("Could not load settings: {error:#}");
            Self::default()
        })
    }

    fn try_load() -> Result<Self> {
        let path = settings_path()?;
        if !path.is_file() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("could not read {}", path.display()))?;
        serde_json::from_str(&contents)
            .with_context(|| format!("could not parse {}", path.display()))
    }

    pub(crate) fn save(&self) -> Result<()> {
        let path = settings_path()?;
        let temporary = path.with_extension("json.tmp");
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&temporary, format!("{contents}\n"))
            .with_context(|| format!("could not write {}", temporary.display()))?;
        fs::rename(&temporary, &path)
            .with_context(|| format!("could not replace {}", path.display()))
    }
}

fn settings_path() -> Result<PathBuf> {
    let models = rambler::models::assets_dir()?;
    Ok(models
        .parent()
        .unwrap_or(models.as_path())
        .join("settings.json"))
}

#[derive(Clone)]
pub(crate) struct ShortcutControl {
    enabled: Arc<AtomicBool>,
}

impl PartialEq for ShortcutControl {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.enabled, &other.enabled)
    }
}

impl ShortcutControl {
    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(enabled)),
        }
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub(crate) fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_accept_missing_new_fields() {
        let preferences: Preferences = serde_json::from_str(r#"{"theme":"dark"}"#).unwrap();
        assert_eq!(preferences.theme, ThemePreference::Dark);
        assert!(preferences.shortcut_enabled);
        assert!(preferences.always_on_top);
    }
}
