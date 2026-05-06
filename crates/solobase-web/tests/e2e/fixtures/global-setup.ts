import { request as playwrightRequest, type FullConfig } from '@playwright/test';
import { mkdirSync, readFileSync } from 'node:fs';
import { dirname } from 'node:path';

/**
 * Phase 5d Item A — single login per test run, shared via storageState.
 *
 * Before this hook landed, every test in `visual-baseline.spec.ts` did a
 * fresh form-submit login (~38 logins per CI run). Past ~35 logins the auth
 * block's cumulative rate-limit kicked in and form-submit redirects started
 * timing out. PR-3 had to drop the `admin-mobile storage-buckets` baseline as
 * a result.
 *
 * The structural fix: log in once here, save the `auth_token` cookie via
 * Playwright's storageState, then admin-scoped describes opt-in via
 * `test.use({ storageState: ADMIN_STATE_PATH })`. Anonymous describes leave
 * the config default (no storageState) so they get a clean context.
 *
 * This file is referenced by `playwright.config.ts` via `globalSetup`. It runs
 * once before the test runner spawns workers. The output file lives at
 * `tests/.auth/admin-state.json` and is gitignored — it carries a real session
 * token and must not be committed.
 *
 * Login uses the JSON endpoint (`POST /b/auth/api/login`) rather than the
 * form-submit page so we don't depend on page rendering. The endpoint sets
 * the same `auth_token` HttpOnly cookie that form-submit does (see
 * `crates/solobase-core/src/blocks/auth/mod.rs::build_auth_cookie`).
 */

export const ADMIN_EMAIL = process.env.SUPPERS_AI__AUTH__ADMIN_EMAIL ?? 'admin@example.com';
export const ADMIN_PASSWORD = process.env.SUPPERS_AI__AUTH__ADMIN_PASSWORD ?? 'admin123';

export const ADMIN_STATE_PATH = new URL('../../.auth/admin-state.json', import.meta.url).pathname;

export default async function globalSetup(config: FullConfig): Promise<void> {
  const baseURL =
    config.projects[0]?.use?.baseURL ??
    `http://127.0.0.1:${process.env.TEST_PORT ?? 8080}`;

  // Ensure the output directory exists before storageState() tries to write.
  mkdirSync(dirname(ADMIN_STATE_PATH), { recursive: true });

  // Use a request-only context — no browser launch needed for a JSON login.
  // The resulting storageState is browser-compatible (cookies are domain-scoped,
  // not bound to a browser session).
  const requestContext = await playwrightRequest.newContext({ baseURL });

  const res = await requestContext.post('/b/auth/api/login', {
    data: { email: ADMIN_EMAIL, password: ADMIN_PASSWORD },
    headers: { 'Content-Type': 'application/json' },
  });

  if (res.status() !== 200) {
    const body = await res.text().catch(() => '<unreadable>');
    throw new Error(
      `globalSetup: admin login failed (status=${res.status()}). ` +
        `Is the server up at ${baseURL}? Body: ${body}`,
    );
  }

  await requestContext.storageState({ path: ADMIN_STATE_PATH });
  await requestContext.dispose();

  // Verify the cookie we expect actually landed in storageState. Without this
  // a misconfigured server (e.g. one that returned 200 but didn't set a cookie)
  // would silently produce an empty state file and every admin test would
  // redirect back to /b/auth/login.
  const saved = JSON.parse(readFileSync(ADMIN_STATE_PATH, 'utf8')) as {
    cookies: Array<{ name: string }>;
  };
  const hasAuthCookie = saved.cookies.some((c) => c.name === 'auth_token');
  if (!hasAuthCookie) {
    throw new Error(
      `globalSetup: login returned 200 but no auth_token cookie was saved. ` +
        `storageState cookies: ${JSON.stringify(saved.cookies.map((c) => c.name))}`,
    );
  }
}
