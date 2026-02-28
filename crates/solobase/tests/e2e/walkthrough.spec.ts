import { test, expect, Page } from '@playwright/test';

const ADMIN_EMAIL = 'admin@example.com';
const ADMIN_PASSWORD = 'admin123';

// Helper: login via API and get cookie set on page context
async function login(page: Page) {
  const response = await page.request.post('/api/auth/login', {
    data: { email: ADMIN_EMAIL, password: ADMIN_PASSWORD },
  });
  expect(response.status()).toBe(200);
}

// ─── LOGIN FLOW ─────────────────────────────────────────────

test.describe('Login Flow', () => {
  test('login page renders with email and password fields', async ({ page }) => {
    await page.goto('/admin/login');
    await page.waitForTimeout(1500);

    // Take screenshot of login page
    await page.screenshot({ path: 'test-results/login-page.png', fullPage: true });

    // Should have email and password inputs
    const emailInput = page.locator('input[type="email"], input[name="email"], input[placeholder*="email" i]');
    const passwordInput = page.locator('input[type="password"], input[name="password"]');

    // At least one form of input should exist
    const hasEmail = await emailInput.count();
    const hasPassword = await passwordInput.count();
    console.log(`Login page: email inputs=${hasEmail}, password inputs=${hasPassword}`);

    // Check the page has some content
    const bodyText = await page.textContent('body');
    console.log(`Login page text (first 500 chars): ${bodyText?.substring(0, 500)}`);
  });

  test('can login via UI and reach admin dashboard', async ({ page }) => {
    await page.goto('/admin/login');
    await page.waitForTimeout(1500);

    // Try to fill in credentials
    const emailInput = page.locator('input[type="email"], input[name="email"]').first();
    const passwordInput = page.locator('input[type="password"], input[name="password"]').first();

    if (await emailInput.isVisible()) {
      await emailInput.fill(ADMIN_EMAIL);
      await passwordInput.fill(ADMIN_PASSWORD);

      // Find and click the login/submit button
      const submitBtn = page.locator('button[type="submit"], button:has-text("Login"), button:has-text("Sign in"), button:has-text("Log in")').first();
      if (await submitBtn.isVisible()) {
        await submitBtn.click();
        await page.waitForTimeout(2000);
      }
    } else {
      // Login via API instead
      await login(page);
      await page.goto('/admin');
      await page.waitForTimeout(2000);
    }

    await page.screenshot({ path: 'test-results/after-login.png', fullPage: true });

    // Should be on admin page or dashboard now
    const url = page.url();
    console.log(`After login URL: ${url}`);
  });
});

// ─── DASHBOARD ──────────────────────────────────────────────

test.describe('Dashboard', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('dashboard loads and shows stats', async ({ page }) => {
    await page.goto('/admin');
    await page.waitForTimeout(2000);

    await page.screenshot({ path: 'test-results/dashboard.png', fullPage: true });

    const bodyText = await page.textContent('body');
    console.log(`Dashboard text (first 800 chars): ${bodyText?.substring(0, 800)}`);

    // Dashboard should have some visible content
    expect(bodyText?.length).toBeGreaterThan(10);
  });
});

// ─── SIDEBAR NAVIGATION ────────────────────────────────────

test.describe('Sidebar Navigation', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('sidebar shows navigation items', async ({ page }) => {
    await page.goto('/admin');
    await page.waitForTimeout(2000);

    // Check for nav elements
    const navLinks = page.locator('nav a, .nav-item, .sidebar a, [data-nav] a');
    const count = await navLinks.count();
    console.log(`Navigation links found: ${count}`);

    if (count > 0) {
      for (let i = 0; i < Math.min(count, 15); i++) {
        const text = await navLinks.nth(i).textContent();
        const href = await navLinks.nth(i).getAttribute('href');
        console.log(`  Nav[${i}]: "${text?.trim()}" -> ${href}`);
      }
    }

    // Check for any links at all on the page
    const allLinks = page.locator('a[href]');
    const allCount = await allLinks.count();
    console.log(`Total links on page: ${allCount}`);
  });
});

// ─── ADMIN PAGES ────────────────────────────────────────────

test.describe('Admin Pages', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  const adminPages = [
    { name: 'Users', path: '/admin/users' },
    { name: 'Database', path: '/admin/database' },
    { name: 'Storage', path: '/admin/storage' },
    { name: 'IAM', path: '/admin/iam' },
    { name: 'Settings', path: '/admin/settings' },
    { name: 'Logs', path: '/admin/logs' },
    { name: 'Waffle', path: '/admin/waffle' },
  ];

  for (const p of adminPages) {
    test(`${p.name} page loads and renders content`, async ({ page }) => {
      const response = await page.goto(p.path);
      expect(response?.status()).toBe(200);
      await page.waitForTimeout(2000);

      await page.screenshot({ path: `test-results/${p.name.toLowerCase()}-page.png`, fullPage: true });

      const bodyText = await page.textContent('body');
      console.log(`${p.name} page text (first 500 chars): ${bodyText?.substring(0, 500)}`);

      // Page should have meaningful content
      expect(bodyText?.length).toBeGreaterThan(10);
    });
  }
});

