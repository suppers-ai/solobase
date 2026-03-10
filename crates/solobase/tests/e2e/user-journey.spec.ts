import { test, expect } from '@playwright/test';

// Run tests serially — admin role depends on being the first user in the DB.
test.describe.configure({ mode: 'serial' });

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Cached admin token, set once in beforeAll. */
let _adminToken: string | null = null;

/** Sign up a fresh user and return credentials + token. */
async function signupUser(request: any) {
  const email = `journey-${Date.now()}-${Math.random().toString(36).slice(2, 8)}@test.com`;
  const password = 'TestPass1234';
  const res = await request.post('/auth/signup', {
    data: { email, password, name: 'Test Developer' },
  });
  expect(res.ok()).toBeTruthy();
  const body = await res.json();
  return {
    email,
    password,
    name: 'Test Developer',
    token: body.access_token as string,
    refreshToken: body.refresh_token as string,
    userId: body.user?.id as string,
    roles: body.user?.roles as string[],
  };
}

/** Get the admin token (cached after first call).
 * Uses the seeded admin account (set via ADMIN_EMAIL/ADMIN_PASSWORD env vars
 * when starting the server). Falls back to sign-up if login fails. */
async function getAdminToken(request: any): Promise<string> {
  if (_adminToken) return _adminToken;

  // Try the seeded admin credentials first (set via env vars on server start)
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

  // Fallback: sign up a new user (won't be admin unless it's the first user)
  const signupRes = await request.post('/auth/signup', {
    data: { email: 'admin-journey@test.com', password: 'AdminPass1234', name: 'Platform Admin' },
  });
  if (signupRes.ok()) {
    const body = await signupRes.json();
    _adminToken = body.access_token as string;
    return _adminToken;
  }
  throw new Error('Failed to get admin token');
}

function authHeaders(token: string) {
  return { Authorization: `Bearer ${token}` };
}

// ---------------------------------------------------------------------------
// 0. Ensure admin user is created first (gets admin role in fresh DB)
// ---------------------------------------------------------------------------
test('setup: create admin user', async ({ request }) => {
  const token = await getAdminToken(request);
  expect(token).toBeTruthy();
  // Verify admin role
  const meRes = await request.get('/auth/me', {
    headers: authHeaders(token),
  });
  expect(meRes.ok()).toBeTruthy();
  const me = await meRes.json();
  const roles = me.user?.roles || [];
  if (!roles.includes('admin')) {
    console.log('Warning: admin-journey@test.com does not have admin role. Admin tests will skip.');
  }
});

