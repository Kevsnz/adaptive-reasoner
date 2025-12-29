# Adaptive Reasoner Project Map

Adaptive Reasoner is a Rust-based HTTP service that implements adaptive reasoning for large language models (LLMs). The service acts as a proxy between clients and upstream LLM APIs, limiting the amount of reasoning tokens a model can generate before producing its final answer. This approach helps control costs and response times for reasoning-intensive models. The service exposes OpenAI-compatible endpoints on port 8080, supporting both streaming and non-streaming modes, and can be configured to output reasoning tokens either inline within content or as a separate `reasoning_content` field. The architecture is designed with testability in mind, using dependency injection, trait-based abstractions, and pure function extraction.

## Configuration Management

The configuration system is responsible for loading model configurations from a JSON file specified by the `AR_CONFIG_FILE` environment variable (defaulting to `./config.json`). The configuration module defines the core data structures and loading logic, along with a trait for abstraction. The `Config` structure contains a HashMap mapping served model names to their configurations, while `ModelConfig` captures the parameters for each model including the source model name, API base URL, API key, maximum reasoning budget, and optional extra parameters. The `ConfigLoader` trait enables testability by allowing mock implementations (e.g., `InMemoryConfigLoader`) for testing without filesystem access. The `load_config()` function reads and parses the configuration file, then resolves API keys by reading them from environment variables. This flexible configuration allows the service to serve multiple model configurations simultaneously, each potentially pointing to different upstream providers with different reasoning budget limits.

**Source files:** `src/config/mod.rs`

## HTTP API Server

The HTTP server is built using the actix-web framework and exposes OpenAI-compatible endpoints for model listing and chat completion requests. The `main()` function initializes the service by loading the configuration, setting up logging with env_logger, creating an HTTP client, and initializing the reasoning service with dependency injection. The server binds to 0.0.0.0:8080 using constants from `consts.rs`. The server registers two routes under `/v1`: a GET endpoint at `/models` that returns a list of available models, and a POST endpoint at `/chat/completions` that handles chat completion requests. The application construction is handled by `create_app()` in the `app` module, which uses dependency injection to provide the reasoning service and config to handlers. Request timeouts are managed with 30-second connection timeouts and 60-second read timeouts, defined as constants in `consts.rs`. HTTP handlers are separated into their own module for testability.

**Source files:** `src/main.rs`, `src/app.rs`, `src/handlers.rs`

## HTTP Handlers

The handlers module contains the HTTP request handlers that process incoming requests and return appropriate responses. The `models()` handler iterates through the configured models and constructs a model list response with metadata including model IDs and ownership information. The `chat_completion()` handler is the core request processor that extracts model configurations from injected dependencies, delegates to the reasoning service, and returns responses in either non-streaming or streaming mode. Handlers receive dependencies through actix-web's `Data<T>` extractor, enabling easy mocking for testing. The streaming handler uses a channel with configurable buffer size to stream responses to clients.

**Source files:** `src/handlers.rs`

## LLM Client

The LLM client module provides a wrapper around the reqwest HTTP client for making requests to upstream LLM APIs, with abstraction for testability. The `LLMClientTrait` defines the interface for HTTP clients, enabling mock implementations in tests. The `LLMClient` struct implements this trait and encapsulates a reqwest client instance, the base URL for the upstream API, an API key for authentication, and optional extra body parameters. The client's `request_chat_completion()` method constructs and sends POST requests to the `/chat/completions` endpoint, including the Authorization header with a bearer token, the request body as JSON, and proper content type handling. This method returns `Result<Response, ReasonerError>` and performs error checking by verifying HTTP status codes and ensuring the response content type matches expectations (either `application/json` for non-streaming or `text/event-stream` for streaming). The trait-based design allows the service layer to use either the real client or mocks without changing business logic.

**Source files:** `src/llm_client/mod.rs`

## Service Layer

The service layer contains the core business logic for adaptive reasoning, separated from HTTP concerns for better testability. The `ReasoningService` struct holds a trait object (`Box<dyn LLMClientTrait>`) to enable dependency injection. The service provides two main methods: `create_completion()` for non-streaming requests and `stream_completion()` for streaming requests. Both methods orchestrate the two-phase completion process (reasoning phase followed by answer phase) using extracted pure helper functions. The service validates requests using `validate_chat_request()`, calculates token budgets using `calculate_remaining_tokens()`, and constructs requests using `build_reasoning_request()` and `build_answer_request()`. Error handling uses the custom `ReasonerError` type throughout. The service layer is fully testable without HTTP infrastructure by mocking the LLM client trait.

**Source files:** `src/service/mod.rs`

## Adaptive Reasoning Logic

The adaptive reasoning logic is the core innovation of this service, now implemented as pure functions for easy testing. The logic splits the chat completion process into two distinct phases: a reasoning phase and an answer phase. Pure functions handle request validation (`validate_chat_request()`), token calculation (`calculate_remaining_tokens()`), and request construction (`build_reasoning_request()`, `build_answer_request()`). For non-streaming requests, the reasoning request is constructed by appending an assistant message with an opening `think` tag to the conversation history, setting a stop sequence at the closing `think` tag, and limiting max tokens to the configured reasoning budget. The reasoning response is parsed, and if it hit the length limit, a cutoff stub is appended. The answer request contains the full reasoning content wrapped in `think` tags, calculates remaining tokens from the original request's max tokens, and requests the final answer. For streaming requests, the same two-phase approach is used but processes chunks incrementally, managing chunk parsing, event extraction from Server-Sent Events (SSE), and proper delta construction. Throughout both modes, usage statistics (prompt tokens, reasoning tokens, and answer tokens) are accumulated and combined in the final response. The logic handles edge cases like empty messages, partial assistant responses, and situations where the reasoning budget exceeds the available max tokens.

