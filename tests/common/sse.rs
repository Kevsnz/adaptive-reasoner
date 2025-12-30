use serde::Serialize;

pub fn build_sse_stream<T: Serialize>(chunks: &[T]) -> String {
    let mut sse = String::new();
    for chunk in chunks {
        let json_str = serde_json::to_string(chunk).unwrap();
        sse.push_str(&format!("data: {}\n\n", json_str));
    }
    sse.push_str("data: [DONE]\n\n");
    sse
}

pub fn build_sse_stream_with_custom_delimiter<T: Serialize>(chunks: &[T], delimiter: &str) -> String {
    let mut sse = String::new();
    for chunk in chunks {
        let json_str = serde_json::to_string(chunk).unwrap();
        sse.push_str(&format!("data: {}{}{}", json_str, delimiter, delimiter));
    }
    sse.push_str(&format!("data: [DONE]{}{}", delimiter, delimiter));
    sse
}

pub fn build_sse_response(chunk: &serde_json::Value) -> String {
    let json_str = serde_json::to_string(chunk).unwrap();
    format!("data: {}\n\n", json_str)
}

pub fn build_sse_response_with_delimiter(chunk: &serde_json::Value, delimiter: &str) -> String {
    let json_str = serde_json::to_string(chunk).unwrap();
    format!("data: {}{}{}", json_str, delimiter, delimiter)
}
