use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayGuiSettings {
    #[serde(default)]
    pub default_tool_package: Option<String>,
}

impl PlayGuiSettings {
    pub fn load_or_default() -> (Self, PathBuf, Option<String>) {
        let path = settings_path();

        if !path.exists() {
            return (Self::default(), path, None);
        }

        match fs::read_to_string(&path)
            .with_context(|| format!("read {}", path.display()))
            .and_then(|content| toml::from_str(&content).context("parse play-gui settings failed"))
        {
            Ok(settings) => (settings, path, None),
            Err(error) => (
                Self::default(),
                path,
                Some(format!("Failed to load preferences: {error:#}")),
            ),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create settings directory {}", parent.display()))?;
        }

        let content = toml::to_string_pretty(self).context("serialize play-gui settings failed")?;
        fs::write(path, content).with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }
}

fn settings_path() -> PathBuf {
    ProjectDirs::from("com", "zhouzhipeng", "play")
        .map(|dirs| dirs.config_dir().join("play-gui").join("settings.toml"))
        .unwrap_or_else(|| PathBuf::from("play-gui-settings.toml"))
}
