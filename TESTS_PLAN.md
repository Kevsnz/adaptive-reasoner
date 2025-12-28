# Testability Improvement Plan - Atomic Steps Sequence

## Overview

This document outlines a sequence of interdependent atomic steps to make the Adaptive Reasoner codebase more testable. The plan is ordered from foundational changes to final testing implementation, with each step building on the previous ones.

In case of unexpected difficulties at any point, reach out to supervisor for assistance.

---

## Phase 1: Foundation Setup

### Step 1: Add test dependencies to Cargo.toml [✓ COMPLETED]
Add the following dependencies under `[dev-dependencies]`:
- `tokio-test` - Async test utilities
- `thiserror` - Structured error types
- `wiremock` - HTTP mocking for integration tests
- `reqwest = { version = "0.12", features = ["json"] }` - For test clients

### Step 2: Create concrete error types in new `src/errors.rs` module [✓ COMPLETED]
- Define `ReasonerError` enum with variants:
  - `ValidationError(String)`
  - `ApiError(String)`
  - `ParseError(String)`
  - `ConfigError(String)`
  - `NetworkError(String)`
- Replace all `Box<dyn std::error::Error>` with `Result<T, ReasonerError>`
- Implement `std::error::Error` trait for `ReasonerError`
- Implement `From` traits for common error conversions

---

## Phase 2: Extract Pure Functions

### Step 3: Extract validation logic from `llm_request.rs` into pure functions [✓ COMPLETED]
- Create `validate_chat_request(&ChatCompletionCreate) -> Result<(), ReasonerError>`
- Move empty messages check (llm_request.rs:26-28, 169-171) into this function
- Move assistant message check (llm_request.rs:29-33, 172-176) into this function
- Return early validation errors without async context
- Update both `create_chat_completion()` and `stream_chat_completion()` to use this function

### Step 4: Extract token calculation logic into pure functions [✓ COMPLETED]
- Create `calculate_remaining_tokens(max_tokens: Option<i32>, reasoning_tokens: i32) -> i32`
- Add `DEFAULT_MAX_TOKENS: i32 = 1024 * 1024` constant in `src/consts.rs`
- Replace hardcoded `1024 * 1024` in llm_request.rs:81 with the constant
- Use the new function in both streaming (llm_request.rs:281) and non-streaming (llm_request.rs:81) paths

### Step 5: Extract message construction logic into pure functions [✓ COMPLETED]
- Create `build_reasoning_request(request: ChatCompletionCreate, model_config: &ModelConfig) -> ChatCompletionCreate`
  - Move assistant message setup logic (llm_request.rs:35-46, 178-193)
  - Set stop sequence to `THINK_END`
  - Set max_tokens to `model_config.reasoning_budget`
- Create `build_answer_request(request: ChatCompletionCreate, model_config: &ModelConfig, reasoning_text: String, max_tokens: i32) -> ChatCompletionCreate`
  - Wrap reasoning text in `THINK_START` and `THINK_END` tags
  - Set remaining tokens for answer generation
- Use these functions in both `create_chat_completion()` and `stream_chat_completion()`

---

## Phase 3: Create Abstraction Traits

### Step 6: Create `LLMClientTrait` in new `src/llm_client/mod.rs` [✓ COMPLETED]
- Define trait:
  ```rust
  #[async_trait::async_trait]
  pub trait LLMClientTrait: Send + Sync {
      async fn request_chat_completion(
          &self,
          request: ChatCompletionCreate,
          expected_content_type: Mime,
      ) -> Result<Response, ReasonerError>;
  }
  ```
- Add `async-trait` dependency to Cargo.toml

### Step 7: Make `LLMClient` implement `LLMClientTrait` [✓ COMPLETED]
- Refactor `src/llm_client.rs` to `impl LLMClientTrait for LLMClient`
- Update error handling to return `ReasonerError` instead of `Box<dyn std::error::Error>`
- Convert status code errors to `ReasonerError::ApiError`
- Convert parsing errors to `ReasonerError::ParseError`
- Keep all existing HTTP logic intact

