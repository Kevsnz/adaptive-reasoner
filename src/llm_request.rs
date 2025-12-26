use std::collections::VecDeque;

use actix_web::mime;
use actix_web::web::Bytes;
use reqwest::Error;
use tokio::sync::mpsc::Sender;

use crate::config;
use crate::consts;
use crate::errors::ReasonerError;
use crate::llm_client::LLMClient;
use crate::models::FinishReason;
use crate::models::Usage;
use crate::models::request;
use crate::models::request::ChatCompletionCreate;
use crate::models::response_direct;
use crate::models::response_direct::ChatCompletion;
use crate::models::response_stream;
use crate::models::response_stream::ChatCompletionChunk;
use crate::models::response_stream::ChunkChoiceDelta;

pub(crate) fn calculate_remaining_tokens(max_tokens: Option<i32>, reasoning_tokens: i32) -> i32 {
    max_tokens.unwrap_or(consts::DEFAULT_MAX_TOKENS) - reasoning_tokens
}

pub(crate) fn validate_chat_request(request: &request::ChatCompletionCreate) -> Result<(), ReasonerError> {
    if request.messages.is_empty() {
        return Err(ReasonerError::ValidationError("error: empty messages".to_string()));
    }
    if let request::Message::Assistant(_) = request.messages.last().unwrap() {
        return Err(ReasonerError::ValidationError(
            "error: cannot process partial assistant response content in messages yet!".to_string(),
        ));
    }
    Ok(())
}

pub(crate) async fn create_chat_completion(
    client: &LLMClient,
    request: request::ChatCompletionCreate,
    model_config: &config::ModelConfig,
) -> Result<response_direct::ChatCompletion, ReasonerError> {
    validate_chat_request(&request)?;

    let mut message_assistant = request::MessageAssistant {
        reasoning_content: None,
        content: Some(consts::THINK_START.to_string()),
        tool_calls: None,
    };

    let mut reasoning_request: ChatCompletionCreate = request.clone();
    reasoning_request.model = model_config.model_name.to_string();
    reasoning_request
        .messages
        .push(request::Message::Assistant(message_assistant.clone()));
    reasoning_request.stop = Some(vec![consts::THINK_END.to_string()]);
    reasoning_request.max_tokens = Some(model_config.reasoning_budget);

    let response = client
        .request_chat_completion(reasoning_request, mime::APPLICATION_JSON)
        .await?;

    let reasoning_response = response.json::<response_direct::ChatCompletion>().await?;
    let reasoning_choice = match reasoning_response.choices.first() {
        Some(choice) => choice,
        None => {
            return Err(ReasonerError::ApiError(
                "error: no reasoning response".to_string(),
            ))
        }
    };
    let prompt_tokens = reasoning_response.usage.prompt_tokens;
    let reasoning_tokens = reasoning_response.usage.completion_tokens;
    let mut reasoning_text: String = match &reasoning_choice.message.content {
        Some(content) => content.trim().to_string(),
        None => "".to_string(),
    };

    log::debug!(
        "Completion {} reasoning text: {}",
        reasoning_response.id,
        reasoning_text
    );
    log::debug!(
        "Completion {} reasoning usage: prompt_tokens: {}, reasoning_tokens: {}",
        reasoning_response.id,
        prompt_tokens,
        reasoning_tokens
    );

    let answer_text: String;
    let answer_tool_calls: Option<Vec<serde_json::Value>>;
    let answer_tokens: i32;
    let finish_reason: FinishReason;
    let remaining_tokens = calculate_remaining_tokens(request.max_tokens, reasoning_tokens);
    if remaining_tokens > 0 {
        if let FinishReason::Length = reasoning_choice.finish_reason {
            reasoning_text = format!(
                "{}...\n\n{}\n",
                reasoning_text,
                consts::REASONING_CUTOFF_STUB
            );
        }

        message_assistant.content = Some(format!(
            "{}{}{}",
            consts::THINK_START,
            reasoning_text,
            consts::THINK_END,
        ));

        let mut answer_request: ChatCompletionCreate = request.clone();
        answer_request.model = model_config.model_name.to_string();
        answer_request
            .messages
            .push(request::Message::Assistant(message_assistant.clone()));
        answer_request.max_tokens = Some(remaining_tokens);

        let response = client
            .request_chat_completion(answer_request, mime::APPLICATION_JSON)
            .await?;

        let answer_response = response.json::<response_direct::ChatCompletion>().await?;
        let answer_choice = match answer_response.choices.first() {
            Some(choice) => choice,
            None => {
                return Err(ReasonerError::ApiError(
                    "error: no answer response".to_string(),
                ))
            }
        };

        answer_text = match &answer_choice.message.content {
            Some(content) => content.trim().to_string(),
            None => "".to_string(),
        };
        answer_tool_calls = answer_choice.message.tool_calls.clone();
        answer_tokens = answer_response.usage.completion_tokens;
        finish_reason = answer_choice.finish_reason;

        log::debug!(
            "Completion {} answer text: {}",
            reasoning_response.id,
            answer_text
        );
        log::debug!(
            "Completion {} answer usage: answer_tokens: {}",
            reasoning_response.id,
            answer_tokens
        );
    } else {
        answer_text = "".to_string();
        answer_tool_calls = None;
        answer_tokens = 0;
        finish_reason = FinishReason::Length;
        log::debug!(
            "Completion {} reasoning length exceeded, finishing without an answer.",
            reasoning_response.id
        );
    }

    Ok(ChatCompletion {
        id: reasoning_response.id,
        object: reasoning_response.object,
        created: reasoning_response.created,
        model: request.model.clone(),
        choices: vec![response_direct::Choice {
            index: 0,
            message: request::MessageAssistant::new(reasoning_text, answer_text, answer_tool_calls),
            logprobs: None,
            finish_reason: finish_reason,
        }],
        usage: Usage {
            prompt_tokens: prompt_tokens,
            completion_tokens: reasoning_tokens + answer_tokens,
            total_tokens: prompt_tokens + reasoning_tokens + answer_tokens,
        },
    })
}

