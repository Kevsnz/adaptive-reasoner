use crate::config;
use crate::errors::ReasonerError;
use crate::models::request;

pub(crate) fn calculate_remaining_tokens(max_tokens: Option<i32>, reasoning_tokens: i32) -> i32 {
    max_tokens.unwrap_or(crate::consts::DEFAULT_MAX_TOKENS) - reasoning_tokens
}

pub(crate) fn build_reasoning_request(
    request: request::ChatCompletionCreate,
    model_config: &config::ModelConfig,
) -> request::ChatCompletionCreate {
    let mut reasoning_request: request::ChatCompletionCreate = request.clone();
    reasoning_request.model = model_config.model_name.to_string();

    let message_assistant = request::MessageAssistant {
        reasoning_content: None,
        content: Some(crate::consts::THINK_START.to_string()),
        tool_calls: None,
    };
    reasoning_request
        .messages
        .push(request::Message::Assistant(message_assistant));
    reasoning_request.stop = Some(vec![crate::consts::THINK_END.to_string()]);
    reasoning_request.max_tokens = Some(model_config.reasoning_budget);

    reasoning_request
}

pub(crate) fn build_answer_request(
    request: request::ChatCompletionCreate,
    model_config: &config::ModelConfig,
    reasoning_text: &str,
    max_tokens: i32,
) -> request::ChatCompletionCreate {
    let mut answer_request: request::ChatCompletionCreate = request.clone();
    answer_request.model = model_config.model_name.to_string();

    let message_assistant = request::MessageAssistant {
        reasoning_content: None,
        content: Some(format!(
            "{}{}{}",
            crate::consts::THINK_START,
            reasoning_text,
            crate::consts::THINK_END,
        )),
        tool_calls: None,
    };
    answer_request
        .messages
        .push(request::Message::Assistant(message_assistant));
    answer_request.max_tokens = Some(max_tokens);

    answer_request
}

pub(crate) fn validate_chat_request(
    request: &request::ChatCompletionCreate,
) -> Result<(), ReasonerError> {
    if request.messages.is_empty() {
        return Err(ReasonerError::ValidationError(
            "error: empty messages".to_string(),
        ));
    }
    if let request::Message::Assistant(_) = request.messages.last().unwrap() {
        return Err(ReasonerError::ValidationError(
            "error: cannot process partial assistant response content in messages yet!".to_string(),
        ));
    }
    Ok(())
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::request::{MessageAssistant, MessageSystemUser, MessageContent};

    #[test]
    fn test_validate_chat_request_valid() {
        let request = request::ChatCompletionCreate {
            model: "test".to_string(),
            messages: vec![
                request::Message::User(MessageSystemUser {
                    content: MessageContent::String("Hello".to_string()),
                }),
            ],
            max_tokens: None,
            stop: None,
            stream: None,
            stream_options: None,
            tools: None,
            tool_choice: None,
            extra: Default::default(),
        };

        assert!(validate_chat_request(&request).is_ok());
    }

    #[test]
    fn test_validate_chat_request_empty_messages() {
        let request = request::ChatCompletionCreate {
            model: "test".to_string(),
            messages: vec![],
            max_tokens: None,
            stop: None,
            stream: None,
            stream_options: None,
            tools: None,
            tool_choice: None,
            extra: Default::default(),
        };

        let result = validate_chat_request(&request);
        assert!(result.is_err());
        match result.unwrap_err() {
            ReasonerError::ValidationError(msg) => {
                assert!(msg.contains("empty messages"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_validate_chat_request_assistant_last() {
        let request = request::ChatCompletionCreate {
            model: "test".to_string(),
            messages: vec![
                request::Message::User(MessageSystemUser {
                    content: MessageContent::String("Hello".to_string()),
                }),
                request::Message::Assistant(MessageAssistant {
                    reasoning_content: None,
                    content: Some("Hi".to_string()),
                    tool_calls: None,
                }),
            ],
            max_tokens: None,
            stop: None,
            stream: None,
            stream_options: None,
            tools: None,
            tool_choice: None,
            extra: Default::default(),
        };

        let result = validate_chat_request(&request);
        assert!(result.is_err());
        match result.unwrap_err() {
            ReasonerError::ValidationError(msg) => {
                assert!(msg.contains("cannot process partial assistant"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_calculate_remaining_tokens_with_max_tokens() {
        let result = calculate_remaining_tokens(Some(1000), 200);
        assert_eq!(result, 800);
    }

    #[test]
    fn test_calculate_remaining_tokens_with_none_max_tokens() {
        let result = calculate_remaining_tokens(None, 200);
        assert_eq!(result, crate::consts::DEFAULT_MAX_TOKENS - 200);
    }

    #[test]
    fn test_calculate_remaining_tokens_exceeding_budget() {
        let result = calculate_remaining_tokens(Some(100), 150);
        assert_eq!(result, -50);
    }

    #[test]
    fn test_build_reasoning_request() {
        let original_request = request::ChatCompletionCreate {
            model: "test".to_string(),
            messages: vec![
                request::Message::User(MessageSystemUser {
                    content: MessageContent::String("Hello".to_string()),
                }),
            ],
            max_tokens: Some(1000),
            stop: None,
            stream: None,
            stream_options: None,
            tools: None,
            tool_choice: None,
            extra: Default::default(),
        };

        let model_config = config::ModelConfig {
            model_name: "upstream-model".to_string(),
            api_url: "http://test.com".to_string(),
            api_key: "test-key".to_string(),
            reasoning_budget: 100,
            extra: None,
        };

        let reasoning_request = build_reasoning_request(original_request, &model_config);

        assert_eq!(reasoning_request.model, "upstream-model");
        assert_eq!(reasoning_request.max_tokens, Some(100));
        assert_eq!(reasoning_request.stop, Some(vec![crate::consts::THINK_END.to_string()]));
        assert_eq!(reasoning_request.messages.len(), 2);
        match &reasoning_request.messages[1] {
            request::Message::Assistant(msg) => {
                assert_eq!(msg.content, Some(crate::consts::THINK_START.to_string()));
            }
            _ => panic!("Expected Assistant message"),
        }
    }

    #[test]
    fn test_build_answer_request() {
        let original_request = request::ChatCompletionCreate {
            model: "test".to_string(),
            messages: vec![
                request::Message::User(MessageSystemUser {
                    content: MessageContent::String("Hello".to_string()),
                }),
            ],
            max_tokens: Some(1000),
            stop: None,
            stream: None,
            stream_options: None,
            tools: None,
            tool_choice: None,
            extra: Default::default(),
        };

        let model_config = config::ModelConfig {
            model_name: "upstream-model".to_string(),
            api_url: "http://test.com".to_string(),
            api_key: "test-key".to_string(),
            reasoning_budget: 100,
            extra: None,
        };

        let reasoning_text = "Let me think about this";
        let answer_request = build_answer_request(original_request, &model_config, reasoning_text, 500);

        assert_eq!(answer_request.model, "upstream-model");
        assert_eq!(answer_request.max_tokens, Some(500));
        assert_eq!(answer_request.messages.len(), 2);
        match &answer_request.messages[1] {
            request::Message::Assistant(msg) => {
                let expected = format!(
                    "{}{}{}",
                    crate::consts::THINK_START,
                    reasoning_text,
                    crate::consts::THINK_END
                );
                assert_eq!(msg.content, Some(expected));
            }
            _ => panic!("Expected Assistant message"),
        }
    }
}
