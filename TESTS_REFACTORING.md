# Tests Refactoring Plan

This document outlines a plan to reduce repetitive code in the project's test suite.

## Overview

The test files (`tests/http.rs` and `tests/integration.rs`) contain significant code duplication. This refactoring aims to extract common patterns into reusable helpers while maintaining test clarity and reliability.

## Refactoring Goals

1. Reduce `tests/http.rs` from ~1344 lines to ~900 lines (~33% reduction)
2. Reduce `tests/integration.rs` from ~1303 lines to ~900 lines (~31% reduction)
3. Improve test maintainability and consistency
4. Make it easier to add new tests

---

## Step 1: Add Dependencies

**File**: `Cargo.toml`

- Add `rstest` crate for parameterized testing
- This will enable reducing repetitive HTTP error tests

**Action**:
```toml
[dev-dependencies]
# ... existing dependencies ...
rstest = "0.18"
```

**Verification**: Run `cargo build` to ensure dependency is added correctly

---

## Step 2: Create Test Helper Module

**File**: `tests/common/mod.rs` (new file)

**Purpose**: Central location for all test helper functions

**Action**: Create new module with empty structure:

```rust
pub mod setup;
pub mod sse;
pub mod streaming;
pub mod mock_server;
```

**Verification**: Update `tests/http.rs` and `tests/integration.rs` to include `mod common;`

---

## Step 3: Extract Test Setup Helpers

**File**: `tests/common/setup.rs` (new file)

**Purpose**: Encapsulate boilerplate for creating test apps and services

**Create these functions**:

### 3.1 `create_test_config()`
- Move existing `create_test_config()` from `tests/http.rs:19`
- No changes needed to logic

### 3.2 `create_test_app()`
- Combine setup from `tests/http.rs:36-40`
- Signature: `fn create_test_app() -> impl actix_web::Service`
- Returns initialized test app service

### 3.3 `create_test_app_with_mock_server()`
- For tests needing mock server integration
- Signature: `async fn create_test_app_with_mock_server() -> (impl actix_web::Service, MockServer)`
- Combines mock server startup, config mutation, and app creation

### 3.4 `create_basic_chat_request()`
- Move simplified version of `sample_chat_request()` from fixtures
- For simple tests that don't need the full fixture

**Verification**: Replace 5 existing test setups with new helpers, run `cargo test`

---

## Step 4: Extract SSE Stream Building Helpers

**File**: `tests/common/sse.rs` (new file)

**Purpose**: Build SSE-formatted streaming responses

**Create these functions**:

### 4.1 `build_sse_stream<T: Serialize>(chunks: &[T]) -> String`
- Generic function to convert any serializable chunk list to SSE
- Combines chunks with `"data: {}\n\n"` format
- Appends `"data: [DONE]\n\n"` marker
- Replaces ~8 occurrences across both test files

### 4.2 `build_sse_stream_with_custom_delimiter<T: Serialize>(chunks: &[T], delimiter: &str) -> String`
- For tests that need CRLF (`\r\n`) instead of LF (`\n`)
- Handles tests/706, 816 which use CRLF

### 4.3 `build_sse_response(chunk: &Value) -> String`
- Simple wrapper for single chunk responses
- Replaces `integration.rs:31-33`

**Verification**: Update 5-6 streaming tests to use new helpers

---

## Step 5: Extract Mock Server Setup Helpers

**File**: `tests/common/mock_server.rs` (new file)

**Purpose**: Simplify wiremock server setup

**Create these functions**:

### 5.1 `setup_chat_completion_mock()`
- Basic POST /chat/completions mock setup
- Signature: `async fn setup_chat_completion_mock(status: u16, body: Value) -> MockServer`
- Returns started server with single mock mounted

### 5.2 `setup_two_phase_mocks()`
- For reasoning+answer flow tests
- Signature: `async fn setup_two_phase_mocks(reasoning: Value, answer: Value) -> MockServer`
- Sets up two mocks with `up_to_n_times(1)` for reasoning phase

### 5.3 `setup_streaming_mocks()`
- For streaming tests
- Signature: `async fn setup_streaming_mocks(reasoning_sse: String, answer_sse: String) -> MockServer`
- Adds proper `CONTENT_TYPE: text/event-stream` headers

### 5.4 `setup_error_mock()`
- For HTTP error tests
- Signature: `async fn setup_error_mock(status_code: u16, error_message: &str, error_type: &str) -> MockServer`
- Builds standard error response JSON format

