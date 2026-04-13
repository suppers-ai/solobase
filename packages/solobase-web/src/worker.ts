// Re-export the WASM module's initialize and handleRequest for composable mode.
// Developers who have an existing SW can import these directly.

import init, { initialize as wasmInitialize, handle_request as wasmHandleRequest } from './wasm/solobase_web.js';

let initialized = false;
let routes: string[] = ['/b/', '/health', '/openapi.json', '/.well-known/agent.json'];

/**
 * Initialize the Solobase WASM runtime.
 * Call once before handling requests.
 */
export async function initialize(): Promise<void> {
  if (initialized) return;
  await init();
  await wasmInitialize();
  initialized = true;
}

/**
 * Handle an incoming fetch request through the Solobase WASM runtime.
 */
export async function handleRequest(request: Request): Promise<Response> {
  if (!initialized) {
    return new Response('Solobase not initialized', { status: 503 });
  }
  return await wasmHandleRequest(request);
}

/**
 * Check if a URL path should be handled by Solobase.
 */
function shouldIntercept(pathname: string): boolean {
  return routes.some((route) => pathname.startsWith(route));
}

// --- Batteries-included SW entry point ---
// When this file is loaded as a Service Worker directly, it auto-initializes
// and intercepts matching fetch events.

declare const self: ServiceWorkerGlobalScope;

if (typeof ServiceWorkerGlobalScope !== 'undefined') {
  self.addEventListener('install', (event) => {
    event.waitUntil(initialize().then(() => self.skipWaiting()));
  });

  self.addEventListener('activate', (event) => {
    event.waitUntil(self.clients.claim());
  });

  self.addEventListener('message', (event) => {
    if (event.data?.type === 'solobase:config' && Array.isArray(event.data.routes)) {
      routes = event.data.routes;
    }
  });

  self.addEventListener('fetch', (event) => {
    const url = new URL(event.request.url);
    if (shouldIntercept(url.pathname)) {
      event.respondWith(handleRequest(event.request));
    }
  });
}
