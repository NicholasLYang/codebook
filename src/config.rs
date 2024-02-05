use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub test_command: String,
}

impl Config {
    pub fn load(dir: &Path) -> Result<Self, anyhow::Error> {
        let config_path = dir.join("codebook.toml");
        println!("config_path: {:?}", config_path);
        let config = std::fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config)?;
        Ok(config)
    }
}
