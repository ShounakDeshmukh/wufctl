use std::io::ErrorKind;
#[cfg(unix)]
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{fs, path::PathBuf};

use color_eyre::eyre::{ContextCompat, Result, WrapErr};
use keyring::Entry;
use serde::{Deserialize, Serialize};

use crate::utils;

const KEYRING_SERVICE: &str = "wufctl";
const KEYRING_USERNAME: &str = "token";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    token: String,
}

impl Config {
    pub fn new(token: impl Into<String>) -> Self {
        Config {
            token: token.into(),
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    fn path() -> Option<PathBuf> {
        utils::get_config_dir()
    }

    /// Tries the OS keyring first; falls back to the config file
    pub fn load() -> Result<Option<Config>> {
        if let Some(config) = Self::load_from_keyring() {
            return Ok(Some(config));
        }
        Self::load_from_file()
    }

    fn load_from_keyring() -> Option<Config> {
        let token = Entry::new(KEYRING_SERVICE, KEYRING_USERNAME)
            .ok()?
            .get_password()
            .ok()?;
        Some(Config::new(token))
    }

    fn load_from_file() -> Result<Option<Config>> {
        let Some(dir) = Self::path() else {
            return Ok(None);
        };

        let config_path = dir.join("config.toml");
        let contents = match fs::read_to_string(&config_path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err).context("failed to read config.toml"),
        };

        let config = toml::from_str(&contents).context("failed to parse config.toml")?;

        Ok(Some(config))
    }

    /// A successful keyring save removes any leftover plaintext file.
    pub fn save(&self) -> Result<()> {
        if let Ok(entry) = Entry::new(KEYRING_SERVICE, KEYRING_USERNAME)
            && entry.set_password(&self.token).is_ok()
        {
            self.remove_file();
            return Ok(());
        }
        self.save_to_file()
    }

    fn remove_file(&self) {
        if let Some(dir) = Self::path() {
            let _ = fs::remove_file(dir.join("config.toml"));
        }
    }

    fn save_to_file(&self) -> Result<()> {
        let dir = Self::path().context("could not determine config directory for this OS")?;
        fs::create_dir_all(&dir).context("failed to create config directory")?;

        let config_path = dir.join("config.toml");
        let contents = toml::to_string(self).context("failed to serialize config")?;

        // Create it with 0600 from the start to avoid a brief world-readable window.
        #[cfg(unix)]
        {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&config_path)
                .context("failed to open config.toml")?;
            file.write_all(contents.as_bytes())
                .context("failed to write config.toml")?;
        }

        #[cfg(not(unix))]
        fs::write(&config_path, contents).context("failed to write config.toml")?;

        Ok(())
    }
}
