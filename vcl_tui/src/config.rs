use std::{fs, path::PathBuf};

use color_eyre::eyre::{ContextCompat, Result, WrapErr};
use serde::{Deserialize, Serialize};

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
        crate::utils::get_config_dir()
    }

    pub fn load() -> Result<Option<Config>> {
        let Some(dir) = Self::path() else {
            return Ok(None);
        };

        let config_path = dir.join("config.toml");
        let contents = match fs::read_to_string(&config_path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err).context("failed to read config.toml"),
        };

        let config = toml::from_str(&contents).context("failed to parse config.toml")?;

        Ok(Some(config))
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::path().context("could not determine config directory for this OS")?;
        fs::create_dir_all(&dir).context("failed to create config directory")?;

        let config_path = dir.join("config.toml");
        let contents = toml::to_string(self).context("failed to serialize config")?;

        // Create it with 0600 from the start to avoid a brief
        // world-readable window. Unix-only; Windows has no equivalent.
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;

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
