# Testing Documentation

This document provides comprehensive guidance for testing the Adaptive Reasoner codebase.

## Test Execution Commands

### Running All Tests

```bash
# Run all tests (unit, integration, and HTTP endpoint tests)
cargo test

# Run tests with output (useful for debugging)
cargo test -- --nocapture

# Run tests with stdout/stderr captured
cargo test -- --show-output
```

### Running Specific Test Suites

```bash
# Run only unit tests in library code
cargo test --lib

# Run only integration tests
cargo test --test integration

# Run only HTTP endpoint tests
cargo test --test http

# Run tests in a specific module
cargo test --lib service
cargo test --lib errors
cargo test --lib llm_request
```

### Running Individual Tests

```bash
# Run a specific test by name
cargo test test_validate_chat_request_valid

# Run tests matching a pattern
cargo test validation

# Run tests with verbose output
cargo test -- --exact -- --nocapture
```

### Running Tests with Different Profiles

```bash
# Run tests in debug mode (default)
cargo test

# Run tests in release mode (faster but longer compilation)
cargo test --release

# Build without running tests
cargo build --test integration
```

## Test Structure

### Unit Tests

Unit tests are located within the source code modules they test, marked with `#[cfg(test)]`.

**Location:** `src/` directories alongside production code

**Examples:**
- `src/errors.rs` - Error type tests
- `src/llm_request.rs` - Pure function tests (validation, token calculation, message construction)
- `src/service/mod.rs` - Service layer tests

**Structure:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Test implementation
    }
}
```

### Integration Tests

Integration tests test the interaction between multiple components and external dependencies using mock servers.

**Location:** `tests/` directory at the project root

**Files:**
- `tests/integration.rs` - Service layer integration tests with wiremock
- `tests/http.rs` - HTTP endpoint tests using actix-web test utilities
- `tests/fixtures/mod.rs` - Test data fixtures
- `tests/mocks/mod.rs` - Mock implementations of traits

**Structure:**
```rust
use adaptive_reasoner::config::ModelConfig;
use adaptive_reasoner::service::ReasoningService;
use wiremock::{Mock, MockServer, ...};

#[tokio::test]
async fn test_integration_scenario() {
    // Test implementation
}
```

### Test Fixtures

Test fixtures are reusable test data objects defined in `tests/fixtures/mod.rs`.

**Available Fixtures:**
- `sample_chat_request()` - Basic valid chat completion request
- `sample_reasoning_response()` - Successful reasoning phase response
- `sample_answer_response()` - Successful answer phase response
- `sample_reasoning_chunks()` - Sequence of reasoning stream chunks
- `sample_answer_chunks()` - Sequence of answer stream chunks

### Test Mocks

Mock implementations of traits for isolated testing are in `tests/mocks/mod.rs`.

**Available Mocks:**
- `InMemoryConfigLoader` - Mock implementation of `ConfigLoader`

## Test Naming Conventions

### Unit Tests

- **Pattern:** `test_<functionality>_<scenario>`
- **Examples:**
  - `test_validate_chat_request_valid` - Test validation with valid input
  - `test_validate_chat_request_empty_messages` - Test validation with empty messages
  - `test_calculate_remaining_tokens_with_max_tokens` - Test token calculation
  - `test_error_display_api_error` - Test error display formatting

### Integration Tests

- **Pattern:** `test_integration_<scenario>`
- **Examples:**
  - `test_integration_complete_reasoning_and_answer_flow` - Complete flow test
  - `test_integration_streaming_flow_with_multiple_chunks` - Streaming test
  - `test_integration_api_failure_at_reasoning_phase` - Error scenario test
  - `test_integration_tool_calls_propagation` - Feature test

### HTTP Endpoint Tests

- **Pattern:** `test_http_<endpoint>_<scenario>`
- **Examples:**
  - `test_http_models_endpoint` - Model list endpoint
  - `test_http_chat_completion_empty_messages` - Validation test
  - `test_http_chat_completion_api_error` - Error propagation test

## Adding New Tests

### Adding Unit Tests to Existing Modules

1. Navigate to the source file you want to test
2. Find or add the `#[cfg(test)] mod tests` section
3. Add your test function following the naming convention

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_functionality() {
        let input = create_test_input();
        let result = function_under_test(input);
        assert_eq!(result, expected_output);
    }
}
```

### Creating New Integration Tests

1. Add test function to `tests/integration.rs` or create new test file
2. Set up mock server with wiremock
3. Configure mock responses
4. Execute test scenario
5. Assert on results

```rust
#[tokio::test]
async fn test_new_integration_scenario() {
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;
    
    let service = ReasoningService::new(/* ... */);
    let result = service.create_completion(request, &config).await;
    
    assert!(result.is_ok());
}
```

### Creating New Test Fixtures

1. Open `tests/fixtures/mod.rs`
2. Add function following existing patterns
3. Return fully constructed test data

```rust
pub fn new_test_fixture() -> RequestType {
    RequestType {
        field1: "value".to_string(),
        field2: Some(123),
        // ... other fields
    }
}
```

## Mock Usage Patterns

### Configuring Multiple Mock Responses

For multi-phase flows (reasoning + answer), configure multiple responses:

```rust
Mock::given(method("POST"))
    .and(path("/chat/completions"))
    .respond_with(ResponseTemplate::new(200)
        .set_body_json(reasoning_response))
    .mount(&mock_server)
    .await;

