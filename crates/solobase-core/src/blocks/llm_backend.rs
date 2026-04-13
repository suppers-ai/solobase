//! Shared trait and types for LLM backend blocks.
//!
//! Defines the contract that `suppers-ai/provider-llm` and `suppers-ai/local-llm`
//! implement. The `suppers-ai/llm` orchestrator routes requests to whichever
//! backend is configured.
//!
//! Currently the orchestrator dispatches via inter-block JSON calls
//! (`ctx.call_block`). This trait documents the shared interface and provides
//! typed structures for direct Rust-level integration in the future.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Configuration for a chat completion request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatConfig {
    /// Model identifier (e.g. "gpt-4o", "SmolLM2-1.7B-Instruct-q4f32_1-MLC").
    #[serde(default)]
    pub model: String,

    /// Provider-specific identifier (for multi-provider setups).
    #[serde(default)]
    pub provider_id: String,

    /// Maximum tokens to generate. `None` = provider default.
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

/// Information about an available model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Unique model identifier.
    pub id: String,

    /// Human-readable display name.
    #[serde(default)]
    pub name: String,

    /// Which provider offers this model.
    #[serde(default)]
    pub provider_id: String,

    /// Provider display name.
    #[serde(default)]
    pub provider_name: String,

    /// Provider type (e.g. "openai", "anthropic", "local").
    #[serde(default)]
    pub provider_type: String,
}

/// Non-streaming chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// The generated text content.
    pub content: String,

    /// The model that actually generated the response.
    #[serde(default)]
    pub model: String,

    /// Token usage statistics (if available).
    #[serde(default)]
    pub usage: Option<TokenUsage>,
}

/// Token usage statistics for a completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Backend trait for LLM providers.
///
/// Both `suppers-ai/provider-llm` (remote APIs) and `suppers-ai/local-llm`
/// (browser WebLLM) conform to this interface. The `suppers-ai/llm` orchestrator
/// can dispatch to any backend that implements this trait.
///
/// Uses the same conditional Send pattern as the rest of the codebase:
/// `#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]`
/// `#[cfg_attr(not(target_arch = "wasm32"), async_trait)]`
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait LlmBackend {
    /// Run a chat completion and return the full response.
    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        config: ChatConfig,
    ) -> Result<ChatResponse, String>;

    /// List models available from this backend.
    async fn list_models(&self) -> Result<Vec<ModelInfo>, String>;
}
