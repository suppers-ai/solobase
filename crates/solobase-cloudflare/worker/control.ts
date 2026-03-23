/**
 * Control plane API — platform-level project management.
 *
 * All endpoints under /_control/ require X-Admin-Secret header.
 */

import type { Env, ProjectConfig, ProjectAppConfig } from './types';
import { RESERVED_SUBDOMAINS } from './project';

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

  const kv = env.PROJECTS;
  const db = env.DB;

  // Health
  if (method === 'GET' && path === 'health') {
    const projects = await listProjects(kv);
    return json({ status: 'ok', project_count: projects.length, version: '1.0.0' }, 200);
  }

  // List projects
  if (method === 'GET' && path === 'projects') {
    const projects = await listProjects(kv);
    return json({ projects }, 200);
  }

  // Get project
  if (method === 'GET' && path.startsWith('projects/')) {
    const subdomain = path.slice('projects/'.length);
    const config = await getProject(kv, subdomain);
    if (!config) return json({ error: 'not_found', message: 'project not found' }, 404);
    return json(config, 200);
  }

  // Create project
  if (method === 'POST' && path === 'projects') {
    try {
      const body = await request.json() as { subdomain: string; plan?: string; config?: ProjectAppConfig };
      const project = await createProject(kv, db, body.subdomain, body.plan ?? 'hobby', body.config);
      return json(project, 201);
    } catch (e: any) {
      const msg = e?.message ?? 'failed to create project';
      const status = msg.includes('already exists') ? 409 : (msg.includes('subdomain must') || msg.includes('reserved')) ? 400 : 500;
      return json({ error: 'failed', message: msg }, status);
    }
  }

  // Update project
  if ((method === 'PUT' || method === 'PATCH') && path.startsWith('projects/')) {
    const subdomain = path.slice('projects/'.length);
    const current = await getProject(kv, subdomain);
    if (!current) return json({ error: 'not_found', message: 'project not found' }, 404);

    const updates = await request.json() as Record<string, unknown>;
    if (typeof updates.plan === 'string') current.plan = updates.plan;
    if (updates.config) current.config = updates.config as ProjectAppConfig;

    await updateProject(kv, subdomain, current);
    return json(current, 200);
  }

  // Delete project
  if (method === 'DELETE' && path.startsWith('projects/')) {
    const subdomain = path.slice('projects/'.length);
    await deleteProject(kv, subdomain);
    return json({ deleted: true }, 200);
  }

  return json({ error: 'not_found', message: 'control endpoint not found' }, 404);
}

// ---------------------------------------------------------------------------
// Project CRUD
// ---------------------------------------------------------------------------

async function listProjects(kv: KVNamespace): Promise<string[]> {
  const raw = await kv.get('projects:list');
  return raw ? JSON.parse(raw) : [];
}

async function getProject(kv: KVNamespace, subdomain: string): Promise<ProjectConfig | null> {
  const raw = await kv.get(`project:${subdomain}:config`);
  return raw ? JSON.parse(raw) : null;
}

async function createProject(
  kv: KVNamespace,
  _db: D1Database,
  subdomain: string,
  plan: string,
  appConfig?: ProjectAppConfig,
): Promise<ProjectConfig> {
  const existing = await getProject(kv, subdomain);
  if (existing) throw new Error(`project '${subdomain}' already exists`);

  // Validate subdomain: alphanumeric + hyphens only, 3-63 chars
  if (!/^[a-z0-9][a-z0-9-]{1,61}[a-z0-9]$/.test(subdomain)) {
    throw new Error('subdomain must be 3-63 lowercase alphanumeric characters or hyphens');
  }

  // Reject reserved subdomains
  if (RESERVED_SUBDOMAINS.has(subdomain)) {
    throw new Error('this subdomain is reserved');
  }

  const config: ProjectConfig = {
    id: crypto.randomUUID(),
    subdomain,
    plan,
    config: appConfig ?? allFeaturesEnabled(),
    blocks: [],
  };

  // Migrations are applied via: npx wrangler d1 migrations apply solobase-db
  await kv.put(`project:${subdomain}:config`, JSON.stringify(config));

  // Add to list
  const list = await listProjects(kv);
  if (!list.includes(subdomain)) {
    list.push(subdomain);
    await kv.put('projects:list', JSON.stringify(list));
  }

  return config;
}

async function updateProject(kv: KVNamespace, subdomain: string, config: ProjectConfig): Promise<void> {
  await kv.put(`project:${subdomain}:config`, JSON.stringify(config));
}

async function deleteProject(kv: KVNamespace, subdomain: string): Promise<void> {
  await kv.delete(`project:${subdomain}:config`);
  const list = await listProjects(kv);
  await kv.put('projects:list', JSON.stringify(list.filter(s => s !== subdomain)));
}

function allFeaturesEnabled(): ProjectAppConfig {
  return { auth: {}, admin: {}, files: {}, products: {}, projects: {}, legalpages: {}, userportal: {} };
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
