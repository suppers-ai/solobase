export interface SolobaseConfig {
  /** URL paths to intercept (default: ['/b/', '/health']) */
  routes?: string[];
  /** Service Worker scope (default: '/') */
  scope?: string;
}

const DEFAULT_ROUTES = ['/b/', '/health', '/openapi.json', '/.well-known/agent.json'];

/**
 * Register a Service Worker that runs the Solobase WASM backend.
 * All matching requests are intercepted and handled by the WASM runtime.
 */
export async function setupSolobase(config?: SolobaseConfig): Promise<void> {
  if (!('serviceWorker' in navigator)) {
    throw new Error('Service Workers are not supported in this browser');
  }

  const scope = config?.scope ?? '/';
  const routes = config?.routes ?? DEFAULT_ROUTES;

  const registration = await navigator.serviceWorker.register(
    new URL('./worker.js', import.meta.url),
    { scope, type: 'module' }
  );

  // Wait for the SW to be active
  const sw = registration.installing || registration.waiting || registration.active;
  if (sw && sw.state !== 'activated') {
    await new Promise<void>((resolve) => {
      sw.addEventListener('statechange', () => {
        if (sw.state === 'activated') resolve();
      });
    });
  }

  // Send route config to the SW
  registration.active?.postMessage({ type: 'solobase:config', routes });
}
