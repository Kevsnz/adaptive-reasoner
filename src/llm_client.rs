use actix_web::mime;
use reqwest::Response;

use crate::models::request;

pub(crate) struct LLMClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl LLMClient {
    pub(crate) fn new(client: reqwest::Client, base_url: &str, api_key: &str) -> Self {
        Self {
            client,
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
        }
    }
    pub(crate) async fn request_chat_completion(
        &self,
        request: request::ChatCompletionCreate,
        expected_content_type: mime::Mime,
    ) -> Result<Response, Box<dyn std::error::Error>> {
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

            return Err(format!("error: status {status}, text {text}").into());
        }

        let content_type: mime::Mime = response.headers()[reqwest::header::CONTENT_TYPE]
            .to_str()?
            .parse()?;
        if content_type.essence_str() != expected_content_type.essence_str() {
            return Err(
                format!("content-type: {content_type}, expected: {expected_content_type}").into(),
            );
        }

        Ok(response)
    }
}
