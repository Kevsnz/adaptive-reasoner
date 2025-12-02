use crate::models::request::MessageAssistant;

use super::{FinishReason, LogProbs, Usage};
use serde::{self, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Choice {
    pub(crate) index: i32,
    pub(crate) message: MessageAssistant,
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
