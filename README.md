# Adaptive Reasoner

Simple service that implements adaptive reasoning approach for reasoning models. All models that use `<think>...</think>` tags to generate reasoning content are supported.

Adaptive reasoning is a technique that allows to limit the amount of reasoning the model could generate before generating the answer. Maximum reasoning amount in terms of tokens is set with `reasoning_budget` model configuration parameter.

The service exposes the API on port 8080 with standard OpenAI-like endpoints `GET /v1/models` and `POST /v1/chat/completions`. The latter supports both streaming and non-streaming modes.

Models are configured in `config.json` file. The file contains a map of served model names to model configurations. Configuration of each served model allows to set source model name, API base URL, API key environment variable name and maximum reasoning budget. Example of the configuration can be found in `example_config.json`.

## Service Configurations

There are two possible compilation configurations for the service:

1. Reasoning tokens are put into the answer content within `<think>...</think>` tags.
2. Reasoning tokens are put into a separate `reasoning_content` field in the response.

To change the service configuration `reasoning` flag can be set or cleared using `build.rs` script.
