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
  // Native argon2id completes in milliseconds; allow up to 10s for the
  // login fetch + redirect to complete.
  await page.waitForURL((url) => !url.pathname.includes('/b/auth/login'), { timeout: 10_000 });
}
