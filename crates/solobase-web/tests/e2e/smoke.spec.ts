import { test, expect } from '@playwright/test';

/**
 * Lightweight smoke test that doesn't rebuild mid-test. Catches regressions
 * like the `/manifest.json` bypass bug and the `/sql-wasm-esm.js` import
 * path bug (both of which silently prevented SW registration).
 */
test('service worker registers and controls the page', async ({ page }) => {
  // `commit` is the right waitUntil here: this test exercises SW registration,
  // which `loader.js` triggers as soon as it parses (registration is async on
  // top of that). The downstream `waitForFunction(() => navigator.serviceWorker
  // .controller)` provides the actual assertion timing. Default `load` blocks
  // on every subresource; even `domcontentloaded` is delayed by deferred and
  // module scripts. Neither fires reliably here because the loader page imports
  // `/webllm-engine.js` and `/embed-engine.js` (type="module"), and a slow
  // jsdelivr CDN response for either one used to push the goto past the 60s
  // test timeout. Lazy-loading the WebLLM ESM (see webllm-engine.js) removed
  // most of the slowness, but `commit` is still the semantically correct
  // waitUntil for an SW-registration smoke and survives future regressions.
  await page.goto('/', { waitUntil: 'commit' });
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
  await page.goto('/', { waitUntil: 'commit' });
  // loader.js redirects to `/b/system/` as soon as the SW takes control.
  // Wait for that redirect to land instead of issuing our own goto — an
  // explicit `page.goto('/b/system/')` here would race with the loader's
  // `window.location.href` assignment and abort with
  // `net::ERR_ABORTED; maybe frame was detached?`. The redirect itself
  // exercises the SW serving the admin UI through WAFER, which is what
  // this smoke is verifying.
  //
  // The 50s timeout accounts for the SW's first-request init: loading
  // the multi-MB solobase-web wasm + sql.js (FTS5) + Transformers.js
  // wiring on a cold CI cache. Locally everything is cached so the
  // redirect lands in <1s, but CI cold-starts can take 25-30s before
  // the SW responds to its first fetch.
  await page.waitForURL(/\/b\/system\/?$/, { timeout: 50_000 });
  const bodyText = await page.locator('body').textContent();
  expect(bodyText ?? '').not.toBe('');
});