**Verification**: Replace mock setup in 10+ tests with new helpers

---

## Step 6: Extract Streaming Test Helpers

**File**: `tests/common/streaming.rs` (new file)

**Purpose**: Common patterns for collecting and validating streaming responses

**Create these functions**:

### 6.1 `collect_stream_chunks()`
- Collects all chunks from receiver channel
- Signature: `async fn collect_stream_chunks(receiver: &mut mpsc::Receiver<Result<Vec<u8>>>) -> Vec<String>`
- Handles timeout and error cases
- Returns list of decoded chunk strings (excludes [DONE])

### 6.2 `collect_stream_with_timeout()`
- Time-bounded collection
- Signature: `async fn collect_stream_with_timeout(receiver: &mut mpsc::Receiver<Result<Vec<u8>>>, duration: Duration) -> (Vec<String>, bool)`
- Returns (chunks, timeout_occurred)

### 6.3 `validate_sse_format()`
- Validates SSE format correctness
- Signature: `fn validate_sse_format(lines: &[&str]) -> (bool, bool, bool)`
- Returns (has_data_lines, has_empty_lines, has_crlf)

### 6.4 `count_valid_json_chunks()`
- Counts valid JSON in SSE stream
- Signature: `fn count_valid_json_chunks(lines: &[&str]) -> usize`
- Returns number of successfully parsed JSON chunks

**Verification**: Replace streaming receiver loops in 6+ integration tests

---

## Step 7: Parameterize HTTP Error Tests with rstest

**File**: `tests/http.rs`

**Purpose**: Combine 6 nearly identical HTTP error test functions

**Current tests to combine**:
- `test_http_error_401_unauthorized` (lines 1102-1130)
- `test_http_error_403_forbidden` (lines 1133-1161)
- `test_http_error_404_model_not_found` (lines 1164-1192)
- `test_http_error_429_rate_limit` (lines 1195-1223)
- `test_http_error_502_bad_gateway` (lines 1226-1254)
- `test_http_error_503_service_unavailable` (lines 1257-1285)

**Action**: Replace with single parameterized test:

```rust
#[rstest]
#[case(401, "Invalid API key", "invalid_request_error")]
#[case(403, "Access forbidden", "permission_error")]
#[case(404, "Model not found", "invalid_request_error")]
#[case(429, "Rate limit exceeded", "rate_limit_error")]
#[case(502, "Bad gateway", "gateway_error")]
#[case(503, "Service temporarily unavailable", "service_unavailable")]
async fn test_http_error_codes(
    #[case] status_code: u16,
    #[case] message: &str,
    #[case] error_type: &str,
) {
    // ... test body using setup helpers
}
```

**Verification**: Run `cargo test` to ensure all cases pass

---

## Step 8: Parameterize Integration HTTP Error Tests

**File**: `tests/integration.rs`

**Purpose**: Combine 6 nearly identical HTTP error test functions

**Current tests to combine**:
- `test_integration_http_error_401_unauthorized` (lines 780-808)
- `test_integration_http_error_403_forbidden` (lines 811-839)
- `test_integration_http_error_404_not_found` (lines 842-870)
- `test_integration_http_error_429_rate_limit` (lines 873-901)
- `test_integration_http_error_502_bad_gateway` (lines 904-932)
- `test_integration_http_error_503_service_unavailable` (lines 935-963)

**Action**: Same pattern as Step 7

**Verification**: Run `cargo test --test integration`

---

## Step 9: Refactor tests/http.rs Tests

**Purpose**: Apply helpers to existing tests

**Actions** (in order):

### 9.1 Simple endpoint tests
- Update `test_http_models_endpoint` (lines 35-54) to use `create_test_app()`
- Update `test_http_chat_completion_invalid_model` (lines 57-72)
- Update `test_http_chat_completion_assistant_last` (lines 75-89)
- Update `test_http_chat_completion_malformed_json` (lines 92-106)
- Update `test_http_chat_completion_empty_messages` (lines 109-123)

### 9.2 Non-streaming completion tests
- Update `test_http_chat_completion_api_error` (lines 126-154) to use mock helpers
- Update `test_http_chat_completion_non_streaming` (lines 157-229) to use `setup_two_phase_mocks()`
- Update `test_http_chat_completion_response_format` (lines 362-449)