pub(crate) async fn stream_chat_completion(
    client: &LLMClient,
    request: request::ChatCompletionCreate,
    model_config: &config::ModelConfig,
    sender: Sender<Result<Bytes, ReasonerError>>,
) -> Result<(), ReasonerError> {
    validate_chat_request(&request)?;

    let mut message_assistant = request::MessageAssistant {
        reasoning_content: None,
        content: Some(consts::THINK_START.to_string()),
        tool_calls: None,
    };

    let mut reasoning_request: ChatCompletionCreate = request.clone();
    reasoning_request.model = model_config.model_name.to_string();
    reasoning_request
        .messages
        .push(request::Message::Assistant(message_assistant.clone()));
    reasoning_request.stop = Some(vec![consts::THINK_END.to_string()]);
    reasoning_request.max_tokens = Some(model_config.reasoning_budget);
    reasoning_request.stream_options = Some(request::StreamOptions {
        include_usage: Some(true),
    });

    let mut reasoning_text = "".to_string();
    let mut prompt_tokens = 0;
    let mut reasoning_tokens = 0;
    let mut answer_tokens = 0;
    let mut reasoning_finish_reason = FinishReason::Stop;

    let mut outgoing_chunk = response_stream::ChatCompletionChunk {
        id: "".to_string(),
        object: "chat.completion.chunk".to_string(),
        created: 0,
        model: request.model.clone(),
        choices: vec![],
        usage: None,
    };

    // Reasoning stream
    let mut response = client
        .request_chat_completion(reasoning_request, mime::TEXT_EVENT_STREAM)
        .await?;

    let mut first_chunk = true;
    let mut chunks_to_process: VecDeque<ChatCompletionChunk> = VecDeque::new();
    loop {
        if chunks_to_process.len() == 0 {
            match extract_chunks_from_event(response.chunk().await)? {
                Some(chunks) => chunks_to_process.extend(chunks),
                None => break,
            };
        }
        let chunk = match chunks_to_process.pop_front() {
            Some(chunk) => chunk,
            None => continue,
        };

        outgoing_chunk.id = chunk.id.clone();
        outgoing_chunk.created = chunk.created;

        if let Some(usage) = chunk.usage {
            prompt_tokens = usage.prompt_tokens;
            reasoning_tokens = usage.completion_tokens;
        }

        let reasoning_choice = match chunk.choices.first() {
            Some(choice) => choice,
            None => continue,
        };

        if let Some(finisg_reason) = reasoning_choice.finish_reason {
            reasoning_finish_reason = finisg_reason;
        }

        if first_chunk {
            send_delta(
                &sender,
                outgoing_chunk.clone(),
                ChunkChoiceDelta::chunk_choice_delta_opening(),
            )
            .await?;
            first_chunk = false;
        }

        if let Some(content) = reasoning_choice.delta.content.clone() {
            reasoning_text = format!("{}{}", reasoning_text, content);
            log::debug!(
                "Completion {} reasoning content delta: {:?}",
                outgoing_chunk.id,
                content
            );

            send_delta(
                &sender,
                outgoing_chunk.clone(),
                ChunkChoiceDelta::chunk_choice_delta_reasoning(content),
            )
            .await?;
        }
    }

    log::debug!(
        "Completion {} reasoning usage: prompt_tokens: {}, reasoning_tokens: {}",
        outgoing_chunk.id,
        prompt_tokens,
        reasoning_tokens
    );

    // Answer stream
    let remaining_tokens = calculate_remaining_tokens(request.max_tokens, reasoning_tokens);
    if remaining_tokens > 0 {
        if let FinishReason::Length = reasoning_finish_reason {
            reasoning_text = format!(
                "{}...\n\n{}\n",
                reasoning_text,
                consts::REASONING_CUTOFF_STUB
            );
            send_delta(
                &sender,
                outgoing_chunk.clone(),
                ChunkChoiceDelta::chunk_choice_delta_reasoning(
                    format!("...\n\n{}\n", consts::REASONING_CUTOFF_STUB).to_string(),
                ),
            )
            .await?;
        }

        message_assistant.content = Some(format!(
            "{}{}{}",
            consts::THINK_START,
            reasoning_text,
            consts::THINK_END,
        ));
        send_delta_thinking_end(&sender, &outgoing_chunk).await?;

        let mut answer_request: ChatCompletionCreate = request.clone();
        answer_request.model = model_config.model_name.to_string();
        answer_request
            .messages
            .push(request::Message::Assistant(message_assistant.clone()));
        answer_request.max_tokens = Some(remaining_tokens);
        answer_request.stream_options = Some(request::StreamOptions {
            include_usage: Some(true),
        });

        let mut response = client
            .request_chat_completion(answer_request, mime::TEXT_EVENT_STREAM)
            .await?;

        let mut chunks_to_process: VecDeque<ChatCompletionChunk> = VecDeque::new();
        loop {
            if chunks_to_process.len() == 0 {
                match extract_chunks_from_event(response.chunk().await)? {
                    Some(chunks) => chunks_to_process.extend(chunks),
                    None => break,
                };
            }
            let chunk = match chunks_to_process.pop_front() {
                Some(chunk) => chunk,
                None => continue,
            };

            if let Some(usage) = chunk.usage {
                answer_tokens = usage.completion_tokens;
            }

            let answer_choice = match chunk.choices.first() {
                Some(choice) => choice,
                None => continue,
            };

            if let Some(content) = answer_choice.delta.content.clone() {
                log::debug!(
                    "Completion {} answer content delta: {:?}",
                    outgoing_chunk.id,
                    content
                );
            }
            outgoing_chunk.choices = vec![answer_choice.clone()];
            send_chunk(&sender, &outgoing_chunk).await?;
        }

        log::debug!(
            "Completion {} answer usage: answer_tokens: {}",
            outgoing_chunk.id,
            answer_tokens
        );
    } else {
        outgoing_chunk.choices = vec![response_stream::ChunkChoice {
            index: 0,
            delta: ChunkChoiceDelta::chunk_choice_delta_empty(),
            logprobs: None,
            finish_reason: Some(FinishReason::Length),
        }];
        log::debug!(
            "Completion {} reasoning length exceeded, finishing without an answer.",
            outgoing_chunk.id
        );
        send_chunk(&sender, &outgoing_chunk).await?;
    }

    if let Some(stream_options) = request.stream_options
        && stream_options.include_usage.unwrap_or(false)
    {
        outgoing_chunk.choices = vec![];
        outgoing_chunk.usage = Some(Usage {
            prompt_tokens,
            completion_tokens: reasoning_tokens + answer_tokens,
            total_tokens: prompt_tokens + reasoning_tokens + answer_tokens,
        });
        send_chunk(&sender, &outgoing_chunk).await?;
    }

    send_data(&sender, "[DONE]".into()).await?;

    Ok(())
}

