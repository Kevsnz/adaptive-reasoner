use std::collections::HashMap;

use serde::{self, Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum MessageContent {
    String(String),
    Array(Vec<MessageContentPart>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageSystemUser {
    pub content: MessageContent,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageAssistant {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_calls: Option<Vec<Value>>,
}

impl MessageAssistant {
    #[cfg(reasoning)]
    pub(crate) fn new(
        reasoning_content: String,
        content: String,
        tool_calls: Option<Vec<Value>>,
    ) -> MessageAssistant {
        MessageAssistant {
            reasoning_content: Some(reasoning_content),
            content: Some(content),
            tool_calls: tool_calls,
        }
    }

    #[cfg(not(reasoning))]
    pub(crate) fn new(
        reasoning_content: String,
        content: String,
        tool_calls: Option<Vec<serde_json::Value>>,
    ) -> MessageAssistant {
        MessageAssistant {
            reasoning_content: None,
            content: Some(format!(
                "{}\n{}\n{}\n{}",
                crate::consts::THINK_START,
                reasoning_content.trim(),
                crate::consts::THINK_END,
                content
            )),
            tool_calls: tool_calls,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageTool {
    pub tool_call_id: String,
    pub content: MessageContent,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum Message {
    User(MessageSystemUser),
    System(MessageSystemUser),
    Assistant(MessageAssistant),
    Tool(MessageTool),
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct StreamOptions {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub include_usage: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatCompletionCreate {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub stream_options: Option<StreamOptions>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_choice: Option<ToolChoice>,
    #[serde(flatten, skip_deserializing, default)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    None,
    Required,
}
