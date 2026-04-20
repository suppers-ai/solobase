import { test, expect } from '@playwright/test';

/**
 * Lightweight smoke test that doesn't rebuild mid-test. Catches regressions
 * like the `/manifest.json` bypass bug and the `/sql-wasm-esm.js` import
 * path bug (both of which silently prevented SW registration).
 */
test('service worker registers and controls the page', async ({ page }) => {
  await page.goto('/');
  await page.waitForFunction(
    () => navigator.serviceWorker.controller !== null,
    null,
    { timeout: 20_000 },
  );
  const controllerURL = await page.evaluate(
    () => navigator.serviceWorker.controller?.scriptURL ?? null,
  );
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
