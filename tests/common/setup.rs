use std::collections::HashMap;
use std::sync::Arc;

use reqwest::Client;
use wiremock::MockServer;

use adaptive_reasoner::config::{Config, ModelConfig};
use adaptive_reasoner::models::request;
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

pub async fn create_test_app_components() -> (
    Arc<Config>,
    Arc<ReasoningService>,
) {
    let config = Arc::new(create_test_config());
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    (config, reasoning_service)
}

pub async fn create_test_app_with_mock_server() -> (
    Arc<Config>,
    Arc<ReasoningService>,
    MockServer,
) {
    let mock_server = MockServer::start().await;

    let mut config = create_test_config();
    config
        .models
        .get_mut("test-model")
        .unwrap()
        .api_url = mock_server.uri();

    let config = Arc::new(config);
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    (config, reasoning_service, mock_server)
}

pub fn create_basic_chat_request() -> request::ChatCompletionCreate {
    request::ChatCompletionCreate {
        model: "test-model".to_string(),
        messages: vec![request::Message::User(request::MessageSystemUser {
            content: request::MessageContent::String("Hello, how are you?".to_string()),
        })],
        max_tokens: Some(100),
        stop: None,
        stream: None,
        stream_options: None,
        tools: None,
        tool_choice: None,
        extra: Default::default(),
    }
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
