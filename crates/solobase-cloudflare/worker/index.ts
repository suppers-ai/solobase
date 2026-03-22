// Solobase Cloudflare Worker — TypeScript entry point.
//
// Two modes based on hostname:
//
// 1. Platform (app.solobase.dev / localhost):
//    - Platform SPA (dashboard/admin/auth) from R2 `_site/`
//    - Platform API: auth (signup/login), deployments, admin, billing
//    - Control plane at /_control/*
//    - Uses the shared DB directly (no tenant lookup)
//    - Marketing site (solobase.dev) is served by Cloudflare Pages, not here
//
// 2. Tenant instance ({tenant}.solobase.dev):
//    - Tenant SPA from R2 `{tenantId}/site/`
//    - Tenant API: all block endpoints
//    - Tenant resolved from KV

import type { Env, TenantConfig } from './types';
import { requestToMessage, blockResultToResponse } from './convert';
import { resolveTenant, getD1ForTenant } from './tenant';
import { createHost } from './host';
import { dispatchToBlock } from './dispatch';
import { handleControlPlane } from './control';
import { serveStatic, isMarketingHost } from './static';
import { checkAndIncrementUsage } from './usage';
// Schema is managed via D1 migrations, generated from block declarations:
//   npm run generate:migration > migrations/0001_init.sql
//   npm run db:migrate (local) or npm run db:migrate:prod (production)

export default {
  async fetch(
    request: Request,
    env: Env,
    _ctx: ExecutionContext,
  ): Promise<Response> {
    const url = new URL(request.url);
    const pathname = url.pathname;

    // 1. CORS preflight
    if (request.method === 'OPTIONS') {
      return corsPreflightResponse(request);
    }

    // 2. Control plane routes (/_control/*) — platform admin
    if (pathname.startsWith('/_control/')) {
      return addCorsHeaders(await handleControlPlane(request, env, url), request);
    }

    const isPlatform = isMarketingHost(url.hostname);

    // 3. API routes → dispatch
    if (pathname.startsWith('/api/') || pathname === '/api' || isApiRoute(pathname)) {
      if (isPlatform) {
        // Platform: use shared DB directly, no tenant lookup
        return handlePlatformApi(request, env, url);
      } else {
        // Tenant: resolve from KV
        return handleTenantApi(request, env, url);
      }
    }

    // 4. Static file serving from R2
    if (isPlatform) {
      // Platform (app.solobase.dev): serve SPA (dashboard/admin/auth) from _site/
      const response = await serveStatic(env.STORAGE, '_site/', pathname);
      if (response) return addSecurityHeaders(response);
    } else {
      // Tenant: resolve tenant, serve from {tenantId}/site/
      const tenant = await resolveTenant(url.hostname, env);
      if (!tenant) {
        return addCorsHeaders(jsonError('not_found', 'tenant not found', 404), request);
      }
      const response = await serveStatic(env.STORAGE, `${tenant.id}/site/`, pathname);
      if (response) return addSecurityHeaders(response);
    }

    return addSecurityHeaders(new Response('Not Found', { status: 404 }));
  },
} satisfies ExportedHandler<Env>;

// ---------------------------------------------------------------------------
// Platform API (app.solobase.dev) — uses shared DB, no tenant
// ---------------------------------------------------------------------------

async function handlePlatformApi(
  request: Request,
  env: Env,
  url: URL,
): Promise<Response> {
  const pathname = url.pathname;

  // Billing routes — handled directly, not through block dispatch
  if (pathname.startsWith('/api/billing/') || pathname.startsWith('/billing/')) {
    const { handleBillingRoute } = await import('./billing');
    return addCorsHeaders(await handleBillingRoute(request, env, url), request);
  }

  // Platform uses the shared DB directly
  const db = env.DB;

  // Build a platform "config" with all features enabled
  const platformConfig: TenantConfig = {
    id: 'platform',
    subdomain: 'solobase',
    plan: 'platform',
    config: {
      auth: {}, admin: {}, files: {}, products: {},
      deployments: {}, legalpages: {}, userportal: {},
    },
    blocks: [],
  };

  const host = createHost(env, platformConfig, db);
  const msg = await readRequestMessage(request, url);
  if ('error' in msg) return addCorsHeaders(msg.error, request);

  const result = await dispatchToBlock(msg.message, platformConfig, host, env);
  return addCorsHeaders(blockResultToResponse(result), request);
}

// ---------------------------------------------------------------------------
// Tenant API ({tenant}.solobase.dev, excluding app) — resolves tenant from KV
// ---------------------------------------------------------------------------

