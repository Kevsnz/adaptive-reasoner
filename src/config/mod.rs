use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::ReasonerError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ModelConfig {
    pub(crate) model_name: String,
    pub(crate) api_url: String,
    pub(crate) api_key: String,
    pub(crate) reasoning_budget: i32,
    pub(crate) extra: Option<HashMap<String, Value>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Config {
    pub(crate) models: HashMap<String, ModelConfig>,
}

pub(crate) trait ConfigLoader: Send + Sync {
    fn load_config(&self) -> Result<Config, ReasonerError>;
}

pub(crate) struct FileConfigLoader;

impl FileConfigLoader {
    pub(crate) fn new() -> Self {
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

pub(crate) fn load_config() -> Result<Config, ReasonerError> {
    let loader = FileConfigLoader::new();
    loader.load_config()
}
