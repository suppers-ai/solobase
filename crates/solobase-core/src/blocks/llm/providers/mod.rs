//! `ProviderLlmService` — concrete `LlmService` impl for HTTP-based LLM
//! providers. Handles OpenAI native API, Anthropic native API, and any
//! OpenAI-compatible endpoint (Ollama, llama-server, LM Studio, vLLM,
//! Azure OpenAI, Groq, Together, OpenRouter, etc.).

pub mod anthropic;
pub mod config;
pub mod openai;
pub mod openai_compatible;
