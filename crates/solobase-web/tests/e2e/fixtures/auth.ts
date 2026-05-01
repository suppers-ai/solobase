import type { Page } from '@playwright/test';

/**
 * Log in as the seeded admin user. solobase-web seeds
 * `admin@solobase.local` / `admin` on first boot via
 * `SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_EMAIL` /
 * `SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_PASSWORD`.
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
  await page.fill('#email', 'admin@solobase.local');
  await page.fill('#password', 'admin');
  await page.click('#btn');
  // Native argon2id completes in milliseconds; allow up to 10s for the
  // login fetch + redirect to complete.
  await page.waitForURL((url) => !url.pathname.includes('/b/auth/login'), { timeout: 10_000 });
}