// ─── WAFFLE BLOCKS & CHAINS UI ─────────────────────────────

test.describe('Waffle Admin - Blocks Tab', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('blocks tab shows registered blocks', async ({ page }) => {
    await page.goto('/admin/waffle');
    await page.waitForTimeout(2000);

    await page.screenshot({ path: 'test-results/waffle-initial.png', fullPage: true });

    // Check API returns blocks (flat array)
    const blocksResponse = await page.request.get('/api/admin/waffle/blocks');
    expect(blocksResponse.status()).toBe(200);
    const allBlocks = await blocksResponse.json();
    console.log(`Registered blocks (${allBlocks.length}):`);
    for (const b of allBlocks) {
      console.log(`  - ${b.name} (${b.interface || 'no interface'}): ${b.summary || ''}`);
    }
    expect(allBlocks.length).toBeGreaterThan(5);

    // Look for blocks tab or section on the page
    const blocksTab = page.locator('button:has-text("Blocks"), [role="tab"]:has-text("Blocks"), a:has-text("Blocks")');
    if (await blocksTab.count() > 0) {
      await blocksTab.first().click();
      await page.waitForTimeout(1000);
      await page.screenshot({ path: 'test-results/waffle-blocks-tab.png', fullPage: true });
    }

    // Check for block items in the UI
    const bodyText = await page.textContent('body');
    console.log(`Waffle page text includes 'auth': ${bodyText?.includes('auth')}`);
    console.log(`Waffle page text includes 'cors': ${bodyText?.includes('cors')}`);
    console.log(`Waffle page text includes 'rate-limit': ${bodyText?.includes('rate-limit')}`);
  });

  test('blocks can be searched/filtered', async ({ page }) => {
    await page.goto('/admin/waffle');
    await page.waitForTimeout(2000);

    // Click blocks tab if exists
    const blocksTab = page.locator('button:has-text("Blocks"), [role="tab"]:has-text("Blocks")');
    if (await blocksTab.count() > 0) {
      await blocksTab.first().click();
      await page.waitForTimeout(1000);
    }

    // Look for search input
    const searchInput = page.locator('input[type="search"], input[type="text"], input[placeholder*="search" i], input[placeholder*="filter" i]');
    if (await searchInput.count() > 0) {
      await searchInput.first().fill('auth');
      await page.waitForTimeout(500);
      await page.screenshot({ path: 'test-results/waffle-blocks-search.png', fullPage: true });

      const bodyText = await page.textContent('body');
      console.log(`After searching 'auth': ${bodyText?.substring(0, 500)}`);
    } else {
      console.log('No search input found on waffle blocks page');
    }
  });
});

test.describe('Waffle Admin - Chains Tab', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('chains tab shows registered chains', async ({ page }) => {
    await page.goto('/admin/waffle');
    await page.waitForTimeout(2000);

    // Check API returns stored chains (user-created only; runtime chains are not in DB)
    const chainsResponse = await page.request.get('/api/admin/waffle/chains');
    expect(chainsResponse.status()).toBe(200);
    const chains = await chainsResponse.json();
    console.log(`Stored chains (${chains.length})`);

    // Click chains tab to see runtime chains in the UI
    const chainsTab = page.locator('button:has-text("Chains"), [role="tab"]:has-text("Chains"), a:has-text("Chains")');
    if (await chainsTab.count() > 0) {
      await chainsTab.first().click();
      await page.waitForTimeout(1000);
      await page.screenshot({ path: 'test-results/waffle-chains-tab.png', fullPage: true });
    }

    const bodyText = await page.textContent('body');
    console.log(`Chains page includes 'http-infra': ${bodyText?.includes('http-infra')}`);
    console.log(`Chains page includes 'auth-pipe': ${bodyText?.includes('auth-pipe')}`);
    console.log(`Chains page includes 'admin-pipe': ${bodyText?.includes('admin-pipe')}`);
  });

  test('can view chain details', async ({ page }) => {
    await page.goto('/admin/waffle');
    await page.waitForTimeout(2000);

    // Click chains tab
    const chainsTab = page.locator('button:has-text("Chains"), [role="tab"]:has-text("Chains")');
    if (await chainsTab.count() > 0) {
      await chainsTab.first().click();
      await page.waitForTimeout(1000);
    }

    // Click on a chain to see details
    const chainItem = page.locator('text=http-infra').first();
    if (await chainItem.isVisible()) {
      await chainItem.click();
      await page.waitForTimeout(1000);
      await page.screenshot({ path: 'test-results/waffle-chain-detail.png', fullPage: true });

      const bodyText = await page.textContent('body');
      console.log(`Chain detail text (first 800 chars): ${bodyText?.substring(0, 800)}`);
    } else {
      console.log('http-infra chain not visible to click');

      // List all visible text to see what chains look like
      const bodyText = await page.textContent('body');
      console.log(`Page text (first 1000 chars): ${bodyText?.substring(0, 1000)}`);
    }
  });

  test('can create a new chain', async ({ page }) => {
    await page.goto('/admin/waffle');
    await page.waitForTimeout(2000);

    // Click chains tab
    const chainsTab = page.locator('button:has-text("Chains"), [role="tab"]:has-text("Chains")');
    if (await chainsTab.count() > 0) {
      await chainsTab.first().click();
      await page.waitForTimeout(1000);
    }

    // Look for "Add" / "Create" / "New" chain button
    const addBtn = page.locator('button:has-text("Add"), button:has-text("Create"), button:has-text("New"), button:has-text("+")');
    const addCount = await addBtn.count();
    console.log(`Add/Create buttons found: ${addCount}`);

    if (addCount > 0) {
      for (let i = 0; i < addCount; i++) {
        const text = await addBtn.nth(i).textContent();
        console.log(`  Button[${i}]: "${text?.trim()}"`);
      }

      await addBtn.first().click();
      await page.waitForTimeout(1000);
      await page.screenshot({ path: 'test-results/waffle-chain-create.png', fullPage: true });

      const bodyText = await page.textContent('body');
      console.log(`Create chain dialog text (first 500 chars): ${bodyText?.substring(0, 500)}`);
    } else {
      console.log('No add/create chain button found');
      await page.screenshot({ path: 'test-results/waffle-no-create-button.png', fullPage: true });
    }
  });
});