Mock::given(method("POST"))
    .and(path("/chat/completions"))
    .respond_with(ResponseTemplate::new(200)
        .set_body_json(answer_response))
    .mount(&mock_server)
    .await;
```

### Using InMemoryConfigLoader

For testing with specific model configurations without filesystem access:

```rust
use adaptive_reasoner::config::ConfigLoader;

let config_loader = InMemoryConfigLoader::with_model_config(
    "test-model".to_string(),
    "https://api.example.com".to_string(),
    "test-api-key".to_string(),
    1000,
);

let config = config_loader.load_config().unwrap();
let model_config = config.models.get("test-model").unwrap();
```

### Verifying Captured Requests

Use wiremock's request matching to verify requests:

```rust
use wiremock::matchers::{body_json, method, path};

let request_matcher = request()
    .and(method("POST"))
    .and(path("/chat/completions"))
    .and(body_json(json!({
        "model": "test-model",
        "messages": [...]
    })));

Mock::given(request_matcher)
    .respond_with(ResponseTemplate::new(200).set_body_json(response))
    .expect(1)  // Verify this matcher is called exactly once
    .mount(&mock_server)
    .await;
```

## Test Utilities

The `src/test_utils` module provides helper functions and assertions for writing tests.

### Helper Functions

Located in `src/test_utils/helpers.rs`:

- `create_test_chat_request(model, user_message)` - Create a test chat request
- `create_test_model_config(model_name, api_url, api_key, reasoning_budget)` - Create model config
- `create_test_config_with_model(...)` - Create a Config with a single model

### Assertion Functions

Located in `src/test_utils/assertions.rs`:

- `assert_chat_completion_response(response, expected_model, expected_content)` - Validate response structure
- `assert_usage(usage, prompt_tokens, completion_tokens, total_tokens)` - Validate usage statistics
- `assert_streaming_chunks(chunks, expected_model)` - Validate streaming chunks
- `assert_final_chunk(chunk)` - Validate final chunk has finish_reason and usage
- `assert_choice_structure(choice, index, expected_content)` - Validate choice structure
- `assert_chunk_choice_structure(chunk_choice, index, expected_content)` - Validate chunk choice structure

## Debugging Tests

### Print Test Output

```bash
# Show stdout/stderr from tests
cargo test -- --nocapture

# Show test execution time
cargo test -- --test-threads=1
```

### Run Single Test

```bash
# Run only the failing test
cargo test test_name

