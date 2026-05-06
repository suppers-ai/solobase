import type { Page } from '@playwright/test';

/**
 * Log in as the seeded admin user. solobase-web seeds
 * `admin@example.com` / `admin123` on first boot via
 * `SUPPERS_AI__AUTH__ADMIN_EMAIL` / `SUPPERS_AI__AUTH__ADMIN_PASSWORD`.
 *
 * Works against both the native solobase server (used for visual-baseline
 * snapshot generation, where argon2id runs in native Rust and is fast) and
 * the browser WASM static server (where PBKDF2 is used but is only needed
 * for SW-based integration tests).
 *
 * Caller must ensure the server is up before calling this helper.
 * The function navigates to the login page, submits the form, and waits
 * for the post-login redirect away from /b/auth/login.
 */
export async function loginAsAdmin(page: Page): Promise<void> {
  await page.goto('/b/auth/login');
  await page.fill('#email', 'admin@example.com');
  await page.fill('#password', 'admin123');
  await page.click('#btn');
  // Native argon2id completes in milliseconds. The fixture only needs the
  // URL to leave `/b/auth/login` — not the full target page to finish loading.
  // `waitUntil: 'commit'` fires as soon as the navigation commits to the new
  // URL, avoiding cumulative timeouts late in the suite when the post-login
  // dashboard page is slow to fire `load` (the `request_log` "Recent Errors"
  // table grows with traffic from prior tests; with 36+ tests in the suite
  // the default `'load'` wait can exceed 10s on the last few logins).
  await page.waitForURL((url) => !url.pathname.includes('/b/auth/login'), {
    timeout: 15_000,
    waitUntil: 'commit',
  });
}
