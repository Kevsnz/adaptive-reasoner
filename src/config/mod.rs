use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::ReasonerError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    pub model_name: String,
    pub api_url: String,
    pub api_key: String,
    pub reasoning_budget: i32,
    pub extra: Option<HashMap<String, Value>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub models: HashMap<String, ModelConfig>,
}

pub trait ConfigLoader: Send + Sync {
    fn load_config(&self) -> Result<Config, ReasonerError>;
}

pub struct FileConfigLoader;

impl FileConfigLoader {
    pub fn new() -> Self {
        Self
    }
}

impl ConfigLoader for FileConfigLoader {
    fn load_config(&self) -> Result<Config, ReasonerError> {
        let config_file = std::env::var("AR_CONFIG_FILE").unwrap_or("./config.json".to_string());
        let config_str = std::fs::read_to_string(&config_file)?;
        let mut config: Config = serde_json::from_str(&config_str)?;

        for model_config in config.models.values_mut() {
            model_config.api_key = std::env::var(&model_config.api_key).unwrap_or_default();
        }

        Ok(config)
    }
}

pub fn load_config() -> Result<Config, ReasonerError> {
    let loader = FileConfigLoader::new();
    loader.load_config()
}