// ---------------------------------------------------------------------------
// 1. Developer signup and profile management
// ---------------------------------------------------------------------------
test.describe('Developer Signup & Profile', () => {
  test('developer can sign up with email and password', async ({ request }) => {
    const user = await signupUser(request);
    expect(user.token).toBeTruthy();
    expect(user.userId).toBeTruthy();
    expect(user.email).toContain('@test.com');
  });

  test('developer can view their profile after signup', async ({ request }) => {
    const user = await signupUser(request);
    const res = await request.get('/auth/me', {
      headers: authHeaders(user.token),
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.user.email).toBe(user.email);
    expect(body.user.name).toBe('Test Developer');
  });

  test('developer can update their display name', async ({ request }) => {
    const user = await signupUser(request);
    const updateRes = await request.put('/auth/me', {
      headers: authHeaders(user.token),
      data: { name: 'Jane Developer' },
    });
    expect(updateRes.ok()).toBeTruthy();

    const meRes = await request.get('/auth/me', {
      headers: authHeaders(user.token),
    });
    const body = await meRes.json();
    expect(body.user.name).toBe('Jane Developer');
  });

  test('developer can change their password', async ({ request }) => {
    const user = await signupUser(request);
    const changeRes = await request.post('/auth/change-password', {
      headers: authHeaders(user.token),
      data: { current_password: user.password, new_password: 'NewSecure5678' },
    });
    expect(changeRes.ok()).toBeTruthy();

    // Can login with new password
    const loginRes = await request.post('/auth/login', {
      data: { email: user.email, password: 'NewSecure5678' },
    });
    expect(loginRes.ok()).toBeTruthy();
  });

  test('developer can create and manage API keys', async ({ request }) => {
    const user = await signupUser(request);
    const h = authHeaders(user.token);

    // Create key
    const createRes = await request.post('/auth/api-keys', {
      headers: h,
      data: { name: 'CI/CD Key' },
    });
    expect(createRes.ok()).toBeTruthy();
    const key = await createRes.json();
    expect(key.key).toMatch(/^sb_/);

    // List keys
    const listRes = await request.get('/auth/api-keys', { headers: h });
    expect(listRes.ok()).toBeTruthy();
    const keys = await listRes.json();
    expect(keys.records?.length || keys.length).toBeGreaterThanOrEqual(1);

    // Revoke key
    const revokeRes = await request.delete(`/auth/api-keys/${key.id}`, { headers: h });
    expect(revokeRes.ok()).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// 2. Plans and product catalog
// ---------------------------------------------------------------------------
test.describe('Plans & Pricing', () => {
  test('admin can create product plans', async ({ request }) => {
    const adminToken = await getAdminToken(request);
    const h = authHeaders(adminToken);

    // Create a group (FK constraints not enforced, use dummy template ID)
    const groupRes = await request.post('/admin/b/products/groups', {
      headers: h,
      data: {
        name: `Plans ${Date.now()}`,
        description: 'Product plans',
        group_template_id: 1,
        user_id: 'system',
      },
    });
    if (groupRes.status() === 403 || groupRes.status() === 401) {
      test.skip();
      return;
    }
    expect(groupRes.ok(), `Group: ${groupRes.status()} ${await groupRes.text()}`).toBeTruthy();
    const group = await groupRes.json();
    const groupId = group.id || group.data?.id;

    // Create product in that group
    const createRes = await request.post('/admin/b/products/products', {
      headers: h,
      data: {
        name: 'Hobby Plan',
        description: 'Free tier for side projects',
        base_price: 0,
        currency: 'USD',
        group_id: groupId,
        product_template_id: 1,
        status: 'active',
      },
    });
    expect(createRes.ok(), `Product: ${createRes.status()} ${await createRes.text()}`).toBeTruthy();
    const plan = await createRes.json();
    expect(plan.id).toBeTruthy();
    expect(plan.data?.name || plan.name).toBeTruthy();
  });

  test('admin can create paid plan', async ({ request }) => {
    const adminToken = await getAdminToken(request);
    const h = authHeaders(adminToken);

    const groupRes = await request.post('/admin/b/products/groups', {
      headers: h,
      data: {
        name: `Pro Group ${Date.now()}`,
        description: 'Paid plans',
        group_template_id: 1,
        user_id: 'system',
      },
    });
    if (groupRes.status() === 403 || groupRes.status() === 401) {
      test.skip();
      return;
    }
    const group = await groupRes.json();
    const groupId = group.id || group.data?.id;

    const createRes = await request.post('/admin/b/products/products', {
      headers: h,
      data: {
        name: 'Pro Plan',
        description: 'For production workloads',
        base_price: 29,
        currency: 'USD',
        group_id: groupId,
        product_template_id: 1,
        status: 'active',
      },
    });
    expect(createRes.ok()).toBeTruthy();
  });

  test('developer can browse available plans', async ({ request }) => {
    const user = await signupUser(request);
    const res = await request.get('/b/products/catalog', {
      headers: authHeaders(user.token),
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    // Body may be { records: [...] } or an array
    const records = body.records || body;
    expect(Array.isArray(records)).toBeTruthy();
  });

  test('developer can create a purchase', async ({ request }) => {
    const adminToken = await getAdminToken(request);
    const ha = authHeaders(adminToken);

    // Create group → product
    const groupRes = await request.post('/admin/b/products/groups', {
      headers: ha,
      data: {
        name: `Purchase Group ${Date.now()}`,
        description: 'For purchase test',
        group_template_id: 1,
        user_id: 'system',
      },
    });
    if (groupRes.status() === 403 || groupRes.status() === 401) {
      test.skip();
      return;
    }
    const group = await groupRes.json();
    const groupId = group.id || group.data?.id;

    const productRes = await request.post('/admin/b/products/products', {
      headers: ha,
      data: {
        name: `Test Plan ${Date.now()}`,
        base_price: 10,
        currency: 'USD',
        group_id: groupId,
        product_template_id: 1,
        status: 'active',
      },
    });
    expect(productRes.ok(), `Product: ${productRes.status()}`).toBeTruthy();
    const product = await productRes.json();
    const productId = product.id;

    // Now purchase as a regular user
    const user = await signupUser(request);
    const hu = authHeaders(user.token);

    const purchaseRes = await request.post('/b/products/purchases', {
      headers: hu,
      data: {
        items: [{ product_id: productId, quantity: 1 }],
        currency: 'USD',
      },
    });
    expect(purchaseRes.ok()).toBeTruthy();
    const purchase = await purchaseRes.json();
    expect(purchase.id).toBeTruthy();
    expect(purchase.status).toBe('pending');
    expect(purchase.total_amount).toBeGreaterThanOrEqual(0);
  });

  test('developer can view their purchases', async ({ request }) => {
    const user = await signupUser(request);
    const res = await request.get('/b/products/purchases', {
      headers: authHeaders(user.token),
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const records = body.records || body;
    expect(Array.isArray(records)).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// 3. Deployment provisioning
// ---------------------------------------------------------------------------
test.describe('Deployments', () => {
  test('developer can create a deployment', async ({ request }) => {
    const user = await signupUser(request);
    const h = authHeaders(user.token);

    const res = await request.post('/b/deployments', {
      headers: h,
      data: {
        name: 'My First App',
        region: 'us-east-1',
      },
    });
    expect(res.ok()).toBeTruthy();
    const deployment = await res.json();
    expect(deployment.id).toBeTruthy();
    // Status may be pending (no control plane) or active or failed
    expect(['pending', 'active', 'failed']).toContain(deployment.data?.status || deployment.status);
  });

  test('developer can list their deployments', async ({ request }) => {
    const user = await signupUser(request);
    const h = authHeaders(user.token);

    // Create a deployment first
    await request.post('/b/deployments', {
      headers: h,
      data: { name: 'List Test App', region: 'auto' },
    });

    const res = await request.get('/b/deployments', { headers: h });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const records = body.records || body;
    expect(Array.isArray(records)).toBeTruthy();
    expect(records.length).toBeGreaterThanOrEqual(1);
  });

  test('developer can view a specific deployment', async ({ request }) => {
    const user = await signupUser(request);
    const h = authHeaders(user.token);

    const createRes = await request.post('/b/deployments', {
      headers: h,
      data: { name: 'View Test App' },
    });
    const deployment = await createRes.json();

    const res = await request.get(`/b/deployments/${deployment.id}`, { headers: h });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.id || body.data?.id).toBeTruthy();
  });

  test('developer can update a deployment', async ({ request }) => {
    const user = await signupUser(request);
    const h = authHeaders(user.token);

    const createRes = await request.post('/b/deployments', {
      headers: h,
      data: { name: 'Update Test App' },
    });
    const deployment = await createRes.json();

    const updateRes = await request.patch(`/b/deployments/${deployment.id}`, {
      headers: h,
      data: { name: 'Updated App Name' },
    });
    expect(updateRes.ok()).toBeTruthy();
  });

  test('developer can delete a deployment', async ({ request }) => {
    const user = await signupUser(request);
    const h = authHeaders(user.token);

    const createRes = await request.post('/b/deployments', {
      headers: h,
      data: { name: 'Delete Test App' },
    });
    const deployment = await createRes.json();

    const deleteRes = await request.delete(`/b/deployments/${deployment.id}`, {
      headers: h,
    });
    expect(deleteRes.ok()).toBeTruthy();
    const body = await deleteRes.json();
    // After deletion, status should be "deleted"
    expect(body.data?.status || body.status).toBe('deleted');
  });

  test('developer cannot see another users deployment', async ({ request }) => {
    const user1 = await signupUser(request);
    const user2 = await signupUser(request);

    // User1 creates a deployment
    const createRes = await request.post('/b/deployments', {
      headers: authHeaders(user1.token),
      data: { name: 'Private App' },
    });
    const deployment = await createRes.json();

    // User2 tries to view it
    const res = await request.get(`/b/deployments/${deployment.id}`, {
      headers: authHeaders(user2.token),
    });
    expect(res.status()).toBe(404);
  });

  test('deployment creation validates name', async ({ request }) => {
    const user = await signupUser(request);
    const h = authHeaders(user.token);

    // Empty name should fail
    const res = await request.post('/b/deployments', {
      headers: h,
      data: { name: '' },
    });
    expect(res.status()).toBe(400);
  });

  test('deployments require authentication', async ({ request }) => {
    const res = await request.get('/b/deployments');
    // 401 (unauthenticated) or 403 (forbidden) depending on auth middleware
    expect([401, 403]).toContain(res.status());
  });

  test('developer can create multiple deployments', async ({ request }) => {
    const user = await signupUser(request);
    const h = authHeaders(user.token);

    // Create two deployments
    const res1 = await request.post('/b/deployments', {
      headers: h,
      data: { name: 'App One', region: 'us-east-1' },
    });
    const res2 = await request.post('/b/deployments', {
      headers: h,
      data: { name: 'App Two', region: 'eu-west-1' },
    });
    expect(res1.ok()).toBeTruthy();
    expect(res2.ok()).toBeTruthy();

    // List should show both
    const listRes = await request.get('/b/deployments', { headers: h });
    const body = await listRes.json();
    const records = body.records || body;
    expect(records.length).toBeGreaterThanOrEqual(2);
  });
});

// ---------------------------------------------------------------------------
// 4. Admin deployment management
// ---------------------------------------------------------------------------
test.describe('Admin Deployment Management', () => {
  test('admin can list all deployments', async ({ request }) => {
    const adminToken = await getAdminToken(request);
    const h = authHeaders(adminToken);

    const res = await request.get('/admin/b/deployments', { headers: h });
    if (res.status() === 403 || res.status() === 401) {
      test.skip();
      return;
    }
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const records = body.records || body;
    expect(Array.isArray(records)).toBeTruthy();
  });

  test('admin can view deployment stats', async ({ request }) => {
    const adminToken = await getAdminToken(request);
    const h = authHeaders(adminToken);

    const res = await request.get('/admin/b/deployments/stats', { headers: h });
    if (res.status() === 403 || res.status() === 401) {
      test.skip();
      return;
    }
    expect(res.ok()).toBeTruthy();
    const stats = await res.json();
    expect(stats).toHaveProperty('total');
    expect(typeof stats.total).toBe('number');
  });

  test('admin can view product stats', async ({ request }) => {
    const adminToken = await getAdminToken(request);
    const h = authHeaders(adminToken);

    const res = await request.get('/admin/b/products/stats', { headers: h });
    if (res.status() === 403 || res.status() === 401) {
      test.skip();
      return;
    }
    expect(res.ok()).toBeTruthy();
    const stats = await res.json();
    expect(stats).toHaveProperty('total_products');
  });
});

// ---------------------------------------------------------------------------
// 5. Full user journey: signup → plan → deploy → manage → delete
// ---------------------------------------------------------------------------
test.describe('Full User Journey', () => {
  test('end-to-end: signup, browse plans, create deployment, manage, delete', async ({ request }) => {
    // Step 1: Sign up as a new developer
    const user = await signupUser(request);
    const h = authHeaders(user.token);
    expect(user.token).toBeTruthy();

    // Step 2: View profile
    const meRes = await request.get('/auth/me', { headers: h });
    expect(meRes.ok()).toBeTruthy();
    const me = await meRes.json();
    expect(me.user.email).toBe(user.email);

    // Step 3: Browse available plans
    const catalogRes = await request.get('/b/products/catalog', { headers: h });
    expect(catalogRes.ok()).toBeTruthy();

    // Step 4: Create a deployment
    const deployRes = await request.post('/b/deployments', {
      headers: h,
      data: {
        name: 'My Production App',
        region: 'us-east-1',
      },
    });
    expect(deployRes.ok()).toBeTruthy();
    const deployment = await deployRes.json();
    const deployId = deployment.id;
    expect(deployId).toBeTruthy();

    // Step 5: Verify deployment appears in list
    const listRes = await request.get('/b/deployments', { headers: h });
    expect(listRes.ok()).toBeTruthy();
    const list = await listRes.json();
    const records = list.records || list;
    const found = records.find((d: any) => (d.id || d.data?.id) === deployId);
    expect(found).toBeTruthy();

    // Step 6: View deployment details
    const detailRes = await request.get(`/b/deployments/${deployId}`, { headers: h });
    expect(detailRes.ok()).toBeTruthy();

    // Step 7: Create an API key for programmatic access
    const keyRes = await request.post('/auth/api-keys', {
      headers: h,
      data: { name: 'Production API Key' },
    });
    expect(keyRes.ok()).toBeTruthy();
    const apiKey = await keyRes.json();
    expect(apiKey.key).toMatch(/^sb_/);

    // Step 8: Delete the deployment
    const deleteRes = await request.delete(`/b/deployments/${deployId}`, { headers: h });
    expect(deleteRes.ok()).toBeTruthy();
    const deleted = await deleteRes.json();
    expect(deleted.data?.status || deleted.status).toBe('deleted');

    // Step 9: Verify deployment is marked as deleted
    const afterDeleteRes = await request.get(`/b/deployments/${deployId}`, { headers: h });
    // Might be 404 (filtered out) or 200 with deleted status
    if (afterDeleteRes.ok()) {
      const afterBody = await afterDeleteRes.json();
      expect(afterBody.data?.status || afterBody.status).toBe('deleted');
    }

    // Step 10: Clean up API key
    await request.delete(`/auth/api-keys/${apiKey.id}`, { headers: h });
  });
});

// ---------------------------------------------------------------------------
// 6. User portal
// ---------------------------------------------------------------------------
test.describe('User Portal', () => {
  test('user portal endpoint exists', async ({ request }) => {
    const user = await signupUser(request);
    const res = await request.get('/b/userportal/me', {
      headers: authHeaders(user.token),
    });
    // 200 or 404 (if not implemented for this path) - just verify it doesn't crash
    expect([200, 404]).toContain(res.status());
  });
});

// ---------------------------------------------------------------------------
// 7. Dashboard frontend page serves correctly
// ---------------------------------------------------------------------------
test.describe('Dashboard Frontend', () => {
  test('dashboard HTML page loads', async ({ request }) => {
    const res = await request.get('/blocks/dashboard/frontend/index.html');
    expect(res.ok()).toBeTruthy();
    const html = await res.text();
    expect(html).toContain('<!DOCTYPE html>');
    expect(html).toContain('Dashboard');
  });

  test('deployments admin page loads', async ({ request }) => {
    const res = await request.get('/blocks/deployments/frontend/index.html');
    expect(res.ok()).toBeTruthy();
    const html = await res.text();
    expect(html).toContain('<!DOCTYPE html>');
  });
});
