// webllm-engine.js — page-side WebLLM engine + SW postMessage bridge.
//
// Runs in the window (WebGPU is window-only). Receives requests from the SW
// via navigator.serviceWorker.message, runs WebLLM, streams frames back.
//
// Page → SW message shapes (see bridge.js for the consuming side):
//   { type: 'llm-unload-response', id, error? }            // one-shot
//   { type: 'llm-stream-frame',    id, kind, payload? }    // streams
//     `kind` ∈ {'chunk','progress','done','error'};
//     create-engine emits 'progress' frames during the cold model download
//     and a terminal 'done' / 'error'; chat emits 'chunk' frames per token
//     and a terminal 'done' / 'error'. The unified envelope means one
//     dispatch arm in bridge.js and one Rust pump function for both ops.
//
// The @mlc-ai/web-llm import is lazy: a top-level static import would block
// DOMContentLoaded for every page that loads this script (it's a multi-MB
// jsdelivr ESM bundle), and most page loads never end up invoking the LLM.
// Defer the import until a handler actually needs it.

let _CreateMLCEngine = null;
async function loadCreateMLCEngine() {
    if (_CreateMLCEngine) return _CreateMLCEngine;
    const mod = await import('https://cdn.jsdelivr.net/npm/@mlc-ai/web-llm@0.2.74/+esm');
    _CreateMLCEngine = mod.CreateMLCEngine;
    return _CreateMLCEngine;
}

let _engine = null;
let _engineModel = null;
const _activeStreams = new Map(); // id -> AbortController (chat only)

async function swPost(payload) {
    const reg = await navigator.serviceWorker.ready;
    reg.active?.postMessage(payload);
}

async function swStreamFrame(id, kind, payload) {
    await swPost({ type: 'llm-stream-frame', id, kind, payload });
}

async function handleCreateEngineStream(msg) {
    try {
        if (_engineModel !== msg.modelId) {
            if (_engine) {
                try { await _engine.unload(); } catch (_e) {}
                _engine = null;
                _engineModel = null;
            }
            const CreateMLCEngine = await loadCreateMLCEngine();
            // WebLLM emits InitProgressReport ticks (~1/sec during shard
            // fetch) with a free-form `text` description like
            // "Fetching params [3/14]: 23%". We forward `text` as the
            // progress payload — gizza-app.js parses a percent out of it
            // for the UI bar; consumers that just want to keep the SSE
            // stream "warm" can ignore the contents.
            _engine = await CreateMLCEngine(msg.modelId, {
                initProgressCallback: (report) => {
                    swStreamFrame(msg.id, 'progress', String(report?.text ?? ''));
                },
            });
            _engineModel = msg.modelId;
        }
        await swStreamFrame(msg.id, 'done');
    } catch (e) {
        await swStreamFrame(msg.id, 'error', String(e));
    }
}

async function handleUnload(msg) {
    try {
        if (_engine) {
            await _engine.unload();
            _engine = null;
            _engineModel = null;
        }
        await swPost({ type: 'llm-unload-response', id: msg.id });
    } catch (e) {
        await swPost({ type: 'llm-unload-response', id: msg.id, error: String(e) });
    }
}

async function handleChatStream(msg) {
    if (!_engine) {
        await swStreamFrame(msg.id, 'error', 'no engine loaded');
        return;
    }
    const ac = new AbortController();
    _activeStreams.set(msg.id, ac);
    try {
        const body = JSON.parse(msg.body);
        const iterator = await _engine.chat.completions.create({
            messages: body.messages,
            tools: body.tools,
            stream: true,
        });
        for await (const chunk of iterator) {
            if (ac.signal.aborted) break;
            await swStreamFrame(msg.id, 'chunk', JSON.stringify(chunk));
        }
        await swStreamFrame(msg.id, 'done');
    } catch (e) {
        await swStreamFrame(msg.id, 'error', String(e));
    } finally {
        _activeStreams.delete(msg.id);
    }
}

function handleCancel(msg) {
    const ac = _activeStreams.get(msg.id);
    if (ac) ac.abort();
    // WebLLM's chat.completions iterator doesn't accept an AbortSignal, but
    // `interruptGenerate()` sets an internal flag the token loop checks —
    // this is the only way to actually stop GPU work mid-generation.
    if (_engine && typeof _engine.interruptGenerate === 'function') {
        _engine.interruptGenerate();
    }
}

navigator.serviceWorker.addEventListener('message', (event) => {
    const msg = event.data;
    if (!msg || !msg.type) return;
    switch (msg.type) {
        case 'llm-create-engine-stream-request': handleCreateEngineStream(msg); break;
        case 'llm-unload-request':               handleUnload(msg); break;
        case 'llm-chat-stream-request':          handleChatStream(msg); break;
        case 'llm-stream-cancel':                handleCancel(msg); break;
    }
});

console.log('webllm-engine.js loaded');
