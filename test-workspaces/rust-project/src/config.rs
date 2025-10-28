use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub debug: bool,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&contents)?;
        Ok(config)
    }

    pub fn default() -> Self {
        Self {
            database_url: "localhost:5432".to_string(),
            port: 8080,
            debug: false,
        }
    }
}
