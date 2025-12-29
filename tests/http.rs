use actix_web::http::{StatusCode, header};
use actix_web::test;
use reqwest::Client;
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

mod fixtures;

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
    let config = Arc::new(create_test_config());
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

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
    let config = Arc::new(create_test_config());
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

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
    let config = Arc::new(create_test_config());
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

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
    let config = Arc::new(create_test_config());
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

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
    let config = Arc::new(create_test_config());
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

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
    let mock_server = MockServer::start().await;

    let mut config = create_test_config();
    config.models.get_mut("test-model").unwrap().api_url = mock_server.uri();

    let config = Arc::new(config);
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_json(
            json!({"error": {"message": "Internal server error", "type": "internal_error"}}),
        ))
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
async fn test_http_chat_completion_non_streaming() {
    let mock_server = MockServer::start().await;

    let mut config = create_test_config();
    config.models.get_mut("test-model").unwrap().api_url = mock_server.uri();

    let config = Arc::new(config);
    let http_client = Client::new();
    let reasoning_service = Arc::new(ReasoningService::new(http_client));

    use crate::fixtures::{sample_reasoning_response, sample_answer_response};

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

    let mut reasoning_sse = String::new();
    for chunk in &reasoning_chunks {
        reasoning_sse.push_str(&format!("data: {}\n\n", serde_json::to_string(chunk).unwrap()));
    }
    reasoning_sse.push_str("data: [DONE]\n\n");

    let mut answer_sse = String::new();
    for chunk in &answer_chunks {
        answer_sse.push_str(&format!("data: {}\n\n", serde_json::to_string(chunk).unwrap()));
    }
    answer_sse.push_str("data: [DONE]\n\n");

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