# Run tests in a specific module
cargo test --lib service::tests
```

### Enable Logging in Tests

```rust
#[tokio::test]
#[test]
async fn test_with_logging() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();
    
    // Test code that uses log::debug!(), log::error!, etc.
}
```

## Continuous Integration

Tests run in CI on every commit. Ensure:

- All tests pass: `cargo test`
- Code compiles: `cargo build`
- Clippy checks pass: `cargo clippy --all-features`
- Code is formatted: `cargo fmt -- --check`

## Test Coverage Goals

Current test coverage includes:

- **Pure functions**: 100% (validation, token calculation, message construction)
- **Error types**: All variants and conversions
- **Service layer**: Core methods and validation logic
- **Integration tests**: Complete flows and error scenarios
- **HTTP endpoints**: Routing, validation, and error handling
- Non-streaming chat completion integration tests
- Streaming chat completion integration tests with SSE format verification
- Detailed response format verification tests
- Routing edge case tests (404, 405, etc.)

## Test Helper Functions

The test suite includes reusable helper functions in `tests/common/` to reduce code duplication and improve maintainability.

### Setup Helpers (`tests/common/setup.rs`)

- **`create_test_config()`** - Creates a test configuration with default "test-model"
  ```rust
  let config = create_test_config();
  // Returns Config with test-model configured
  ```

- **`create_test_app_components()`** - Creates test app components (config and reasoning service)
  ```rust
  let (config, reasoning_service) = create_test_app_components().await;
  let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;
  ```

- **`create_model_config(base_url: String)`** - Creates model config for given base URL
  ```rust
  let model_config = create_model_config(mock_server.uri());
  ```

### SSE Stream Helpers (`tests/common/sse.rs`)

- **`build_sse_stream<T: Serialize>(chunks: &[T])`** - Builds SSE-formatted stream from chunks
  ```rust
  let sse = build_sse_stream(&chunks);
  // Returns "data: {}\n\ndata: {}\n\n...data: [DONE]\n\n"
  ```

- **`build_sse_stream_with_custom_delimiter<T: Serialize>(chunks: &[T], delimiter: &str)`** - Builds SSE with custom delimiter
  ```rust
  let sse = build_sse_stream_with_custom_delimiter(&chunks, "\r\n");
  // Use for CRLF tests
  ```

- **`build_sse_response(chunk: &Value)`** - Builds single SSE response from JSON
  ```rust
  let sse = build_sse_response(&json_value);
  // Returns "data: {}\n\n"
  ```

### Mock Server Helpers (`tests/common/mock_server.rs`)

- **`setup_chat_completion_mock(status: u16, body: Value)`** - Sets up basic mock with JSON body
  ```rust
  let mock_server = setup_chat_completion_mock(200, json_body).await;
  // Returns configured mock server
  ```

- **`setup_two_phase_mocks(reasoning: Value, answer: Value)`** - Sets up reasoning + answer flow mocks
  ```rust
  let mock_server = setup_two_phase_mocks(reasoning_response, answer_response).await;
  // First mock has up_to_n_times(1), second has no limit
  ```

- **`setup_streaming_mocks(reasoning_sse: String, answer_sse: String)`** - Sets up streaming mocks with SSE
  ```rust
  let mock_server = setup_streaming_mocks(reasoning_sse, answer_sse).await;
  // Includes text/event-stream Content-Type headers
  ```

- **`setup_error_mock(status_code: u16, error_message: &str, error_type: &str)`** - Sets up error response mock
  ```rust
  let mock_server = setup_error_mock(404, "Model not found", "invalid_request_error").await;
  // Returns mock with standard error JSON format
  ```

### Streaming Helpers (`tests/common/streaming.rs`)

- **`collect_stream_chunks(receiver)`** - Collects all chunks from receiver channel
  ```rust
  let chunks = collect_stream_chunks(&mut receiver).await;
  // Returns Vec<String> of decoded chunks (excludes [DONE])
  // 5 second timeout
  ```

- **`validate_sse_format(lines)`** - Validates SSE format correctness
  ```rust
  let (has_data_lines, has_empty_lines, has_crlf) = validate_sse_format(&lines);
  // Returns (bool, bool, bool)
  ```

## Test Patterns

### Parameterized Tests with rstest

For similar test cases with different inputs, use rstest's `#[case]` attribute:

```rust
use rstest::rstest;

#[rstest]
#[case(401, "Invalid API key", "invalid_request_error")]
#[case(403, "Access forbidden", "permission_error")]
async fn test_http_error_codes(
    #[case] status_code: u16,
    #[case] message: &str,
    #[case] error_type: &str,
) {
    // Test implementation using parameters
}
```

