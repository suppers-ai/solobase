import { test, expect } from '@playwright/test';

/**
 * Lightweight smoke test that doesn't rebuild mid-test. Catches regressions
 * like the `/manifest.json` bypass bug and the `/sql-wasm-esm.js` import
 * path bug (both of which silently prevented SW registration).
 */
test('service worker registers and controls the page', async ({ page }) => {
  await page.goto('/');
  // Read the controller scriptURL inside the waitForFunction predicate so the
  // value is captured atomically. solobase-web's loader.js redirects to
  // `/b/system/` as soon as the SW takes control, which would otherwise
  // destroy the execution context between a separate `waitForFunction` +
  // `evaluate` pair.
  const handle = await page.waitForFunction(
    () => navigator.serviceWorker.controller?.scriptURL ?? null,
    null,
    { timeout: 20_000 },
  );
  const controllerURL = (await handle.jsonValue()) as string | null;
  expect(controllerURL).toMatch(/\/sw\.js$/);
});

test('solobase-web admin UI at /b/system/ renders after SW activation', async ({ page }) => {
  await page.goto('/');
  await page.waitForFunction(
    () => navigator.serviceWorker.controller !== null,
    null,
    { timeout: 20_000 },
  );
  // Navigate to the admin UI that solobase-web ships. Past the SW boundary,
  // the UI block renders HTML served from WAFER. Any title/heading works —
  // this smoke just confirms the runtime responds.
  await page.goto('/b/system/');
  const bodyText = await page.locator('body').textContent();
  expect(bodyText ?? '').not.toBe('');
});
