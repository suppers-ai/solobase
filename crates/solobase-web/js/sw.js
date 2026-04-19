// sw.js — Service Worker that runs Solobase via WASM
import init, { initialize, handle_request } from './solobase_web.js';

let initialized = false;
let initPromise = null;

async function ensureInitialized() {
    if (initialized) return;
    if (initPromise) return await initPromise;
    initPromise = (async () => {
        console.log('[solobase-web] Loading WASM module...');
        await init();
        console.log('[solobase-web] Initializing runtime...');
        await initialize();
        initialized = true;
        console.log('[solobase-web] Runtime ready.');
    })();
    await initPromise;
}

self.addEventListener('install', (event) => {
    console.log('[solobase-web] Service Worker installing...');
    event.waitUntil(self.skipWaiting());
});

self.addEventListener('activate', (event) => {
    console.log('[solobase-web] Service Worker activating...');
    event.waitUntil(self.clients.claim());
});

// ---------------------------------------------------------------------------
// Message bridge — asset-loader replies from the main thread.
// ---------------------------------------------------------------------------

self.addEventListener('message', (event) => {
    const msg = event.data;
    if (!msg) return;

    // Asset loader bridge: route reply to bridge.js's pending-load map.
    // bridge.js exposes the resolver on globalThis because this script
    // (sw.js) doesn't import the wasm-bindgen-generated bridge module.
    if (msg.type === 'load-asset-response') {
        if (typeof globalThis.__solobaseCompleteAssetLoad === 'function') {
            globalThis.__solobaseCompleteAssetLoad(msg.id, {
                status: msg.ok ? 'ready' : 'failed',
                error: msg.ok ? undefined : msg.error,
            });
        }
        return;
    }
});

// ---------------------------------------------------------------------------
// Fetch handler
// ---------------------------------------------------------------------------

self.addEventListener('fetch', (event) => {
    const url = new URL(event.request.url);
    // Only intercept same-origin requests
    if (url.origin !== self.location.origin) return;
    // Don't intercept requests for static assets served from CDN
    if (url.pathname === '/sw.js' ||
        url.pathname === '/loader.js' ||
        url.pathname === '/manifest.json' ||
        url.pathname === '/index.html' ||
        url.pathname === '/' ||
        url.pathname.startsWith('/pkg/') ||
        url.pathname.startsWith('/sql-')) {
        return;
    }
    event.respondWith(handleFetch(event.request));
});

async function handleFetch(request) {
    try {
        await ensureInitialized();
        return await handle_request(request);
    } catch (error) {
        console.error('[solobase-web] Error handling request:', error);
        return new Response(
            JSON.stringify({ error: 'internal_error', message: String(error) }),
            { status: 500, headers: { 'Content-Type': 'application/json' } }
        );
    }
}
