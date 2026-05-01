import type { Page } from '@playwright/test';

/**
 * Log in as the seeded admin user. solobase-web seeds
 * `admin@solobase.local` / `admin` on first boot.
 *
 * Caller must `page.goto('/')` first to register the SW; this helper
 * waits for SW activation, then performs the login form submission and
 * waits for the post-login redirect.
 */
export async function loginAsAdmin(page: Page): Promise<void> {
  await page.goto('/');
  await page.waitForFunction(
    () => navigator.serviceWorker.controller !== null,
    null,
    { timeout: 20_000 },
  );
  await page.goto('/b/auth/login');
  await page.fill('#email', 'admin@solobase.local');
  await page.fill('#password', 'admin');
  await page.click('#btn');
  await page.waitForURL((url) => !url.pathname.includes('/b/auth/login'), { timeout: 10_000 });
}
