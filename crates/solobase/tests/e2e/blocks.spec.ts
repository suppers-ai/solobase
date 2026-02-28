import { test, expect } from '@playwright/test';

const ADMIN_EMAIL = 'admin@example.com';
const ADMIN_PASSWORD = 'admin123';

// Helper: login and get auth cookie
async function login(page) {
  const response = await page.request.post('/api/auth/login', {
    data: { email: ADMIN_EMAIL, password: ADMIN_PASSWORD },
  });
  expect(response.status()).toBe(200);
}

test.describe('API Health', () => {
  test('health endpoint returns ok', async ({ request }) => {
    const response = await request.get('/api/health');
    expect(response.status()).toBe(200);
    const body = await response.json();
    expect(body.status).toBe('ok');
  });
});

test.describe('Auth', () => {
  test('login succeeds with correct credentials', async ({ request }) => {
    const response = await request.post('/api/auth/login', {
      data: { email: ADMIN_EMAIL, password: ADMIN_PASSWORD },
    });
    expect(response.status()).toBe(200);
  });

  test('login fails with wrong password', async ({ request }) => {
    const response = await request.post('/api/auth/login', {
      data: { email: ADMIN_EMAIL, password: 'wrongpassword' },
    });
    expect(response.status()).toBe(401);
  });

  test('/api/auth/me returns user after login', async ({ page }) => {
    await login(page);
    const response = await page.request.get('/api/auth/me');
    expect(response.status()).toBe(200);
    const body = await response.json();
    expect(body.user.email).toBe(ADMIN_EMAIL);
  });
});

test.describe('Block Pages Load', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  const blockPages = [
    { name: 'Dashboard', path: '/admin' },
    { name: 'Waffle', path: '/admin/waffle' },
    { name: 'Users', path: '/admin/users' },
    { name: 'Database', path: '/admin/database' },
    { name: 'Storage', path: '/admin/storage' },
    { name: 'IAM', path: '/admin/iam' },
    { name: 'Settings', path: '/admin/settings' },
    { name: 'Logs', path: '/admin/logs' },
  ];

  for (const block of blockPages) {
    test(`${block.name} page loads at ${block.path}`, async ({ page }) => {
      const response = await page.goto(block.path);
      expect(response?.status()).toBe(200);

      // Page should have content type HTML
      const contentType = response?.headers()['content-type'];
      expect(contentType).toContain('text/html');

      // The #app div should exist
      const app = page.locator('#app');
      await expect(app).toBeAttached();

      // Sidebar should be present (BlockShell wraps all admin pages)
      const sidebar = page.locator('.sidebar');
      await expect(sidebar).toBeAttached({ timeout: 5000 });
    });
  }
});

test.describe('Block Pages Have Sidebar', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('Users page renders content with sidebar', async ({ page }) => {
    await page.goto('/admin/users');
    await expect(page.locator('.sidebar')).toBeAttached({ timeout: 5000 });
    await expect(page.locator('.sidebar-nav')).toBeAttached();
  });

  test('Dashboard page renders content with sidebar', async ({ page }) => {
    await page.goto('/admin');
    await expect(page.locator('.sidebar')).toBeAttached({ timeout: 5000 });
  });
});

test.describe('Admin API Endpoints', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('GET /api/nav returns navigation items', async ({ page }) => {
    const response = await page.request.get('/api/nav');
    expect(response.status()).toBe(200);
    const items = await response.json();
    expect(Array.isArray(items)).toBe(true);
    expect(items.length).toBeGreaterThan(0);
  });

  test('GET /api/admin/monitoring/live returns stats', async ({ page }) => {
    const response = await page.request.get('/api/admin/monitoring/live');
    expect(response.status()).toBe(200);
  });

  test('GET /api/admin/users returns users list', async ({ page }) => {
    const response = await page.request.get('/api/admin/users');
    expect(response.status()).toBe(200);
  });

  test('GET /api/admin/database/tables returns tables', async ({ page }) => {
    const response = await page.request.get('/api/admin/database/tables');
    expect(response.status()).toBe(200);
  });

  test('GET /api/settings returns settings', async ({ page }) => {
    const response = await page.request.get('/api/settings');
    expect(response.status()).toBe(200);
  });
});

test.describe('Login Page', () => {
  test('login page loads', async ({ page }) => {
    const response = await page.goto('/admin/login');
    expect(response?.status()).toBe(200);
    const app = page.locator('#app');
    await expect(app).toBeAttached();
  });
});
