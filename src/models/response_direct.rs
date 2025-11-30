use super::{FinishReason, LogProbs, Role, ToolCall, Usage};
use serde::{self, Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Message {
    pub(crate) role: Role,
    pub(crate) content: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) tool_calls: Option<Vec<Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Choice {
    pub(crate) index: i32,
    pub(crate) message: Message,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) logprobs: Option<LogProbs>,
    pub(crate) finish_reason: FinishReason,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ChatCompletion {
    pub(crate) id: String,
    pub(crate) object: String,
    pub(crate) created: i64,
    pub(crate) model: String,
    pub(crate) choices: Vec<Choice>,
    pub(crate) usage: Usage,
}
