//! # mcpg-backend-llm-types
//!
//! Pure data types shared across MCPG's per-provider LLM binding
//! plugins — the leaf layer beneath `mcpg-backend-llm-shared`.
//!
//! - **Canonical shapes** ([`normalized`]): `NormalizedChatRequest`,
//!   `NormalizedChatResponse`, `Message`, `MessageContent`,
//!   `ContentPart`, `ToolCall`, `ToolDef`, `TokenUsage`,
//!   `FinishReason`, `Role`. The currency the engine and every
//!   provider adapter exchange.
//!
//! - **Error taxonomy** ([`error`]): `ConfigError` (surfaces at
//!   profile-registration time) and `ProviderError` (surfaces during a
//!   live upstream call).
//!
//! These carry no async, transport, or templating dependencies, so an
//! adapter that only needs the shapes can depend on this crate instead
//! of the full `mcpg-backend-llm-shared`. `mcpg-backend-llm-shared`
//! re-exports both modules, so `mcpg_backend_llm_shared::…` paths keep
//! resolving.

pub mod error;
pub mod normalized;
