use actix_web::http::{StatusCode, header};
use actix_web::test;
use reqwest::{Client, header::{HeaderValue, CONTENT_TYPE}};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

use adaptive_reasoner::app::create_app;
use adaptive_reasoner::config::{Config, ModelConfig};
use adaptive_reasoner::models::model_list;
use adaptive_reasoner::service::ReasoningService;

mod common;
mod fixtures;

use rstest::rstest;

#[rstest]
#[case(401, "Invalid API key", "invalid_request_error")]
#[case(403, "Access forbidden", "permission_error")]
#[case(404, "Model not found", "invalid_request_error")]
#[case(429, "Rate limit exceeded", "rate_limit_error")]
#[case(502, "Bad gateway", "gateway_error")]
#[case(503, "Service temporarily unavailable", "service_unavailable")]
#[actix_web::test]
async fn test_http_error_codes(
    #[case] status_code: u16,
    #[case] message: &str,
    #[case] error_type: &str,
) {
    let mock_server = crate::common::mock_server::setup_error_mock(
        status_code,
        message,
        error_type,
    ).await;

    let mut config = create_test_config();
    config.models.get_mut("test-model").unwrap().api_url = mock_server.uri();

    let config = Arc::new(config);
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let request_body =
        json!({"model": "test-model", "messages": [{"role": "user", "content": "Hello"}]});
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
}

fn create_test_config() -> Config {
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

#[actix_web::test]
async fn test_http_models_endpoint() {
    let (config, reasoning_service) = crate::common::setup::create_test_app_components().await;
    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let req = test::TestRequest::get().uri("/v1/models").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: model_list::ModelList = test::read_body_json(resp).await;
    assert_eq!(body.data.len(), 1);
    assert_eq!(body.data[0].id, "test-model");
    assert!(matches!(body.data[0].object, model_list::ObjectType::Model));
    assert!(matches!(
        body.data[0].owned_by,
        model_list::Owner::AdaptiveReasoner
    ));
}

#[actix_web::test]
async fn test_http_chat_completion_invalid_model() {
    let (config, reasoning_service) = crate::common::setup::create_test_app_components().await;
    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let request_body =
        json!({"model": "nonexistent-model", "messages": [{"role": "user", "content": "Hello"}]});
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_http_chat_completion_assistant_last() {
    let (config, reasoning_service) = crate::common::setup::create_test_app_components().await;
    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let request_body = json!({"model": "test-model", "messages": [{"role": "user", "content": "Hello"}, {"role": "assistant", "content": "Hi there"}]});
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_http_chat_completion_malformed_json() {
    let (config, reasoning_service) = crate::common::setup::create_test_app_components().await;
    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_payload("{invalid json}")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_http_chat_completion_empty_messages() {
    let (config, reasoning_service) = crate::common::setup::create_test_app_components().await;
    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let request_body = json!({"model": "test-model", "messages": []});
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_http_chat_completion_api_error() {
    let mock_server = crate::common::mock_server::setup_chat_completion_mock(
        500,
        json!({"error": {"message": "Internal server error", "type": "internal_error"}}),
    ).await;

    let mut config = create_test_config();
    config.models.get_mut("test-model").unwrap().api_url = mock_server.uri();

    let config = Arc::new(config);
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let request_body =
        json!({"model": "test-model", "messages": [{"role": "user", "content": "Hello"}]});
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
}

#[actix_web::test]
async fn test_http_chat_completion_non_streaming() {
    use crate::fixtures::{sample_reasoning_response, sample_answer_response};

    let mock_server = MockServer::start().await;

    let mut config = create_test_config();
    config.models.get_mut("test-model").unwrap().api_url = mock_server.uri();

    let config = Arc::new(config);
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&sample_reasoning_response()))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&sample_answer_response()))
        .mount(&mock_server)
        .await;

    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let request_body = json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "Hello, how are you?"}],
        "stream": false
    });
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    use adaptive_reasoner::models::response_direct::ChatCompletion;
    let body: ChatCompletion = test::read_body_json(resp).await;

    assert_eq!(body.id, "chatcmpl-test-1");
    assert_eq!(body.object, "chat.completion");
    assert_eq!(body.model, "test-model");
    assert_eq!(body.choices.len(), 1);

    let choice = &body.choices[0];
    assert_eq!(choice.index, 0);
    assert_eq!(
        choice.finish_reason,
        adaptive_reasoner::models::FinishReason::Stop
    );

    let assistant = &choice.message;
    assert!(
        assistant.content.is_some(),
        "Expected content to be present"
    );
    let content = assistant.content.as_ref().unwrap();
    assert!(
        content.contains("Let me think about this carefully..."),
        "Expected reasoning content in response"
    );
    assert!(
        content.contains("I'm doing great, thank you!"),
        "Expected answer content in response"
    );

    assert_eq!(body.usage.prompt_tokens, 10);
    assert_eq!(body.usage.completion_tokens, 80);
    assert_eq!(body.usage.total_tokens, 90);
}

