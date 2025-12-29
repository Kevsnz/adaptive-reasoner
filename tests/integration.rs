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

#[tokio::test]
async fn test_integration_chunk_ordering_guarantee() {
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

    let mut chunks_received = 0;
    let mut content_chunks = 0;
    let timeout = tokio::time::Duration::from_secs(5);
    let start_time = tokio::time::Instant::now();

    loop {
        match tokio::time::timeout(timeout, receiver.recv()).await {
            Ok(Some(result)) => match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    if !text.contains("[DONE]") {
                        chunks_received += 1;
                        if text.contains("content") || text.contains("delta") {
                            content_chunks += 1;
                        }
                    }
                }
                Err(e) => {
                    panic!("Received error: {:?}", e);
                }
            },
            Ok(None) => break,
            Err(_) => break,
        }
        if start_time.elapsed() > timeout {
            break;
        }
    }

    assert!(
        chunks_received > 0,
        "Expected to receive at least one chunk, got {}",
        chunks_received
    );

    assert!(
        content_chunks > 0,
        "Expected to receive content chunks, got {}",
        content_chunks
    );
}

#[tokio::test]
async fn test_integration_incomplete_stream_missing_done() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    let reasoning_chunks = sample_reasoning_chunks();

    let mut reasoning_sse = String::new();
    for chunk in &reasoning_chunks {
        reasoning_sse.push_str(&build_response_json(&json!(chunk)));
    }
    reasoning_sse.push_str("data: [DONE]\n\n");

    let answer_sse = String::from("data: {\"id\":\"test\",\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n");

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

    let (sender, mut receiver) = mpsc::channel(consts::CHANNEL_BUFFER_SIZE);

    let service_clone = service.clone();
    tokio::spawn(async move {
        let _ = service_clone
            .stream_completion(request, &model_config, sender)
            .await;
    });

    let mut received_messages = vec![];
    let timeout = tokio::time::Duration::from_secs(5);
    let start_time = tokio::time::Instant::now();

    loop {
        match tokio::time::timeout(timeout, receiver.recv()).await {
            Ok(Some(result)) => match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    if !text.contains("[DONE]") {
                        received_messages.push(text.to_string());
                    }
                }
                Err(_) => break,
            },
            Ok(None) => break,
            Err(_) => break,
        }
        if start_time.elapsed() > timeout {
            break;
        }
    }

    assert!(
        received_messages.len() > 0,
        "Expected to receive some chunks before stream ended"
    );
}

#[tokio::test]
async fn test_integration_incomplete_stream_malformed_chunk() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    let reasoning_chunks = sample_reasoning_chunks();

    let mut reasoning_sse = String::new();
    for chunk in &reasoning_chunks {
        reasoning_sse.push_str(&build_response_json(&json!(chunk)));
    }
    reasoning_sse.push_str("data: [DONE]\n\n");

    let answer_sse = String::from("data: {\"id\":\"test\",\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\ndata: invalid json here\n\ndata: {\"id\":\"test\",\"choices\":[{\"delta\":{\"content\":\"World\"}}]}\n\ndata: [DONE]\n\n");

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

    let (sender, mut receiver) = mpsc::channel(consts::CHANNEL_BUFFER_SIZE);

    let service_clone = service.clone();
    tokio::spawn(async move {
        let _ = service_clone
            .stream_completion(request, &model_config, sender)
            .await;
    });

    let mut _has_error = false;
    let mut chunks_received = 0;
    let timeout = tokio::time::Duration::from_secs(5);
    let start_time = tokio::time::Instant::now();

    loop {
        match tokio::time::timeout(timeout, receiver.recv()).await {
            Ok(Some(result)) => match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    if !text.contains("[DONE]") {
                        chunks_received += 1;
                    }
                }
                Err(_) => {
                    _has_error = true;
                }
            },
            Ok(None) => break,
            Err(_) => break,
        }
        if start_time.elapsed() > timeout {
            break;
        }
    }

    assert!(
        chunks_received > 0,
        "Expected to receive valid chunks despite malformed JSON"
    );
}

#[tokio::test]
async fn test_integration_timeout_during_streaming() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    let reasoning_chunks = sample_reasoning_chunks();

    let mut reasoning_sse = String::new();
    for chunk in &reasoning_chunks {
        reasoning_sse.push_str(&build_response_json(&json!(chunk)));
    }
    reasoning_sse.push_str("data: [DONE]\n\n");

    let mut answer_sse = String::new();
    for chunk in &sample_answer_chunks() {
        answer_sse.push_str(&build_response_json(&json!(chunk)));
    }
    answer_sse.push_str("data: [DONE]\n\n");

    use reqwest::header::{CONTENT_TYPE, HeaderValue};

    let reasoning_bytes = reasoning_sse.as_bytes().to_vec();
    let answer_bytes = answer_sse.as_bytes().to_vec();

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(reasoning_bytes.clone())
                .insert_header(CONTENT_TYPE, HeaderValue::from_static("text/event-stream")),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(answer_bytes)
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

    let mut chunks_received = 0;
    let timeout = tokio::time::Duration::from_secs(3);
    let start_time = tokio::time::Instant::now();

    loop {
        match tokio::time::timeout(timeout, receiver.recv()).await {
            Ok(Some(result)) => match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    if !text.contains("[DONE]") {
                        chunks_received += 1;
                    }
                }
                Err(_) => break,
            },
            Ok(None) => break,
            Err(_) => {
                assert!(chunks_received > 0, "Expected to receive some chunks before timeout");
                break;
            }
        }
        if start_time.elapsed() > timeout {
            break;
        }
    }

    assert!(chunks_received > 0, "Expected to receive chunks before timeout");
}