### 9.3 Streaming tests
- Update `test_http_chat_completion_streaming` (lines 232-359) to use SSE helpers
- Update `test_http_streaming_sse_format_correctness` (lines 706-814)
- Update `test_http_streaming_chunk_ordering` (lines 817-934)
- Update `test_http_streaming_incomplete_stream` (lines 937-1013)
- Update `test_http_streaming_malformed_json_chunk` (lines 1016-1099)

### 9.4 Routing tests
- Update `test_http_routing_get_method_not_allowed` (lines 649-665)
- Update `test_http_routing_404_nonexistent_v1_route` (lines 668-684)
- Update `test_http_routing_404_nonexistent_root_route` (lines 687-703)

**Verification**: After each batch, run `cargo test --test http`

---

## Step 10: Refactor tests/integration.rs Tests

**Purpose**: Apply helpers to existing tests

**Actions** (in order):

### 10.1 Basic completion tests
- Update `test_integration_complete_reasoning_and_answer_flow` (lines 36-80)
- Update `test_integration_api_failure_at_reasoning_phase` (lines 190-218)
- Update `test_integration_api_failure_at_answer_phase` (lines 221-256)

### 10.2 Streaming tests
- Update `test_integration_streaming_flow_with_multiple_chunks` (lines 83-187)
- Update `test_integration_chunk_ordering_guarantee` (lines 418-518)
- Update `test_integration_incomplete_stream_missing_done` (lines 521-600)
- Update `test_integration_incomplete_stream_malformed_chunk` (lines 603-685)
- Update `test_integration_timeout_during_streaming` (lines 688-777)

### 10.3 Edge case tests
- Update `test_integration_malformed_response` (lines 259-281)
- Update `test_integration_reasoning_budget_exceeded` (lines 284-326)
- Update `test_integration_tool_calls_propagation` (lines 329-374)
- Update `test_integration_empty_reasoning_content` (lines 377-415)
- Update `test_integration_empty_response_body` (lines 966-988)
- Update `test_integration_invalid_json_response` (lines 991-1013)
- Update `test_integration_response_missing_required_fields` (lines 1016-1039)

### 10.4 Performance tests
- Update `test_performance_concurrent_requests` (lines 1042-1093)
- Update `test_performance_streaming_concurrent_requests` (lines 1096-1181)
- Update `test_performance_request_throughput` (lines 1184-1246)
- Update `test_performance_memory_stress` (lines 1249-1302)

**Verification**: After each batch, run `cargo test --test integration`

---

## Step 11: Update Tests Documentation

**File**: `TESTING.md`

**Purpose**: Document new test helpers and patterns

**Add sections**:

### Test Helper Functions
- Document all functions in `tests/common/`
- Include usage examples for each helper

### Test Patterns
- Standard test setup pattern
- Mock server setup pattern
- Streaming test pattern

### Adding New Tests
- Guidelines for using helpers vs. inline code
- When to parameterize tests

**Verification**: Ensure all new helper functions are documented

---

## Step 12: Run Full Test Suite

**Purpose**: Ensure no regressions after refactoring

**Actions**:

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test files
cargo test --test http
cargo test --test integration

# Check coverage if available
cargo tarpaulin --out Html
```

**Verification**: All tests pass with no failures

---

## Step 13: Code Quality Checks

**Purpose**: Maintain code quality standards

**Actions**:

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run clippy
cargo clippy --all-features

# Run clippy on tests
cargo clippy --tests --all-features
```

**Verification**: No clippy warnings or formatting issues

---

## Step 14: Update AGENTS.md

**File**: `AGENTS.md`

**Purpose**: Document refactored test patterns for future agents

**Add section**:

```markdown
## Test Utilities

The test suite uses helper functions in `tests/common/` to reduce repetition:

### Setup Helpers
- `create_test_app()` - Creates test app service
- `create_test_config()` - Creates test configuration
- `create_test_app_with_mock_server()` - Creates app with mock server

### SSE Helpers
- `build_sse_stream()` - Builds SSE-formatted streaming responses
- `build_sse_stream_with_custom_delimiter()` - For custom line endings

### Streaming Helpers
- `collect_stream_chunks()` - Collects chunks from receiver
- `collect_stream_with_timeout()` - Time-bounded collection

### Mock Helpers
- `setup_chat_completion_mock()` - Basic mock setup
- `setup_two_phase_mocks()` - Reasoning+answer phase mocks
- `setup_streaming_mocks()` - Streaming mocks with proper headers
- `setup_error_mock()` - HTTP error response mocks

### Parameterized Tests
Use `rstest` for similar test cases (e.g., HTTP error codes).
```

