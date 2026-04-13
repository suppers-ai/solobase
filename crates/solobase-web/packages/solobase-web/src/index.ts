/**
 * solobase-web — Batteries-included setup for running Solobase in the browser.
 *
 * Registers a Service Worker that boots the Solobase WASM runtime and intercepts
 * fetch requests. From the app's perspective, Solobase is a local HTTP server.
 *
 * Usage:
 *   import { setupSolobase } from 'solobase-web';
 *   await setupSolobase();
 */

export interface SolobaseOptions {
  /**
   * URL patterns the Service Worker should intercept.
   * Defaults to ['/b/**', '/health'].
   */
  routes?: string[];

  /**
   * Service Worker scope. Defaults to '/'.
   */
  scope?: string;

  /**
   * Path to the bundled Service Worker script.
   * Defaults to '/sw.js' (resolved relative to the page origin).
   */
  workerUrl?: string;
}

/**
 * Register the Solobase Service Worker, wait for it to activate, and hand off
 * control so all matching fetches are handled by the WASM runtime.
 *
 * Resolves once the SW is active and controlling the page. If this is the first
 * visit the page may need a reload — `setupSolobase` handles that automatically.
 */
export async function setupSolobase(options: SolobaseOptions = {}): Promise<void> {
  const {
    scope = '/',
    workerUrl = '/sw.js',
  } = options;

  if (!('serviceWorker' in navigator)) {
    throw new Error('Service Workers are not supported in this browser');
  }

  const registration = await navigator.serviceWorker.register(workerUrl, {
    type: 'module',
    scope,
  });

  // Wait for the SW to reach the activated state
  const sw = registration.installing || registration.waiting || registration.active;
  if (sw && sw.state !== 'activated') {
    await new Promise<void>((resolve) => {
      sw.addEventListener('statechange', () => {
        if (sw.state === 'activated') resolve();
      });
      // Already activated (race condition guard)
      if (sw.state === 'activated') resolve();
    });
  }

  // On first visit the SW isn't controlling the page yet — reload to hand off
  if (!navigator.serviceWorker.controller) {
    window.location.reload();
    // Never resolves — the page reloads
    return new Promise(() => {});
  }
}
