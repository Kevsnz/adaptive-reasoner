use actix_web::web::Bytes;
use tokio::sync::mpsc::Sender;

use crate::config;
use crate::errors::ReasonerError;
use crate::models::request;
use crate::models::response_direct::ChatCompletion;

use crate::service::ReasoningService;

pub(crate) fn calculate_remaining_tokens(max_tokens: Option<i32>, reasoning_tokens: i32) -> i32 {
    max_tokens.unwrap_or(crate::consts::DEFAULT_MAX_TOKENS) - reasoning_tokens
}

pub(crate) fn build_reasoning_request(
    request: request::ChatCompletionCreate,
    model_config: &config::ModelConfig,
) -> request::ChatCompletionCreate {
    let mut reasoning_request: request::ChatCompletionCreate = request.clone();
    reasoning_request.model = model_config.model_name.to_string();

    let message_assistant = request::MessageAssistant {
        reasoning_content: None,
        content: Some(crate::consts::THINK_START.to_string()),
        tool_calls: None,
    };
    reasoning_request
        .messages
        .push(request::Message::Assistant(message_assistant));
    reasoning_request.stop = Some(vec![crate::consts::THINK_END.to_string()]);
    reasoning_request.max_tokens = Some(model_config.reasoning_budget);

    reasoning_request
}

pub(crate) fn build_answer_request(
    request: request::ChatCompletionCreate,
    model_config: &config::ModelConfig,
    reasoning_text: &str,
    max_tokens: i32,
) -> request::ChatCompletionCreate {
    let mut answer_request: request::ChatCompletionCreate = request.clone();
    answer_request.model = model_config.model_name.to_string();

    let message_assistant = request::MessageAssistant {
        reasoning_content: None,
        content: Some(format!(
            "{}{}{}",
            crate::consts::THINK_START,
            reasoning_text,
            crate::consts::THINK_END,
        )),
        tool_calls: None,
    };
    answer_request
        .messages
        .push(request::Message::Assistant(message_assistant));
    answer_request.max_tokens = Some(max_tokens);

    answer_request
}

pub(crate) fn validate_chat_request(
    request: &request::ChatCompletionCreate,
) -> Result<(), ReasonerError> {
    if request.messages.is_empty() {
        return Err(ReasonerError::ValidationError(
            "error: empty messages".to_string(),
        ));
    }
    if let request::Message::Assistant(_) = request.messages.last().unwrap() {
        return Err(ReasonerError::ValidationError(
            "error: cannot process partial assistant response content in messages yet!".to_string(),
        ));
    }
    Ok(())
}

pub(crate) async fn create_chat_completion(
    http_client: reqwest::Client,
    request: request::ChatCompletionCreate,
    model_config: &config::ModelConfig,
) -> Result<ChatCompletion, ReasonerError> {
    let service = ReasoningService::new(http_client);
    service.create_completion(request, model_config).await
}

pub(crate) async fn stream_chat_completion(
    http_client: reqwest::Client,
    request: request::ChatCompletionCreate,
    model_config: &config::ModelConfig,
    sender: Sender<Result<Bytes, ReasonerError>>,
) -> Result<(), ReasonerError> {
    let service = ReasoningService::new(http_client);
    service.stream_completion(request, model_config, sender).await
}
