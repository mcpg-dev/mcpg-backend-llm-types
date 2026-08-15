# mcpg-backend-llm-types

> Provider-agnostic chat request/response shapes and the error taxonomy shared by MCPG's LLM backend plugins.

This crate is the leaf data layer of MCPG's LLM backend stack. It defines the
canonical, provider-neutral vocabulary that the chat engine and every
per-provider adapter exchange — messages, multimodal content parts, tool
definitions and tool calls, finish reasons, token usage — plus the two error
enums those layers raise. It is deliberately **not** the engine: there is no
async runtime, no HTTP transport, no prompt templating, and no provider wire
format here, so an adapter that only needs the shapes can depend on this crate
instead of the much heavier `mcpg-backend-llm-shared`.

## What's here

- `normalized` — the canonical currency: `NormalizedChatRequest`,
  `NormalizedChatResponse`, `Message`, `MessageContent`, `ContentPart`,
  `ToolDef`, `ToolCall`, `ToolChoiceWire`, `Role`, `FinishReason`, `TokenUsage`.
- Multimodal input types: `ImageContent` / `ImageSource` / `ImageDetail`,
  `AudioContent` / `AudioSource` / `AudioFormat`, `FileContent` / `FileSource`.
  Each source is a URL, inline base64 bytes, or an `mcpg-resource://<id>` URI
  the engine resolves through the host before an adapter ever sees it.
- `MessageContent::Text` for the common single-text case and
  `MessageContent::Parts` for mixed content, with `as_text`, `is_text_only`,
  and `is_empty` so text-only adapter paths can skip multimodal encoding
  entirely.
- `Message` constructors that keep call sites honest: `system`, `user`,
  `user_parts`, `assistant_tool_calls`, `assistant_text_and_tool_calls`,
  `tool_result`.
- `error` — `ConfigError` (`InvalidSpec`, `Template`, `Schema`), raised while a
  binding profile is being registered, and `ProviderError` (`RateLimited`,
  `ContextLimit`, `AuthFailed`, `BadRequest`, `Server`, `Network`, `Malformed`)
  for live upstream failures. `ProviderError::is_retryable` marks the
  rate-limit / server / network arms retryable; `ProviderError::category`
  returns the bounded label used for metrics.

OpenAI Chat Completions is the reference shape — fields mirror that ABI, and
each provider adapter translates to and from its own wire format.

## Used by

- `mcpg-backend-llm-shared`, the provider-agnostic engine crate, which
  re-exports both modules so `mcpg_backend_llm_shared::normalized::…` paths
  resolve unchanged.
- Transitively, every per-provider LLM binding plugin in the workspace —
  `libs/plugins/backend/llms/{openai,anthropic,gemini,compat,stability}`.

## Build / test

```bash
cargo build -p mcpg-backend-llm-types
cargo test  -p mcpg-backend-llm-types
```

## Licence
Apache-2.0.

## See also

- Plugin classes and the plugin ABI: <https://mcpg.dev/docs/plugins/plugins-and-protocol>
- The full plugin catalog, including every LLM provider binding: <https://mcpg.dev/docs/plugins/plugin-catalogue>
- Engine, adapter trait, and streaming primitives: `libs/plugins/backend/llms/shared`
