use super::{FinishReason, LogProbs, Role, Usage};
use serde::{self, Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub(crate) struct ChunkChoiceDelta {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) role: Option<Role>,
    #[cfg(reasoning)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) tool_calls: Option<Vec<Value>>,
}

impl ChunkChoiceDelta {
    #[cfg(reasoning)]
    pub(crate) fn chunk_choice_delta_opening() -> ChunkChoiceDelta {
        ChunkChoiceDelta {
            role: Some(Role::Assistant),
            ..Default::default()
        }
    }

    #[cfg(not(reasoning))]
    pub(crate) fn chunk_choice_delta_opening() -> ChunkChoiceDelta {
        ChunkChoiceDelta {
            role: Some(Role::Assistant),
            content: Some(crate::consts::THINK_START.to_string()),
            ..Default::default()
        }
    }

    pub(crate) fn chunk_choice_delta_empty() -> ChunkChoiceDelta {
        ChunkChoiceDelta {
            content: Some("".to_string()),
            ..Default::default()
        }
    }

    #[cfg(reasoning)]
    pub(crate) fn chunk_choice_delta_reasoning(reasoning_content: String) -> ChunkChoiceDelta {
        ChunkChoiceDelta {
            reasoning_content: Some(reasoning_content),
            ..Default::default()
        }
    }

    #[cfg(not(reasoning))]
    pub(crate) fn chunk_choice_delta_reasoning(reasoning_content: String) -> ChunkChoiceDelta {
        ChunkChoiceDelta {
            content: Some(reasoning_content),
            ..Default::default()
        }
    }
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