#[tokio::test]
async fn test_integration_http_error_401_unauthorized() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": {
                "message": "Invalid API key",
                "type": "invalid_request_error"
            }
        })))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);
    let request = sample_chat_request();

    let result = service.create_completion(request, &model_config).await;

    assert!(result.is_err(), "Expected error from 401 response");
    match result.unwrap_err() {
        adaptive_reasoner::errors::ReasonerError::ApiError(msg) => {
            assert!(msg.contains("status 401"), "Expected 401 status in error");
        }
        _ => panic!("Expected ApiError variant"),
    }
}

#[tokio::test]
async fn test_integration_http_error_403_forbidden() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "error": {
                "message": "Access forbidden",
                "type": "permission_error"
            }
        })))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);
    let request = sample_chat_request();

    let result = service.create_completion(request, &model_config).await;

    assert!(result.is_err(), "Expected error from 403 response");
    match result.unwrap_err() {
        adaptive_reasoner::errors::ReasonerError::ApiError(msg) => {
            assert!(msg.contains("status 403"), "Expected 403 status in error");
        }
        _ => panic!("Expected ApiError variant"),
    }
}

#[tokio::test]
async fn test_integration_http_error_404_not_found() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "error": {
                "message": "Model not found",
                "type": "invalid_request_error"
            }
        })))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);
    let request = sample_chat_request();

    let result = service.create_completion(request, &model_config).await;

    assert!(result.is_err(), "Expected error from 404 response");
    match result.unwrap_err() {
        adaptive_reasoner::errors::ReasonerError::ApiError(msg) => {
            assert!(msg.contains("status 404"), "Expected 404 status in error");
        }
        _ => panic!("Expected ApiError variant"),
    }
}

#[tokio::test]
async fn test_integration_http_error_429_rate_limit() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(429).set_body_json(json!({
            "error": {
                "message": "Rate limit exceeded",
                "type": "rate_limit_error"
            }
        })))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);
    let request = sample_chat_request();

    let result = service.create_completion(request, &model_config).await;

    assert!(result.is_err(), "Expected error from 429 response");
    match result.unwrap_err() {
        adaptive_reasoner::errors::ReasonerError::ApiError(msg) => {
            assert!(msg.contains("status 429"), "Expected 429 status in error");
        }
        _ => panic!("Expected ApiError variant"),
    }
}

#[tokio::test]
async fn test_integration_http_error_502_bad_gateway() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(502).set_body_json(json!({
            "error": {
                "message": "Bad gateway",
                "type": "gateway_error"
            }
        })))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);
    let request = sample_chat_request();

    let result = service.create_completion(request, &model_config).await;

    assert!(result.is_err(), "Expected error from 502 response");
    match result.unwrap_err() {
        adaptive_reasoner::errors::ReasonerError::ApiError(msg) => {
            assert!(msg.contains("status 502"), "Expected 502 status in error");
        }
        _ => panic!("Expected ApiError variant"),
    }
}

#[tokio::test]
async fn test_integration_http_error_503_service_unavailable() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(503).set_body_json(json!({
            "error": {
                "message": "Service temporarily unavailable",
                "type": "service_unavailable"
            }
        })))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);
    let request = sample_chat_request();

    let result = service.create_completion(request, &model_config).await;

    assert!(result.is_err(), "Expected error from 503 response");
    match result.unwrap_err() {
        adaptive_reasoner::errors::ReasonerError::ApiError(msg) => {
            assert!(msg.contains("status 503"), "Expected 503 status in error");
        }
        _ => panic!("Expected ApiError variant"),
    }
}

#[tokio::test]
async fn test_integration_empty_response_body() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);
    let request = sample_chat_request();

    let result = service.create_completion(request, &model_config).await;

    assert!(result.is_err(), "Expected error from empty response");
    match result.unwrap_err() {
        adaptive_reasoner::errors::ReasonerError::ParseError(_) |
        adaptive_reasoner::errors::ReasonerError::ApiError(_) => {}
        _ => panic!("Expected ParseError or ApiError variant"),
    }
}

