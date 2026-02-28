import { test, expect } from '@playwright/test';

// Check if the marketing site is reachable before running
let siteAvailable: boolean | null = null;

async function checkSiteAvailable(): Promise<boolean> {
  if (siteAvailable !== null) return siteAvailable;
  // Try both IPv6 and IPv4 since Vite may bind to either
  for (const host of ['[::1]', '127.0.0.1']) {
    try {
      const resp = await fetch(`http://${host}:5173/`, { signal: AbortSignal.timeout(3000) });
      if (resp.ok) {
        siteAvailable = true;
        return true;
      }
    } catch {
      // try next
    }
  }
  siteAvailable = false;
  return false;
}

test.describe('Marketing Site', () => {
  test.beforeEach(async () => {
    const available = await checkSiteAvailable();
    test.skip(!available, 'Marketing site not reachable on :5173');
  });

  test('home page shows logo and heading', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('h1')).toContainText('Solobase');
    await expect(page.locator('img[src="/images/logo_long.png"]').first()).toBeVisible();
  });

  test('header navigation links are visible', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('a[href="/pricing/"]').first()).toBeVisible();
    await expect(page.locator('a[href="/docs/"]').first()).toBeVisible();
  });

  test('pricing page shows all 5 plan names with prices', async ({ page }) => {
    await page.goto('/pricing/');
    await expect(page.getByText('Free', { exact: false }).first()).toBeVisible();
    await expect(page.getByText('Hobby').first()).toBeVisible();
    await expect(page.getByText('Starter').first()).toBeVisible();
    await expect(page.getByText('Professional').first()).toBeVisible();
    await expect(page.getByText('Business').first()).toBeVisible();

    await expect(page.getByText('$0').first()).toBeVisible();
    await expect(page.getByText('$5').first()).toBeVisible();
    await expect(page.getByText('$15').first()).toBeVisible();
    await expect(page.getByText('$79').first()).toBeVisible();
    await expect(page.getByText('$199').first()).toBeVisible();
  });

  test('professional plan has "Popular" badge', async ({ page }) => {
    await page.goto('/pricing/');
    await expect(page.getByText('Popular').first()).toBeVisible();
  });

  test('docs page has sidebar with "Getting Started"', async ({ page }) => {
    await page.goto('/docs/');
    await expect(page.getByText('Getting Started').first()).toBeVisible();
  });

  test('footer shows copyright text', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('footer')).toContainText('Suppers Software Limited');
  });
});
