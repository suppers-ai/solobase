// ai-bridge.js — WebLLM integration for local model inference
// Runs in the main page (NOT the Service Worker) — needs WebGPU access.
// Uses WebLLM via CDN import.

import { CreateMLCEngine } from 'https://cdn.jsdelivr.net/npm/@mlc-ai/web-llm@0.2.74/+esm';

let engine = null;
let currentModel = null;
let loadProgress = null;

// Models are listed in two tiers:
// - q4f32: works on all WebGPU GPUs (no shader-f16 needed)
// - q4f16: smaller/faster but needs shader-f16 extension
const AVAILABLE_MODELS = [
    // f32 models — broad compatibility
    { id: "SmolLM2-1.7B-Instruct-q4f32_1-MLC", name: "SmolLM2 1.7B (1.1GB)", size_mb: 1100, requires_f16: false },
    { id: "Qwen2.5-1.5B-Instruct-q4f32_1-MLC", name: "Qwen 2.5 1.5B (1.2GB)", size_mb: 1200, requires_f16: false },
    { id: "gemma-2-2b-it-q4f32_1-MLC", name: "Gemma 2 2B (1.7GB)", size_mb: 1700, requires_f16: false },
    { id: "Phi-3.5-mini-instruct-q4f32_1-MLC", name: "Phi 3.5 Mini (2.6GB)", size_mb: 2600, requires_f16: false },
    { id: "Llama-3.2-3B-Instruct-q4f32_1-MLC", name: "Llama 3.2 3B (2GB)", size_mb: 2000, requires_f16: false },
    // f16 models — need shader-f16 support
    { id: "SmolLM2-1.7B-Instruct-q4f16_1-MLC", name: "SmolLM2 1.7B f16 (1GB)", size_mb: 1000, requires_f16: true },
    { id: "Qwen2.5-1.5B-Instruct-q4f16_1-MLC", name: "Qwen 2.5 1.5B f16 (1GB)", size_mb: 1000, requires_f16: true },
];

export async function getAvailableModels() {
    // Filter models based on GPU capabilities
    let hasF16 = false;
    try {
        if (navigator.gpu) {
            const adapter = await navigator.gpu.requestAdapter();
            if (adapter) hasF16 = adapter.features.has('shader-f16');
        }
    } catch (e) { /* ignore */ }
    return AVAILABLE_MODELS.filter(m => !m.requires_f16 || hasF16);
}

export function getStatus() {
    return {
        available: typeof navigator !== 'undefined' && 'gpu' in navigator,
        loaded_model: currentModel,
        load_progress: loadProgress,
        webgpu_supported: typeof navigator !== 'undefined' && 'gpu' in navigator,
    };
}

export async function loadModel(modelId, onProgress) {
    if (engine && currentModel === modelId) return; // Already loaded

    // Unload previous
    if (engine) {
        await engine.unload();
        engine = null;
        currentModel = null;
    }

    loadProgress = { model: modelId, progress: 0, text: 'Starting...' };

    engine = await CreateMLCEngine(modelId, {
        initProgressCallback: (report) => {
            loadProgress = { model: modelId, progress: report.progress, text: report.text };
            if (onProgress) onProgress(loadProgress);
        }
    });

    currentModel = modelId;
    loadProgress = { model: modelId, progress: 1, text: 'Ready' };
}

export async function unloadModel() {
    if (engine) {
        await engine.unload();
        engine = null;
        currentModel = null;
        loadProgress = null;
    }
}

export async function chat(messages, onChunk) {
    if (!engine) throw new Error('No model loaded');

    const response = await engine.chat.completions.create({
        messages: messages,
        stream: true,
    });

    let fullContent = '';
    for await (const chunk of response) {
        const delta = chunk.choices[0]?.delta?.content || '';
        fullContent += delta;
        if (onChunk) onChunk(delta, fullContent);
    }

    return { content: fullContent, model: currentModel };
}

// Make available globally for the chat page JS
window.solobaseAI = { getAvailableModels, getStatus, loadModel, unloadModel, chat };
