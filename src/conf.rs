use secrecy::SecretString;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    Missing(String),
    #[error("failed to deserialise configuration from environment: {0}")]
    Envy(#[from] envy::Error),
}

/// Raw deserialisable struct — all fields optional so we can give precise error messages.
#[derive(Debug, Deserialize)]
struct RawSettings {
    #[serde(rename = "worldline_login")]
    login: Option<String>,
    #[serde(rename = "worldline_password")]
    password: Option<SecretString>,
    #[serde(rename = "worldline_account_id")]
    account_id: Option<String>,
}

/// Validated, ready-to-use configuration.
#[derive(Debug, Clone)]
pub struct Settings {
    pub login: String,
    pub password: SecretString,
    pub account_id: String,
}

impl Settings {
    /// Load configuration from environment variables (also reads `.env` if present).
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Envy`] if environment variable deserialisation fails,
    /// or [`ConfigError::Missing`] if any required variable (`WORLDLINE_LOGIN`,
    /// `WORLDLINE_PASSWORD`, `WORLDLINE_ACCOUNT_ID`) is absent.
    pub fn from_env() -> Result<Self, ConfigError> {
        // Load .env file if available; silently ignore if missing.
        let raw: RawSettings = envy::from_env()?;

        Ok(Self {
            login: raw
                .login
                .ok_or_else(|| ConfigError::Missing("WORLDLINE_LOGIN".into()))?,
            password: raw
                .password
                .ok_or_else(|| ConfigError::Missing("WORLDLINE_PASSWORD".into()))?,
            account_id: raw
                .account_id
                .ok_or_else(|| ConfigError::Missing("WORLDLINE_ACCOUNT_ID".into()))?,
        })
    }
}