#[tokio::test]
async fn test_integration_invalid_json_response() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{bad json"))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);
    let request = sample_chat_request();

    let result = service.create_completion(request, &model_config).await;

    assert!(result.is_err(), "Expected error from invalid JSON");
    match result.unwrap_err() {
        adaptive_reasoner::errors::ReasonerError::ParseError(_) |
        adaptive_reasoner::errors::ReasonerError::ApiError(_) => {}
        _ => panic!("Expected ParseError or ApiError variant"),
    }
}

#[tokio::test]
async fn test_integration_response_missing_required_fields() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "test-id",
            "object": "chat.completion"
        })))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);
    let request = sample_chat_request();

    let result = service.create_completion(request, &model_config).await;

    assert!(
        result.is_err(),
        "Expected error from response missing required fields"
    );
}

#[tokio::test]
async fn test_performance_concurrent_requests() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    let reasoning_response = sample_reasoning_response();
    let answer_response = sample_answer_response();

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&reasoning_response))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&answer_response))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);

    let num_requests = 10;
    let start = tokio::time::Instant::now();

    let mut handles = vec![];
    for _ in 0..num_requests {
        let service_clone = service.clone();
        let request = sample_chat_request();
        let model_config_clone = model_config.clone();

        handles.push(tokio::spawn(async move {
            let _ = service_clone
                .create_completion(request, &model_config_clone)
                .await;
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start.elapsed();

    assert!(
        duration.as_secs() < 10,
        "Concurrent requests should complete in reasonable time, took {:?}",
        duration
    );

    eprintln!("Completed {} requests in {:?}", num_requests, duration);
}

#[tokio::test]
async fn test_performance_streaming_concurrent_requests() {
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
                .set_body_bytes(reasoning_sse.clone().into_bytes())
                .insert_header(CONTENT_TYPE, HeaderValue::from_static("text/event-stream")),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(answer_sse.clone().into_bytes())
                .insert_header(CONTENT_TYPE, HeaderValue::from_static("text/event-stream")),
        )
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);

    let num_requests = 5;
    let start = tokio::time::Instant::now();

    let mut handles = vec![];
    for _ in 0..num_requests {
        let service_clone = service.clone();
        let mut request = sample_chat_request();
        request.stream = Some(true);
        let model_config_clone = model_config.clone();

        handles.push(tokio::spawn(async move {
            let (sender, mut receiver) = mpsc::channel(consts::CHANNEL_BUFFER_SIZE);
            let _ = service_clone
                .stream_completion(request, &model_config_clone, sender)
                .await;

            let mut count = 0;
            while let Some(result) = receiver.recv().await {
                if let Ok(_) = result {
                    count += 1;
                    if count > 10 {
                        break;
                    }
                }
            }
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start.elapsed();

    assert!(
        duration.as_secs() < 10,
        "Concurrent streaming requests should complete in reasonable time, took {:?}",
        duration
    );

    eprintln!("Completed {} streaming requests in {:?}", num_requests, duration);
}

#[tokio::test]
async fn test_performance_request_throughput() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    let reasoning_response = sample_reasoning_response();
    let answer_response = sample_answer_response();

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&reasoning_response))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&answer_response))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);

    let num_requests = 20;
    let start = tokio::time::Instant::now();

    let mut success_count = 0;
    let mut failure_count = 0;

    for _ in 0..num_requests {
        let service_clone = service.clone();
        let request = sample_chat_request();
        let model_config_clone = model_config.clone();

        let result = service_clone
            .create_completion(request, &model_config_clone)
            .await;

        match result {
            Ok(_) => success_count += 1,
            Err(_) => failure_count += 1,
        }
    }

    let duration = start.elapsed();
    let throughput = (success_count as f64) / duration.as_secs_f64();

    assert_eq!(
        failure_count, 0,
        "All requests should succeed, had {} failures",
        failure_count
    );

    assert!(
        throughput > 1.0,
        "Expected throughput > 1 request/second, got {:.2}",
        throughput
    );

    eprintln!(
        "Throughput: {:.2} requests/second ({} successes in {:?})",
        throughput, success_count, duration
    );
}

#[tokio::test]
async fn test_performance_memory_stress() {
    let mock_server = MockServer::start().await;
    let model_config = create_model_config(mock_server.uri());

    let reasoning_response = sample_reasoning_response();
    let answer_response = sample_answer_response();

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&reasoning_response))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&answer_response))
        .mount(&mock_server)
        .await;

    let http_client = Client::new();
    let service = ReasoningService::new(http_client);

    let num_requests = 50;
    let mut success_count = 0;

    for i in 0..num_requests {
        let service_clone = service.clone();
        let request = sample_chat_request();
        let model_config_clone = model_config.clone();

        let result = service_clone
            .create_completion(request, &model_config_clone)
            .await;

        if result.is_ok() {
            success_count += 1;
        }

        if i % 10 == 0 && i > 0 {
            eprintln!("Completed {} requests", i);
        }
    }

    assert_eq!(
        success_count, num_requests,
        "All {} requests should succeed, got {}",
        num_requests, success_count
    );

    eprintln!(
        "Memory stress test completed: {} successful requests out of {}",
        success_count, num_requests
    );
}
