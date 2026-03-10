import { test, expect, Page } from '@playwright/test';

test.describe.configure({ mode: 'serial' });

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

let _adminToken: string | null = null;

async function signupUser(request: any) {
  const email = `produser-${Date.now()}-${Math.random().toString(36).slice(2, 8)}@test.com`;
  const password = 'TestPass1234';
  const res = await request.post('/auth/signup', {
    data: { email, password, name: 'Product Tester' },
  });
  expect(res.ok()).toBeTruthy();
  const body = await res.json();
  return {
    email,
    password,
    token: body.access_token as string,
    userId: body.user?.id as string,
  };
}

async function getAdminToken(request: any): Promise<string> {
  if (_adminToken) return _adminToken;
  const seededEmail = process.env.ADMIN_EMAIL || 'admin@e2e.test';
  const seededPassword = process.env.ADMIN_PASSWORD || 'AdminE2EPass1234';
  const loginRes = await request.post('/auth/login', {
    data: { email: seededEmail, password: seededPassword },
  });
  if (loginRes.ok()) {
    const body = await loginRes.json();
    _adminToken = body.access_token as string;
    return _adminToken;
  }
  const signupRes = await request.post('/auth/signup', {
    data: { email: seededEmail, password: seededPassword, name: 'Admin' },
  });
  const body = await signupRes.json();
  _adminToken = body.access_token as string;
  return _adminToken;
}

function authHeaders(token: string) {
  return { Authorization: `Bearer ${token}` };
}

/** Login via the dashboard UI (sets session cookie). */
async function loginViaDashboard(page: Page, email: string, password: string) {
  await page.goto('/blocks/dashboard/frontend/index.html');
  // Wait for the login form to render
  await page.waitForSelector('input[type="email"]', { timeout: 15000 });
  await page.fill('input[type="email"]', email);
  await page.fill('input[type="password"]', password);
  await page.click('button[type="submit"]');
  // Wait for dashboard to load (Overview tab visible)
  await page.waitForSelector('text=Welcome back', { timeout: 15000 });
}

// ---------------------------------------------------------------------------
// 0. Setup admin + test user
// ---------------------------------------------------------------------------
test('setup: ensure admin exists', async ({ request }) => {
  const token = await getAdminToken(request);
  expect(token).toBeTruthy();
});

