/**
 * Stripe Payment Flow E2E Tests
 *
 * Tests the full purchase → checkout → webhook → completion flow using a
 * local Stripe mock server. The solobase server must be started with:
 *
 *   STRIPE_SECRET_KEY=sk_test_mock STRIPE_WEBHOOK_SECRET=whsec_test_mock_secret_for_e2e \
 *   STRIPE_API_URL=http://127.0.0.1:12111 \
 *   JWT_SECRET=test-secret cargo run --bin solobase
 *
 * The mock intercepts Stripe API calls and lets us simulate webhook callbacks.
 */
import { test, expect } from '@playwright/test';
import {
  startStripeMock, stopStripeMock,
  STRIPE_MOCK_PORT, WEBHOOK_SECRET,
  simulateCheckoutComplete, buildWebhookEvent,
} from './stripe-mock';

test.describe('Stripe Payment Flow', () => {
  test.describe.configure({ mode: 'serial' });

  let adminToken: string;
  let devToken: string;
  let productId: string;
  let purchaseId: string;
  let checkoutSessionId: string;

  const devEmail = `stripe-dev-${Date.now()}@test.com`;
  const devPassword = 'StripeTestPass1234';
  const baseUrl = 'http://127.0.0.1:8090';

  test.beforeAll(async () => {
    await startStripeMock();
  });

  test.afterAll(async () => {
    await stopStripeMock();
  });

  // ── Setup ──────────────────────────────────────────────────────────

  test('get admin token', async ({ request }) => {
    const email = process.env.ADMIN_EMAIL || 'admin@e2e.test';
    const password = process.env.ADMIN_PASSWORD || 'AdminE2EPass1234';

    let res = await request.post('/auth/login', {
      data: { email, password },
    });
    if (!res.ok()) {
      res = await request.post('/auth/signup', {
        data: { email, password, name: 'Admin' },
      });
    }
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    adminToken = body.access_token;
    expect(adminToken).toBeTruthy();
  });

  test('create a developer account', async ({ request }) => {
    const res = await request.post('/auth/signup', {
      data: { email: devEmail, password: devPassword, name: 'Stripe Tester' },
    });
    expect(res.ok()).toBeTruthy();
    devToken = (await res.json()).access_token;
    expect(devToken).toBeTruthy();
  });

  // ── Admin creates a product (group → product) ──────────────────────

  let groupId: string;

  test('admin creates a product group', async ({ request }) => {
    const res = await request.post('/admin/b/products/groups', {
      headers: { Authorization: `Bearer ${adminToken}` },
      data: {
        name: 'Plans',
        group_template_id: 1,
      },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    groupId = body.id?.toString() || body.data?.id?.toString();
    expect(groupId).toBeTruthy();
  });

  test('admin creates a paid product', async ({ request }) => {
    const res = await request.post('/admin/b/products/products', {
      headers: { Authorization: `Bearer ${adminToken}` },
      data: {
        name: 'Pro Plan',
        description: 'Pro plan for testing',
        base_price: 29.00,
        currency: 'usd',
        status: 'active',
        group_id: parseInt(groupId),
        product_template_id: 1,
      },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    productId = body.id?.toString() || body.data?.id?.toString();
    expect(productId).toBeTruthy();
  });

  // ── Developer browses catalog and buys ─────────────────────────────

  test('developer sees the product in the catalog', async ({ request }) => {
    const res = await request.get('/b/products/catalog', {
      headers: { Authorization: `Bearer ${devToken}` },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const records = body.records || body || [];
    const found = records.find((r: any) => {
      const rid = (r.id || r.data?.id)?.toString();
      return rid === productId;
    });
    expect(found).toBeTruthy();
  });

  test('developer creates a purchase', async ({ request }) => {
    const res = await request.post('/b/products/purchases', {
      headers: { Authorization: `Bearer ${devToken}` },
      data: {
        items: [{ product_id: productId, quantity: 1, variables: {} }],
      },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    purchaseId = body.id?.toString() || body.data?.id?.toString();
    expect(purchaseId).toBeTruthy();

    const status = body.status || body.data?.status;
    expect(status).toBe('pending');
  });

  test('developer initiates Stripe checkout', async ({ request }) => {
    const res = await request.post('/b/products/checkout', {
      headers: { Authorization: `Bearer ${devToken}` },
      data: {
        purchase_id: purchaseId,
        success_url: `${baseUrl}/checkout/success`,
        cancel_url: `${baseUrl}/checkout/cancel`,
      },
    });

    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.session_id).toMatch(/^cs_test_/);
    expect(body.checkout_url).toContain('127.0.0.1');
    checkoutSessionId = body.session_id;
  });

  // ── Simulate Stripe webhook ────────────────────────────────────────

  test('Stripe webhook completes the purchase', async ({ request }) => {
    const result = await simulateCheckoutComplete(baseUrl, checkoutSessionId, purchaseId);
    expect(result.status).toBe(200);
    expect(result.body?.received).toBe(true);
  });

  test('purchase is now marked as completed', async ({ request }) => {
    const res = await request.get(`/b/products/purchases/${purchaseId}`, {
      headers: { Authorization: `Bearer ${devToken}` },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    // handle_get returns { purchase: { id, data: {...} }, line_items: [...] }
    const data = body.purchase?.data || body.data || body;
    expect(data.status).toBe('completed');
    expect(data.approved_at).toBeTruthy();
    expect(data.provider_payment_intent_id).toMatch(/^pi_test_/);
  });

  // ── Webhook security ──────────────────────────────────────────────

  test('webhook rejects missing signature', async ({ request }) => {
    const res = await request.post('/b/products/webhooks', {
      headers: { 'Content-Type': 'application/json' },
      data: { type: 'checkout.session.completed', data: { object: {} } },
    });
    expect(res.status()).toBe(401);
  });

  test('webhook rejects invalid signature', async ({ request }) => {
    const res = await request.post('/b/products/webhooks', {
      headers: {
        'Content-Type': 'application/json',
        'Stripe-Signature': 't=1234567890,v1=invalidsignature',
      },
      data: JSON.stringify({ type: 'checkout.session.completed', data: { object: {} } }),
    });
    expect(res.status()).toBe(401);
  });

  test('webhook rejects expired timestamp (replay protection)', async ({ request }) => {
    const event = {
      id: 'evt_test_old',
      object: 'event',
      type: 'checkout.session.completed',
      data: { object: { metadata: { purchase_id: 'fake' } } },
      created: Math.floor(Date.now() / 1000) - 600,
    };
    const payload = JSON.stringify(event);
    const oldTimestamp = (Math.floor(Date.now() / 1000) - 600).toString();
    const crypto = require('crypto');
    const sig = crypto.createHmac('sha256', WEBHOOK_SECRET)
      .update(`${oldTimestamp}.${payload}`)
      .digest('hex');

    const res = await request.post('/b/products/webhooks', {
      headers: {
        'Content-Type': 'application/json',
        'Stripe-Signature': `t=${oldTimestamp},v1=${sig}`,
      },
      data: payload,
    });
    expect(res.status()).toBe(401);
  });

  // ── Simulate refund ────────────────────────────────────────────────

  test('Stripe webhook handles refund', async ({ request }) => {
    // Get the payment_intent from the purchase
    const purchaseRes = await request.get(`/b/products/purchases/${purchaseId}`, {
      headers: { Authorization: `Bearer ${devToken}` },
    });
    const purchaseBody = await purchaseRes.json();
    const purchaseData = purchaseBody.purchase?.data || purchaseBody.data || purchaseBody;
    const paymentIntent = purchaseData.provider_payment_intent_id;
    expect(paymentIntent).toBeTruthy();

    const { payload, signature } = buildWebhookEvent('charge.refunded', {
      id: `ch_test_${Date.now()}`,
      payment_intent: paymentIntent,
      refunded: true,
      amount_refunded: 2900,
    });

    const res = await request.post('/b/products/webhooks', {
      headers: {
        'Content-Type': 'application/json',
        'Stripe-Signature': signature,
      },
      data: payload,
    });
    expect(res.ok()).toBeTruthy();

    // Verify purchase is now refunded
    const checkRes = await request.get(`/b/products/purchases/${purchaseId}`, {
      headers: { Authorization: `Bearer ${devToken}` },
    });
    const checkBody = await checkRes.json();
    const checkData = checkBody.purchase?.data || checkBody.data || checkBody;
    expect(checkData.status).toBe('refunded');
    expect(checkData.refunded_at).toBeTruthy();
  });

  // ── Admin stats ────────────────────────────────────────────────────

  test('admin can see purchase stats', async ({ request }) => {
    const res = await request.get('/admin/b/products/stats', {
      headers: { Authorization: `Bearer ${adminToken}` },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.total_purchases).toBeGreaterThan(0);
    expect(body.active_products).toBeGreaterThan(0);
  });
});