### Standard Test Setup Pattern

```rust
#[actix_web::test]  // For http.rs tests
// or
#[tokio::test]     // For integration.rs tests
async fn test_scenario() {
    let (config, reasoning_service) = create_test_app_components().await;
    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    // Test implementation
}
```

### Mock Server Setup Pattern

```rust
async fn test_with_mock() {
    let (config, reasoning_service, mock_server) = create_test_app_with_mock_server().await;

    // Configure mock responses using mock_server
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&mock_server)
        .await;

    // Test implementation
}
```

### Streaming Test Pattern

```rust
async fn test_streaming() {
    let (sender, mut receiver) = mpsc::channel(consts::CHANNEL_BUFFER_SIZE);

    // Spawn streaming task
    tokio::spawn(async move {
        let _ = service.stream_completion(request, &config, sender).await;
    });

    // Collect chunks using helper
    let chunks = collect_stream_chunks(&mut receiver).await;

    // Assert on chunks
    assert!(!chunks.is_empty());
}
```

## Adding New Tests

### Adding Unit Tests to Existing Modules

1. Navigate to the source file you want to test
2. Find or add the `#[cfg(test)] mod tests` section
3. Add your test function following the naming convention

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_functionality() {
        let input = create_test_input();
        let result = function_under_test(input);
        assert_eq!(result, expected_output);
    }
}
```

### Creating New Integration Tests

1. Add test function to `tests/integration.rs` or create new test file
2. Use helper functions from `tests/common/` where applicable
3. Configure mock responses using helper functions
4. Execute test scenario
5. Assert on results

```rust
#[tokio::test]
async fn test_new_integration_scenario() {
    let mock_server = setup_chat_completion_mock(200, response_body).await;

    let (config, reasoning_service) = create_test_app_components().await;
    let app = test::init_service(create_app(reasoning_service.clone(), config.clone())).await;

    let result = service.create_completion(request, &config).await;

    assert!(result.is_ok());
}
```

### Creating New Test Fixtures

1. Open `tests/fixtures/mod.rs`
2. Add function following existing patterns
3. Return fully constructed test data

```rust
pub fn new_test_fixture() -> RequestType {
    RequestType {
        field1: "value".to_string(),
        field2: Some(123),
        // ... other fields
    }
}
```

## Mock Usage Patterns

### Using Test Helper Functions

Helper functions in `tests/common/` reduce boilerplate:

```rust
use crate::common::setup;
use crate::common::mock_server;
use crate::common::sse;
use crate::common::streaming;

#[tokio::test]
async fn test_with_helpers() {
    // Use setup helpers
    let (config, reasoning_service, mock_server) = setup::create_test_app_with_mock_server().await;

    // Use mock helpers
    let mock_server = mock_server::setup_error_mock(404, "Not found", "error").await;

    // Use SSE helpers for streaming tests
    let sse_stream = sse::build_sse_stream(&chunks);

    // Use streaming helpers for collecting responses
    let chunks = streaming::collect_stream_chunks(&mut receiver).await;

    // Test assertions
}
```

### Configuring Multiple Mock Responses

For multi-phase flows (reasoning + answer), configure multiple responses:

```rust
Mock::given(method("POST"))
    .and(path("/chat/completions"))
    .respond_with(ResponseTemplate::new(200)
        .set_body_json(reasoning_response))
    .mount(&mock_server)
    .await;

Mock::given(method("POST"))
    .and(path("/chat/completions"))
    .respond_with(ResponseTemplate::new(200)
        .set_body_json(answer_response))
    .mount(&mock_server)
    .await;
```

## Additional Resources

- [PROJECTMAP.md](PROJECTMAP.md) - Project architecture overview
- [Rust Testing Book](https://doc.rust-lang.org/book/ch11-00-testing.html) - Official Rust testing documentation
- [Tokio Testing](https://docs.rs/tokio/latest/tokio/#testing) - Async testing utilities
- [Wiremock Documentation](https://docs.rs/wiremock/) - HTTP mocking library
