use serde::{self, Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ImageUrl {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum MessageContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub(crate) enum MessageContent {
    String(String),
    Array(Vec<MessageContentPart>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct MessageSystemUser {
    pub(crate) content: MessageContent,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct MessageAssistant {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) tool_calls: Option<Vec<Value>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct MessageTool {
    pub(crate) tool_call_id: String,
    pub(crate) content: MessageContent,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "role", rename_all = "snake_case")]
pub(crate) enum Message {
    User(MessageSystemUser),
    System(MessageSystemUser),
    Assistant(MessageAssistant),
    Tool(MessageTool),
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub(crate) struct StreamOptions {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) include_usage: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ChatCompletionCreate {
    pub(crate) model: String,
    pub(crate) messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) stream_options: Option<StreamOptions>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) tool_choice: Option<ToolChoice>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ToolChoice {
    Auto,
    None,
    Required,
}