fn extract_chunks_from_event(
    response_event: Result<Option<Bytes>, Error>,
) -> Result<Option<Vec<response_stream::ChatCompletionChunk>>, ReasonerError> {
    let events = match response_event {
        Ok(Some(chunk)) => chunk,
        Ok(None) => {
            log::debug!("extract_chunks_from_event: No more chunks");
            return Ok(None);
        }
        Err(e) => {
            log::debug!("extract_chunks_from_event: Error reading events: {e}");
            return Err(ReasonerError::NetworkError(e.to_string()));
        }
    };

    let text = match str::from_utf8(&events) {
        Ok(text) => text.trim(),
        Err(e) => {
            log::debug!("extract_chunks_from_event: Error decoding events: {e}");
            return Err(ReasonerError::ParseError(e.to_string()));
        }
    };

    let mut chunks = vec![];
    for text_chunk in text.split("\n\n") {
        if !text_chunk.starts_with("data: ") {
            log::debug!("extract_chunks_from_event: Skipping chunk: {text_chunk}");
            continue;
        }

        let text_chunk = &text_chunk["data:".len()..].trim();
        if text_chunk.contains("[DONE]") {
            log::debug!("extract_chunks_from_event: Final chunk received");
            return Ok(None);
        }

        let chunk = match serde_json::from_str::<response_stream::ChatCompletionChunk>(text_chunk) {
            Ok(json) => json,
            Err(e) => {
                log::debug!("extract_chunks_from_event: Error parsing chunk: {e}");
                return Err(ReasonerError::ParseError(e.to_string()));
            }
        };
        chunks.push(chunk);
    }

    Ok(Some(chunks))
}

