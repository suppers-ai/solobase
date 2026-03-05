import { test, expect } from '@playwright/test';

test.describe('Health & System', () => {
  test('GET /health returns ok', async ({ request }) => {
    const res = await request.get('/health');
    expect(res.ok()).toBeTruthy();
    expect(await res.json()).toEqual({ status: 'ok' });
  });

  test('GET /debug/time returns timestamps', async ({ request }) => {
    const res = await request.get('/debug/time');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body).toHaveProperty('utc');
    expect(body).toHaveProperty('unix');
    expect(body).toHaveProperty('unix_ms');
  });

  test('GET /nav requires auth', async ({ request }) => {
    const res = await request.get('/nav');
    expect(res.status()).toBe(401);
  });
});

async function signupAndLogin(request: any) {
  const email = `e2e-${Date.now()}-${Math.random().toString(36).slice(2, 8)}@test.com`;
  const password = 'TestPass1234';
  const signup = await request.post('/auth/signup', {
    data: { email, password },
  });
  const body = await signup.json();
  return { email, password, token: body.access_token, userId: body.user?.id };
}

test.describe('Auth', () => {
  test('POST /auth/signup creates user and returns tokens', async ({ request }) => {
    const { email, token } = await signupAndLogin(request);
    expect(token).toBeTruthy();
  });

  test('POST /auth/signup rejects duplicate email', async ({ request }) => {
    const { email, password } = await signupAndLogin(request);
    const res = await request.post('/auth/signup', {
      data: { email, password },
    });
    expect(res.status()).toBe(409);
  });

  test('POST /auth/login returns tokens', async ({ request }) => {
    const { email, password } = await signupAndLogin(request);
    const res = await request.post('/auth/login', {
      data: { email, password },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body).toHaveProperty('access_token');
    expect(body.user.email).toBe(email);
  });

  test('GET /auth/me returns user profile', async ({ request }) => {
    const { email, token } = await signupAndLogin(request);
    const res = await request.get('/auth/me', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.user.email).toBe(email);
  });

  test('GET /nav returns items when authed', async ({ request }) => {
    const { token } = await signupAndLogin(request);
    const res = await request.get('/nav', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.ok()).toBeTruthy();
    const items = await res.json();
    expect(Array.isArray(items)).toBeTruthy();
    expect(items.length).toBeGreaterThan(0);
  });
});

test.describe('Legal Pages', () => {
  test('GET /ext/legalpages/terms returns HTML', async ({ request }) => {
    const res = await request.get('/ext/legalpages/terms');
    expect(res.ok()).toBeTruthy();
    const ct = res.headers()['content-type'];
    expect(ct).toContain('text/html');
  });

  test('GET /ext/legalpages/privacy returns HTML', async ({ request }) => {
    const res = await request.get('/ext/legalpages/privacy');
    expect(res.ok()).toBeTruthy();
    const ct = res.headers()['content-type'];
    expect(ct).toContain('text/html');
  });
});

test.describe('Frontend SPA', () => {
  test('GET / serves index.html', async ({ request }) => {
    const res = await request.get('/');
    expect(res.ok()).toBeTruthy();
    const html = await res.text();
    expect(html).toContain('Solobase');
  });

  test('unknown path serves SPA fallback', async ({ request }) => {
    const res = await request.get('/some/unknown/path');
    expect(res.ok()).toBeTruthy();
    const html = await res.text();
    expect(html).toContain('Solobase');
  });
});