async function handleTenantApi(
  request: Request,
  env: Env,
  url: URL,
): Promise<Response> {
  const tenant = await resolveTenant(url.hostname, env);
  if (!tenant) {
    return addCorsHeaders(jsonError('not_found', 'tenant not found', 404), request);
  }

  const db = getD1ForTenant(env, tenant);

  // Enforce plan limits
  const usageCheck = await checkAndIncrementUsage(db, tenant);
  if (usageCheck.error) {
    return addCorsHeaders(jsonError('resource-exhausted', usageCheck.error, 429), request);
  }

  const host = createHost(env, tenant, db);
  const msg = await readRequestMessage(request, url);
  if ('error' in msg) return addCorsHeaders(msg.error, request);

  const result = await dispatchToBlock(msg.message, tenant, host, env);
  const response = addCorsHeaders(blockResultToResponse(result), request);

  // Add warning header if payment is failing or usage is high
  if (usageCheck.warning) {
    response.headers.set('X-Solobase-Warning', usageCheck.warning);
  }

  return response;
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

import type { Message } from './types';

async function readRequestMessage(
  request: Request,
  url: URL,
): Promise<{ message: Message } | { error: Response }> {
  const contentLength = parseInt(request.headers.get('content-length') ?? '0', 10);
  if (contentLength > 10 * 1024 * 1024) {
    return { error: jsonError('resource-exhausted', 'request body too large', 413) };
  }
  const body = new Uint8Array(await request.arrayBuffer());
  if (body.length > 10 * 1024 * 1024) {
    return { error: jsonError('resource-exhausted', 'request body too large', 413) };
  }

  const remoteAddr =
    request.headers.get('cf-connecting-ip') ??
    request.headers.get('x-forwarded-for') ??
    'unknown';
  const message = requestToMessage(request, url, body, remoteAddr);

  // Strip /api prefix
  if (message.meta) {
    const idx = message.meta.findIndex(m => m.key === 'req.resource');
    if (idx >= 0 && message.meta[idx].value.startsWith('/api')) {
      message.meta[idx].value = message.meta[idx].value.substring(4) || '/';
    }
  }

  return { message };
}

function isApiRoute(pathname: string): boolean {
  const apiPrefixes = [
    '/health', '/nav', '/debug/',
    '/auth/', '/admin/', '/storage/',
    '/b/', '/profile/', '/settings/',
    '/internal/', '/billing/',
  ];
  return apiPrefixes.some(p => pathname === p.replace(/\/$/, '') || pathname.startsWith(p));
}

// ---------------------------------------------------------------------------
// CORS
// ---------------------------------------------------------------------------

function corsPreflightResponse(request: Request): Response {
  const origin = getAllowedOrigin(request);
  const headers: Record<string, string> = {
    'Access-Control-Allow-Methods': 'GET, POST, PUT, PATCH, DELETE, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type, Authorization',
    'Access-Control-Max-Age': '86400',
    'Vary': 'Origin',
  };
  if (origin) headers['Access-Control-Allow-Origin'] = origin;
  return new Response(null, { status: 204, headers });
}

function getAllowedOrigin(request: Request): string {
  const origin = request.headers.get('Origin');
  if (!origin) return '';
  try {
    const u = new URL(origin);
    if (u.hostname === 'localhost' || u.hostname === '127.0.0.1') return origin;
    if (u.hostname.endsWith('.solobase.dev')) return origin;
    if (u.hostname === 'solobase.dev') return origin;
    const reqHost = request.headers.get('Host')?.split(':')[0];
    if (reqHost && u.hostname === reqHost) return origin;
  } catch { /* invalid origin */ }
  return '';
}

function addCorsHeaders(response: Response, request?: Request): Response {
  const origin = request ? getAllowedOrigin(request) : '';
  const r = new Response(response.body, response);
  if (origin) {
    r.headers.set('Access-Control-Allow-Origin', origin);
    r.headers.set('Vary', 'Origin');
  }
  r.headers.set('X-Content-Type-Options', 'nosniff');
  r.headers.set('X-Frame-Options', 'DENY');
  return r;
}

function addSecurityHeaders(response: Response): Response {
  const r = new Response(response.body, response);
  r.headers.set('X-Content-Type-Options', 'nosniff');
  r.headers.set('X-Frame-Options', 'DENY');
  return r;
}

function jsonError(code: string, message: string, status: number): Response {
  return new Response(JSON.stringify({ error: code, message }), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}
