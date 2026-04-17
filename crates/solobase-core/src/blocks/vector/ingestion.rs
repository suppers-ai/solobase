//! Document chunking and optional contextual retrieval.
//!
//! This module provides the building blocks used by the `POST
//! /b/vector/api/ingest` route to split a document into embedding-sized
//! chunks and (eventually) prepend per-chunk context summaries via an LLM.
//!
//! The chunker is intentionally simple: we whitespace-split as a token
//! proxy, which is close enough for bge-m3 / MiniLM at this granularity.
//! A real tokenizer would drift by a handful of tokens per chunk but
//! costs a dependency + runtime work that buys us nothing at ingest time.
//!
//! `add_context` is a stub right now — it returns the chunks unchanged.
//! Hooking it up to an LLM lives in a follow-up task; wiring the call
//! into the ingest route today means the route doesn't have to change
//! when that lands.

use wafer_run::{context::Context, types::WaferError};

/// Approximate max tokens per chunk. We use whitespace-split as a proxy
/// for tokenization — close enough for bge-m3 / MiniLM at this
/// granularity, and avoids pulling a tokenizer crate into the ingest path.
pub const DEFAULT_CHUNK_TOKENS: usize = 512;

/// Fraction of overlap between adjacent chunks.
///
/// 10% is a safe default: enough to keep entity mentions and sentence
/// boundaries intact across splits without blowing up the number of
/// chunks an average document produces.
pub const DEFAULT_OVERLAP_RATIO: f32 = 0.10;

/// Split `text` into overlapping chunks of approximately `chunk_tokens`
/// tokens, with adjacent chunks sharing `chunk_tokens * overlap_ratio`
/// tokens on each boundary.
///
/// Tokens are approximated by whitespace-splitting — see the module doc.
/// Empty input returns an empty vec; input shorter than `chunk_tokens`
/// returns a single chunk with the original whitespace collapsed.
pub fn chunk(text: &str, chunk_tokens: usize, overlap_ratio: f32) -> Vec<String> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.is_empty() {
        return Vec::new();
    }
    if tokens.len() <= chunk_tokens {
        return vec![tokens.join(" ")];
    }

    // Guard against overlap_ratio >= 1.0 — that would give us a
    // non-advancing stride and an infinite loop. `stride = 1` is the
    // degenerate-but-terminating choice when overlap ties or exceeds the
    // chunk size.
    let overlap = ((chunk_tokens as f32) * overlap_ratio).round() as usize;
    let stride = chunk_tokens.saturating_sub(overlap).max(1);

    let mut out = Vec::new();
    let mut start = 0usize;
    while start < tokens.len() {
        let end = (start + chunk_tokens).min(tokens.len());
        out.push(tokens[start..end].join(" "));
        if end == tokens.len() {
            break;
        }
        start += stride;
    }
    out
}

/// Prepend a context summary to each chunk using an LLM.
///
/// This is a scaffold: today it just returns the chunks unchanged so the
/// ingest route can call into it without a surface-area change when the
/// real LLM integration lands. `document` is the full source document
/// (so a future implementation can ask the LLM to summarize each chunk
/// in the context of the whole), `chunks` is the output of `chunk()`.
pub async fn add_context(
    ctx: &dyn Context,
    document: &str,
    chunks: Vec<String>,
) -> Result<Vec<String>, WaferError> {
    // Explicitly discard the unused args to make the "stub for now"
    // intent visible — rather than leaving `_ctx`/`_document` in the
    // signature, which would mask future compiler warnings once those
    // args start being used.
    let _ = (ctx, document);
    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_empty_returns_empty() {
        assert!(chunk("", 10, 0.1).is_empty());
    }

    #[test]
    fn chunk_small_text_single_chunk() {
        let c = chunk("hello world", 10, 0.1);
        assert_eq!(c, vec!["hello world"]);
    }

    #[test]
    fn chunk_long_text_overlaps() {
        let text = (1..=20)
            .map(|i| format!("w{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        // chunk_tokens=8, overlap_ratio=0.25 → overlap=2, stride=6
        let c = chunk(&text, 8, 0.25);
        assert!(c.len() >= 3);
        // First chunk: w1..w8.
        assert_eq!(c[0], "w1 w2 w3 w4 w5 w6 w7 w8");
        // Second chunk starts at w7 (overlap of 2 from the first chunk).
        assert!(c[1].starts_with("w7 w8 w9"));
    }

    #[test]
    fn chunk_no_overlap_when_ratio_zero() {
        let text = (1..=20)
            .map(|i| format!("w{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        // chunk_tokens=5, overlap_ratio=0 → stride=5, 20 tokens / 5 = 4 chunks.
        let c = chunk(&text, 5, 0.0);
        assert_eq!(c.len(), 4);
        assert!(c[1].starts_with("w6"));
    }
}
