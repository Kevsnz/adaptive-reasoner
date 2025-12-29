use serde_json::Value;
#[cfg(test)]
use std::collections::HashMap;

use crate::config::{Config, ModelConfig};
use crate::models::request;

#[cfg(test)]
pub fn create_test_chat_request(model: &str, user_message: &str) -> request::ChatCompletionCreate {
    request::ChatCompletionCreate {
        model: model.to_string(),
        messages: vec![request::Message::User(request::MessageSystemUser {
            content: request::MessageContent::String(user_message.to_string()),
        })],
        max_tokens: Some(1000),
        stop: None,
        stream: None,
        stream_options: None,
        tools: None,
        tool_choice: None,
        extra: Default::default(),
    }
}

#[cfg(test)]
pub fn create_test_model_config(
    model_name: String,
    api_url: String,
    api_key: String,
    reasoning_budget: i32,
) -> ModelConfig {
    ModelConfig {
        model_name,
        api_url,
        api_key,
        reasoning_budget,
        extra: None,
    }
}

#[cfg(test)]
pub fn create_test_config_with_model(
    model_name: String,
    api_url: String,
    api_key: String,
    reasoning_budget: i32,
) -> Config {
    let mut models = HashMap::new();
    models.insert(
        model_name.clone(),
        create_test_model_config(model_name, api_url, api_key, reasoning_budget),
    );
    Config { models }
}

#[cfg(test)]
pub fn create_test_model_config_with_extra(
    model_name: String,
    api_url: String,
    api_key: String,
    reasoning_budget: i32,
    extra: HashMap<String, Value>,
) -> ModelConfig {
    ModelConfig {
        model_name,
        api_url,
        api_key,
        reasoning_budget,
        extra: Some(extra),
    }
}

#[cfg(test)]
pub fn create_empty_messages_request() -> request::ChatCompletionCreate {
    request::ChatCompletionCreate {
        model: "test-model".to_string(),
        messages: vec![],
        max_tokens: Some(100),
        stop: None,
        stream: None,
        stream_options: None,
        tools: None,
        tool_choice: None,
        extra: Default::default(),
    }
}

#[cfg(test)]
pub fn create_assistant_last_request() -> request::ChatCompletionCreate {
    request::ChatCompletionCreate {
        model: "test-model".to_string(),
        messages: vec![
            request::Message::User(request::MessageSystemUser {
                content: request::MessageContent::String("Hello".to_string()),
            }),
            request::Message::Assistant(request::MessageAssistant {
                reasoning_content: None,
                content: Some("I'm doing well".to_string()),
                tool_calls: None,
            }),
        ],
        max_tokens: Some(100),
        stop: None,
        stream: None,
        stream_options: None,
        tools: None,
        tool_choice: None,
        extra: Default::default(),
    }
}