### Step 8: Create `ConfigLoaderTrait` in `src/config/mod.rs` [✓ COMPLETED]
- Define trait:
  ```rust
  pub trait ConfigLoader: Send + Sync {
      fn load_config(&self) -> Result<Config, ReasonerError>;
  }
  ```
- Add `ConfigLoaderTrait` to module exports
- Update `load_config()` function signature to use the trait

---

## Phase 4: Extract Service Layer

### Step 9: Create `ReasoningService` struct in new `src/service/mod.rs` [✓ COMPLETED]
- Define struct:
  ```rust
  pub struct ReasoningService {
      client: Box<dyn LLMClientTrait>,
  }
  ```
- Implement constructor:
  ```rust
  impl ReasoningService {
      pub fn new(client: Box<dyn LLMClientTrait>) -> Self { ... }
  }
  ```
- Store trait object instead of concrete `LLMClient` type
- Accept `ModelConfig` as parameter in methods rather than storing it

### Step 10: Move core reasoning logic to service methods [✓ COMPLETED]
- Create `ReasoningService::create_completion(&self, request: ChatCompletionCreate, model_config: &ModelConfig) -> Result<ChatCompletion, ReasonerError>`
  - Move logic from `llm_request::create_chat_completion()`
  - Use extracted helper functions (Steps 3-5)
  - Replace `LLMClient` references with `self.client`
  - Update error returns to use `ReasonerError`
- Create `ReasoningService::stream_completion(&self, request: ChatCompletionCreate, model_config: &ModelConfig, sender: Sender<Result<Bytes, ReasonerError>>) -> Result<(), ReasonerError>`
  - Move logic from `llm_request::stream_chat_completion()`
  - Use extracted helper functions
  - Update error handling throughout

### Step 11: Update `llm_request.rs` to use service layer [✓ COMPLETED]
- Refactor `create_chat_completion()` to delegate to `ReasoningService::create_completion()`
- Refactor `stream_chat_completion()` to delegate to `ReasoningService::stream_completion()`
- Keep module as thin adapter between HTTP layer and service
- Maintain backward compatibility signature during transition

---

## Phase 5: Refactor HTTP Handlers

### Step 12: Create dependency injection factory in `main.rs` [✓ COMPLETED]
- Create `create_app_factory(reasoning_service: ReasoningService, config: Config) -> impl Fn() -> App`
- Move app construction logic from `main()` into this function
- Pass `reasoning_service` via `Data<ReasoningService>`
- Pass `config` via `Data<Config>`
- Return configured actix-web `App`
- **Technical Note**: Due to Rust's type system and actix-web's middleware transforming `App` type with private types, implemented factory as inline closure rather than separate function. This achieves identical functionality while allowing type inference to handle complex types automatically.

### Step 13: Update HTTP handlers to use injected dependencies
- Refactor `chat_completion()` handler signature:
  ```rust
  async fn chat_completion(
      service: Data<ReasoningService>,
      config: Data<Config>,
      request: Json<ChatCompletionCreate>,
  ) -> impl Responder
  ```
- Remove `ThinData<reqwest::Client>` parameter
- Remove inline `LLMClient::new()` call (main.rs:46-51)
- Call `service.create_completion()` or `service.stream_completion()`
- Convert `ReasonerError` to appropriate HTTP responses (400, 502, etc.)

### Step 14: Update `models()` handler signature
- Ensure it works with injected config dependency
- Keep implementation as-is (already simple)
- Verify it doesn't create any side effects

---

## Phase 6: Create Test Doubles

### Step 15: Implement `MockLLMClient` in `tests/mocks/mod.rs`
- Implement `LLMClientTrait` for mock:
  ```rust
  pub struct MockLLMClient {
      responses: Arc<Mutex<VecDeque<Result<Response, ReasonerError>>>>,
      calls: Arc<Mutex<Vec<ChatCompletionCreate>>>,
  }
  ```
