use super::{FinishReason, LogProbs, Role, Usage};
use serde::{self, Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ChunkChoiceDelta {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) role: Option<Role>,
    // #[serde(skip_serializing_if = "Option::is_none", default)]
    // pub(crate) reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) tool_calls: Option<Vec<Value>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ChunkChoice {
    pub(crate) index: i32,
    pub(crate) delta: ChunkChoiceDelta,
    #[serde(default)]
    pub(crate) logprobs: Option<LogProbs>,
    #[serde(default)]
    pub(crate) finish_reason: Option<FinishReason>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ChatCompletionChunk {
    pub(crate) id: String,
    pub(crate) object: String,
    pub(crate) created: i64,
    pub(crate) model: String,
    pub(crate) choices: Vec<ChunkChoice>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) usage: Option<Usage>,
}
