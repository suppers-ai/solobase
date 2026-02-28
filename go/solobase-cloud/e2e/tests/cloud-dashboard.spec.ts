import { test, expect, type BrowserContext } from '@playwright/test';

/**
 * Helper: creates a dev session via /api/dev/session and adds the session
 * cookie to the browser context so subsequent page navigations are authenticated.
 * Returns the session token.
 */
async function authenticateContext(context: BrowserContext, baseURL: string): Promise<string> {
  const response = await context.request.get(`${baseURL}/api/dev/session`);
  expect(response.ok()).toBeTruthy();
  const body = await response.json();

  const cookies = await context.cookies(baseURL);
  const sessionCookie = cookies.find(c => c.name === 'session');
  if (!sessionCookie) {
    await context.addCookies([{
      name: 'session',
      value: body.token,
      domain: 'localhost',
      path: '/',
    }]);
  }
  return body.token;
}

/**
 * Helper: delete all tenants for the current session so each test starts clean.
 */
async function deleteAllTenants(context: BrowserContext, baseURL: string, token: string) {
  const resp = await context.request.get(`${baseURL}/api/tenants`, {
    headers: { Cookie: `session=${token}` },
  });
  if (!resp.ok()) return;
  const tenants = await resp.json();
  if (!Array.isArray(tenants)) return;
  for (const t of tenants) {
    await context.request.delete(`${baseURL}/api/tenants/${t.id}`, {
      headers: { Cookie: `session=${token}` },
    });
  }
}

test.describe('Cloud Dashboard', () => {
  test('unauthenticated user is redirected to /auth/login', async ({ page }) => {
    await page.goto('/');
    // The dashboard JS calls /api/tenants, gets 401, then does
    // window.location.href = '/auth/login'
    await page.waitForURL('**/auth/login**', { timeout: 10_000 });
  });

  test('authenticated user sees "Your Instances" heading', async ({ context, page, baseURL }) => {
    await authenticateContext(context, baseURL!);
    await page.goto('/');
    await expect(page.locator('h2')).toContainText('Your Instances');
  });

  test('empty state shows "No instances yet" message', async ({ context, page, baseURL }) => {
    const token = await authenticateContext(context, baseURL!);
    await deleteAllTenants(context, baseURL!, token);
    await page.goto('/');
    await expect(page.locator('.empty')).toContainText('No instances yet');
  });

  test('create tenant via dialog and see it in list', async ({ context, page, baseURL }) => {
    const token = await authenticateContext(context, baseURL!);
    await deleteAllTenants(context, baseURL!, token);
    await page.goto('/');
    await expect(page.locator('.empty')).toContainText('No instances yet');

    page.on('dialog', async dialog => {
      expect(dialog.type()).toBe('prompt');
      await dialog.accept('mytest');
    });

    await page.click('button:has-text("New Instance")');

    await expect(page.locator('.tenant-list li')).toHaveCount(1, { timeout: 5000 });
    await expect(page.locator('.tenant-name')).toContainText('mytest.solobase.app');
  });

  test('new tenant shows running status badge', async ({ context, page, baseURL }) => {
    const token = await authenticateContext(context, baseURL!);
    await deleteAllTenants(context, baseURL!, token);
    await page.goto('/');

    page.on('dialog', async dialog => {
      await dialog.accept('statustest');
    });

    await page.click('button:has-text("New Instance")');
    await expect(page.locator('.tenant-list li')).toHaveCount(1, { timeout: 5000 });
    await expect(page.locator('.status-running')).toBeVisible();
    await expect(page.locator('.status-running')).toContainText('running');
  });

  test('create two tenants and both are visible', async ({ context, page, baseURL }) => {
    const token = await authenticateContext(context, baseURL!);
    await deleteAllTenants(context, baseURL!, token);
    await page.goto('/');

    let promptCount = 0;
    const subdomains = ['first-app', 'second-app'];

    page.on('dialog', async dialog => {
      const idx = promptCount++;
      await dialog.accept(subdomains[idx] ?? '');
    });

    // Create first tenant
    await page.click('button:has-text("New Instance")');
    await expect(page.locator('.tenant-list li')).toHaveCount(1, { timeout: 5000 });

    // Create second tenant
    await page.click('button:has-text("New Instance")');
    await expect(page.locator('.tenant-list li')).toHaveCount(2, { timeout: 5000 });

    await expect(page.locator('.tenant-name').nth(0)).toBeVisible();
    await expect(page.locator('.tenant-name').nth(1)).toBeVisible();
  });

  test('canceling prompt dialog does not create a tenant', async ({ context, page, baseURL }) => {
    const token = await authenticateContext(context, baseURL!);
    await deleteAllTenants(context, baseURL!, token);
    await page.goto('/');
    await expect(page.locator('.empty')).toContainText('No instances yet');

    page.on('dialog', async dialog => {
      await dialog.dismiss();
    });

    await page.click('button:has-text("New Instance")');

    // Short wait to ensure no tenant was created
    await page.waitForTimeout(500);
    await expect(page.locator('.empty')).toContainText('No instances yet');
  });

  test('dashboard page title is "Solobase Cloud"', async ({ context, page, baseURL }) => {
    await authenticateContext(context, baseURL!);
    await page.goto('/');
    await expect(page).toHaveTitle('Solobase Cloud');
  });
});