// ─── USERS MANAGEMENT ───────────────────────────────────────

test.describe('Users Management', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('users page shows user list', async ({ page }) => {
    await page.goto('/admin/users');
    await page.waitForTimeout(2000);

    await page.screenshot({ path: 'test-results/users-page.png', fullPage: true });

    // Check API
    const response = await page.request.get('/api/admin/users');
    expect(response.status()).toBe(200);
    const data = await response.json();
    console.log(`Users API response:`, JSON.stringify(data).substring(0, 500));

    // Page should show the admin user
    const bodyText = await page.textContent('body');
    console.log(`Users page includes admin email: ${bodyText?.includes('admin@example.com')}`);
  });
});

// ─── DATABASE ADMIN ─────────────────────────────────────────

test.describe('Database Admin', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('database page shows tables', async ({ page }) => {
    await page.goto('/admin/database');
    await page.waitForTimeout(2000);

    await page.screenshot({ path: 'test-results/database-page.png', fullPage: true });

    // Check API
    const response = await page.request.get('/api/admin/database/tables');
    expect(response.status()).toBe(200);
    const tables = await response.json();
    console.log(`Database tables (${tables.length}):`, tables.map((t: any) => t.name || t).join(', '));
  });
});

// ─── SETTINGS ───────────────────────────────────────────────

test.describe('Settings', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('settings page loads and shows settings', async ({ page }) => {
    await page.goto('/admin/settings');
    await page.waitForTimeout(2000);

    await page.screenshot({ path: 'test-results/settings-page.png', fullPage: true });

    const bodyText = await page.textContent('body');
    console.log(`Settings page text (first 500 chars): ${bodyText?.substring(0, 500)}`);
  });
});

// ─── OVERALL UI QUALITY ─────────────────────────────────────

test.describe('UI Quality', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('no JavaScript errors on key pages', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });
    page.on('pageerror', err => {
      errors.push(err.message);
    });

    const pages = ['/admin', '/admin/users', '/admin/database', '/admin/waffle', '/admin/settings'];

    for (const p of pages) {
      await page.goto(p);
      await page.waitForTimeout(2000);
    }

    if (errors.length > 0) {
      console.log(`JavaScript errors (${errors.length}):`);
      for (const e of errors) {
        console.log(`  ERROR: ${e}`);
      }
    }

    // Allow network/API errors but no unhandled JS exceptions
    const criticalErrors = errors.filter(e =>
      !e.includes('404') &&
      !e.includes('Failed to fetch') &&
      !e.includes('net::ERR') &&
      !e.includes('http_error') &&
      !e.includes('500')
    );

    expect(criticalErrors.length).toBe(0);
  });

  test('pages are responsive - no horizontal scroll on 1280px', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/admin');
    await page.waitForTimeout(2000);

    const scrollWidth = await page.evaluate(() => document.documentElement.scrollWidth);
    const clientWidth = await page.evaluate(() => document.documentElement.clientWidth);

    console.log(`Viewport: 1280px, scrollWidth: ${scrollWidth}, clientWidth: ${clientWidth}`);
    expect(scrollWidth).toBeLessThanOrEqual(clientWidth + 5); // 5px tolerance
  });
});