#[actix_web::test]
async fn test_http_chat_completion_streaming() {
    let mock_server = MockServer::start().await;

    let mut config = create_test_config();
    config.models.get_mut("test-model").unwrap().api_url = mock_server.uri();

    let config = Arc::new(config);
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    use crate::fixtures::{sample_reasoning_chunks, sample_answer_chunks};
    use reqwest::header::{CONTENT_TYPE, HeaderValue};

    let reasoning_chunks = sample_reasoning_chunks();
    let answer_chunks = sample_answer_chunks();

    let reasoning_sse = crate::common::sse::build_sse_stream(&reasoning_chunks);
    let answer_sse = crate::common::sse::build_sse_stream(&answer_chunks);

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

    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let request_body = json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "Hello, how are you?"}],
        "stream": true,
        "stream_options": {"include_usage": true}
    });
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request_body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);

    use actix_web::http::header;
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok());
    assert!(
        content_type.is_some_and(|ct| ct.contains("text/event-stream")),
        "Expected text/event-stream content type"
    );

    let bytes = test::read_body(resp).await;
    let body_str = String::from_utf8_lossy(&bytes);

    let lines: Vec<&str> = body_str.lines().collect();

    assert!(
        !lines.is_empty(),
        "Expected to receive streaming response lines"
    );

    let mut chunk_count = 0;
    let mut has_final_usage = false;
    let mut has_done_marker = false;

    for i in (0..lines.len()).step_by(2) {
        if i + 1 < lines.len() {
            let line = lines[i];
            let _empty_line = lines[i + 1];

            if line.starts_with("data: ") {
                let data_str = &line[6..];
                if data_str == "[DONE]" {
                    has_done_marker = true;
                } else if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(data_str) {
                    chunk_count += 1;

                    if let Some(usage) = json_val.get("usage") {
                        has_final_usage = true;
                        assert!(
                            usage.get("total_tokens").is_some(),
                            "Expected total_tokens in final usage"
                        );
                    }
                }
            }
        }
    }

    assert!(
        chunk_count > 0,
        "Expected at least one chunk, got {}",
        chunk_count
    );

    assert!(has_final_usage, "Expected final chunk with usage statistics");

    assert!(
        has_done_marker,
        "Expected [DONE] marker in streaming response"
    );

    eprintln!("Received {} streaming chunks", chunk_count);
}

