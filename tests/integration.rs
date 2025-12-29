use adaptive_reasoner::config::ModelConfig;
use adaptive_reasoner::consts;
use adaptive_reasoner::models::request;
use adaptive_reasoner::service::ReasoningService;
use reqwest::Client;
use serde_json::json;
use tokio::sync::mpsc;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

use crate::fixtures::{
    sample_answer_chunks, sample_answer_response, sample_chat_request, sample_reasoning_chunks,
    sample_reasoning_response,
};

mod fixtures;
mod mocks;

fn create_model_config(base_url: String) -> ModelConfig {
    ModelConfig {
        model_name: "test-model".to_string(),
        api_url: base_url,
        api_key: "test-key".to_string(),
        reasoning_budget: 100,
        extra: None,
    }
}

fn build_response_json(response: &serde_json::Value) -> String {
    format!("data: {}\n\n", serde_json::to_string(response).unwrap())
}

#[tokio::test]
async fn test_integration_complete_reasoning_and_answer_flow() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    let reasoning_response = sample_reasoning_response();
    let answer_response = sample_answer_response();

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&reasoning_response))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&answer_response))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);
    let request = sample_chat_request();

    let result = service.create_completion(request, &model_config).await;

    assert!(result.is_ok(), "Expected successful completion");
    let completion = result.unwrap();

    assert_eq!(completion.id, "chatcmpl-test-1");
    assert_eq!(completion.object, "chat.completion");
    assert_eq!(completion.model, "test-model");
    assert_eq!(completion.choices.len(), 1);

    let choice = &completion.choices[0];
    assert_eq!(choice.index, 0);
    assert_eq!(
        choice.finish_reason,
        adaptive_reasoner::models::FinishReason::Stop
    );

    assert_eq!(completion.usage.prompt_tokens, 10);
    assert_eq!(completion.usage.completion_tokens, 80);
    assert_eq!(completion.usage.total_tokens, 90);
}

#[tokio::test]
async fn test_integration_streaming_flow_with_multiple_chunks() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    let reasoning_chunks = sample_reasoning_chunks();
    let answer_chunks = sample_answer_chunks();

    let mut reasoning_sse = String::new();
    for chunk in &reasoning_chunks {
        reasoning_sse.push_str(&build_response_json(&json!(chunk)));
    }
    reasoning_sse.push_str("data: [DONE]\n\n");

    let mut answer_sse = String::new();
    for chunk in &answer_chunks {
        answer_sse.push_str(&build_response_json(&json!(chunk)));
    }
    answer_sse.push_str("data: [DONE]\n\n");

    use reqwest::header::{CONTENT_TYPE, HeaderValue};

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(reasoning_sse.into_bytes())
                .insert_header(CONTENT_TYPE, HeaderValue::from_static("text/event-stream")),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(answer_sse.into_bytes())
                .insert_header(CONTENT_TYPE, HeaderValue::from_static("text/event-stream")),
        )
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);

    let mut request = sample_chat_request();
    request.stream = Some(true);
    request.stream_options = Some(request::StreamOptions {
        include_usage: Some(true),
    });

    let (sender, mut receiver) = mpsc::channel(consts::CHANNEL_BUFFER_SIZE);

    let service_clone = service.clone();
    tokio::spawn(async move {
        let _ = service_clone
            .stream_completion(request, &model_config, sender)
            .await;
    });

    let mut received_messages = vec![];
    let mut messages_received = 0;
    let timeout = tokio::time::Duration::from_secs(5);
    let start_time = tokio::time::Instant::now();

    loop {
        match tokio::time::timeout(timeout, receiver.recv()).await {
            Ok(Some(result)) => match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    if !text.contains("[DONE]") {
                        received_messages.push(text.to_string());
                        messages_received += 1;
                    }
                }
                Err(e) => {
                    eprintln!("Received error: {:?}", e);
                    panic!("Received error: {:?}", e);
                }
            },
            Ok(None) => {
                break;
            }
            Err(_) => {
                break;
            }
        }
        if start_time.elapsed() > timeout {
            eprintln!("Timeout waiting for chunks");
            break;
        }
    }

    assert!(
        messages_received > 0,
        "Expected to receive streaming chunks, got {}",
        messages_received
    );

    let final_chunk = received_messages.last().unwrap();
    assert!(
        final_chunk.contains("\"total_tokens\"") || final_chunk.contains("\"usage\""),
        "Expected final chunk with usage statistics"
    );
}

