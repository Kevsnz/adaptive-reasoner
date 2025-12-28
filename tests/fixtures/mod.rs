use crate::models::request;
use crate::models::response_direct::{ChatCompletion, Choice, MessageAssistant};
use crate::models::response_stream::{ChatCompletionChunk, ChunkChoice, ChunkChoiceDelta, Usage};
use crate::models::FinishReason;

pub fn sample_chat_request() -> request::ChatCompletionCreate {
    request::ChatCompletionCreate {
        model: "test-model".to_string(),
        messages: vec![
            request::Message::User(request::MessageUser {
                content: request::Content::Text("Hello, how are you?".to_string()),
            }),
        ],
        max_tokens: Some(100),
        temperature: Some(0.7),
        top_p: None,
        n: None,
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        user: None,
        response_format: None,
        seed: None,
        tools: None,
        tool_choice: None,
        parallel_tool_calls: None,
        stream: None,
        stream_options: None,
    }
}

pub fn empty_messages_request() -> request::ChatCompletionCreate {
    request::ChatCompletionCreate {
        model: "test-model".to_string(),
        messages: vec![],
        max_tokens: Some(100),
        temperature: Some(0.7),
        top_p: None,
        n: None,
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        user: None,
        response_format: None,
        seed: None,
        tools: None,
        tool_choice: None,
        parallel_tool_calls: None,
        stream: None,
        stream_options: None,
    }
}

pub fn assistant_last_request() -> request::ChatCompletionCreate {
    request::ChatCompletionCreate {
        model: "test-model".to_string(),
        messages: vec![
            request::Message::User(request::MessageUser {
                content: request::Content::Text("Hello".to_string()),
            }),
            request::Message::Assistant(MessageAssistant {
                reasoning_content: None,
                content: Some("I'm doing well".to_string()),
                tool_calls: None,
            }),
        ],
        max_tokens: Some(100),
        temperature: Some(0.7),
        top_p: None,
        n: None,
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        user: None,
        response_format: None,
        seed: None,
        tools: None,
        tool_choice: None,
        parallel_tool_calls: None,
        stream: None,
        stream_options: None,
    }
}

pub fn sample_reasoning_response() -> ChatCompletion {
    ChatCompletion {
        id: "chatcmpl-test-1".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567890,
        model: "test-model".to_string(),
        choices: vec![Choice {
            index: 0,
            message: MessageAssistant {
                reasoning_content: None,
                content: Some("Let me think about this carefully...".to_string()),
                tool_calls: None,
            },
            logprobs: None,
            finish_reason: FinishReason::Stop,
        }],
        usage: Usage {
            prompt_tokens: 10,
            completion_tokens: 50,
            total_tokens: 60,
        },
    }
}

pub fn sample_answer_response() -> ChatCompletion {
    ChatCompletion {
        id: "chatcmpl-test-2".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567891,
        model: "test-model".to_string(),
        choices: vec![Choice {
            index: 0,
            message: MessageAssistant {
                reasoning_content: None,
                content: Some("I'm doing great, thank you!".to_string()),
                tool_calls: None,
            },
            logprobs: None,
            finish_reason: FinishReason::Stop,
        }],
        usage: Usage {
            prompt_tokens: 10,
            completion_tokens: 30,
            total_tokens: 40,
        },
    }
}

pub fn sample_error_response() -> ChatCompletion {
    ChatCompletion {
        id: "chatcmpl-error".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567892,
        model: "test-model".to_string(),
        choices: vec![],
        usage: Usage {
            prompt_tokens: 10,
            completion_tokens: 0,
            total_tokens: 10,
        },
    }
}

pub fn sample_reasoning_chunks() -> Vec<ChatCompletionChunk> {
    vec![
        ChatCompletionChunk {
            id: "chatcmpl-test-1".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "test-model".to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: ChunkChoiceDelta {
                    content: Some("Let".to_string()),
                    role: None,
                    reasoning_content: None,
                    tool_calls: None,
                },
                logprobs: None,
                finish_reason: None,
            }],
            usage: None,
        },
        ChatCompletionChunk {
            id: "chatcmpl-test-1".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "test-model".to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: ChunkChoiceDelta {
                    content: Some(" me".to_string()),
                    role: None,
                    reasoning_content: None,
                    tool_calls: None,
                },
                logprobs: None,
                finish_reason: None,
            }],
            usage: None,
        },
        ChatCompletionChunk {
            id: "chatcmpl-test-1".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "test-model".to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: ChunkChoiceDelta {
                    content: Some(" think".to_string()),
                    role: None,
                    reasoning_content: None,
                    tool_calls: None,
                },
                logprobs: None,
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 10,
                total_tokens: 20,
            }),
        },
    ]
}

pub fn sample_answer_chunks() -> Vec<ChatCompletionChunk> {
    vec![
        ChatCompletionChunk {
            id: "chatcmpl-test-2".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567891,
            model: "test-model".to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: ChunkChoiceDelta {
                    content: Some("I'm".to_string()),
                    role: None,
                    reasoning_content: None,
                    tool_calls: None,
                },
                logprobs: None,
                finish_reason: None,
            }],
            usage: None,
        },
        ChatCompletionChunk {
            id: "chatcmpl-test-2".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567891,
            model: "test-model".to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: ChunkChoiceDelta {
                    content: Some(" doing".to_string()),
                    role: None,
                    reasoning_content: None,
                    tool_calls: None,
                },
                logprobs: None,
                finish_reason: None,
            }],
            usage: None,
        },
        ChatCompletionChunk {
            id: "chatcmpl-test-2".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567891,
            model: "test-model".to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: ChunkChoiceDelta {
                    content: Some(" great!".to_string()),
                    role: None,
                    reasoning_content: None,
                    tool_calls: None,
                },
                logprobs: None,
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 8,
                total_tokens: 18,
            }),
        },
    ]
}
