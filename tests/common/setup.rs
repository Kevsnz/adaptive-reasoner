use std::collections::HashMap;
use std::sync::Arc;

use reqwest::Client;

use adaptive_reasoner::config::{Config, ModelConfig};
use adaptive_reasoner::service::ReasoningService;

pub fn create_test_config() -> Config {
    let mut models = HashMap::new();
    models.insert(
        "test-model".to_string(),
        ModelConfig {
            model_name: "test-model".to_string(),
            api_url: "http://localhost:8081".to_string(),
            api_key: "test-key".to_string(),
            reasoning_budget: 100,
            extra: None,
        },
    );
    Config { models }
}

pub async fn create_test_app_components() -> (Arc<Config>, Arc<ReasoningService>) {
    let config = Arc::new(create_test_config());
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    (config, reasoning_service)
}

pub fn create_model_config(base_url: String) -> ModelConfig {
    ModelConfig {
        model_name: "test-model".to_string(),
        api_url: base_url,
        api_key: "test-key".to_string(),
        reasoning_budget: 100,
        extra: None,
    }
}