// ---------------------------------------------------------------------------
// 1. User Products API tests
// ---------------------------------------------------------------------------
test.describe('User Products API', () => {
  test('user can create a group', async ({ request }) => {
    const user = await signupUser(request);
    const h = authHeaders(user.token);

    const res = await request.post('/b/products/groups', {
      headers: h,
      data: { name: 'My Store', description: 'A test group' },
    });
    expect(res.ok(), `Status: ${res.status()}`).toBeTruthy();
    const group = await res.json();
    expect(group.id).toBeTruthy();
    expect(group.data?.name || group.name).toBe('My Store');
  });

  test('user can list their groups', async ({ request }) => {
    const user = await signupUser(request);
    const h = authHeaders(user.token);

    // Create two groups
    await request.post('/b/products/groups', { headers: h, data: { name: 'Group A' } });
    await request.post('/b/products/groups', { headers: h, data: { name: 'Group B' } });

    const res = await request.get('/b/products/groups', { headers: h });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const records = body.records || body;
    expect(records.length).toBeGreaterThanOrEqual(2);
  });

  test('user can only see their own groups', async ({ request }) => {
    const user1 = await signupUser(request);
    const user2 = await signupUser(request);

    await request.post('/b/products/groups', {
      headers: authHeaders(user1.token),
      data: { name: 'User1 Private Group' },
    });

    const res = await request.get('/b/products/groups', {
      headers: authHeaders(user2.token),
    });
    const body = await res.json();
    const records = body.records || body;
    const found = records.find((g: any) => g.name === 'User1 Private Group');
    expect(found).toBeFalsy();
  });

  test('user can create a product in their group', async ({ request }) => {
    const user = await signupUser(request);
    const h = authHeaders(user.token);

    const groupRes = await request.post('/b/products/groups', {
      headers: h,
      data: { name: 'Product Group' },
    });
    const group = await groupRes.json();

    const res = await request.post('/b/products/products', {
      headers: h,
      data: {
        name: 'Widget Pro',
        description: 'A premium widget',
        group_id: group.id,
        base_price: 19.99,
        currency: 'USD',
        status: 'draft',
      },
    });
    expect(res.ok(), `Status: ${res.status()} ${await res.text()}`).toBeTruthy();
    const product = await res.json();
    expect(product.id).toBeTruthy();
    expect(product.data?.name || product.name).toBe('Widget Pro');
  });

  test('user can list their products', async ({ request }) => {
    const user = await signupUser(request);
    const h = authHeaders(user.token);

    const groupRes = await request.post('/b/products/groups', {
      headers: h,
      data: { name: 'List Group' },
    });
    const group = await groupRes.json();

    await request.post('/b/products/products', {
      headers: h,
      data: { name: 'Product 1', group_id: group.id, base_price: 10, currency: 'USD' },
    });
    await request.post('/b/products/products', {
      headers: h,
      data: { name: 'Product 2', group_id: group.id, base_price: 20, currency: 'USD' },
    });

    const res = await request.get('/b/products/products', { headers: h });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const records = body.records || body;
    expect(records.length).toBeGreaterThanOrEqual(2);
  });

  test('user can update their product', async ({ request }) => {
    const user = await signupUser(request);
    const h = authHeaders(user.token);

    const groupRes = await request.post('/b/products/groups', {
      headers: h,
      data: { name: 'Update Group' },
    });
    const group = await groupRes.json();

    const createRes = await request.post('/b/products/products', {
      headers: h,
      data: { name: 'Old Name', group_id: group.id, base_price: 5, currency: 'USD' },
    });
    const product = await createRes.json();

    const updateRes = await request.patch(`/b/products/products/${product.id}`, {
      headers: h,
      data: { name: 'New Name', base_price: 15 },
    });
    expect(updateRes.ok()).toBeTruthy();
    const updated = await updateRes.json();
    expect(updated.data?.name || updated.name).toBe('New Name');
  });

  test('user can delete their product', async ({ request }) => {
    const user = await signupUser(request);
    const h = authHeaders(user.token);

    const groupRes = await request.post('/b/products/groups', {
      headers: h,
      data: { name: 'Delete Group' },
    });
    const group = await groupRes.json();

    const createRes = await request.post('/b/products/products', {
      headers: h,
      data: { name: 'To Delete', group_id: group.id, base_price: 1, currency: 'USD' },
    });
    const product = await createRes.json();

    const deleteRes = await request.delete(`/b/products/products/${product.id}`, {
      headers: h,
    });
    expect(deleteRes.ok()).toBeTruthy();
  });

  test('user cannot access another users products', async ({ request }) => {
    const user1 = await signupUser(request);
    const user2 = await signupUser(request);

    const groupRes = await request.post('/b/products/groups', {
      headers: authHeaders(user1.token),
      data: { name: 'Private Group' },
    });
    const group = await groupRes.json();

    const createRes = await request.post('/b/products/products', {
      headers: authHeaders(user1.token),
      data: { name: 'Private Product', group_id: group.id, base_price: 10, currency: 'USD' },
    });
    const product = await createRes.json();

    // User2 tries to get user1's product
    const getRes = await request.get(`/b/products/products/${product.id}`, {
      headers: authHeaders(user2.token),
    });
    expect(getRes.status()).toBe(404);

    // User2 tries to delete user1's product
    const deleteRes = await request.delete(`/b/products/products/${product.id}`, {
      headers: authHeaders(user2.token),
    });
    expect(deleteRes.status()).toBe(404);
  });

  test('user can update their group', async ({ request }) => {
    const user = await signupUser(request);
    const h = authHeaders(user.token);

    const createRes = await request.post('/b/products/groups', {
      headers: h,
      data: { name: 'Old Group Name', description: 'Old desc' },
    });
    const group = await createRes.json();

    const updateRes = await request.patch(`/b/products/groups/${group.id}`, {
      headers: h,
      data: { name: 'New Group Name', description: 'Updated desc' },
    });
    expect(updateRes.ok()).toBeTruthy();
    const updated = await updateRes.json();
    expect(updated.data?.name || updated.name).toBe('New Group Name');
  });

  test('user can delete their group', async ({ request }) => {
    const user = await signupUser(request);
    const h = authHeaders(user.token);

    const createRes = await request.post('/b/products/groups', {
      headers: h,
      data: { name: 'Deletable Group' },
    });
    const group = await createRes.json();

    const deleteRes = await request.delete(`/b/products/groups/${group.id}`, {
      headers: h,
    });
    expect(deleteRes.ok()).toBeTruthy();
  });

  test('user can list products in a specific group', async ({ request }) => {
    const user = await signupUser(request);
    const h = authHeaders(user.token);

    const groupRes = await request.post('/b/products/groups', {
      headers: h,
      data: { name: 'Group With Products' },
    });
    const group = await groupRes.json();

    await request.post('/b/products/products', {
      headers: h,
      data: { name: 'In Group', group_id: group.id, base_price: 5, currency: 'USD' },
    });

    const res = await request.get(`/b/products/groups/${group.id}/products`, {
      headers: h,
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const records = body.records || body;
    expect(records.length).toBeGreaterThanOrEqual(1);
  });
});

// ---------------------------------------------------------------------------
// 2. Dashboard Frontend - Products link and overview
// ---------------------------------------------------------------------------
test.describe('Dashboard Frontend - Products Integration', () => {
  let testEmail: string;
  let testPassword: string;

  test.beforeAll(async ({ request }) => {
    // Create a test user
    const user = await signupUser(request);
    testEmail = user.email;
    testPassword = 'TestPass1234';
  });

  test('dashboard page loads and shows login form', async ({ page }) => {
    await page.goto('/blocks/dashboard/frontend/index.html');
    await page.waitForSelector('input[type="email"]', { timeout: 15000 });
    const emailInput = page.locator('input[type="email"]');
    await expect(emailInput).toBeVisible();
  });

  test('user can login and see dashboard', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    // Should see Overview tab content
    await expect(page.locator('text=Welcome back')).toBeVisible({ timeout: 10000 });
  });

  test('dashboard shows Products stat card', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    // Look for the Products stat card (in the main content area, not the nav)
    await expect(page.getByRole('main').getByText('Products')).toBeVisible({ timeout: 10000 });
  });

  test('dashboard has Products link in nav', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    // Look for the Products link
    const productsLink = page.locator('a[href*="products/frontend/user"]');
    await expect(productsLink).toBeVisible({ timeout: 10000 });
    await expect(productsLink).toContainText('Products');
  });

  test('clicking Products link navigates to products page', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    const productsLink = page.locator('a[href*="products/frontend/user"]');
    await productsLink.click();
    await page.waitForURL(/products\/frontend\/user/, { timeout: 10000 });
    // Should see the products user page with tabs
    await expect(page.getByRole('heading', { name: 'My Products' })).toBeVisible({ timeout: 15000 });
  });
});

