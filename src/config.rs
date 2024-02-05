use camino::Utf8PathBuf;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub files: Vec<Utf8PathBuf>,
    pub test: Option<TestConfig>,
}

#[derive(Debug, Deserialize)]
pub struct TestConfig {
    pub command: Option<String>,
}

impl Config {
    pub fn load(dir: &Path) -> Result<Self, anyhow::Error> {
        let config_path = dir.join("codebook.toml");
        let config = std::fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config)?;
        Ok(config)
    }
}