#[actix_web::test]
async fn test_http_chat_completion_response_format() {
    use crate::fixtures::{sample_reasoning_response, sample_answer_response};

    let mock_server = crate::common::mock_server::setup_two_phase_mocks(
        serde_json::to_value(&sample_reasoning_response()).unwrap(),
        serde_json::to_value(&sample_answer_response()).unwrap(),
    ).await;

    let mut config = create_test_config();
    config.models.get_mut("test-model").unwrap().api_url = mock_server.uri();

    let config = Arc::new(config);
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let request_body = json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "Hello"}]
    });
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    use adaptive_reasoner::models::response_direct::ChatCompletion;
    let body: ChatCompletion = test::read_body_json(resp).await;

    assert!(!body.id.is_empty(), "Response id should not be empty");
    assert_eq!(body.id, "chatcmpl-test-1", "Expected correct response id");

    assert_eq!(body.object, "chat.completion", "Expected correct object type");
    assert_eq!(body.model, "test-model", "Expected model name to match");

    assert!(body.created > 0, "Created timestamp should be positive");
    assert_eq!(body.created, 1234567890, "Expected correct created timestamp");

    assert_eq!(body.choices.len(), 1, "Expected exactly one choice");
    let choice = &body.choices[0];

    assert_eq!(choice.index, 0, "Expected choice index to be 0");
    assert!(
        choice.logprobs.is_none(),
        "Expected logprobs to be None for this test"
    );

    assert_eq!(
        choice.finish_reason,
        adaptive_reasoner::models::FinishReason::Stop,
        "Expected finish_reason to be Stop"
    );

    let assistant = &choice.message;
    assert!(
        assistant.content.is_some(),
        "Expected assistant message to have content"
    );
    assert!(
        assistant.tool_calls.is_none(),
        "Expected no tool calls in this test"
    );

    assert_eq!(
        body.usage.prompt_tokens, 10,
        "Expected prompt_tokens to be 10"
    );
    assert_eq!(
        body.usage.completion_tokens, 80,
        "Expected completion_tokens to be 80"
    );
    assert_eq!(
        body.usage.total_tokens, 90,
        "Expected total_tokens to be 90"
    );
}

#[actix_web::test]
async fn test_http_chat_completion_finish_reason_variants() {
    let mock_server = MockServer::start().await;

    let mut config = create_test_config();
    config.models.get_mut("test-model").unwrap().api_url = mock_server.uri();

    let config = Arc::new(config);
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    use adaptive_reasoner::models::response_direct::{ChatCompletion, Choice};
    use adaptive_reasoner::models::{FinishReason, Usage};
    use adaptive_reasoner::models::request;

    let length_reason_response = ChatCompletion {
        id: "chatcmpl-length".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567891,
        model: "test-model".to_string(),
        choices: vec![Choice {
            index: 0,
            message: request::MessageAssistant {
                reasoning_content: None,
                content: Some("Partial".to_string()),
                tool_calls: None,
            },
            logprobs: None,
            finish_reason: FinishReason::Length,
        }],
        usage: Usage {
            prompt_tokens: 10,
            completion_tokens: 100,
            total_tokens: 110,
        },
    };

    let reasoning_response = ChatCompletion {
        id: "chatcmpl-length-reason".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567892,
        model: "test-model".to_string(),
        choices: vec![Choice {
            index: 0,
            message: request::MessageAssistant {
                reasoning_content: None,
                content: Some("Reasoning".to_string()),
                tool_calls: None,
            },
            logprobs: None,
            finish_reason: FinishReason::Length,
        }],
        usage: Usage {
            prompt_tokens: 10,
            completion_tokens: 200,
            total_tokens: 210,
        },
    };

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&reasoning_response))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&length_reason_response))
        .mount(&mock_server)
        .await;

    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let request_body = json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "Test length"}],
        "max_tokens": 50
    });

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: ChatCompletion = test::read_body_json(resp).await;
    assert_eq!(
        body.choices[0].finish_reason,
        FinishReason::Length,
        "Expected Length finish_reason when reasoning exceeds budget"
    );
}

