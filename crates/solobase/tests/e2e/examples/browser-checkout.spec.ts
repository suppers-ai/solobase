/**
 * Browser-Based Stripe Checkout E2E Example
 *
 * This test opens a real browser, navigates to the Stripe mock checkout page,
 * clicks "Pay Now", and verifies the purchase completes end-to-end.
 *
 * Run with:
 *   ./scripts/test.sh browser-checkout          # headless
 *   ./scripts/test.sh browser-checkout --headed  # watch it happen
 *
 * Prerequisites (handled by test.sh):
 *   - Solobase server running on :8090
 *   - Stripe mock running on :12111
 */
import { test, expect, type Page, type APIRequestContext } from '@playwright/test';
import { startStripeMock, stopStripeMock } from '../stripe-mock';

const BASE = 'http://127.0.0.1:8090';

test.describe('Browser Checkout Flow', () => {
  test.describe.configure({ mode: 'serial' });

  let adminToken: string;
  let userToken: string;
  let groupId: string;
  let productId: string;
  let purchaseId: string;
  let checkoutUrl: string;
  let checkoutSessionId: string;

  const userEmail = `browser-checkout-${Date.now()}@test.com`;
  const userPassword = 'BrowserTest1234';

  test.beforeAll(async () => {
    await startStripeMock();
  });

  test.afterAll(async () => {
    await stopStripeMock();
  });

  // ── Helpers ────────────────────────────────────────────────────────

  async function apiPost(request: APIRequestContext, path: string, data: any, token?: string) {
    const headers: Record<string, string> = {};
    if (token) headers['Authorization'] = `Bearer ${token}`;
    const res = await request.post(path, { headers, data });
    expect(res.ok(), `POST ${path} failed: ${res.status()}`).toBeTruthy();
    return res.json();
  }

  async function apiGet(request: APIRequestContext, path: string, token?: string) {
    const headers: Record<string, string> = {};
    if (token) headers['Authorization'] = `Bearer ${token}`;
    const res = await request.get(path, { headers });
    expect(res.ok(), `GET ${path} failed: ${res.status()}`).toBeTruthy();
    return res.json();
  }

  // ── 1. Setup: create admin, user, product ──────────────────────────

  test('setup: get admin token', async ({ request }) => {
    const email = process.env.ADMIN_EMAIL || 'admin@e2e.test';
    const password = process.env.ADMIN_PASSWORD || 'AdminE2EPass1234';

    let res = await request.post('/auth/login', { data: { email, password } });
    if (!res.ok()) {
      res = await request.post('/auth/signup', { data: { email, password, name: 'Admin' } });
    }
    expect(res.ok()).toBeTruthy();
    adminToken = (await res.json()).access_token;
    expect(adminToken).toBeTruthy();
  });

  test('setup: create user account', async ({ request }) => {
    const body = await apiPost(request, '/auth/signup', {
      email: userEmail, password: userPassword, name: 'Browser Tester',
    });
    userToken = body.access_token;
    expect(userToken).toBeTruthy();
  });

  test('setup: admin creates product group', async ({ request }) => {
    const body = await apiPost(request, '/admin/b/products/groups', {
      name: 'Browser Test Plans',
      group_template_id: 1,
    }, adminToken);
    groupId = (body.id || body.data?.id).toString();
    expect(groupId).toBeTruthy();
  });

  test('setup: admin creates a $49 product', async ({ request }) => {
    const body = await apiPost(request, '/admin/b/products/products', {
      name: 'Premium Plan',
      description: 'Premium plan — browser checkout test',
      base_price: 49.00,
      currency: 'usd',
      status: 'active',
      group_id: parseInt(groupId),
      product_template_id: 1,
    }, adminToken);
    productId = (body.id || body.data?.id).toString();
    expect(productId).toBeTruthy();
  });

  // ── 2. User browses catalog and creates a purchase ─────────────────

  test('user sees product in catalog', async ({ request }) => {
    const body = await apiGet(request, '/b/products/catalog', userToken);
    const records = body.records || body || [];
    const found = records.find((r: any) => (r.id || r.data?.id)?.toString() === productId);
    expect(found, 'Product not found in catalog').toBeTruthy();
  });

  test('user creates a purchase', async ({ request }) => {
    const body = await apiPost(request, '/b/products/purchases', {
      items: [{ product_id: productId, quantity: 1, variables: {} }],
    }, userToken);
    purchaseId = (body.id || body.data?.id).toString();
    expect(purchaseId).toBeTruthy();
    expect(body.status || body.data?.status).toBe('pending');
  });

  test('user initiates Stripe checkout', async ({ request }) => {
    const body = await apiPost(request, '/b/products/checkout', {
      purchase_id: purchaseId,
      success_url: `${BASE}/?checkout=success&purchase_id=${purchaseId}`,
      cancel_url: `${BASE}/?checkout=cancel`,
    }, userToken);
    checkoutUrl = body.checkout_url;
    checkoutSessionId = body.session_id;
    expect(checkoutUrl).toContain('127.0.0.1:12111/checkout/');
    expect(checkoutSessionId).toMatch(/^cs_test_/);
  });

  // ── 3. Browser: open checkout page and pay ─────────────────────────

  test('browser: navigate to checkout and click Pay Now', async ({ page }) => {
    // Go to the Stripe mock checkout page
    await page.goto(checkoutUrl);

    // Verify we're on the mock checkout page
    await expect(page.locator('h1')).toHaveText('Mock Stripe Checkout');

    // Verify the amount is shown
    await expect(page.locator('body')).toContainText('49.00 USD');

    // Verify the purchase ID is displayed
    await expect(page.locator('body')).toContainText(purchaseId);

    // Click the "Pay Now (Test)" button
    await page.click('button:has-text("Pay Now")');

    // Wait for "Processing..." then "Payment successful!"
    await expect(page.locator('#status')).toContainText('Payment successful!', { timeout: 10_000 });

    // The page will redirect to the success URL after 1.5s
    await page.waitForURL(url => url.searchParams.get('checkout') === 'success', { timeout: 10_000 });

    // Verify we landed on the solobase server with success params
    const url = new URL(page.url());
    expect(url.searchParams.get('checkout')).toBe('success');
    expect(url.searchParams.get('purchase_id')).toBe(purchaseId);
  });

  // ── 4. Verify the purchase was completed ───────────────────────────

  test('purchase is marked as completed after browser checkout', async ({ request }) => {
    // Small delay to let the webhook fully process
    await new Promise(r => setTimeout(r, 500));

    const body = await apiGet(request, `/b/products/purchases/${purchaseId}`, userToken);
    const data = body.purchase?.data || body.data || body;
    expect(data.status).toBe('completed');
    expect(data.approved_at).toBeTruthy();
    expect(data.provider_payment_intent_id).toMatch(/^pi_test_/);
  });

  test('admin sees the completed purchase', async ({ request }) => {
    const body = await apiGet(request, `/admin/b/products/purchases/${purchaseId}`, adminToken);
    const data = body.purchase?.data || body.data || body;
    expect(data.status).toBe('completed');
  });

  // ── 5. Multi-item purchase with browser checkout ───────────────────

  let product2Id: string;
  let multiPurchaseId: string;
  let multiCheckoutUrl: string;

  test('admin creates a second product ($19)', async ({ request }) => {
    const body = await apiPost(request, '/admin/b/products/products', {
      name: 'Starter Plan',
      description: 'Starter plan — multi-item test',
      base_price: 19.00,
      currency: 'usd',
      status: 'active',
      group_id: parseInt(groupId),
      product_template_id: 1,
    }, adminToken);
    product2Id = (body.id || body.data?.id).toString();
    expect(product2Id).toBeTruthy();
  });

  test('user creates a multi-item purchase', async ({ request }) => {
    const body = await apiPost(request, '/b/products/purchases', {
      items: [
        { product_id: productId, quantity: 2, variables: {} },  // 2x $49 = $98
        { product_id: product2Id, quantity: 1, variables: {} }, // 1x $19 = $19
      ],
    }, userToken);
    multiPurchaseId = (body.id || body.data?.id).toString();
    expect(multiPurchaseId).toBeTruthy();
  });

  test('user checks out multi-item purchase', async ({ request }) => {
    const body = await apiPost(request, '/b/products/checkout', {
      purchase_id: multiPurchaseId,
      success_url: `${BASE}/?checkout=success&purchase_id=${multiPurchaseId}`,
      cancel_url: `${BASE}/?checkout=cancel`,
    }, userToken);
    multiCheckoutUrl = body.checkout_url;
    expect(multiCheckoutUrl).toBeTruthy();
  });

  test('browser: complete multi-item checkout', async ({ page }) => {
    await page.goto(multiCheckoutUrl);
    await expect(page.locator('h1')).toHaveText('Mock Stripe Checkout');

    // Click pay
    await page.click('button:has-text("Pay Now")');
    await expect(page.locator('#status')).toContainText('Payment successful!', { timeout: 10_000 });
    await page.waitForURL(url => url.searchParams.get('checkout') === 'success', { timeout: 10_000 });
  });

  test('multi-item purchase is completed', async ({ request }) => {
    await new Promise(r => setTimeout(r, 500));

    const body = await apiGet(request, `/b/products/purchases/${multiPurchaseId}`, userToken);
    const data = body.purchase?.data || body.data || body;
    expect(data.status).toBe('completed');
  });

  // ── 6. Cancelled checkout flow ─────────────────────────────────────

  let cancelPurchaseId: string;
  let cancelCheckoutUrl: string;

  test('user creates a purchase for cancel test', async ({ request }) => {
    const body = await apiPost(request, '/b/products/purchases', {
      items: [{ product_id: productId, quantity: 1, variables: {} }],
    }, userToken);
    cancelPurchaseId = (body.id || body.data?.id).toString();
  });

  test('user initiates checkout for cancel test', async ({ request }) => {
    const body = await apiPost(request, '/b/products/checkout', {
      purchase_id: cancelPurchaseId,
      success_url: `${BASE}/?checkout=success`,
      cancel_url: `${BASE}/?checkout=cancel`,
    }, userToken);
    cancelCheckoutUrl = body.checkout_url;
  });

  test('browser: navigate to checkout but do not pay', async ({ page }) => {
    await page.goto(cancelCheckoutUrl);
    await expect(page.locator('h1')).toHaveText('Mock Stripe Checkout');

    // User sees the page but navigates away without paying
    await page.goto(`${BASE}/?checkout=cancel`);
    const url = new URL(page.url());
    expect(url.searchParams.get('checkout')).toBe('cancel');
  });

  test('cancelled purchase remains pending', async ({ request }) => {
    const body = await apiGet(request, `/b/products/purchases/${cancelPurchaseId}`, userToken);
    const data = body.purchase?.data || body.data || body;
    expect(data.status).toBe('pending');
  });
});
