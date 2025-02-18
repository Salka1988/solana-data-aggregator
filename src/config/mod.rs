use crate::utils::error::{AggregatorError, AggregatorResult};
use config::{Config as RawConfig, Environment, File, FileFormat};
use dotenv::dotenv;
use serde::Deserialize;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::time::Duration;

fn default_port() -> u16 {
    3000
}

fn default_host() -> Ipv4Addr {
    Ipv4Addr::new(127, 0, 0, 1)
}
fn default_interval() -> Duration {
    Duration::from_secs(2)
}
fn default_max_retry_attempts() -> usize {
    3
}

fn default_retry_delay_secs() -> u64 {
    5
}

fn default_api_key() -> String {
    "mysecretkey".to_string()
}

fn human_readable_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let duration_str: String = Deserialize::deserialize(deserializer)?;
    humantime::parse_duration(&duration_str).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct BlockchainConfig {
    pub rpc_url: String,
    #[serde(default)]
    pub max_batch_size: Option<usize>,
    #[serde(default = "default_max_retry_attempts")]
    pub max_retry_attempts: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_host")]
    pub host: Ipv4Addr,
    #[serde(default = "default_api_key")]
    pub apy_key: String,
}
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            host: default_host(),
            apy_key: default_api_key(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PollingConfig {
    #[serde(
        default = "default_interval",
        deserialize_with = "human_readable_duration"
    )]
    pub interval: Duration,
    #[serde(default = "default_max_retry_attempts")]
    pub max_retry_attempts: usize,
    #[serde(default = "default_retry_delay_secs")]
    pub retry_delay_secs: u64,
    pub batch_size: Option<usize>,
}

impl Default for PollingConfig {
    fn default() -> Self {
        Self {
            interval: default_interval(),
            max_retry_attempts: default_max_retry_attempts(),
            retry_delay_secs: default_retry_delay_secs(),
            batch_size: None,
        }
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub blockchain: BlockchainConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub polling: PollingConfig,
}
impl Config {
    /// Loads the configuration from an optional file and environment variables.
    pub fn new(config_path: Option<PathBuf>) -> AggregatorResult<Self> {
        dotenv().ok(); // Add this line

        let mut builder = RawConfig::builder();

        if let Some(path) = config_path {
            builder = builder.add_source(File::from(path).format(FileFormat::Toml).required(false));
        }

        builder = builder.add_source(
            Environment::with_prefix("APP")
                .separator("__")
                .try_parsing(true),
        );

        let raw = builder
            .build()
            .map_err(|e| AggregatorError::ConfigError(e.to_string()))?;
        let cfg: Config = raw
            .try_deserialize()
            .map_err(|e| AggregatorError::ConfigError(e.to_string()))?;

        Ok(cfg)
    }

    pub fn validate(&self) -> AggregatorResult<()> {
        if self.polling.interval == Duration::from_secs(0) {
            return Err(AggregatorError::ConfigError(
                "polling.interval must be greater than 0".to_string(),
            ));
        }
        if self.blockchain.max_batch_size == Some(0) {
            return Err(AggregatorError::ConfigError(
                "blockchain.max_batch_size must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_new_and_validate() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"
            [blockchain]
            rpc_url = "https://api.devnet.solana.com"
            max_batch_size = 100

            [server]
            port = 3000
            host = "127.0.0.1"

            [polling]
            interval = "5s"
            max_retry_attempts = 3
            retry_delay_secs = 5
            batch_size = 100
        "#
        )
        .unwrap();
        let config = Config::new(Some(tmp.path().to_path_buf())).unwrap();
        config.validate().unwrap();
        assert_eq!(config.server.port, 3000);
    }
}
