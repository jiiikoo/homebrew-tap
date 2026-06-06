//! User preferences (`config.toml`). Kept minimal for now; theming and keybinding
//! overrides are filled in at M6.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::paths::Paths;

/// Default sort for the host list when no search query is typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Sort {
    /// Most-used-recently first (atuin-style).
    #[default]
    Frecency,
    /// Alphabetical by name.
    Name,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Frecency decay rate (per day). Higher = recency matters more. Default 0.2.
    pub decay_rate: f64,
    /// Default list ordering when idle.
    pub default_sort: Sort,
    /// Accent color name (black/red/green/yellow/blue/magenta/cyan/white/gray).
    pub accent: String,
    /// Custom host-database path. `None` = the default under the config dir.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosts_file: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            decay_rate: 0.2,
            default_sort: Sort::default(),
            accent: "cyan".to_string(),
            hosts_file: None,
        }
    }
}

/// A commented default `config.toml`, written on first run so the options are discoverable.
pub const DEFAULT_CONFIG_TOML: &str = "\
# sshelf configuration

# Frecency decay per day. Higher = recency matters more; lower = frequency dominates.
decay_rate = 0.2

# Default list order when not searching: \"frecency\" or \"name\".
default_sort = \"frecency\"

# Accent color: black red green yellow blue magenta cyan white gray
accent = \"cyan\"
";

impl Config {
    /// Load preferences; a missing file yields defaults.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let cfg = toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        Ok(cfg)
    }

    /// Write the commented default config if the file does not yet exist.
    pub fn ensure_default_file(path: &Path) -> Result<()> {
        if path.exists() {
            return Ok(());
        }
        crate::store::atomic_write(path, DEFAULT_CONFIG_TOML.as_bytes(), 0o600)
    }

    /// Persist the current preferences (atomic). Note: rewrites the file as plain TOML, so the
    /// commented template from first run is replaced.
    pub fn save(&self, path: &Path) -> Result<()> {
        let text = toml::to_string_pretty(self).context("serializing config")?;
        crate::store::atomic_write(path, text.as_bytes(), 0o600)
    }

    /// The resolved host-database path: `hosts_file` if set (with `~` expanded), else the
    /// default under the config dir.
    pub fn hosts_path(&self, paths: &Paths) -> PathBuf {
        match self.hosts_file.as_deref().map(str::trim) {
            Some(s) if !s.is_empty() => crate::paths::expand_user_path(s),
            _ => paths.hosts_file(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let c = Config::default();
        assert_eq!(c.decay_rate, 0.2);
        assert_eq!(c.default_sort, Sort::Frecency);
    }

    #[test]
    fn partial_toml_uses_defaults_for_missing() {
        let c: Config = toml::from_str("decay_rate = 0.5").unwrap();
        assert_eq!(c.decay_rate, 0.5);
        assert_eq!(c.default_sort, Sort::Frecency);
        assert_eq!(c.accent, "cyan");
    }

    #[test]
    fn default_template_parses_into_defaults() {
        let c: Config = toml::from_str(DEFAULT_CONFIG_TOML).unwrap();
        assert_eq!(c.decay_rate, 0.2);
        assert_eq!(c.default_sort, Sort::Frecency);
        assert_eq!(c.accent, "cyan");
    }
}
