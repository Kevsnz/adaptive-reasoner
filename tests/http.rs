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
