use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use actix_web::mime;
use async_trait::async_trait;
use reqwest::Response;

use crate::errors::ReasonerError;
use crate::llm_client::LLMClientTrait;
use crate::models::request;

pub struct MockLLMClient {
    base_url: String,
    responses: Arc<Mutex<VecDeque<Result<Response, ReasonerError>>>>,
    calls: Arc<Mutex<Vec<request::ChatCompletionCreate>>>,
}

impl MockLLMClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            responses: Arc::new(Mutex::new(VecDeque::new())),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_response(&self, response: Result<Response, ReasonerError>) {
        self.responses.lock().unwrap().push_back(response);
    }

    pub fn get_calls(&self) -> Vec<request::ChatCompletionCreate> {
        self.calls.lock().unwrap().clone()
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[async_trait]
impl LLMClientTrait for MockLLMClient {
    async fn request_chat_completion(
        &self,
        request: request::ChatCompletionCreate,
        _expected_content_type: mime::Mime,
    ) -> Result<Response, ReasonerError> {
        self.calls.lock().unwrap().push(request);

        let client = reqwest::Client::new();
        let url = format!("{}{}", self.base_url, "/chat/completions");

        client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ReasonerError::NetworkError(e.to_string()))
    }
}