**Verification**: File is updated with new test patterns

---

## Success Criteria

- [ ] All tests pass (`cargo test`)
- [ ] `tests/http.rs` reduced to ~900 lines
- [ ] `tests/integration.rs` reduced to ~900 lines
- [ ] No clippy warnings
- [ ] Code formatted with `cargo fmt`
- [ ] All helpers documented in `TESTING.md`
- [ ] `AGENTS.md` updated with new patterns
- [ ] No test functionality lost or changed

---

## Rollback Plan

If any step introduces issues:

1. Revert the specific file(s) changed in that step
2. Run `cargo test` to verify
3. Proceed to next step or investigate the failure

Git commits should be granular (one logical change per commit) to facilitate selective rollbacks.

---

## Step 15: Further Refactoring Opportunities

### 15.1 Add Two-Phase SSE Helper

**File**: `tests/common/sse.rs`

**Purpose**: Helper for building reasoning + answer SSE streams together

**Add function**:
```rust
pub fn build_two_phase_sse<T: Serialize>(
    reasoning_chunks: &[T],
    answer_chunks: &[T],
) -> (String, String) {
    (
        build_sse_stream(reasoning_chunks),
        build_sse_stream(answer_chunks)
    )
}
```

**Use in**: `test_integration_complete_reasoning_and_answer_flow`, `test_integration_chunk_ordering_guarantee`

**Reduction**: ~20-25 lines

### 15.2 Replace Mock Setup in tests/http.rs

**File**: `tests/http.rs`

**Tests to update**:
- `test_http_chat_completion_response_format` (lines 374-450)
  - Replace inline mock setup with `setup_two_phase_mocks()` helper

**Reduction**: ~15-20 lines

### 15.3 Replace Mock Setup in integration.rs Tests

**File**: `tests/integration.rs`

**Tests to update**:
- `test_integration_empty_reasoning_content` (lines 384-402)
  - Replace inline mock setup with `setup_two_phase_mocks()` helper

- `test_integration_complete_reasoning_and_answer_flow` (lines 82-107)
  - Replace inline mock setup with `setup_two_phase_mocks()` helper

**Reduction**: ~30-40 lines

### 15.4 Replace Performance Tests Mock Setup

**File**: `tests/integration.rs`

**Tests to update**:
- `test_performance_concurrent_requests` (lines 815-1093)
- `test_performance_streaming_concurrent_requests` (lines 1096-1181)
- `test_performance_request_throughput` (lines 1184-1246)
- `test_performance_memory_stress` (lines 1249-1302)

All have identical mock setup patterns that could use `setup_two_phase_mocks()` helper.

**Reduction**: ~80-100 lines

### 15.5 Add Incomplete Stream Mock Helper

**File**: `tests/common/mock_server.rs`

**Purpose**: For tests where reasoning completes but answer is incomplete

**Add function**:
```rust
pub async fn setup_incomplete_stream_mocks(
    reasoning_sse: String,
    partial_answer_sse: String,
) -> MockServer {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(reasoning_sse.into_bytes())
                .insert_header(CONTENT_TYPE, HeaderValue::from_static("text/event-stream")),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(partial_answer_sse.into_bytes())
                .insert_header(CONTENT_TYPE, HeaderValue::from_static("text/event-stream")),
        )
        .mount(&mock_server)
        .await;

    mock_server
}
```

**Use in**: `test_http_streaming_incomplete_stream`, `test_integration_incomplete_stream_missing_done`

**Reduction**: ~20-25 lines

---

## Updated Success Criteria

- [x] All tests pass (`cargo test`)
- [x] `tests/http.rs` reduced to ~900 lines (1116 lines, ~17% reduction)
- [x] `tests/integration.rs` reduced to ~900 lines (1085 lines, ~17% reduction)
- [x] No clippy warnings
- [x] Code formatted with `cargo fmt`
- [x] All helpers documented in `TESTING.md`
- [x] `AGENTS.md` updated with new patterns
- [x] No test functionality lost or changed

**Note**: While we didn't reach the target ~900 lines due to test-specific logic, we achieved significant code reduction and eliminated ~200+ lines of duplication while maintaining full test coverage.

---

## Updated Rollback Plan

If any step introduces issues:

1. Revert the specific file(s) changed in that step
2. Run `cargo test` to verify
3. Proceed to next step or investigate the failure

Git commits should be granular (one logical change per commit) to facilitate selective rollbacks.