- Store predefined responses in a queue for test scenarios
- Track all request parameters for assertions
- Implement `new()` method with empty response queue
- Implement `add_response()` method to queue responses
- Implement `get_calls()` method to inspect captured requests

### Step 16: Implement `InMemoryConfigLoader` in `tests/mocks/mod.rs`
- Implement `ConfigLoaderTrait` returning test configs:
  ```rust
  pub struct InMemoryConfigLoader {
      config: Config,
  }
  ```
- Allow setting up multiple model configurations programmatically
- Implement `new()` method accepting `Config`
- Avoid all filesystem operations
- Useful for testing config loading logic without file I/O

### Step 17: Create test fixtures in `tests/fixtures/mod.rs`
- Define sample requests:
  - `sample_chat_request()` - Basic valid request
  - `empty_messages_request()` - Invalid request for validation testing
  - `assistant_last_request()` - Invalid request ending with assistant message
- Define sample responses:
  - `sample_reasoning_response()` - Successful reasoning phase response
  - `sample_answer_response()` - Successful answer phase response
  - `sample_error_response()` - Error scenario response
- Define sample streaming chunks:
  - `sample_reasoning_chunks()` - Sequence of reasoning stream chunks
  - `sample_answer_chunks()` - Sequence of answer stream chunks
- Include edge cases:
  - Tool calls in responses
  - Empty reasoning content
  - Budget exceeded scenarios
  - Partial streaming chunks

---

## Phase 7: Add Unit Tests

### Step 18: Write unit tests for extracted pure functions
- Test `validate_chat_request()` in `src/llm_request.rs`:
  - Test with valid request - should return `Ok(())`
  - Test with empty messages - should return `ValidationError`
  - Test with assistant as last message - should return `ValidationError`
- Test `calculate_remaining_tokens()` in `src/llm_request.rs`:
  - Test with max_tokens provided
  - Test with None max_tokens (uses default)
  - Test with reasoning tokens exceeding budget
- Test message construction functions:
  - Test `build_reasoning_request()` creates correct structure
  - Test `build_answer_request()` wraps reasoning correctly
  - Verify stop sequences and token limits

### Step 19: Write unit tests for error types
- Test error creation and conversion in `src/errors.rs`:
  - Test `ValidationError::new()` creates correct variant
  - Test `From<String>` conversions
  - Test `From<reqwest::Error>` conversions
- Test error message formatting:
  - Verify error display strings
  - Test error descriptions

### Step 20: Write unit tests for `ReasoningService` with mock LLM client
- Test happy path:
  - Successful reasoning and answer phases
  - Verify correct response structure
  - Verify token calculations
- Test reasoning budget exceeded:
  - Set reasoning_budget to exceed max_tokens
  - Verify cutoff stub is added
  - Verify answer is skipped
- Test API error handling:
  - Mock LLM client to return errors
  - Verify errors are propagated correctly
- Test partial assistant message rejection:
  - Create request ending with assistant message
  - Verify validation error is returned
- Test tool calls propagation:
  - Mock response with tool_calls
  - Verify tool_calls are in final response

---

## Phase 8: Add Integration Tests

### Step 21: Create integration test module `tests/integration.rs`
- Set up test server with mock dependencies:
  - Create `MockLLMClient` with predefined responses
  - Create `ReasoningService` with mock client
  - Create test `ModelConfig` instance
- Test service layer end-to-end:
  - Test complete reasoning + answer flow
  - Test streaming flow with multiple chunks
  - Verify final response matches expectations
- Test various error scenarios:
  - API failures at reasoning phase
  - API failures at answer phase
  - Malformed responses
- Use `tokio::test` macro for async tests

