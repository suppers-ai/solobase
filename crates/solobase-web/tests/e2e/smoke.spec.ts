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
  // `boot_redirect` as soon as the SW takes control, which would otherwise
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

test('boot redirect lands on the auth login page', async ({ page, context }) => {
  // The vector backend ships sql.js (FTS5) + the Transformers.js bridge,
  // which makes the SW's first-request init substantially heavier than
  // before this PR. Locally with a warm browser cache the redirect lands
  // in <1s; on a cold CI runner it can take well over 50s before the SW
  // responds to its first fetch. Bump the per-test timeout and the
  // waitForURL timeout so the smoke survives the cold path. Override
  // applies to this test only — the SW-registration smoke above stays
  // on the file-level 60s timeout.
  test.setTimeout(180_000);

  // Instrumentation — dump console messages, page errors, and SW-relevant
  // network activity so a CI failure surfaces what's actually happening
  // inside the SW init. Remove once the smoke is green on CI.
  const start = Date.now();
  const ts = () => `+${(Date.now() - start).toString().padStart(6, ' ')}ms`;
  page.on('console', (msg) => {
    console.log(`${ts()} [console:${msg.type()}] ${msg.text()}`);
  });
  page.on('pageerror', (err) => {
    console.log(`${ts()} [pageerror] ${err.message}`);
  });
  page.on('requestfailed', (req) => {
    console.log(
      `${ts()} [reqfail] ${req.method()} ${req.url()} — ${req.failure()?.errorText ?? '?'}`,
    );
  });
  page.on('response', (res) => {
    const u = res.url();
    if (u.includes('localhost:8080') || u.includes('127.0.0.1:8080')) {
      console.log(`${ts()} [res] ${res.status()} ${u.replace(/^https?:\/\/[^/]+/, '')}`);
    }
  });
  // SW console isn't captured by page.on('console') — hook every SW that
  // shows up on the context so we can see ensureInitialized progress.
  context.on('serviceworker', (sw) => {
    console.log(`${ts()} [sw:new] ${sw.url()}`);
    sw.on('console', (msg) => {
      console.log(`${ts()} [sw:${msg.type()}] ${msg.text()}`);
    });
  });
  // Belt-and-suspenders: also poll the SW state from the page so we know
  // whether registration and activation are happening at all.
  const pollSwState = async (label: string) => {
    try {
      const state = await page.evaluate(async () => {
        const reg = await navigator.serviceWorker.getRegistration();
        return {
          hasReg: !!reg,
          installing: reg?.installing?.state ?? null,
          waiting: reg?.waiting?.state ?? null,
          active: reg?.active?.state ?? null,
          controller: navigator.serviceWorker.controller?.scriptURL ?? null,
        };
      });
      console.log(`${ts()} [sw:state ${label}] ${JSON.stringify(state)}`);
    } catch (e) {
      console.log(`${ts()} [sw:state ${label}] err: ${(e as Error).message}`);
    }
  };

  // boot_redirect is "/" (intercepted by SW → wasm router → 302 →
  // /b/auth/login for anonymous visitors). loader.js sets
  // `window.location.href = boot_redirect` once the SW takes control;
  // waiting for the resulting URL match avoids the
  // `net::ERR_ABORTED; maybe frame was detached?` race that an explicit
  // second goto would hit.
  //
  // Asserting on the rendered Sign In form rather than a non-empty body
  // catches the regression where boot_redirect pointed at /b/system/ —
  // an unclaimed path that returned a non-empty 404 page and silently
  // passed the smoke.
  console.log(`${ts()} starting goto`);
  await page.goto('/', { waitUntil: 'commit' });
  console.log(`${ts()} goto returned, waiting for /b/auth/login`);
  // Periodic SW-state snapshots so a stalled state shows up.
  const poller = setInterval(() => {
    void pollSwState('poll');
  }, 5000);
  try {
    await page.waitForURL(/\/b\/auth\/login/, { timeout: 150_000 });
    console.log(`${ts()} reached /b/auth/login`);
  } catch (e) {
    console.log(`${ts()} waitForURL FAILED: page.url()=${page.url()}`);
    await pollSwState('on-fail');
    throw e;
  } finally {
    clearInterval(poller);
  }
  await expect(page.locator('input#email')).toBeVisible();
  await expect(page.locator('input#password')).toBeVisible();
});
