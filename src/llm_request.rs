use actix_web::mime;
use actix_web::web::Bytes;
use reqwest::Error;
use reqwest::Response;
use tokio::sync::mpsc::Sender;

use crate::config;
use crate::consts;
use crate::models::FinishReason;
use crate::models::Usage;
use crate::models::request;
use crate::models::request::ChatCompletionCreate;
use crate::models::response_direct;
use crate::models::response_direct::ChatCompletion;
use crate::models::response_stream;
use crate::models::response_stream::ChunkChoiceDelta;
use std::io;
use std::io::Write;
use std::time::Duration;

pub(crate) struct LLMClient {
    client: reqwest::Client,
    base_url: String,
}

impl LLMClient {
    pub(crate) fn new(client: reqwest::Client, base_url: &str) -> Self {
        Self {
            client,
            base_url: base_url.to_string(),
        }
    }
    fn post(&self, url: &str) -> reqwest::RequestBuilder {
        self.client.post(format!("{}{}", self.base_url, url))
    }
}

async fn perform_request(
    client: &LLMClient,
    request: request::ChatCompletionCreate,
    expected_content_type: mime::Mime,
    timeout: Duration,
) -> Result<Response, Box<dyn std::error::Error>> {
    let response = client
        .post("/chat/completions")
        .json(&request)
        .timeout(timeout)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        return Err(format!("error: status {:?}, text {:?}", status, text).into());
    }

    let content_type: mime::Mime = response.headers()[reqwest::header::CONTENT_TYPE]
        .to_str()?
        .parse()?;
    if content_type.essence_str() != expected_content_type.essence_str() {
        return Err(format!(
            "content-type: {:?}, expected: {:?}",
            content_type, expected_content_type
        )
        .into());
    }

    Ok(response)
}