async fn send_data(
    sender: &Sender<Result<Bytes, ReasonerError>>,
    data: String,
) -> Result<(), ReasonerError> {
    let event_data = format!("data: {}\n\n", data);
    if let Err(e) = sender.send(Ok(event_data.into())).await {
        log::warn!("failed to send message: {:?}", e.0);
        return Err(ReasonerError::NetworkError("failed to send message".to_string()));
    }
    Ok(())
}

async fn send_chunk(
    sender: &Sender<Result<Bytes, ReasonerError>>,
    chunk: &response_stream::ChatCompletionChunk,
) -> Result<(), ReasonerError> {
    send_data(sender, serde_json::to_string(chunk).unwrap()).await
}

async fn send_delta(
    sender: &Sender<Result<Bytes, ReasonerError>>,
    mut chunk: response_stream::ChatCompletionChunk,
    delta: response_stream::ChunkChoiceDelta,
) -> Result<(), ReasonerError> {
    chunk.choices = vec![response_stream::ChunkChoice {
        index: 0,
        delta: delta,
        logprobs: None,
        finish_reason: None,
    }];
    send_chunk(&sender, &chunk).await
}

#[cfg(reasoning)]
async fn send_delta_thinking_end(
    _: &Sender<Result<Bytes, ReasonerError>>,
    _: &response_stream::ChatCompletionChunk,
) -> Result<(), ReasonerError> {
    Ok(())
}

#[cfg(not(reasoning))]
async fn send_delta_thinking_end(
    sender: &Sender<Result<Bytes, ReasonerError>>,
    chunk: &response_stream::ChatCompletionChunk,
) -> Result<(), ReasonerError> {
    send_delta(
        sender,
        chunk.clone(),
        response_stream::ChunkChoiceDelta {
            content: Some(consts::THINK_END.to_string()),
            ..Default::default()
        },
    )
    .await
}
