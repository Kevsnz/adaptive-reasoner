pub mod model_list;
pub mod request;
pub mod response_direct;
pub mod response_stream;

use serde::{self, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FinishReason {
    Stop,
    Length,
    ToolCalls,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Usage {
    pub(crate) prompt_tokens: i32,
    pub(crate) completion_tokens: i32,
    pub(crate) total_tokens: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct LogProbs {
    pub(crate) tokens: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ToolCall {
    pub(crate) name: String,
    pub(crate) arguments: String,
    pub(crate) function_call: String,
}