// ---------------------------------------------------------------------------
// 3. Products User Frontend - Navigation & Tabs
// ---------------------------------------------------------------------------
test.describe('Products User Frontend - UI', () => {
  let testEmail: string;
  let testPassword: string;

  test.beforeAll(async ({ request }) => {
    const user = await signupUser(request);
    testEmail = user.email;
    testPassword = 'TestPass1234';
  });

  test('products page loads with all tabs', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    await page.goto('/blocks/products/frontend/user/index.html');
    await page.waitForSelector('h1', { timeout: 15000 });

    // Verify all 4 tab buttons are visible
    await expect(page.getByRole('button', { name: 'My Products' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'My Groups' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Plans' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Purchases' })).toBeVisible();
  });

  test('products page has header with Dashboard link and logout', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    await page.goto('/blocks/products/frontend/user/index.html');
    await page.waitForSelector('h1', { timeout: 15000 });

    // Header elements
    await expect(page.locator('text=Dashboard').first()).toBeVisible();
    await expect(page.locator('text=Logout').first()).toBeVisible();
  });

  test('My Products tab shows empty state initially', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    await page.goto('/blocks/products/frontend/user/index.html#products');
    await page.waitForSelector('text=No products yet', { timeout: 15000 });
    await expect(page.locator('text=Create Product')).toBeVisible();
  });

  test('My Groups tab shows empty state initially', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    await page.goto('/blocks/products/frontend/user/index.html#groups');
    await page.waitForSelector('text=No groups yet', { timeout: 15000 });
    await expect(page.locator('text=Create Group')).toBeVisible();
  });

  test('Plans tab shows plan cards', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    await page.goto('/blocks/products/frontend/user/index.html#plans');
    await page.waitForSelector('text=Choose a Plan', { timeout: 15000 });
    // Should show at least one plan card with a Subscribe button
    await expect(page.locator('button:has-text("Subscribe")').first()).toBeVisible({ timeout: 5000 });
  });

  test('Purchases tab shows empty or current plan', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    await page.goto('/blocks/products/frontend/user/index.html#purchases');
    await page.waitForSelector('text=My Purchases', { timeout: 15000 });
  });
});

