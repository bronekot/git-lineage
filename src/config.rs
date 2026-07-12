use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    cli::SummaryMode,
    error::{ConfigIssue, LineageError, Result},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LineageConfig {
    pub version: u32,
    pub root: String,
    pub remote: String,
    pub fetch: bool,
    pub parents: BTreeMap<String, String>,
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    pub collapse_synced_upstreams: bool,
    pub show_remote_only: bool,
    pub summary: SummaryMode,
}

impl Default for LineageConfig {
    fn default() -> Self {
        Self {
            version: 1,
            root: "main".into(),
            remote: "origin".into(),
            fetch: true,
            parents: BTreeMap::new(),
            display: DisplayConfig::default(),
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            collapse_synced_upstreams: true,
            show_remote_only: false,
            summary: SummaryMode::Brief,
        }
    }
}

impl LineageConfig {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path).map_err(|source| LineageError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let config: Self = toml::from_str(&text).map_err(|source| LineageError::Toml {
            path: path.to_path_buf(),
            source: Box::new(source),
        })?;
        config.validate_version()?;
        Ok(config)
    }

    pub fn save(&self, path: &Path, force: bool) -> Result<()> {
        if path.exists() && !force {
            return Err(LineageError::ConfigExists(path.to_path_buf()));
        }
        let text = toml::to_string_pretty(self).map_err(Box::new)?;
        fs::write(path, text).map_err(|source| LineageError::Write {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn validate_version(&self) -> Result<()> {
        if self.version != 1 {
            return Err(ConfigIssue::UnsupportedVersion(self.version).into());
        }
        Ok(())
    }
}

pub fn config_path(root: &Path, requested: Option<&PathBuf>) -> PathBuf {
    match requested {
        Some(path) if path.is_absolute() => path.clone(),
        Some(path) => root.join(path),
        None => root.join(".git-lineage.toml"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config() {
        let config: LineageConfig =
            toml::from_str("version = 1\nroot = \"trunk\"\n[parents]\nfeature = \"trunk\"\n")
                .unwrap();
        assert_eq!(config.root, "trunk");
        assert_eq!(config.remote, "origin");
        assert_eq!(config.parents["feature"], "trunk");
    }
}
