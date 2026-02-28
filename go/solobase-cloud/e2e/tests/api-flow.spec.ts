import { test, expect, type APIRequestContext } from '@playwright/test';

/** Generate a unique subdomain suffix to avoid collisions across runs. */
const RUN_ID = `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`;
function uniqueSubdomain(prefix: string) {
  return `${prefix}-${RUN_ID}`;
}

/** Auth header helper to reduce repetition. */
function authHeaders(token: string) {
  return { Cookie: `session=${token}` };
}

/**
 * Helper: creates a dev session and returns the session token.
 * Uses baseURL from Playwright config via the request fixture.
 * Pass an optional user identifier to create a distinct user.
 */
async function getSessionToken(request: APIRequestContext, user?: string) {
  const url = user ? `/api/dev/session?user=${user}` : '/api/dev/session';
  const resp = await request.get(url);
  expect(resp.ok()).toBeTruthy();
  const data = await resp.json();
  expect(data.token).toBeTruthy();
  return data.token as string;
}

/**
 * Helper: create a tenant, assert 201, return parsed response.
 * Caller is responsible for cleanup.
 */
async function createTenant(
  request: APIRequestContext,
  token: string,
  subdomain: string,
  plan = 'free',
) {
  const resp = await request.post('/api/tenants', {
    headers: { ...authHeaders(token), 'Content-Type': 'application/json' },
    data: { subdomain, plan },
  });
  expect(resp.status()).toBe(201);
  return resp.json();
}

/** Helper: delete a tenant (best-effort, for cleanup). */
async function deleteTenant(request: APIRequestContext, token: string, tenantId: string) {
  await request.delete(`/api/tenants/${tenantId}`, {
    headers: authHeaders(token),
  });
}