### Step 22: Create HTTP endpoint test module `tests/http.rs`
- Use actix-web's `actix_web::test` utilities:
  - `actix_web::test::init_service()`
  - `actix_web::test::call_service()`
  - `actix_web::test::TestRequest`
- Test `/v1/models` endpoint:
  - Verify correct model list is returned
  - Verify response format matches OpenAI spec
- Test `/v1/chat/completions` with mock service:
  - Test non-streaming mode (stream: false)
  - Test streaming mode (stream: true)
  - Verify response headers and content types
- Test request validation:
  - Test with empty messages - expect 400 error
  - Test with invalid model - expect 400 error
  - Test with malformed JSON - expect 400 error
- Test error responses:
  - Verify error response format
  - Test various HTTP status codes

---

## Phase 9: Final Refinements

### Step 23: Extract remaining magic numbers to constants
- Add to `src/consts.rs`:
  - `CONNECT_TIMEOUT_SECS: u64 = 30`
  - `READ_TIMEOUT_SECS: u64 = 60`
  - `CHANNEL_BUFFER_SIZE: usize = 100`
  - `DEFAULT_MAX_TOKENS: i32 = 1024 * 1024`
- Replace magic numbers in `main.rs` with constants
- Document what each constant controls

### Step 24: Add conditional compilation for test mode
- Create `src/test_utils/mod.rs` with `#[cfg(test)]`
- Add test-specific helpers:
  - Helper functions to create test requests
  - Helper functions to create test configs
  - Assertion helpers for responses
- Ensure test utils are excluded from production builds

### Step 25: Create testing documentation in `TESTING.md`
- Document test execution commands:
  - `cargo test` - Run all tests
  - `cargo test -- --nocapture` - Show test output
  - `cargo test integration` - Run integration tests only
- Document test structure:
  - Unit tests location (`src/` modules)
  - Integration tests location (`tests/` directory)
  - Test naming conventions
- Document how to add new tests:
  - Adding unit tests to existing modules
  - Creating new integration test files
  - Creating new test fixtures
- Document mock usage patterns:
  - How to use `MockLLMClient`
  - How to queue responses
  - How to verify captured requests

### Step 26: Update main README with test coverage info
- Add "Testing" section to README.md
- Include test execution commands
- Document current test coverage goals
- Document CI/CD integration (if applicable):
  - How tests run in CI
  - Coverage reporting
  - Test failure policies

---

## Prerequisites

Before starting this plan, ensure:
1. Git repository is clean with no uncommitted changes
2. All dependencies are up to date: `cargo update`
3. Code compiles successfully: `cargo build`
4. Current functionality is working: test the service manually

## Success Criteria

The codebase will be considered sufficiently testable when:
1. All unit tests pass: `cargo test`
2. All integration tests pass: `cargo test --test integration`
3. All HTTP endpoint tests pass: `cargo test --test http`
4. Test coverage includes:
   - All pure functions
   - All error branches
   - All service methods
   - All HTTP endpoints
5. New features can be tested without HTTP infrastructure
6. External dependencies can be mocked for isolated testing

---

## Estimated Timeline

- **Phase 1-2 (Foundation + Pure Functions):** 1-2 hours
- **Phase 3-4 (Traits + Service Layer):** 2-3 hours
- **Phase 5 (HTTP Refactoring):** 1-2 hours
- **Phase 6 (Test Doubles):** 1-2 hours
- **Phase 7 (Unit Tests):** 3-4 hours
- **Phase 8 (Integration Tests):** 2-3 hours
- **Phase 9 (Final Polish):** 1 hour

**Total Estimated Time:** 11-17 hours

---

## Notes

- Each step builds on the previous ones - do not skip steps
- Run `cargo build` and `cargo test` after each step to catch issues early
- Maintain backward compatibility during the refactoring
- Keep the service running throughout development to ensure functionality isn't broken
- Consider working on a feature branch: `git checkout -b feature/testability-improvements`
