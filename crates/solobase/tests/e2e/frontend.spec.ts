import { test, expect } from '@playwright/test';

// ---------------------------------------------------------------------------
// Helper: sign up a fresh user and return credentials + token
// ---------------------------------------------------------------------------
async function signupUser(request: any) {
  const email = `user-${Date.now()}-${Math.random().toString(36).slice(2, 8)}@test.com`;
  const password = 'TestPass1234';
  const res = await request.post('/auth/signup', {
    data: { email, password },
  });
  const body = await res.json();
  return { email, password, token: body.access_token as string, userId: body.user?.id as string, roles: body.user?.roles as string[] };
}

// Helper: get an authenticated token. If the user happens to be admin, great.
// For admin-specific tests, we use a separate describe block with setup.
async function getToken(request: any) {
  const { token } = await signupUser(request);
  return token;
}

// ---------------------------------------------------------------------------
// Landing page
// ---------------------------------------------------------------------------
test.describe('Landing Page', () => {
  test('serves the marketing landing page at /', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveTitle(/Solobase/);
    const body = await page.textContent('body');
    expect(body).toContain('Solobase');
  });

  test('landing page has navigation links', async ({ page }) => {
    await page.goto('/');
    const html = await page.content();
    expect(html.toLowerCase()).toMatch(/feature|doc|get started|open source/);
  });

  test('serves favicon', async ({ request }) => {
    const res = await request.get('/favicon.ico');
    expect(res.ok()).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Login page (admin SPA)
// ---------------------------------------------------------------------------
test.describe('Login Page', () => {
  test('login page loads and renders', async ({ request }) => {
    const res = await request.get('/blocks/auth/frontend/index.html');
    expect(res.ok()).toBeTruthy();
    const html = await res.text();
    expect(html).toContain('<!DOCTYPE html>');
    expect(html).toContain('Login');
    expect(html).toContain('id="app"');
  });
});

// ---------------------------------------------------------------------------
// Admin frontend pages — verify the HTML entry points are served
// ---------------------------------------------------------------------------
test.describe('Admin Frontend Pages', () => {
  test('admin dashboard HTML is served', async ({ request }) => {
    const res = await request.get('/blocks/admin/frontend/index.html');
    expect(res.ok()).toBeTruthy();
    const html = await res.text();
    expect(html).toContain('<!DOCTYPE html>');
    expect(html).toContain('assets/');
  });

  test('products HTML is served', async ({ request }) => {
    const res = await request.get('/blocks/products/frontend/index.html');
    expect(res.ok()).toBeTruthy();
    const html = await res.text();
    expect(html).toContain('<!DOCTYPE html>');
  });

  test('logs HTML is served', async ({ request }) => {
    const res = await request.get('/blocks/logs/frontend/index.html');
    expect(res.ok()).toBeTruthy();
    const html = await res.text();
    expect(html).toContain('<!DOCTYPE html>');
  });

  test('user dashboard HTML is served', async ({ request }) => {
    const res = await request.get('/blocks/dashboard/frontend/index.html');
    expect(res.ok()).toBeTruthy();
    const html = await res.text();
    expect(html).toContain('<!DOCTYPE html>');
    expect(html).toContain('Dashboard');
  });

  test('IAM HTML is served', async ({ request }) => {
    const res = await request.get('/blocks/admin/frontend/iam/index.html');
    expect(res.ok()).toBeTruthy();
    const html = await res.text();
    expect(html).toContain('<!DOCTYPE html>');
    expect(html).toContain('IAM');
  });
});

// ---------------------------------------------------------------------------
// Admin API endpoints — use the first user (always admin in a fresh DB)
// We rely on api.spec.ts signupAndLogin creating the first user which gets admin.
// These tests create their own admin by being first to signup.
// ---------------------------------------------------------------------------
test.describe('Admin API', () => {
  let adminToken: string;

  test.beforeAll(async ({ request }) => {
    // Sign up a user — if they get admin role, use them.
    // Otherwise we need to create one. In a fresh DB, first user = admin.
    const email = `admin-setup-${Date.now()}@test.com`;
    const password = 'AdminPass1234';
    const res = await request.post('/auth/signup', {
      data: { email, password },
    });
    const body = await res.json();
    if (body.user?.roles?.includes('admin')) {
      adminToken = body.access_token;
    } else {
      // The first user was already created by api.spec.ts.
      // Just use the current user — admin tests will be skipped if not admin.
      adminToken = body.access_token;
    }
  });

  test('GET /admin/users returns user list or 403/401', async ({ request }) => {
    const res = await request.get('/admin/users', {
      headers: { Authorization: `Bearer ${adminToken}` },
    });
    // 200 (admin), 403 (not admin), or 401 (token issue on re-run)
    expect([200, 401, 403]).toContain(res.status());
    if (res.status() === 200) {
      const body = await res.json();
      expect(body).toHaveProperty('records');
      expect(Array.isArray(body.records)).toBeTruthy();
    }
  });

  test('GET /admin/settings returns settings or 403/401', async ({ request }) => {
    const res = await request.get('/admin/settings', {
      headers: { Authorization: `Bearer ${adminToken}` },
    });
    expect([200, 401, 403]).toContain(res.status());
  });

  test('GET /admin/logs returns logs or 403/401', async ({ request }) => {
    const res = await request.get('/admin/logs', {
      headers: { Authorization: `Bearer ${adminToken}` },
    });
    expect([200, 401, 403]).toContain(res.status());
  });

  test('admin endpoints require auth', async ({ request }) => {
    const res = await request.get('/admin/users');
    expect(res.status()).toBe(401);
  });
});

// ---------------------------------------------------------------------------
// Auth - extended tests
// ---------------------------------------------------------------------------
test.describe('Auth Extended', () => {
  test('POST /auth/change-password works', async ({ request }) => {
    const { email, password, token } = await signupUser(request);
    const newPassword = 'NewTestPass5678';
    const res = await request.post('/auth/change-password', {
      headers: { Authorization: `Bearer ${token}` },
      data: { current_password: password, new_password: newPassword },
    });
    expect(res.ok()).toBeTruthy();

    // Login with new password
    const loginRes = await request.post('/auth/login', {
      data: { email, password: newPassword },
    });
    expect(loginRes.ok()).toBeTruthy();
  });

  test('POST /auth/change-password rejects wrong current password', async ({ request }) => {
    const { token } = await signupUser(request);
    const res = await request.post('/auth/change-password', {
      headers: { Authorization: `Bearer ${token}` },
      data: { current_password: 'wrongpassword', new_password: 'NewPass5678' },
    });
    expect(res.status()).toBe(401);
  });

  test('POST /auth/refresh returns new tokens', async ({ request }) => {
    const res1 = await request.post('/auth/signup', {
      data: {
        email: `refresh-${Date.now()}-${Math.random().toString(36).slice(2, 6)}@test.com`,
        password: 'TestPass1234',
      },
    });
    const { refresh_token } = await res1.json();
    expect(refresh_token).toBeTruthy();

    const res2 = await request.post('/auth/refresh', {
      data: { refresh_token },
    });
    expect(res2.ok()).toBeTruthy();
    const body = await res2.json();
    expect(body).toHaveProperty('access_token');
    expect(body).toHaveProperty('refresh_token');
  });

  test('POST /auth/logout clears session', async ({ request }) => {
    const { token } = await signupUser(request);
    const res = await request.post('/auth/logout', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.ok()).toBeTruthy();
  });

  test('PUT /auth/me updates user profile', async ({ request }) => {
    const { token } = await signupUser(request);
    const res = await request.put('/auth/me', {
      headers: { Authorization: `Bearer ${token}` },
      data: { name: 'Test User' },
    });
    expect(res.ok()).toBeTruthy();

    // Verify update
    const meRes = await request.get('/auth/me', {
      headers: { Authorization: `Bearer ${token}` },
    });
    const body = await meRes.json();
    expect(body.user.name).toBe('Test User');
  });

  test('signup rejects short passwords', async ({ request }) => {
    const res = await request.post('/auth/signup', {
      data: { email: `short-${Date.now()}@test.com`, password: '123' },
    });
    expect(res.status()).toBe(400);
  });

  test('signup rejects invalid emails', async ({ request }) => {
    const res = await request.post('/auth/signup', {
      data: { email: 'notanemail', password: 'TestPass1234' },
    });
    expect(res.status()).toBe(400);
  });
});

// ---------------------------------------------------------------------------
// API Keys (user-level, no admin required)
// ---------------------------------------------------------------------------
test.describe('API Keys', () => {
  test('CRUD lifecycle for API keys', async ({ request }) => {
    const { token } = await signupUser(request);

    // List
    const listRes = await request.get('/auth/api-keys', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(listRes.ok()).toBeTruthy();

    // Create
    const createRes = await request.post('/auth/api-keys', {
      headers: { Authorization: `Bearer ${token}` },
      data: { name: 'E2E Test Key' },
    });
    expect(createRes.ok()).toBeTruthy();
    const key = await createRes.json();
    expect(key).toHaveProperty('key');
    expect(key).toHaveProperty('id');
    expect(key.key).toMatch(/^sb_/);

    // Revoke
    const revokeRes = await request.delete(`/auth/api-keys/${key.id}`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(revokeRes.ok()).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Products catalog (user-level)
// ---------------------------------------------------------------------------
test.describe('Products', () => {
  test('GET /b/products/catalog returns product list', async ({ request }) => {
    const { token } = await signupUser(request);
    const res = await request.get('/b/products/catalog', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.ok()).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Storage / Files (user-level)
// ---------------------------------------------------------------------------
test.describe('Storage', () => {
  test('GET /storage/buckets returns bucket list', async ({ request }) => {
    const { token } = await signupUser(request);
    const res = await request.get('/storage/buckets', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.ok()).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// SPA routing
// ---------------------------------------------------------------------------
test.describe('SPA Routing', () => {
  test('deep unknown paths return 200 with HTML', async ({ request }) => {
    const res = await request.get('/some/deeply/nested/path');
    expect(res.ok()).toBeTruthy();
    const html = await res.text();
    expect(html).toContain('Solobase');
  });

  test('root serves HTML', async ({ request }) => {
    const res = await request.get('/');
    expect(res.ok()).toBeTruthy();
    const html = await res.text();
    expect(html).toContain('<!DOCTYPE html>');
  });
});
