use serde_json::{json, Value};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

use reqwest::header::{CONTENT_TYPE, HeaderValue};

pub async fn setup_chat_completion_mock(status: u16, body: impl Into<Value>) -> MockServer {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(status).set_body_json(body.into()))
        .mount(&mock_server)
        .await;

    mock_server
}

pub async fn setup_two_phase_mocks(
    reasoning: Value,
    answer: Value,
) -> MockServer {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(reasoning))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(answer))
        .mount(&mock_server)
        .await;

    mock_server
}

pub async fn setup_streaming_mocks(reasoning_sse: String, answer_sse: String) -> MockServer {
    let mock_server = MockServer::start().await;

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

    mock_server
}

pub async fn setup_error_mock(
    status_code: u16,
    error_message: &str,
    error_type: &str,
) -> MockServer {
    let mock_server = MockServer::start().await;

    let error_body = json!({
        "error": {
            "message": error_message,
            "type": error_type
        }
    });

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(status_code).set_body_json(error_body))
        .mount(&mock_server)
        .await;

    mock_server
}
