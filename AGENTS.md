# AGENTS.md

This file contains guidelines for agentic coding agents working on the Adaptive Reasoner codebase.

## Build/Lint/Test Commands

```bash
# Build the project
cargo build

# Run all tests
cargo test

# Run a single test
cargo test test_name

# Run tests in a specific module
cargo test --lib module_name

# Run tests with output
cargo test -- --nocapture

# Build for release
cargo build --release

# Check code without building
cargo check

# Format code (requires nightly or rustfmt)
cargo fmt

# Check formatting without writing
cargo fmt -- --check

# Run clippy linter
cargo clippy

# Run clippy with all features
cargo clippy --all-features

# Build with reasoning feature enabled (uncomment in build.rs first)
cargo build --features reasoning
```

## Code Style Guidelines

### Import Organization
Order imports: std → external → local crates. Use explicit paths, avoid glob imports.

```rust
use std::collections::HashMap;

use actix_web::mime;
use serde::{Deserialize, Serialize};

use crate::config;
```

### Formatting & Types
- 4-space indentation, lines < 100 chars, blank lines between functions
- Derive `Debug`, `Clone`, `Serialize`, `Deserialize` for structs/enums
- Use `pub(crate)` for module-private exports
- `#[serde(skip_serializing_if = "Option::is_none", default)]` for optional fields

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Role { System, User, Assistant }
```

### Naming Conventions
- Structs/Enums: PascalCase (`ChatCompletionCreate`)
- Functions: snake_case (`create_chat_completion`)
- Constants: SCREAMING_SNAKE_CASE (`THINK_START`, `DEFAULT_MAX_TOKENS`)

### Error Handling
- Use `Result<T, ReasonerError>` with `?` operator
- Log at appropriate levels: `log::debug!`, `log::error!`
- Avoid `.unwrap()` except in tests
- Provide descriptive error messages

### Async & HTTP
- Use `async fn` with `.await`
- Handlers return `impl actix_web::Responder`
- Use `actix_web::web::Json<T>` and `Data<T>`

### Testing
- Unit tests: `#[cfg(test)]` in same file
- Integration tests: `tests/` directory
- Use `#[tokio::test]` for async tests
- Test both success and error paths

### Configuration & Constants
- Config from `AR_CONFIG_FILE` env var (default: `./config.json`)
- Define constants in `src/consts.rs`

## Architecture Notes

- **HTTP Layer**: `src/main.rs` - Actix-web handlers and server setup
- **Business Logic**: `src/llm_request.rs` - Core reasoning and chat completion logic
- **Data Models**: `src/models/` - Request/response types
- **External API**: `src/llm_client.rs` - HTTP client for upstream LLM APIs
- **Configuration**: `src/config.rs` - Model configuration loading

The service implements a two-phase completion:
1. Reasoning phase with limited token budget
2. Answer phase with remaining tokens

Reasoning content is either inline in content with `\<think\>` tags or in a separate `reasoning_content` field based on the `reasoning` feature flag.

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

Detailed map of the project could be found in [PROJECTMAP.md](PROJECTMAP.md).
Comprehensive guidance for testing the Adaptive Reasoner codebase could be found in [TESTING.md](TESTING.md).
