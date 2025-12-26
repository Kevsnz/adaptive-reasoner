use std::collections::HashMap;

use actix_web::mime;
use reqwest::Response;
use serde_json::Value;

use crate::errors::ReasonerError;
use crate::models::request;

pub(crate) struct LLMClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    extra_body: Option<HashMap<String, Value>>,
}

impl LLMClient {
    pub(crate) fn new(
        client: reqwest::Client,
        base_url: &str,
        api_key: &str,
        extra_body: &Option<HashMap<String, Value>>,
    ) -> Self {
        Self {
            client,
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            extra_body: extra_body.clone(),
        }
    }
    pub(crate) async fn request_chat_completion(
        &self,
        mut request: request::ChatCompletionCreate,
        expected_content_type: mime::Mime,
    ) -> Result<Response, ReasonerError> {
        if let Some(extra_body) = self.extra_body.clone() {
            request.extra = extra_body;
        }

        let response = self
            .client
            .post(format!("{}{}", self.base_url, "/chat/completions"))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();

            return Err(ReasonerError::ApiError(format!(
                "error: status {status}, text {text}"
            )));
        }

        let content_type: mime::Mime = response.headers()[reqwest::header::CONTENT_TYPE]
            .to_str()?
            .parse()?;
        if content_type.essence_str() != expected_content_type.essence_str() {
            return Err(ReasonerError::ParseError(format!(
                "content-type: {content_type}, expected: {expected_content_type}"
            )));
        }

        Ok(response)
    }
}