pub(crate) async fn create_chat_completion(
    client: &LLMClient,
    request: request::ChatCompletionCreate,
    timeout: Duration,
) -> Result<response_direct::ChatCompletion, Box<dyn std::error::Error>> {
    if request.messages.len() == 0 {
        return Err("error: empty messages".into());
    }
    if let request::Message::Assistant(_) = request.messages.last().unwrap() {
        return Err(
            "error: cannot process partial assistant response content in messages yet!".into(),
        );
    }

    let (model, reasoning_budget) = config::MODEL_MAPPING.get(&request.model).unwrap();
    let mut message_assistant = request::MessageAssistant {
        reasoning_content: None,
        content: Some(consts::THINK_START.to_string()),
        tool_calls: None,
    };

    let mut reasoning_request: ChatCompletionCreate = request.clone();
    reasoning_request.model = model.to_string();
    reasoning_request
        .messages
        .push(request::Message::Assistant(message_assistant.clone()));
    reasoning_request.stop = Some(vec![consts::THINK_END.to_string()]);
    reasoning_request.max_tokens = Some(*reasoning_budget);

    let response =
        perform_request(client, reasoning_request, mime::APPLICATION_JSON, timeout).await?;

    let reasoning_response = response.json::<response_direct::ChatCompletion>().await?;
    let reasoning_choice = match reasoning_response.choices.first() {
        Some(choice) => choice,
        None => return Err("error: no reasoning response".into()),
    };
    let prompt_tokens = reasoning_response.usage.prompt_tokens;
    let reasoning_tokens = reasoning_response.usage.completion_tokens;
    let mut reasoning_text: String = match &reasoning_choice.message.content {
        Some(content) => content.trim().to_string(),
        None => "".to_string(),
    };

    println!("   ***   Reasoning text: {}", reasoning_text);
    println!(
        "   ***   Reasoning usage: prompt_tokens: {}, reasoning_tokens: {}",
        prompt_tokens, reasoning_tokens
    );

    let answer_text: String;
    let answer_tool_calls: Option<Vec<serde_json::Value>>;
    let answer_tokens: i32;
    let finish_reason: FinishReason;
    let remaining_tokens = request.max_tokens.unwrap_or(1024 * 1024) - reasoning_tokens;
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
        answer_request.model = model.to_string();
        answer_request
            .messages
            .push(request::Message::Assistant(message_assistant.clone()));
        answer_request.max_tokens = Some(remaining_tokens);

        let response =
            perform_request(client, answer_request, mime::APPLICATION_JSON, timeout).await?;

        let answer_response = response.json::<response_direct::ChatCompletion>().await?;
        let answer_choice = match answer_response.choices.first() {
            Some(choice) => choice,
            None => return Err("error: no answer response".into()),
        };

        answer_text = match &answer_choice.message.content {
            Some(content) => content.trim().to_string(),
            None => "".to_string(),
        };
        answer_tool_calls = answer_choice.message.tool_calls.clone();
        answer_tokens = answer_response.usage.completion_tokens;
        finish_reason = answer_choice.finish_reason;

        println!("   ***   Answer text: {}", answer_text);
        println!("   ***   Answer usage: answer_tokens: {}", answer_tokens);
    } else {
        answer_text = "".to_string();
        answer_tool_calls = None;
        answer_tokens = 0;
        finish_reason = FinishReason::Length;
        println!("   ***   Reasoning length exceeded, finishing without answer.");
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

async fn send_data(
    sender: &Sender<Result<Bytes, Box<dyn std::error::Error>>>,
    data: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let event_data = format!("data: {}\n\n", data);
    // println!("   ***   event_data: {:}", event_data);
    if let Err(e) = sender.send(Ok(event_data.into())).await {
        println!("error: failed to send message: {:?}", e.0);
        return Err("failed to send message".into());
    }
    Ok(())
}

async fn send_chunk(
    sender: &Sender<Result<Bytes, Box<dyn std::error::Error>>>,
    chunk: &response_stream::ChatCompletionChunk,
) -> Result<(), Box<dyn std::error::Error>> {
    send_data(sender, serde_json::to_string(chunk).unwrap()).await
}

async fn send_delta(
    sender: &Sender<Result<Bytes, Box<dyn std::error::Error>>>,
    mut chunk: response_stream::ChatCompletionChunk,
    delta: response_stream::ChunkChoiceDelta,
) -> Result<(), Box<dyn std::error::Error>> {
    chunk.choices = vec![response_stream::ChunkChoice {
        index: 0,
        delta: delta,
        logprobs: None,
        finish_reason: None,
    }];
    send_chunk(&sender, &chunk).await
}

pub(crate) async fn stream_chat_completion(
    client: &LLMClient,
    request: request::ChatCompletionCreate,
    sender: Sender<Result<Bytes, Box<dyn std::error::Error>>>,
    timeout: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    if request.messages.len() == 0 {
        return Err("error: empty messages".into());
    }
    if let request::Message::Assistant(_) = request.messages.last().unwrap() {
        return Err(
            "error: cannot process partial assistant response content in messages yet!".into(),
        );
    }

    let (model, reasoning_budget) = config::MODEL_MAPPING.get(&request.model).unwrap();

    let mut message_assistant = request::MessageAssistant {
        reasoning_content: None,
        content: Some(consts::THINK_START.to_string()),
        tool_calls: None,
    };

    let mut reasoning_request: ChatCompletionCreate = request.clone();
    reasoning_request.model = model.to_string();
    reasoning_request
        .messages
        .push(request::Message::Assistant(message_assistant.clone()));
    reasoning_request.stop = Some(vec![consts::THINK_END.to_string()]);
    reasoning_request.max_tokens = Some(*reasoning_budget);
    reasoning_request.stream_options = Some(request::StreamOptions {
        include_usage: Some(true),
    });

    let mut reasoning_text = "".to_string();
    let mut answer_text = "".to_string();
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
    let mut response =
        perform_request(client, reasoning_request, mime::TEXT_EVENT_STREAM, timeout).await?;

    let mut first_chunk = true;
    print!("   ***   Reasoning text:");
    loop {
        let chunk = match extract_chunk_from_event(response.chunk().await)? {
            Some(chunk) => chunk,
            None => break,
        };
        // println!("   ***   chunk: {:?}", chunk);

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
            print!("{content:}");
            io::stdout().flush().unwrap();

            send_delta(
                &sender,
                outgoing_chunk.clone(),
                ChunkChoiceDelta::chunk_choice_delta_reasoning(content),
            )
            .await?;
        }
    }
    println!();

    println!(
        "   ***   Reasoning usage: prompt_tokens: {}, reasoning_tokens: {}",
        prompt_tokens, reasoning_tokens
    );

    // Answer stream
    let remaining_tokens = request.max_tokens.unwrap_or(1024 * 1024) - reasoning_tokens;
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
        answer_request.model = model.to_string();
        answer_request
            .messages
            .push(request::Message::Assistant(message_assistant.clone()));
        answer_request.max_tokens = Some(remaining_tokens);
        answer_request.stream_options = Some(request::StreamOptions {
            include_usage: Some(true),
        });

        let mut response =
            perform_request(client, answer_request, mime::TEXT_EVENT_STREAM, timeout).await?;

        print!("   ***   Answer text: ");
        loop {
            let chunk = match extract_chunk_from_event(response.chunk().await)? {
                Some(chunk) => chunk,
                None => break,
            };

            if let Some(usage) = chunk.usage {
                answer_tokens = usage.completion_tokens;
            }

            let answer_choice = match chunk.choices.first() {
                Some(choice) => choice,
                None => continue,
            };

            if let Some(content) = answer_choice.delta.content.clone() {
                answer_text = format!("{}{}", answer_text, content);
                print!("{content:}");
                io::stdout().flush().unwrap();
            }
            outgoing_chunk.choices = vec![answer_choice.clone()];
            send_chunk(&sender, &outgoing_chunk).await?;
        }
        println!();

        println!("   ***   Answer usage: answer_tokens: {answer_tokens:}");
    } else {
        outgoing_chunk.choices = vec![response_stream::ChunkChoice {
            index: 0,
            delta: ChunkChoiceDelta::chunk_choice_delta_empty(),
            logprobs: None,
            finish_reason: Some(FinishReason::Length),
        }];
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

fn extract_chunk_from_event(
    response_event: Result<Option<Bytes>, Error>,
) -> Result<Option<response_stream::ChatCompletionChunk>, Box<dyn std::error::Error>> {
    let event = match response_event {
        Ok(Some(chunk)) => chunk,
        Ok(None) => return Ok(None),
        Err(e) => return Err(Box::new(e)),
    };

    let text = match str::from_utf8(&event) {
        Ok(text) => text,
        Err(e) => return Err(Box::new(e)),
    };

    if !text.starts_with("data: ") {
        return Ok(None); // TODO: Invent something to skip the chunk instead of ending the stream
    }

    let text = &text["data:".len()..].trim();
    if text.contains("[DONE]") {
        return Ok(None);
    }

    let chunk = match serde_json::from_str::<response_stream::ChatCompletionChunk>(text) {
        Ok(json) => json,
        Err(e) => return Err(Box::new(e)),
    };
    Ok(Some(chunk))
}

#[cfg(reasoning)]
async fn send_delta_thinking_end(
    _: &Sender<Result<Bytes, Box<dyn std::error::Error>>>,
    _: &response_stream::ChatCompletionChunk,
) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg(not(reasoning))]
async fn send_delta_thinking_end(
    sender: &Sender<Result<Bytes, Box<dyn std::error::Error>>>,
    chunk: &response_stream::ChatCompletionChunk,
) -> Result<(), Box<dyn std::error::Error>> {
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