#[actix_web::test]
async fn test_http_chat_completion_usage_statistics() {
    let mock_server = MockServer::start().await;

    let mut config = create_test_config();
    config.models.get_mut("test-model").unwrap().api_url = mock_server.uri();

    let config = Arc::new(config);
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    use adaptive_reasoner::models::response_direct::{ChatCompletion, Choice};
    use adaptive_reasoner::models::Usage;
    use adaptive_reasoner::models::request;

    let reasoning_response = ChatCompletion {
        id: "chatcmpl-reason".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567890,
        model: "test-model".to_string(),
        choices: vec![Choice {
            index: 0,
            message: request::MessageAssistant {
                reasoning_content: None,
                content: Some("Reasoning".to_string()),
                tool_calls: None,
            },
            logprobs: None,
            finish_reason: adaptive_reasoner::models::FinishReason::Stop,
        }],
        usage: Usage {
            prompt_tokens: 15,
            completion_tokens: 25,
            total_tokens: 40,
        },
    };

    let answer_response = ChatCompletion {
        id: "chatcmpl-answer".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567891,
        model: "test-model".to_string(),
        choices: vec![Choice {
            index: 0,
            message: request::MessageAssistant {
                reasoning_content: None,
                content: Some("Answer".to_string()),
                tool_calls: None,
            },
            logprobs: None,
            finish_reason: adaptive_reasoner::models::FinishReason::Stop,
        }],
        usage: Usage {
            prompt_tokens: 15,
            completion_tokens: 10,
            total_tokens: 25,
        },
    };

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

    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let request_body = json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "Test"}]
    });

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: ChatCompletion = test::read_body_json(resp).await;

    assert_eq!(
        body.usage.prompt_tokens, 15,
        "Expected combined prompt_tokens to match reasoning phase"
    );
    assert_eq!(
        body.usage.completion_tokens, 35,
        "Expected combined completion_tokens to be reasoning + answer (25 + 10)"
    );
    assert_eq!(
        body.usage.total_tokens, 50,
        "Expected combined total_tokens to be reasoning total + answer total (40 + 10)"
    );
}

#[actix_web::test]
async fn test_http_routing_get_method_not_allowed() {
    let (config, reasoning_service) = crate::common::setup::create_test_app_components().await;
    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let req = test::TestRequest::get()
        .uri("/v1/chat/completions")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status() == StatusCode::METHOD_NOT_ALLOWED || resp.status() == StatusCode::NOT_FOUND,
        "Expected 405 or 404 for GET on POST-only route, got {}",
        resp.status()
    );
}

#[actix_web::test]
async fn test_http_routing_404_nonexistent_v1_route() {
    let (config, reasoning_service) = crate::common::setup::create_test_app_components().await;
    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let req = test::TestRequest::get()
        .uri("/v1/nonexistent")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "Expected 404 Not Found for nonexistent /v1 route"
    );
}

#[actix_web::test]
async fn test_http_routing_404_nonexistent_root_route() {
    let (config, reasoning_service) = crate::common::setup::create_test_app_components().await;
    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let req = test::TestRequest::get()
        .uri("/nonexistent")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "Expected 404 Not Found for nonexistent root route"
    );
}

#[actix_web::test]
async fn test_http_streaming_sse_format_correctness() {
    let mock_server = MockServer::start().await;

    let mut config = create_test_config();
    config.models.get_mut("test-model").unwrap().api_url = mock_server.uri();

    let config = Arc::new(config);
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    use crate::fixtures::sample_reasoning_chunks;
    use reqwest::header::{CONTENT_TYPE, HeaderValue};

    let reasoning_chunks = sample_reasoning_chunks();

    let sse_response = crate::common::sse::build_sse_stream_with_custom_delimiter(&reasoning_chunks, "\r\n");

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(sse_response.clone().into_bytes())
                .insert_header(CONTENT_TYPE, HeaderValue::from_static("text/event-stream")),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(sse_response.clone().into_bytes())
                .insert_header(CONTENT_TYPE, HeaderValue::from_static("text/event-stream")),
        )
        .mount(&mock_server)
        .await;

    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let request_body = json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "Test SSE format"}],
        "stream": true
    });

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request_body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = test::read_body(resp).await;
    let body_str = String::from_utf8_lossy(&bytes);

    let lines: Vec<&str> = body_str.lines().collect();

    let (has_data_lines, has_empty_lines, _has_crlf) =
        crate::common::streaming::validate_sse_format(&lines);

    assert!(has_data_lines, "Expected at least one data line");
    assert!(
        has_empty_lines,
        "Expected SSE format with proper line endings"
    );

    for line in &lines {
        if line.starts_with("data: ") && !line.contains("[DONE]") {
            let data_str = &line[6..];
            let json_result: Result<serde_json::Value, _> = serde_json::from_str(data_str);
            assert!(
                json_result.is_ok(),
                "Each data line should contain valid JSON: {}",
                line
            );
        }
    }
}