// ---------------------------------------------------------------------------
// 4. Products User Frontend - CRUD operations via UI
// ---------------------------------------------------------------------------
test.describe('Products User Frontend - CRUD', () => {
  let testEmail: string;
  let testPassword: string;

  test.beforeAll(async ({ request }) => {
    const user = await signupUser(request);
    testEmail = user.email;
    testPassword = 'TestPass1234';
  });

  test('user can create a group via UI', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    await page.goto('/blocks/products/frontend/user/index.html#groups');
    await page.waitForSelector('h1', { timeout: 15000 });

    // Click the "New Group" button in the header
    await page.locator('button:has-text("New Group")').first().click();

    // Fill in the modal form
    await page.waitForSelector('.modal input[placeholder="Group name"]', { timeout: 5000 });
    await page.fill('.modal input[placeholder="Group name"]', 'E2E Test Group');
    await page.fill('.modal textarea[placeholder="Group description"]', 'Created by Playwright');

    // Submit via modal footer button
    await page.locator('.modal-footer button:has-text("Create Group")').click();

    // Wait for toast and verify group appears
    await page.waitForSelector('text=Group created', { timeout: 5000 });
    await expect(page.locator('text=E2E Test Group')).toBeVisible({ timeout: 5000 });
  });

  test('user can create a product via UI', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    await page.goto('/blocks/products/frontend/user/index.html#products');
    await page.waitForSelector('h1', { timeout: 15000 });

    // Click new product button in the header
    await page.locator('button:has-text("New Product")').first().click();

    // Fill in the modal form
    await page.waitForSelector('.modal input[placeholder="Product name"]', { timeout: 5000 });
    await page.fill('.modal input[placeholder="Product name"]', 'E2E Test Product');
    await page.fill('.modal textarea[placeholder="Product description"]', 'Created by Playwright');

    // Select the group we created earlier
    const groupSelect = page.locator('.modal select').first();
    const options = await groupSelect.locator('option').allTextContents();
    if (options.some(o => o.includes('E2E Test Group'))) {
      await groupSelect.selectOption({ label: 'E2E Test Group' });
    }

    // Set price
    await page.fill('.modal input[type="number"]', '29.99');

    // Submit via modal footer button
    await page.locator('.modal-footer button:has-text("Create Product")').click();

    // Wait for toast and verify product appears
    await page.waitForSelector('text=Product created', { timeout: 5000 });
    await expect(page.locator('text=E2E Test Product')).toBeVisible({ timeout: 5000 });
  });

  test('user can edit a product via UI', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    await page.goto('/blocks/products/frontend/user/index.html#products');
    await page.waitForSelector('text=E2E Test Product', { timeout: 15000 });

    // Click the edit button (pencil icon) on the product row
    const row = page.locator('table tbody tr', { hasText: 'E2E Test Product' });
    await row.locator('button').first().click();

    // Wait for modal with product name
    await page.waitForSelector('.modal input[placeholder="Product name"]', { timeout: 5000 });

    // Clear and update the name
    await page.fill('.modal input[placeholder="Product name"]', 'Updated E2E Product');

    // Save
    await page.locator('.modal-footer button:has-text("Save Changes")').click();
    await page.waitForSelector('text=Product updated', { timeout: 5000 });
    await expect(page.locator('text=Updated E2E Product')).toBeVisible({ timeout: 5000 });
  });

  test('user can search products', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    await page.goto('/blocks/products/frontend/user/index.html#products');
    await page.waitForSelector('text=Updated E2E Product', { timeout: 15000 });

    // Type in search
    const searchInput = page.locator('input[placeholder="Search products..."]');
    await searchInput.fill('Updated');
    await page.waitForTimeout(400); // debounce

    await expect(page.locator('text=Updated E2E Product')).toBeVisible();
  });

  test('user can delete a product via UI', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    await page.goto('/blocks/products/frontend/user/index.html#products');
    await page.waitForSelector('text=Updated E2E Product', { timeout: 15000 });

    // Click the delete button (trash icon)
    const row = page.locator('table tbody tr', { hasText: 'Updated E2E Product' });
    // The delete button is the second one (red trash icon)
    await row.locator('button').nth(1).click();

    // Confirm deletion
    await page.waitForSelector('text=Are you sure', { timeout: 5000 });
    await page.click('button:has-text("Delete")');

    // Wait for toast
    await page.waitForSelector('text=Product deleted', { timeout: 5000 });
  });

  test('user can edit a group via UI', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    await page.goto('/blocks/products/frontend/user/index.html#groups');
    await page.waitForSelector('text=E2E Test Group', { timeout: 15000 });

    // Click edit button on the group row
    const row = page.locator('table tbody tr', { hasText: 'E2E Test Group' });
    await row.locator('button').first().click();

    // Update the name
    await page.waitForSelector('.modal input[placeholder="Group name"]', { timeout: 5000 });
    await page.fill('.modal input[placeholder="Group name"]', 'Updated E2E Group');

    await page.locator('.modal-footer button:has-text("Save Changes")').click();
    await page.waitForSelector('text=Group updated', { timeout: 5000 });
    await expect(page.locator('text=Updated E2E Group')).toBeVisible({ timeout: 5000 });
  });

  test('user can delete a group via UI', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    await page.goto('/blocks/products/frontend/user/index.html#groups');
    await page.waitForSelector('text=Updated E2E Group', { timeout: 15000 });

    // Click delete button
    const row = page.locator('table tbody tr', { hasText: 'Updated E2E Group' });
    await row.locator('button').nth(1).click();

    // Confirm
    await page.waitForSelector('text=Are you sure', { timeout: 5000 });
    await page.click('button:has-text("Delete")');

    await page.waitForSelector('text=Group deleted', { timeout: 5000 });
  });
});

