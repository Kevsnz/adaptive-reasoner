#[cfg(test)]
use crate::models::response_direct::{ChatCompletion, Choice};
use crate::models::response_stream::{ChatCompletionChunk, ChunkChoice};
use crate::models::{FinishReason, Usage};

#[cfg(test)]
pub fn assert_chat_completion_response(
    response: &ChatCompletion,
    expected_model: &str,
    expected_content: &str,
) {
    assert_eq!(&response.model, expected_model);
    assert_eq!(response.choices.len(), 1);
    
    let choice = &response.choices[0];
    assert_eq!(choice.index, 0);
    assert_eq!(choice.finish_reason, FinishReason::Stop);
    assert_eq!(
        choice.message.content.as_ref().expect("Content should be present"),
        expected_content
    );
}

#[cfg(test)]
pub fn assert_usage(
    usage: &Usage,
    expected_prompt_tokens: i32,
    expected_completion_tokens: i32,
    expected_total_tokens: i32,
) {
    assert_eq!(usage.prompt_tokens, expected_prompt_tokens);
    assert_eq!(usage.completion_tokens, expected_completion_tokens);
    assert_eq!(usage.total_tokens, expected_total_tokens);
}

#[cfg(test)]
pub fn assert_streaming_chunks(chunks: &[ChatCompletionChunk], expected_model: &str) {
    assert!(!chunks.is_empty(), "Should have at least one chunk");
    
    for chunk in chunks {
        assert_eq!(&chunk.model, expected_model);
        assert_eq!(chunk.object, "chat.completion.chunk");
    }
}

#[cfg(test)]
pub fn assert_final_chunk(chunk: &ChatCompletionChunk) {
    assert_eq!(chunk.choices.len(), 1);
    let choice = &chunk.choices[0];
    assert!(
        choice.finish_reason.is_some(),
        "Final chunk should have finish_reason set"
    );
    assert!(
        chunk.usage.is_some(),
        "Final chunk should have usage statistics"
    );
}

#[cfg(test)]
pub fn assert_choice_structure(choice: &Choice, index: i32, expected_content: &str) {
    assert_eq!(choice.index, index);
    assert_eq!(
        choice.message.content.as_ref().expect("Content should be present"),
        expected_content
    );
    assert_eq!(choice.finish_reason, FinishReason::Stop);
    assert!(choice.logprobs.is_none());
}

#[cfg(test)]
pub fn assert_chunk_choice_structure(
    chunk_choice: &ChunkChoice,
    index: i32,
    expected_content: Option<&str>,
) {
    assert_eq!(chunk_choice.index, index);
    assert_eq!(chunk_choice.delta.content.as_deref(), expected_content);
    assert!(chunk_choice.logprobs.is_none());
}
