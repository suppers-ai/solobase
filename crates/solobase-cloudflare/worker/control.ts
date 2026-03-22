/**
 * Control plane API — platform-level tenant management.
 *
 * All endpoints under /_control/ require X-Admin-Secret header.
 */

import type { Env, TenantConfig, TenantAppConfig } from './types';

export async function handleControlPlane(
  request: Request,
  env: Env,
  url: URL,
): Promise<Response> {
  const path = url.pathname.replace(/^\/_control\/?/, '');
  const method = request.method;

  // Verify admin secret (constant-time comparison to prevent timing attacks)
  const provided = request.headers.get('x-admin-secret') ?? '';
  const expected = (env.ADMIN_SECRET as string) ?? '';
  if (!expected || !(await timingSafeEqual(provided, expected))) {
    return json({ error: 'unauthorized', message: 'invalid admin secret' }, 401);
  }

  const kv = env.TENANTS;
  const db = env.DB;

  // Health
  if (method === 'GET' && path === 'health') {
    const tenants = await listTenants(kv);
    return json({ status: 'ok', tenant_count: tenants.length, version: '1.0.0' }, 200);
  }

  // List tenants
  if (method === 'GET' && path === 'tenants') {
    const tenants = await listTenants(kv);
    return json({ tenants }, 200);
  }

  // Get tenant
  if (method === 'GET' && path.startsWith('tenants/')) {
    const subdomain = path.slice('tenants/'.length);
    const config = await getTenant(kv, subdomain);
    if (!config) return json({ error: 'not_found', message: 'tenant not found' }, 404);
    return json(config, 200);
  }

  // Create tenant
  if (method === 'POST' && path === 'tenants') {
    try {
      const body = await request.json() as { subdomain: string; plan?: string; config?: TenantAppConfig };
      const tenant = await createTenant(kv, db, body.subdomain, body.plan ?? 'hobby', body.config);
      return json(tenant, 201);
    } catch (e: any) {
      const msg = e?.message ?? 'failed to create tenant';
      const status = msg.includes('already exists') ? 409 : msg.includes('subdomain must') ? 400 : 500;
      return json({ error: 'failed', message: msg }, status);
    }
  }

  // Update tenant
  if ((method === 'PUT' || method === 'PATCH') && path.startsWith('tenants/')) {
    const subdomain = path.slice('tenants/'.length);
    const current = await getTenant(kv, subdomain);
    if (!current) return json({ error: 'not_found', message: 'tenant not found' }, 404);

    const updates = await request.json() as Record<string, unknown>;
    if (typeof updates.plan === 'string') current.plan = updates.plan;
    if (updates.config) current.config = updates.config as TenantAppConfig;

    await updateTenant(kv, subdomain, current);
    return json(current, 200);
  }

  // Delete tenant
  if (method === 'DELETE' && path.startsWith('tenants/')) {
    const subdomain = path.slice('tenants/'.length);
    await deleteTenant(kv, subdomain);
    return json({ deleted: true }, 200);
  }

  return json({ error: 'not_found', message: 'control endpoint not found' }, 404);
}

// ---------------------------------------------------------------------------
// Tenant CRUD
// ---------------------------------------------------------------------------

async function listTenants(kv: KVNamespace): Promise<string[]> {
  const raw = await kv.get('tenants:list');
  return raw ? JSON.parse(raw) : [];
}

async function getTenant(kv: KVNamespace, subdomain: string): Promise<TenantConfig | null> {
  const raw = await kv.get(`tenant:${subdomain}:config`);
  return raw ? JSON.parse(raw) : null;
}

async function createTenant(
  kv: KVNamespace,
  _db: D1Database,
  subdomain: string,
  plan: string,
  appConfig?: TenantAppConfig,
): Promise<TenantConfig> {
  const existing = await getTenant(kv, subdomain);
  if (existing) throw new Error(`tenant '${subdomain}' already exists`);

  // Validate subdomain: alphanumeric + hyphens only, 3-63 chars
  if (!/^[a-z0-9][a-z0-9-]{1,61}[a-z0-9]$/.test(subdomain)) {
    throw new Error('subdomain must be 3-63 lowercase alphanumeric characters or hyphens');
  }

  const config: TenantConfig = {
    id: crypto.randomUUID(),
    subdomain,
    plan,
    config: appConfig ?? allFeaturesEnabled(),
    blocks: [],
  };

  // Migrations are applied via: npx wrangler d1 migrations apply solobase-db
  await kv.put(`tenant:${subdomain}:config`, JSON.stringify(config));

  // Add to list
  const list = await listTenants(kv);
  if (!list.includes(subdomain)) {
    list.push(subdomain);
    await kv.put('tenants:list', JSON.stringify(list));
  }

  return config;
}

async function updateTenant(kv: KVNamespace, subdomain: string, config: TenantConfig): Promise<void> {
  await kv.put(`tenant:${subdomain}:config`, JSON.stringify(config));
}

async function deleteTenant(kv: KVNamespace, subdomain: string): Promise<void> {
  await kv.delete(`tenant:${subdomain}:config`);
  const list = await listTenants(kv);
  await kv.put('tenants:list', JSON.stringify(list.filter(s => s !== subdomain)));
}

function allFeaturesEnabled(): TenantAppConfig {
  return { auth: {}, admin: {}, files: {}, products: {}, deployments: {}, legalpages: {}, userportal: {} };
}

function json(data: unknown, status: number): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

/** Constant-time string comparison via SHA-256 to prevent timing and length leaks. */
async function timingSafeEqual(a: string, b: string): Promise<boolean> {
  const enc = new TextEncoder();
  // Hash both values to fixed 32-byte digests — eliminates length leak
  const [ha, hb] = await Promise.all([
    crypto.subtle.digest('SHA-256', enc.encode(a)),
    crypto.subtle.digest('SHA-256', enc.encode(b)),
  ]);
  const ua = new Uint8Array(ha);
  const ub = new Uint8Array(hb);
  // Constant-time comparison on fixed-size hashes
  let result = 0;
  for (let i = 0; i < ua.length; i++) {
    result |= ua[i] ^ ub[i];
  }
  return result === 0;
}
