//! Error taxonomy shared across every per-provider LLM binding plugin.
//!
//! - [`ConfigError`] surfaces at `register_profile` time; the plugin
//!   lifts it to [`mcpg_plugin_protocol::BackendError::InvalidSpec`].
//! - [`ProviderError`] surfaces during a live call when the upstream
//!   HTTP API misbehaves; the engine maps it to
//!   [`mcpg_plugin_protocol::BackendError::Transport`] after retry
//!   exhaustion.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid spec: {0}")]
    InvalidSpec(String),
    #[error("template parse failed: {0}")]
    Template(String),
    #[error("output schema invalid: {0}")]
    Schema(String),
}

#[derive(Debug, Error)]
pub enum ProviderError {
    /// HTTP 429 or provider-specific rate-limit indication.
    #[error("rate limited: {message}")]
    RateLimited { message: String },
    /// Token-window or input-size limit hit (HTTP 400 with specific body
    /// shape, or HTTP 413).
    #[error("context limit: {message}")]
    ContextLimit { message: String },
    /// HTTP 401 / 403 — bad API key, missing scope.
    #[error("auth failed: {message}")]
    AuthFailed { message: String },
    /// HTTP 4xx other than the above. Operator misconfiguration.
    #[error("bad request: {message}")]
    BadRequest { message: String },
    /// HTTP 5xx or transport-level upstream failure.
    #[error("server error: {message}")]
    Server { message: String },
    /// Network failure: DNS, connection, TLS handshake, body read.
    #[error("network error: {message}")]
    Network { message: String },
    /// Provider returned a response that didn't parse as the expected
    /// shape. Suggests a provider/version skew or adapter bug.
    #[error("malformed response: {message}")]
    Malformed { message: String },
}

impl ProviderError {
    /// Whether this kind of failure is worth retrying. The retry policy
    /// (`RetrySpec`) further filters by operator-allowed reasons.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited { .. } | Self::Server { .. } | Self::Network { .. }
        )
    }

    pub fn category(&self) -> &'static str {
        match self {
            Self::RateLimited { .. } => "rate_limited",
            Self::ContextLimit { .. } => "context_limit",
            Self::AuthFailed { .. } => "auth_failed",
            Self::BadRequest { .. } => "bad_request",
            Self::Server { .. } => "server",
            Self::Network { .. } => "network",
            Self::Malformed { .. } => "malformed",
        }
    }
}
