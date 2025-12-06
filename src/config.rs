use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ModelConfig {
    pub(crate) model_name: String,
    pub(crate) api_url: String,
    pub(crate) api_key: String,
    pub(crate) reasoning_budget: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Config {
    pub(crate) models: HashMap<String, ModelConfig>,
}

pub(crate) fn load_config() -> Config {
    let config_file = std::env::var("AR_CONFIG_FILE").unwrap_or("./config.json".to_string());
    let config_str = std::fs::read_to_string(config_file).unwrap();
    let mut config: Config = serde_json::from_str(&config_str).unwrap();

    for model_config in config.models.values_mut() {
        model_config.api_key = std::env::var(model_config.api_key.clone()).unwrap_or_default();
    }

    config
}