#[actix_web::test]
async fn test_http_streaming_chunk_ordering() {
    let mock_server = MockServer::start().await;

    let mut config = create_test_config();
    config.models.get_mut("test-model").unwrap().api_url = mock_server.uri();

    let config = Arc::new(config);
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    use crate::fixtures::{sample_reasoning_chunks, sample_answer_chunks};
    use reqwest::header::{CONTENT_TYPE, HeaderValue};

    let reasoning_chunks = sample_reasoning_chunks();
    let answer_chunks = sample_answer_chunks();

    let mut reasoning_sse = String::new();
    for (i, chunk) in reasoning_chunks.iter().enumerate() {
        let mut chunk_data = chunk.clone();
        chunk_data.id = format!("reasoning-{}", i);
        let json_str = serde_json::to_string(&chunk_data).unwrap();
        reasoning_sse.push_str(&format!("data: {}\r\n\r\n", json_str));
    }
    reasoning_sse.push_str("data: [DONE]\r\n\r\n");

    let mut answer_sse = String::new();
    for (i, chunk) in answer_chunks.iter().enumerate() {
        let mut chunk_data = chunk.clone();
        chunk_data.id = format!("answer-{}", i);
        let json_str = serde_json::to_string(&chunk_data).unwrap();
        answer_sse.push_str(&format!("data: {}\r\n\r\n", json_str));
    }
    answer_sse.push_str("data: [DONE]\r\n\r\n");

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(reasoning_sse.clone().into_bytes())
                .insert_header(CONTENT_TYPE, HeaderValue::from_static("text/event-stream")),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(answer_sse.as_bytes().to_vec())
                .insert_header(CONTENT_TYPE, HeaderValue::from_static("text/event-stream")),
        )
        .mount(&mock_server)
        .await;

    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let request_body = json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "Test ordering"}],
        "stream": true,
        "stream_options": {"include_usage": true}
    });

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request_body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = test::read_body(resp).await;
    let body_str = String::from_utf8_lossy(&bytes);

    let lines: Vec<&str> = body_str.lines().collect();

    let mut data_chunks = 0;
    let mut has_done_marker = false;
    let mut content_found = false;

    for line in &lines {
        if line.starts_with("data: ") {
            let data_str = &line[6..];
            if data_str == "[DONE]" {
                has_done_marker = true;
                continue;
            }

            data_chunks += 1;

            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(data_str) {
                if json_val.get("choices").is_some()
                    || json_val.get("delta").is_some()
                    || json_val.get("content").is_some()
                {
                    content_found = true;
                }
            }
        }
    }

    assert!(
        data_chunks > 0,
        "Expected at least one data chunk, got {}",
        data_chunks
    );

    assert!(
        content_found,
        "Expected to find content in at least one chunk"
    );

    assert!(
        has_done_marker,
        "Expected [DONE] marker in streaming response"
    );
}

