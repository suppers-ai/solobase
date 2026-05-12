// t2i-engine.js — page-side text-to-image engine.
//
// Runs in the window (WebGPU is window-only). Driven by SW postMessages from
// bridge.js's `image*` family. Structurally parallel to webllm-engine.js.
//
// SW → Page request shapes (see bridge.js for the producing side):
//   { type: 'image-load-request',          id, modelId }
//   { type: 'image-unload-request',        id }
//   { type: 'image-generate-stream-request', id, body }   // body = JSON ImageRequest
//   { type: 'image-stream-cancel',         id }
//
// Page → SW reply shapes:
//   { type: 'image-load-response',   id, error? }
//   { type: 'image-unload-response', id, error? }
//   { type: 'image-stream-frame',    id, kind, payload? }
//     `kind` ∈ {'progress','done','error'}.
//
// The @huggingface/transformers import is lazy: a top-level import would
// download a multi-MB ESM bundle on every page load, even for pages that
// never touch image generation. Defer until first use.

let _Pipeline = null;
async function loadTransformers() {
    if (_Pipeline) return _Pipeline;
    const mod = await import('https://cdn.jsdelivr.net/npm/@huggingface/transformers@3.0.0');
    _Pipeline = mod;
    return _Pipeline;
}

let _pipe = null;
let _pipeModel = null;
const _activeStreams = new Map(); // id -> AbortController

async function swPost(payload) {
    const reg = await navigator.serviceWorker.ready;
    reg.active?.postMessage(payload);
}

async function swStreamFrame(id, kind, payload) {
    await swPost({ type: 'image-stream-frame', id, kind, payload });
}

async function handleLoadEngine(msg) {
    try {
        const { pipeline } = await loadTransformers();
        if (_pipeModel === msg.modelId && _pipe) {
            await swPost({ type: 'image-load-response', id: msg.id });
            return;
        }
        if (_pipe) {
            try { await _pipe.dispose?.(); } catch (_e) {}
            _pipe = null;
            _pipeModel = null;
        }
        _pipe = await pipeline('text-to-image', msg.modelId, { device: 'webgpu' });
        _pipeModel = msg.modelId;
        await swPost({ type: 'image-load-response', id: msg.id });
    } catch (e) {
        await swPost({ type: 'image-load-response', id: msg.id, error: String(e) });
    }
}

async function handleUnloadEngine(msg) {
    try {
        if (_pipe) {
            try { await _pipe.dispose?.(); } catch (_e) {}
            _pipe = null;
            _pipeModel = null;
        }
        await swPost({ type: 'image-unload-response', id: msg.id });
    } catch (e) {
        await swPost({ type: 'image-unload-response', id: msg.id, error: String(e) });
    }
}

async function handleGenerateStream(msg) {
    if (!_pipe) {
        await swStreamFrame(msg.id, 'error', 'model not loaded; call load_model first');
        return;
    }
    const ac = new AbortController();
    _activeStreams.set(msg.id, ac);
    try {
        const req = JSON.parse(msg.body);
        const params = req.params || {};
        const result = await _pipe(req.prompt, {
            num_inference_steps: params.steps ?? 1,
            guidance_scale: params.guidance_scale ?? 0.0,
            width: params.width ?? 512,
            height: params.height ?? 512,
            negative_prompt: params.negative_prompt,
            seed: params.seed,
        });
        if (ac.signal.aborted) {
            await swStreamFrame(msg.id, 'error', 'cancelled');
            return;
        }
        const pngBytes = await rawImageToPng(result);
        const data = uint8ToBase64(pngBytes);
        await swStreamFrame(msg.id, 'done', { data, mime_type: 'image/png' });
    } catch (e) {
        await swStreamFrame(msg.id, 'error', String(e?.message ?? e));
    } finally {
        _activeStreams.delete(msg.id);
    }
}

function handleCancel(msg) {
    const ac = _activeStreams.get(msg.id);
    if (ac) ac.abort();
}

async function rawImageToPng(rawImage) {
    // transformers.js v3 returns a RawImage. Convert via OffscreenCanvas → PNG.
    // RawImage may be RGB (3 ch) or RGBA (4 ch); normalize to RGBA for canvas.
    const { width, height, data, channels } = rawImage;
    const canvas = new OffscreenCanvas(width, height);
    const ctx = canvas.getContext('2d');
    const imgData = ctx.createImageData(width, height);
    if (channels === 4) {
        imgData.data.set(data);
    } else if (channels === 3) {
        for (let i = 0, j = 0; i < width * height; i++, j += 3) {
            imgData.data[i * 4] = data[j];
            imgData.data[i * 4 + 1] = data[j + 1];
            imgData.data[i * 4 + 2] = data[j + 2];
            imgData.data[i * 4 + 3] = 255;
        }
    } else {
        throw new Error(`unsupported channel count: ${channels}`);
    }
    ctx.putImageData(imgData, 0, 0);
    const blob = await canvas.convertToBlob({ type: 'image/png' });
    return new Uint8Array(await blob.arrayBuffer());
}

function uint8ToBase64(u8) {
    let binary = '';
    const chunk = 0x8000;
    for (let i = 0; i < u8.length; i += chunk) {
        binary += String.fromCharCode.apply(null, u8.subarray(i, i + chunk));
    }
    return btoa(binary);
}

navigator.serviceWorker.addEventListener('message', (event) => {
    const msg = event.data;
    if (!msg || !msg.type) return;
    switch (msg.type) {
        case 'image-load-request':             handleLoadEngine(msg); break;
        case 'image-unload-request':           handleUnloadEngine(msg); break;
        case 'image-generate-stream-request':  handleGenerateStream(msg); break;
        case 'image-stream-cancel':            handleCancel(msg); break;
    }
});

// ---------------------------------------------------------------------------
// Page-direct load API (ESM exports).
//
// Mirrors `webllm-engine.js::{loadEngine,unloadEngine}`. Page-side consumers
// (e.g. the gizza-ai model picker) import these to drive the transformers.js
// pipeline directly in the window without bouncing through the service-worker
// bridge. Required because Chrome's FetchEvent.respondWith() lifetime cap
// kills SW-routed model downloads on cold (~500 MB) loads.
//
// `_pipe` / `_pipeModel` are the same module-scoped state used by the SW
// generate path's `handleGenerateStream` above. ESM modules are singletons
// within a realm, so `import { loadEngine } from '/t2i-engine.js'` from
// another script in the same window shares this state.
// ---------------------------------------------------------------------------
export async function loadEngine(modelId, onProgress) {
    const { pipeline } = await loadTransformers();
    if (_pipeModel === modelId && _pipe) {
        return; // already loaded
    }
    if (_pipe) {
        try { await _pipe.dispose?.(); } catch (_e) {}
        _pipe = null;
        _pipeModel = null;
    }
    _pipe = await pipeline('text-to-image', modelId, {
        device: 'webgpu',
        progress_callback: (report) => {
            if (typeof onProgress === 'function') {
                onProgress(String(report?.status ?? report?.file ?? ''));
            }
        },
    });
    _pipeModel = modelId;
}

export async function unloadEngine() {
    if (!_pipe) return;
    try { await _pipe.dispose?.(); } catch (_e) {}
    _pipe = null;
    _pipeModel = null;
}

console.log('t2i-engine.js loaded');