test.describe('API Flow', () => {
  test('GET /api/plans returns 5 plans with correct structure', async ({ request }) => {
    const response = await request.get('/api/plans');
    expect(response.status()).toBe(200);

    const plans = await response.json();
    expect(plans).toHaveLength(5);

    const planIds = plans.map((p: { id: string }) => p.id);
    expect(planIds).toEqual(['free', 'hobby', 'starter', 'professional', 'business']);

    for (const plan of plans) {
      expect(typeof plan.id).toBe('string');
      expect(typeof plan.name).toBe('string');
      expect(typeof plan.price_cents).toBe('number');
      expect(typeof plan.max_vms).toBe('number');
      expect(Array.isArray(plan.features)).toBe(true);
    }

    // Verify all plan prices
    expect(plans[0].price_cents).toBe(0);      // Free
    expect(plans[1].price_cents).toBe(500);     // Hobby
    expect(plans[2].price_cents).toBe(1500);    // Starter
    expect(plans[3].price_cents).toBe(7900);    // Professional
    expect(plans[4].price_cents).toBe(19900);   // Business
  });

  test('GET /api/dev/session returns session cookie and token', async ({ request }) => {
    const response = await request.get('/api/dev/session');
    expect(response.status()).toBe(200);

    const body = await response.json();
    expect(body.message).toBe('dev session created');
    expect(body.user_email).toBe('dev@localhost');
    expect(body.token).toBeTruthy();

    const setCookie = response.headers()['set-cookie'];
    expect(setCookie).toContain('session=');
  });

  test('GET /api/me with session returns user info', async ({ request }) => {
    const token = await getSessionToken(request);

    const response = await request.get('/api/me', {
      headers: authHeaders(token),
    });
    expect(response.status()).toBe(200);

    const user = await response.json();
    expect(user.name).toBe('Local Dev');
    expect(user.email).toBe('dev@localhost');
  });

  test('POST /api/tenants creates a tenant', async ({ request }) => {
    const token = await getSessionToken(request);
    const subdomain = uniqueSubdomain('create');
    let tenantId: string | undefined;

    try {
      const response = await request.post('/api/tenants', {
        headers: { ...authHeaders(token), 'Content-Type': 'application/json' },
        data: { subdomain, plan: 'free' },
      });
      expect(response.status()).toBe(201);

      const tenant = await response.json();
      tenantId = tenant.id;
      expect(tenant.subdomain).toBe(subdomain);
      expect(tenant.state).toBe('running');
      expect(tenant.id).toBeTruthy();
    } finally {
      if (tenantId) await deleteTenant(request, token, tenantId);
    }
  });

  test('GET /api/tenants lists created tenants', async ({ request }) => {
    const token = await getSessionToken(request);
    const subdomain = uniqueSubdomain('list');
    let tenantId: string | undefined;

    try {
      const created = await createTenant(request, token, subdomain, 'hobby');
      tenantId = created.id;

      const response = await request.get('/api/tenants', {
        headers: authHeaders(token),
      });
      expect(response.status()).toBe(200);

      const tenants = await response.json();
      expect(tenants.length).toBeGreaterThanOrEqual(1);
      const found = tenants.find((t: { subdomain: string }) => t.subdomain === subdomain);
      expect(found).toBeTruthy();
    } finally {
      if (tenantId) await deleteTenant(request, token, tenantId);
    }
  });

  test('GET /api/tenants/{id} returns tenant detail', async ({ request }) => {
    const token = await getSessionToken(request);
    const subdomain = uniqueSubdomain('detail');
    let tenantId: string | undefined;

    try {
      const created = await createTenant(request, token, subdomain, 'starter');
      tenantId = created.id;

      const response = await request.get(`/api/tenants/${created.id}`, {
        headers: authHeaders(token),
      });
      expect(response.status()).toBe(200);

      const detail = await response.json();
      expect(detail.id).toBe(created.id);
      expect(detail.subdomain).toBe(subdomain);
    } finally {
      if (tenantId) await deleteTenant(request, token, tenantId);
    }
  });

  test('POST /api/tenants/{id}/pause updates state to paused', async ({ request }) => {
    const token = await getSessionToken(request);
    let tenantId: string | undefined;

    try {
      const created = await createTenant(request, token, uniqueSubdomain('pause'));
      tenantId = created.id;
      expect(created.state).toBe('running');

      const response = await request.post(`/api/tenants/${created.id}/pause`, {
        headers: authHeaders(token),
      });
      expect(response.status()).toBe(200);

      // Verify state changed via GET
      const getResp = await request.get(`/api/tenants/${created.id}`, {
        headers: authHeaders(token),
      });
      expect(getResp.status()).toBe(200);
      const detail = await getResp.json();
      expect(detail.state).toBe('paused');
    } finally {
      if (tenantId) await deleteTenant(request, token, tenantId);
    }
  });

  test('POST /api/tenants/{id}/resume updates state to running', async ({ request }) => {
    const token = await getSessionToken(request);
    let tenantId: string | undefined;

    try {
      const created = await createTenant(request, token, uniqueSubdomain('resume'));
      tenantId = created.id;

      // Pause first
      const pauseResp = await request.post(`/api/tenants/${created.id}/pause`, {
        headers: authHeaders(token),
      });
      expect(pauseResp.status()).toBe(200);

      // Verify paused
      const pausedGet = await request.get(`/api/tenants/${created.id}`, {
        headers: authHeaders(token),
      });
      const pausedDetail = await pausedGet.json();
      expect(pausedDetail.state).toBe('paused');

      // Resume
      const response = await request.post(`/api/tenants/${created.id}/resume`, {
        headers: authHeaders(token),
      });
      expect(response.status()).toBe(200);

      // Verify running again
      const resumedGet = await request.get(`/api/tenants/${created.id}`, {
        headers: authHeaders(token),
      });
      expect(resumedGet.status()).toBe(200);
      const resumedDetail = await resumedGet.json();
      expect(resumedDetail.state).toBe('running');
    } finally {
      if (tenantId) await deleteTenant(request, token, tenantId);
    }
  });

  test('DELETE /api/tenants/{id} returns 204', async ({ request }) => {
    const token = await getSessionToken(request);
    const created = await createTenant(request, token, uniqueSubdomain('delete'));

    const response = await request.delete(`/api/tenants/${created.id}`, {
      headers: authHeaders(token),
    });
    expect(response.status()).toBe(204);

    // Verify gone
    const getResp = await request.get(`/api/tenants/${created.id}`, {
      headers: authHeaders(token),
    });
    expect(getResp.status()).toBe(404);
  });

  test('unauthenticated requests return 401', async ({ request }) => {
    const tenantsResp = await request.get('/api/tenants');
    expect(tenantsResp.status()).toBe(401);

    const meResp = await request.get('/api/me');
    expect(meResp.status()).toBe(401);

    const createResp = await request.post('/api/tenants', {
      data: { subdomain: 'unauth', plan: 'free' },
    });
    expect(createResp.status()).toBe(401);
  });

  test('creating tenant with empty subdomain returns 400', async ({ request }) => {
    const token = await getSessionToken(request);

    const resp = await request.post('/api/tenants', {
      headers: { ...authHeaders(token), 'Content-Type': 'application/json' },
      data: { subdomain: '', plan: 'free' },
    });
    expect(resp.status()).toBe(400);
  });

  test('creating tenant with invalid plan returns 400', async ({ request }) => {
    const token = await getSessionToken(request);

    const resp = await request.post('/api/tenants', {
      headers: { ...authHeaders(token), 'Content-Type': 'application/json' },
      data: { subdomain: uniqueSubdomain('badplan'), plan: 'enterprise' },
    });
    expect(resp.status()).toBe(400);
  });

  test('creating tenant with duplicate subdomain returns 500', async ({ request }) => {
    const token = await getSessionToken(request);
    const subdomain = uniqueSubdomain('dupe');
    let tenantId: string | undefined;

    try {
      const created = await createTenant(request, token, subdomain);
      tenantId = created.id;

      // Try to create another with the same subdomain
      const resp = await request.post('/api/tenants', {
        headers: { ...authHeaders(token), 'Content-Type': 'application/json' },
        data: { subdomain, plan: 'free' },
      });
      // Server returns 500 for duplicate subdomain (wraps the error from tenant service)
      expect(resp.status()).toBe(500);
    } finally {
      if (tenantId) await deleteTenant(request, token, tenantId);
    }
  });

  test('admin endpoints require Bearer token auth', async ({ request }) => {
    // Without auth
    const noAuth = await request.get('/api/admin/nodes');
    expect(noAuth.status()).toBe(403);

    // With wrong token
    const wrongAuth = await request.get('/api/admin/nodes', {
      headers: { Authorization: 'Bearer wrong-secret' },
    });
    expect(wrongAuth.status()).toBe(403);

    // With correct token
    const goodAuth = await request.get('/api/admin/nodes', {
      headers: { Authorization: 'Bearer dev-secret' },
    });
    expect(goodAuth.status()).toBe(200);
  });

  test('user A cannot see user B tenants in list', async ({ request }) => {
    const tokenA = await getSessionToken(request, 'user-a');
    const tokenB = await getSessionToken(request, 'user-b');
    let tenantId: string | undefined;

    try {
      // User B creates a tenant
      const created = await createTenant(request, tokenB, uniqueSubdomain('iso-list'));
      tenantId = created.id;

      // User A lists tenants — should not see user B's tenant
      const listResp = await request.get('/api/tenants', {
        headers: authHeaders(tokenA),
      });
      expect(listResp.status()).toBe(200);
      const tenants = await listResp.json();
      // ListByUser returns null (not []) when user has no tenants
      const found = (tenants ?? []).find((t: { id: string }) => t.id === created.id);
      expect(found).toBeUndefined();
    } finally {
      if (tenantId) await deleteTenant(request, tokenB, tenantId);
    }
  });

  test('user A cannot access user B tenant by ID (403)', async ({ request }) => {
    const tokenA = await getSessionToken(request, 'user-a');
    const tokenB = await getSessionToken(request, 'user-b');
    let tenantId: string | undefined;

    try {
      // User B creates a tenant
      const created = await createTenant(request, tokenB, uniqueSubdomain('iso-get'));
      tenantId = created.id;

      // User A tries to access it directly
      const getResp = await request.get(`/api/tenants/${created.id}`, {
        headers: authHeaders(tokenA),
      });
      expect(getResp.status()).toBe(403);
    } finally {
      if (tenantId) await deleteTenant(request, tokenB, tenantId);
    }
  });
});
