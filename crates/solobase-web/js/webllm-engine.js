// webllm-engine.js — minimal JS surface for the Rust BrowserLlmService.
//
// This module is the thin wasm-bindgen-facing layer over WebLLM's MLCEngine.
// It intentionally exposes the smallest possible surface: engine construction,
// unload, a streaming chat call, and an iterator pump. All higher-level logic
// (model filtering, request/chunk encoding, lifecycle tracking) lives in
// Rust (see `src/llm.rs`).
//
// Runs in the main page (NOT the Service Worker) — WebLLM requires WebGPU,
// which is only available in window contexts.

import { CreateMLCEngine } from 'https://cdn.jsdelivr.net/npm/@mlc-ai/web-llm@0.2.74/+esm';

/**
 * Create and fully initialize an MLCEngine for the given model id.
 *
 * @param {string} modelId
 * @param {(progress: {progress:number, text?:string}) => void} [onProgress]
 *        Called with WebLLM's raw initProgressCallback payload.
 * @returns {Promise<object>} The MLCEngine instance.
 */
export async function createEngine(modelId, onProgress) {
    return await CreateMLCEngine(modelId, {
        initProgressCallback: (p) => {
            if (onProgress) onProgress(p);
        },
    });
}

/**
 * Unload weights + free GPU buffers.
 * @param {object} engine MLCEngine handle returned by createEngine.
 */
export async function unloadEngine(engine) {
    return await engine.unload();
}

/**
 * Start a streaming chat completion.
 *
 * @param {object} engine         MLCEngine handle.
 * @param {string} messagesJson   JSON-encoded OpenAI-format messages array.
 * @returns {Promise<AsyncIterator>} An async iterator over OpenAI-format chunks.
 *                                  Caller pumps via `nextChunk`.
 */
export async function chatStream(engine, messagesJson) {
    const messages = JSON.parse(messagesJson);
    return await engine.chat.completions.create({
        messages,
        stream: true,
    });
}

/**
 * Pull the next chunk from an iterator obtained via `chatStream`.
 *
 * @param {AsyncIterator} iterator
 * @returns {Promise<string|null>} JSON-encoded OpenAI chunk, or null on done.
 */
export async function nextChunk(iterator) {
    const { value, done } = await iterator.next();
    if (done) return null;
    return JSON.stringify(value);
}
