//! Configuration management for Merlin CLI
//!
//! Handles loading and auto-saving configuration to `~/.merlin/config.toml`.
//! Uses a `Drop`-based auto-save mechanism: when you get a mutable reference and drop it,
//! the config is automatically persisted to disk.

use crate::ui::theme::Theme;
use merlin_deps::dirs::home_dir;
use merlin_deps::toml::{from_str, to_string_pretty};
use serde::{Deserialize, Serialize};
use std::env;
use std::io;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::fs as async_fs;

const ENV_OPENROUTER_API_KEY: &str = "OPENROUTER_API_KEY";

/// Main configuration for the agentic optimizer
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Provider configuration (API keys and models)
    #[serde(default)]
    pub providers: ProvidersConfig,
    /// UI theme
    #[serde(default)]
    pub theme: Theme,
}

/// Configuration for remote model providers
///
/// This is where you configure which models to use for different task complexities.
/// Add or modify models here to customize the AI behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    /// `OpenRouter` API key for accessing remote models
    pub openrouter_key: Option<String>,
    /// High-complexity model for demanding tasks (default: anthropic/claude-sonnet-4-20250514)
    pub high_model: Option<String>,
    /// Medium-complexity model for balanced performance (default: anthropic/claude-3.5-sonnet)
    pub medium_model: Option<String>,
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            openrouter_key: env::var(ENV_OPENROUTER_API_KEY).ok(),
            high_model: Some("anthropic/claude-sonnet-4-20250514".to_owned()),
            medium_model: Some("anthropic/claude-3.5-sonnet".to_owned()),
        }
    }
}

/// Shared configuration manager with auto-save on mutable drop
#[derive(Clone)]
pub struct ConfigManager {
    /// Shared config state
    inner: Arc<RwLock<Config>>,
    /// Path to config file
    config_path: PathBuf,
}

impl ConfigManager {
    /// Creates a new config manager
    ///
    /// # Errors
    /// Returns an error if the home directory cannot be determined or config directory cannot be created
    pub async fn new() -> io::Result<Self> {
        let config_path = Self::get_config_path()?;

        // Ensure ~/.merlin directory exists
        if let Some(parent) = config_path.parent() {
            async_fs::create_dir_all(parent).await?;
        }

        // Load or create config
        let config = Self::load_from_disk(&config_path).await?;

        Ok(Self {
            inner: Arc::new(RwLock::new(config)),
            config_path,
        })
    }

    /// Gets the config file path (~/.merlin/config.toml)
    ///
    /// # Errors
    /// Returns error if home directory cannot be determined
    fn get_config_path() -> io::Result<PathBuf> {
        let home = home_dir().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine home directory",
            )
        })?;
        Ok(home.join(".merlin").join("config.toml"))
    }

    /// Loads config from disk, returns default if file doesn't exist
    ///
    /// # Errors
    /// Returns error if file cannot be read or parsed
    async fn load_from_disk(path: &PathBuf) -> io::Result<Config> {
        if !async_fs::try_exists(path).await.unwrap_or(false) {
            return Ok(Config::default());
        }

        let contents = async_fs::read_to_string(path).await?;
        from_str(&contents).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    /// Saves config to disk (async)
    ///
    /// # Errors
    /// Returns error if serialization fails or file cannot be written
    async fn save_to_disk(&self) -> io::Result<()> {
        let toml_str = {
            let config = self
                .inner
                .read()
                .map_err(|err| io::Error::other(format!("Lock poisoned: {err}")))?;

            to_string_pretty(&*config)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?
        };

        async_fs::write(&self.config_path, toml_str).await?;
        Ok(())
    }

    /// Gets immutable reference to config
    ///
    /// # Errors
    /// Returns error if lock is poisoned
    pub fn get(&self) -> io::Result<impl Deref<Target = Config> + '_> {
        self.inner
            .read()
            .map_err(|err| io::Error::other(format!("Lock poisoned: {err}")))
    }

    /// Gets mutable reference that auto-saves on drop
    ///
    /// # Errors
    /// Returns error if lock is poisoned
    pub fn get_mut(&self) -> io::Result<ConfigGuard<'_>> {
        let guard = self
            .inner
            .write()
            .map_err(|err| io::Error::other(format!("Lock poisoned: {err}")))?;

        Ok(ConfigGuard {
            guard,
            manager: self,
        })
    }
}

use std::sync::RwLockWriteGuard;
use tokio::task::spawn;
#[cfg(test)]
use tokio::time::{Duration, sleep};

/// RAII guard that auto-saves config when dropped
pub struct ConfigGuard<'guard> {
    /// Write lock guard
    guard: RwLockWriteGuard<'guard, Config>,
    /// Reference to manager for saving
    manager: &'guard ConfigManager,
}

impl Deref for ConfigGuard<'_> {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl DerefMut for ConfigGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

impl Drop for ConfigGuard<'_> {
    fn drop(&mut self) {
        // Clone config before dropping the guard
        let config_clone = self.guard.clone();
        let manager_clone = self.manager.clone();

        // Save asynchronously in background (fire and forget)
        // This happens after the guard is dropped (at end of this function)
        spawn(async move {
            // Update the config with our changes
            if let Ok(mut config) = manager_clone.inner.write() {
                *config = config_clone;
            }

            if let Err(err) = manager_clone.save_to_disk().await {
                merlin_deps::tracing::warn!("Failed to auto-save config: {err}");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use merlin_deps::toml::to_string_pretty;

    /// # Panics
    /// Test function - panics indicate test failure
    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.providers.high_model.is_some());
        assert!(config.providers.medium_model.is_some());
        assert_eq!(config.theme, Theme::default());
    }

    /// # Panics
    /// Test function - panics indicate test failure
    #[test]
    fn test_providers_config_default() {
        let config = ProvidersConfig::default();
        assert!(config.high_model.is_some());
        assert!(config.medium_model.is_some());
    }

    /// # Panics
    /// Test function - panics indicate test failure
    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = match to_string_pretty(&config) {
            Ok(str) => str,
            Err(err) => panic!("Failed to serialize: {err}"),
        };
        assert!(toml_str.contains("providers"));
        assert!(toml_str.contains("theme"));
    }

    /// # Panics
    /// Test function - panics indicate test failure
    #[tokio::test]
    async fn test_config_manager_new() {
        let manager = match ConfigManager::new().await {
            Ok(mgr) => mgr,
            Err(err) => panic!("Failed to create manager: {err}"),
        };
        let config = match manager.get() {
            Ok(cfg) => cfg,
            Err(err) => panic!("Failed to get config: {err}"),
        };
        assert!(config.providers.high_model.is_some());
    }

    /// # Panics
    /// Test function - panics indicate test failure
    #[tokio::test]
    async fn test_config_manager_get_mut() {
        let manager = match ConfigManager::new().await {
            Ok(mgr) => mgr,
            Err(err) => panic!("Failed to create manager: {err}"),
        };

        {
            let mut config_guard = match manager.get_mut() {
                Ok(guard) => guard,
                Err(err) => panic!("Failed to get mutable config: {err}"),
            };
            config_guard.theme = Theme::Nord;
        } // Drop happens here, triggering auto-save

        // Give async save a moment to complete
        sleep(Duration::from_millis(100)).await;

        let config = match manager.get() {
            Ok(cfg) => cfg,
            Err(err) => panic!("Failed to get config: {err}"),
        };
        assert_eq!(config.theme, Theme::Nord);
    }
}