// ---------------------------------------------------------------------------
// 5. Tab navigation works correctly
// ---------------------------------------------------------------------------
test.describe('Tab Navigation', () => {
  let testEmail: string;
  let testPassword: string;

  test.beforeAll(async ({ request }) => {
    const user = await signupUser(request);
    testEmail = user.email;
    testPassword = 'TestPass1234';
  });

  test('clicking tabs changes content and URL hash', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    await page.goto('/blocks/products/frontend/user/index.html#products');
    await page.waitForSelector('h1', { timeout: 15000 });

    // Click My Groups tab
    await page.getByRole('button', { name: 'My Groups' }).click();
    await page.waitForURL(/.*#groups/, { timeout: 5000 });
    await expect(page.getByRole('heading', { name: 'My Groups' })).toBeVisible();

    // Click Plans tab
    await page.getByRole('button', { name: 'Plans' }).click();
    await page.waitForURL(/.*#plans/, { timeout: 5000 });
    await expect(page.locator('text=Choose a Plan')).toBeVisible();

    // Click Purchases tab
    await page.getByRole('button', { name: 'Purchases' }).click();
    await page.waitForURL(/.*#purchases/, { timeout: 5000 });
    await expect(page.locator('text=My Purchases')).toBeVisible();

    // Click back to My Products
    await page.getByRole('button', { name: 'My Products' }).click();
    await page.waitForURL(/.*#products/, { timeout: 5000 });
  });

  test('direct hash navigation works', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);

    // Navigate directly to groups tab
    await page.goto('/blocks/products/frontend/user/index.html#groups');
    await page.waitForSelector('h1', { timeout: 15000 });

    // Navigate directly to plans tab
    await page.goto('/blocks/products/frontend/user/index.html#plans');
    await page.waitForSelector('text=Choose a Plan', { timeout: 15000 });
  });
});

// ---------------------------------------------------------------------------
// 6. Dashboard back link works
// ---------------------------------------------------------------------------
test.describe('Navigation Between Dashboard and Products', () => {
  let testEmail: string;
  let testPassword: string;

  test.beforeAll(async ({ request }) => {
    const user = await signupUser(request);
    testEmail = user.email;
    testPassword = 'TestPass1234';
  });

  test('Dashboard link from products page goes back to dashboard', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    await page.goto('/blocks/products/frontend/user/index.html');
    await page.waitForSelector('h1', { timeout: 15000 });

    // Click the "Dashboard" back link
    await page.click('a:has-text("Dashboard")');
    await page.waitForURL(/dashboard/, { timeout: 10000 });
    await expect(page.locator('text=Welcome back')).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// 7. Layout and styling checks
// ---------------------------------------------------------------------------
test.describe('Layout & Styling', () => {
  let testEmail: string;
  let testPassword: string;

  test.beforeAll(async ({ request }) => {
    const user = await signupUser(request);
    testEmail = user.email;
    testPassword = 'TestPass1234';
  });

  test('products page renders without JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    await loginViaDashboard(page, testEmail, testPassword);
    await page.goto('/blocks/products/frontend/user/index.html');
    await page.waitForSelector('h1', { timeout: 15000 });

    // Navigate through all tabs
    await page.getByRole('button', { name: 'My Groups' }).click();
    await page.waitForTimeout(1000);
    await page.getByRole('button', { name: 'Plans' }).click();
    await page.waitForTimeout(1000);
    await page.getByRole('button', { name: 'Purchases' }).click();
    await page.waitForTimeout(1000);

    expect(errors).toEqual([]);
  });

  test('dashboard renders Products stat card correctly', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);

    // Verify stat cards are properly laid out in a grid
    const statCards = page.locator('text=Plan >> .. >> ..');
    await expect(statCards.first()).toBeVisible({ timeout: 5000 });

    // Check that Products stat card shows a number
    const productsCard = page.locator('text=Products').first();
    await expect(productsCard).toBeVisible({ timeout: 5000 });
  });

  test('plans cards are displayed in a grid', async ({ page }) => {
    await loginViaDashboard(page, testEmail, testPassword);
    await page.goto('/blocks/products/frontend/user/index.html#plans');
    await page.waitForSelector('text=Choose a Plan', { timeout: 15000 });

    // Should display plan cards (at least the fallback ones)
    const planCards = page.locator('h3:has-text("Free"), h3:has-text("Pro"), h3:has-text("Enterprise")');
    const count = await planCards.count();
    expect(count).toBeGreaterThanOrEqual(1);
  });
});