#[tokio::test]
async fn test_integration_api_failure_at_reasoning_phase() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "error": {
                "message": "Internal server error",
                "type": "internal_error"
            }
        })))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);
    let request = sample_chat_request();

    let result = service.create_completion(request, &model_config).await;

    assert!(result.is_err(), "Expected error from reasoning phase");
    match result.unwrap_err() {
        adaptive_reasoner::errors::ReasonerError::ApiError(msg) => {
            assert!(msg.contains("status 500"), "Expected 500 status in error");
        }
        _ => panic!("Expected ApiError variant"),
    }
}

#[tokio::test]
async fn test_integration_api_failure_at_answer_phase() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&sample_reasoning_response()))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "error": {
                "message": "Internal server error",
                "type": "internal_error"
            }
        })))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);
    let request = sample_chat_request();

    let result = service.create_completion(request, &model_config).await;

    assert!(result.is_err(), "Expected error from answer phase");
    match result.unwrap_err() {
        adaptive_reasoner::errors::ReasonerError::ApiError(msg) => {
            assert!(msg.contains("status 500"), "Expected 500 status in error");
        }
        _ => panic!("Expected ApiError variant"),
    }
}

#[tokio::test]
async fn test_integration_malformed_response() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{invalid json}"))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);
    let request = sample_chat_request();

    let result = service.create_completion(request, &model_config).await;

    assert!(result.is_err(), "Expected error from malformed response");
    match result.unwrap_err() {
        adaptive_reasoner::errors::ReasonerError::ParseError(_)
        | adaptive_reasoner::errors::ReasonerError::ApiError(_) => {}
        _ => panic!("Expected ParseError or ApiError variant"),
    }
}

#[tokio::test]
async fn test_integration_reasoning_budget_exceeded() {
    let mock_server = MockServer::start().await;

    let mut reasoning_response = sample_reasoning_response();
    reasoning_response.choices[0].finish_reason = adaptive_reasoner::models::FinishReason::Length;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&reasoning_response))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);

    let mut request = sample_chat_request();
    request.max_tokens = Some(150);

    let mut model_config = create_model_config(mock_server.uri());
    model_config.reasoning_budget = 200;

    let result = service.create_completion(request, &model_config).await;

    assert!(
        result.is_ok(),
        "Expected successful completion with budget exceeded"
    );
    let completion = result.unwrap();

    assert_eq!(
        completion.choices[0].finish_reason,
        adaptive_reasoner::models::FinishReason::Length
    );

    let choice = &completion.choices[0];
    let assistant = &choice.message;
    if let Some(content) = &assistant.content {
        assert!(
            content.contains("Right, this is taking too long"),
            "Expected cutoff stub in content"
        );
    }
}

#[tokio::test]
async fn test_integration_tool_calls_propagation() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    let reasoning_response = sample_reasoning_response();
    let mut answer_response = sample_answer_response();
    answer_response.choices[0].message.tool_calls = Some(vec![json!({
        "id": "call_123",
        "type": "function",
        "function": {
            "name": "test_function",
            "arguments": "{\"arg\": \"value\"}"
        }
    })]);

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&reasoning_response))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&answer_response))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);
    let request = sample_chat_request();

    let result = service.create_completion(request, &model_config).await;

    assert!(result.is_ok(), "Expected successful completion");
    let completion = result.unwrap();

    let choice = &completion.choices[0];
    let assistant = &choice.message;
    assert!(
        assistant.tool_calls.is_some(),
        "Expected tool_calls to be present"
    );
    let tool_calls = assistant.tool_calls.as_ref().unwrap();
    assert_eq!(tool_calls.len(), 1);
}

#[tokio::test]
async fn test_integration_empty_reasoning_content() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    let mut reasoning_response = sample_reasoning_response();
    reasoning_response.choices[0].message.content = None;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&reasoning_response))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&sample_answer_response()))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);
    let request = sample_chat_request();

    let result = service.create_completion(request, &model_config).await;

    assert!(
        result.is_ok(),
        "Expected successful completion with empty reasoning"
    );
    let completion = result.unwrap();

    let choice = &completion.choices[0];
    let assistant = &choice.message;
    assert!(
        assistant.content.is_some(),
        "Expected content to be present"
    );
}