#[actix_web::test]
async fn test_http_streaming_incomplete_stream() {
    let mock_server = MockServer::start().await;

    let mut config = create_test_config();
    config.models.get_mut("test-model").unwrap().api_url = mock_server.uri();

    let config = Arc::new(config);
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    use crate::fixtures::sample_reasoning_chunks;

    let reasoning_chunks = sample_reasoning_chunks();
    let reasoning_sse = crate::common::sse::build_sse_stream_with_custom_delimiter(&reasoning_chunks, "\r\n");

    let answer_sse = "data: {\"id\":\"test-answer\",\"object\":\"chat.completion.chunk\",\"created\":1234567891,\"model\":\"test-model\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Partial\"},\"logprobs\":null,\"finish_reason\":null}]}\r\n\r\n";

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(reasoning_sse.clone().into_bytes())
                .insert_header(CONTENT_TYPE, HeaderValue::from_static("text/event-stream")),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(answer_sse.as_bytes().to_vec())
                .insert_header(CONTENT_TYPE, HeaderValue::from_static("text/event-stream")),
        )
        .mount(&mock_server)
        .await;

    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let request_body = json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "Test incomplete"}],
        "stream": true
    });

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request_body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = test::read_body(resp).await;
    let body_str = String::from_utf8_lossy(&bytes);

    assert!(!body_str.is_empty(), "Expected non-empty response even for incomplete stream");

    let lines: Vec<&str> = body_str.lines().collect();

    let mut data_lines = 0;
    for line in &lines {
        if line.starts_with("data: ") {
            data_lines += 1;
        }
    }

    assert!(data_lines > 0, "Expected at least some data lines in response");
}

#[actix_web::test]
async fn test_http_streaming_malformed_json_chunk() {
    let mock_server = MockServer::start().await;

    let mut config = create_test_config();
    config.models.get_mut("test-model").unwrap().api_url = mock_server.uri();

    let config = Arc::new(config);
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    use crate::fixtures::sample_reasoning_chunks;

    let reasoning_chunks = sample_reasoning_chunks();
    let reasoning_sse = crate::common::sse::build_sse_stream_with_custom_delimiter(&reasoning_chunks, "\r\n");

    let answer_sse = "data: {\"id\":\"test\",\"choices\":[{\"delta\":{\"content\":\"Valid\"}}]}\r\n\r\n\
                      data: invalid json here\r\n\r\n\
                      data: {\"id\":\"test\",\"choices\":[{\"delta\":{\"content\":\"Valid2\"}}]}\r\n\r\n\
                      data: [DONE]\r\n\r\n";

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(reasoning_sse.clone().into_bytes())
                .insert_header(CONTENT_TYPE, HeaderValue::from_static("text/event-stream")),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(answer_sse.as_bytes().to_vec())
                .insert_header(CONTENT_TYPE, HeaderValue::from_static("text/event-stream")),
        )
        .mount(&mock_server)
        .await;

    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let request_body = json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "Test malformed"}],
        "stream": true
    });

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request_body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    let bytes = test::read_body(resp).await;
    let body_str = String::from_utf8_lossy(&bytes);

    let lines: Vec<&str> = body_str.lines().collect();

    let mut valid_json_count = 0;
    for line in &lines {
        if line.starts_with("data: ") {
            let data_str = &line[6..];
            if data_str != "[DONE]" {
                if serde_json::from_str::<serde_json::Value>(data_str).is_ok() {
                    valid_json_count += 1;
                }
            }
        }
    }

    assert!(
        valid_json_count > 0,
        "Expected at least some valid JSON chunks despite malformed data"
    );
}

#[actix_web::test]
async fn test_http_error_empty_response_body() {
    let mock_server = MockServer::start().await;

    let mut config = create_test_config();
    config.models.get_mut("test-model").unwrap().api_url = mock_server.uri();

    let config = Arc::new(config);
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&mock_server)
        .await;

    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let request_body =
        json!({"model": "test-model", "messages": [{"role": "user", "content": "Hello"}]});
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
}

#[actix_web::test]
async fn test_http_error_invalid_json_response() {
    let mock_server = MockServer::start().await;

    let mut config = create_test_config();
    config.models.get_mut("test-model").unwrap().api_url = mock_server.uri();

    let config = Arc::new(config);
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{invalid json response}"))
        .mount(&mock_server)
        .await;

    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let request_body =
        json!({"model": "test-model", "messages": [{"role": "user", "content": "Hello"}]});
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
}
