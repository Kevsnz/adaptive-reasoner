use super::{FinishReason, LogProbs, Role, Usage};
use serde::{self, Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ChunkChoiceDelta {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub role: Option<Role>,
    #[cfg(reasoning)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_calls: Option<Vec<Value>>,
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
pub struct ChunkChoice {
    pub index: i32,
    pub delta: ChunkChoiceDelta,
    #[serde(default)]
    pub logprobs: Option<LogProbs>,
    #[serde(default)]
    pub finish_reason: Option<FinishReason>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChunkChoice>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub usage: Option<Usage>,
}
