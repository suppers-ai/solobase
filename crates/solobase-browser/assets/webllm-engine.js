// webllm-engine.js — page-side WebLLM engine + SW postMessage bridge.
//
// Runs in the window (WebGPU is window-only). Receives requests from the SW
// via navigator.serviceWorker.message, runs WebLLM, streams chunks back.
//
// The @mlc-ai/web-llm import is lazy: a top-level static import would block
// DOMContentLoaded for every page that loads this script (it's a multi-MB
// jsdelivr ESM bundle), and most page loads never end up invoking the LLM.
// Defer the import until handleCreateEngine actually needs it.

let _CreateMLCEngine = null;
async function loadCreateMLCEngine() {
    if (_CreateMLCEngine) return _CreateMLCEngine;
    const mod = await import('https://cdn.jsdelivr.net/npm/@mlc-ai/web-llm@0.2.74/+esm');
    _CreateMLCEngine = mod.CreateMLCEngine;
    return _CreateMLCEngine;
}

let _engine = null;
let _engineModel = null;
const _activeStreams = new Map(); // id -> AbortController

async function swReply(payload) {
    const reg = await navigator.serviceWorker.ready;
    reg.active?.postMessage(payload);
}

async function handleCreateEngine(msg) {
    try {
        if (_engineModel !== msg.modelId) {
            if (_engine) {
                try { await _engine.unload(); } catch (_e) {}
                _engine = null;
                _engineModel = null;
            }
            const CreateMLCEngine = await loadCreateMLCEngine();
            _engine = await CreateMLCEngine(msg.modelId, { /* progress swallowed */ });
            _engineModel = msg.modelId;
        }
        await swReply({ type: 'llm-create-engine-response', id: msg.id });
    } catch (e) {
        await swReply({ type: 'llm-create-engine-response', id: msg.id, error: String(e) });
    }
}

async function handleUnload(msg) {
    try {
        if (_engine) {
            await _engine.unload();
            _engine = null;
            _engineModel = null;
        }
        await swReply({ type: 'llm-unload-response', id: msg.id });
    } catch (e) {
        await swReply({ type: 'llm-unload-response', id: msg.id, error: String(e) });
    }
}

async function handleChatStream(msg) {
    if (!_engine) {
        await swReply({ type: 'llm-chat-stream-error', id: msg.id, error: 'no engine loaded' });
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
            await swReply({
                type: 'llm-chat-stream-chunk',
                id: msg.id,
                chunk: JSON.stringify(chunk),
            });
        }
        await swReply({ type: 'llm-chat-stream-done', id: msg.id });
    } catch (e) {
        await swReply({ type: 'llm-chat-stream-error', id: msg.id, error: String(e) });
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
        case 'llm-create-engine-request': handleCreateEngine(msg); break;
        case 'llm-unload-request':        handleUnload(msg); break;
        case 'llm-chat-stream-request':   handleChatStream(msg); break;
        case 'llm-stream-cancel':         handleCancel(msg); break;
    }
});

console.log('webllm-engine.js loaded');