**Source files:** `src/llm_request.rs`

## Request and Response Models

The models module defines the comprehensive data structures for OpenAI-compatible request and response formats. The request structures include `ChatCompletionCreate` which captures parameters like model name, messages array, max tokens, stop sequences, streaming options, tools, and tool choice preferences. Messages support multiple roles (system, user, assistant, tool) and flexible content types including plain text or structured arrays with text and image URLs. The response models are split into two variants: `response_direct` for non-streaming responses containing complete `ChatCompletion` objects with choices, usage statistics, and finish reasons, and `response_stream` for streaming responses containing `ChatCompletionChunk` objects with incremental deltas. The streaming delta structure can contain either a separate `reasoning_content` field (when the `reasoning` feature flag is enabled) or inline content within the main content field. The models support conditional compilation through the `reasoning` feature flag, which controls whether reasoning content appears in a dedicated field or is embedded within the main content using special `think` tags.

**Source files:** `src/models/mod.rs`, `src/models/request.rs`, `src/models/response_direct.rs`, `src/models/response_stream.rs`, `src/models/model_list.rs`

## Error Handling

The error handling module provides a unified error type for all operations in the service. The `ReasonerError` enum captures various error scenarios: `ValidationError` for request validation failures, `ApiError` for upstream API failures, `ParseError` for JSON parsing failures, `ConfigError` for configuration loading failures, and `NetworkError` for network-related failures. The error type implements standard traits (`Debug`, `Display`, `Error`, `Clone`) for proper error handling and reporting. `From` implementations are provided for converting common error types (like `reqwest::Error` and `String`) into `ReasonerError`, enabling idiomatic error propagation with the `?` operator. All operations return `Result<T, ReasonerError>` instead of `Box<dyn std::error::Error>`, making error types explicit and testable.

**Source files:** `src/errors.rs`

## Constants

The constants module defines all magic numbers and string literals used throughout the codebase, promoting maintainability and avoiding duplication. Key constants include `THINK_START` and `THINK_END` for reasoning content delimiters, `REASONING_CUTOFF_STUB` for the message shown when reasoning budget is exceeded, `DEFAULT_MAX_TOKENS` for the default token limit (1,048,576), `CONNECT_TIMEOUT_SECS` and `READ_TIMEOUT_SECS` for HTTP timeouts, `CHANNEL_BUFFER_SIZE` for streaming response channels, and `SERVER_PORT` for the HTTP server port. Centralizing these values makes configuration changes easier and helps document what each value controls.

**Source files:** `src/consts.rs`

## Build Configuration

The build system controls an important feature flag that affects how reasoning content is delivered to clients. The `build.rs` script currently has the `reasoning` configuration commented out, meaning the default behavior is to embed reasoning content inline using special XML-style `think` tags within the main content field. When the `reasoning` flag is enabled (by uncommenting the appropriate line in build.rs), the service instead uses a dedicated `reasoning_content` field in the response objects. This compile-time configuration affects both the request/response model structures (through conditional compilation attributes like `#[cfg(reasoning)]`) and the behavior of functions that construct and send response chunks. The feature flag also controls the inclusion of the `reasoning_content` field in streaming deltas and determines whether the closing `think` tag is sent as a separate chunk or included in the content. This dual-mode support allows the service to be compiled for different client preferences and API compatibility requirements without runtime configuration changes.

**Source files:** `build.rs`, `Cargo.toml`

## Test Infrastructure

The project includes comprehensive testing infrastructure to ensure reliability and maintainability. Test utilities are conditionally compiled with `#[cfg(test)]` and excluded from production builds. The `src/test_utils` module provides helper functions for creating test requests and configs, along with assertion helpers for validating responses. Integration tests in the `tests/` directory use wiremock to mock external HTTP servers, enabling end-to-end testing of service logic without real API calls. HTTP endpoint tests use actix-web's test utilities to verify routing, validation, and error handling. Test fixtures in `tests/fixtures/mod.rs` provide reusable test data objects, while mocks in `tests/mocks/mod.rs` implement trait abstractions for isolated testing. The test suite includes 62 tests covering pure functions, error types, service methods, integration flows, and HTTP endpoints.

**Source files:** `src/test_utils/mod.rs`, `src/test_utils/helpers.rs`, `src/test_utils/assertions.rs`, `tests/integration.rs`, `tests/http.rs`, `tests/fixtures/mod.rs`, `tests/mocks/mod.rs`, `TESTING.md`

## Testing Documentation

Comprehensive testing documentation is provided in `TESTING.md`, covering test execution commands, test structure, naming conventions, and detailed guides for adding new tests. The documentation explains how to use mock objects, test fixtures, and assertion helpers. It also includes debugging techniques and CI/CD integration guidelines, making it easy for developers to understand and extend the test suite.

**Source files:** `TESTING.md`, `TESTS_PLAN.md`
