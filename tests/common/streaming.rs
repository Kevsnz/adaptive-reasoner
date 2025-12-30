use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;

use actix_web::web::Bytes;
use adaptive_reasoner::errors::ReasonerError;

pub async fn collect_stream_chunks(
    receiver: &mut mpsc::Receiver<Result<Bytes, ReasonerError>>,
) -> Vec<String> {
    let mut received_messages = vec![];
    let timeout = Duration::from_secs(5);
    let start_time = Instant::now();

    loop {
        match tokio::time::timeout(timeout, receiver.recv()).await {
            Ok(Some(result)) => match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    if !text.contains("[DONE]") {
                        received_messages.push(text.to_string());
                    }
                }
                Err(e) => {
                    eprintln!("Received error: {:?}", e);
                    panic!("Received error: {:?}", e);
                }
            },
            Ok(None) => {
                break;
            }
            Err(_) => {
                break;
            }
        }
        if start_time.elapsed() > timeout {
            eprintln!("Timeout waiting for chunks");
            break;
        }
    }

    received_messages
}

pub async fn collect_stream_with_timeout(
    receiver: &mut mpsc::Receiver<Result<Bytes, ReasonerError>>,
    duration: Duration,
) -> (Vec<String>, bool) {
    let mut received_messages = vec![];
    let mut timeout_occurred = false;
    let start_time = Instant::now();

    loop {
        match tokio::time::timeout(duration, receiver.recv()).await {
            Ok(Some(result)) => match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    if !text.contains("[DONE]") {
                        received_messages.push(text.to_string());
                    }
                }
                Err(e) => {
                    eprintln!("Received error: {:?}", e);
                    panic!("Received error: {:?}", e);
                }
            },
            Ok(None) => {
                break;
            }
            Err(_) => {
                timeout_occurred = true;
                break;
            }
        }
        if start_time.elapsed() > duration {
            timeout_occurred = true;
            break;
        }
    }

    (received_messages, timeout_occurred)
}

pub fn validate_sse_format(lines: &[&str]) -> (bool, bool, bool) {
    let mut has_data_lines = false;
    let mut has_empty_lines = false;
    let mut has_crlf = false;

    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("data: ") {
            has_data_lines = true;
            let data_content = &line[6..];
            if data_content == "[DONE]" {
                continue;
            }
            if serde_json::from_str::<serde_json::Value>(data_content).is_ok() {
                has_crlf = true;
            }
        }
        if *line == "" && i > 0 {
            if lines[i - 1].starts_with("data: ") {
                has_empty_lines = true;
            }
        }
    }

    (has_data_lines, has_empty_lines, has_crlf)
}

pub fn count_valid_json_chunks(lines: &[&str]) -> usize {
    let mut count = 0;

    for line in lines {
        if line.starts_with("data: ") && !line.contains("[DONE]") {
            let data_str = &line[6..];
            if serde_json::from_str::<serde_json::Value>(data_str).is_ok() {
                count += 1;
            }
        }
    }

    count
}
