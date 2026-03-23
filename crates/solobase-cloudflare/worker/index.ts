// Solobase Cloudflare Worker — TypeScript entry point.
//
// Two modes based on hostname:
//
// 1. Platform (cloud.solobase.dev / localhost):
//    - Platform SPA (dashboard/admin/auth) from R2 `_site/`
//    - Platform API: auth (signup/login), projects, admin, billing
//    - Control plane at /_control/*
//    - Uses the shared DB directly (no project lookup)
//    - Marketing site (solobase.dev) is served by Cloudflare Pages, not here
//
// 2. Project instance ({project}.solobase.dev):
//    - Project SPA from R2 `{projectId}/site/`
//    - Project API: all block endpoints
//    - Project: resolve from KV

import type { Env, ProjectConfig } from './types';
import { requestToMessage, blockResultToResponse } from './convert';
import { resolveProject, getD1ForProject } from './project';
import { createHost } from './host';
import { dispatchToBlock } from './dispatch';
import { handleControlPlane } from './control';
import { serveStatic, isMarketingHost } from './static';
import { checkAndIncrementUsage } from './usage';
import { classifyRequest, checkRateLimit, rateLimitHeaders } from './rate-limit';
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
      // Rate limiting (per-IP, using KV)
      const clientIp =
        request.headers.get('cf-connecting-ip') ??
        request.headers.get('x-forwarded-for')?.split(',')[0]?.trim() ??
        'unknown';
      const apiPath = pathname.startsWith('/api') ? pathname.substring(4) || '/' : pathname;
      const rlBucket = classifyRequest(apiPath);
      if (rlBucket && env.PROJECTS) {
        const rl = await checkRateLimit(env.PROJECTS, clientIp, rlBucket);
        if (!rl.allowed) {
          const rlHeaders = rateLimitHeaders(rl);
          return addCorsHeaders(
            new Response(JSON.stringify({ error: 'rate-limited', message: 'too many requests' }), {
              status: 429,
              headers: { 'Content-Type': 'application/json', ...rlHeaders },
            }),
            request,
          );
        }
      }

      if (isPlatform) {
        // Platform: use shared DB directly, no project lookup
        return handlePlatformApi(request, env, url);
      } else {
        // Project: resolve from KV
        return handleTenantApi(request, env, url);
      }
    }

    // 4. Static file serving from R2
    if (isPlatform) {
      // Redirect cloud.solobase.dev root to dashboard
      if (pathname === '/' || pathname === '') {
        const dashUrl = isDev(env) ? '/blocks/dashboard/frontend/' : 'https://cloud.solobase.dev/blocks/dashboard/frontend/';
        return new Response(null, { status: 302, headers: { 'Location': dashUrl } });
      }

      // Platform (cloud.solobase.dev): serve SPA (dashboard/admin/auth) from _site/
      const response = await serveStatic(env.STORAGE, '_site/', pathname);
      if (response) return addSecurityHeaders(response);
    } else {
      // Project: resolve project, serve from {tenantId}/site/
      const project = await resolveProject(url.hostname, env);
      if (!project) {
        return addCorsHeaders(jsonError('not_found', 'project not found', 404), request);
      }
      if (project.status === 'inactive') {
        return addSecurityHeaders(new Response(
          '<html><body style="font-family:sans-serif;display:flex;align-items:center;justify-content:center;height:100vh;margin:0"><div style="text-align:center"><h1>Project Inactive</h1><p>This project is inactive. The owner needs to upgrade their plan to activate it.</p></div></body></html>',
          { status: 403, headers: { 'Content-Type': 'text/html' } },
        ));
      }
      const response = await serveStatic(env.STORAGE, `${project.id}/site/`, pathname);
      if (response) return addSecurityHeaders(response);
    }

    return addSecurityHeaders(new Response('Not Found', { status: 404 }));
  },
} satisfies ExportedHandler<Env>;

// ---------------------------------------------------------------------------
// Platform API (cloud.solobase.dev) — uses shared DB, no project
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
  const platformConfig: ProjectConfig = {
    id: 'platform',
    subdomain: 'solobase',
    plan: 'platform',
    config: {
      auth: {}, admin: {}, files: {}, products: {},
      projects: {}, legalpages: {}, userportal: {},
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
// Project API ({project}.solobase.dev, excluding app) — resolves project from KV
// ---------------------------------------------------------------------------

async function handleTenantApi(
  request: Request,
  env: Env,
  url: URL,
): Promise<Response> {
  const project = await resolveProject(url.hostname, env);
  if (!project) {
    return addCorsHeaders(jsonError('not_found', 'project not found', 404), request);
  }

  // Check if the project is active — inactive projects cannot serve API responses
  if (project.status === 'inactive') {
    return addCorsHeaders(
      new Response(
        JSON.stringify({
          error: 'project-inactive',
          message: 'This project is inactive. Upgrade your plan to activate it.',
        }),
        { status: 403, headers: { 'Content-Type': 'application/json' } },
      ),
      request,
    );
  }

  const db = getD1ForProject(env, project);

  // Enforce plan limits
  const usageCheck = await checkAndIncrementUsage(db, project);
  if (usageCheck.error) {
    return addCorsHeaders(jsonError('resource-exhausted', usageCheck.error, 429), request);
  }

  const host = createHost(env, project, db);
  const msg = await readRequestMessage(request, url);
  if ('error' in msg) return addCorsHeaders(msg.error, request);

  const result = await dispatchToBlock(msg.message, project, host, env);
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

  // Strip /api prefix and normalize /ext/ to /b/
  if (message.meta) {
    const idx = message.meta.findIndex(m => m.key === 'req.resource');
    if (idx >= 0) {
      let path = message.meta[idx].value;
      if (path.startsWith('/api')) path = path.substring(4) || '/';
      if (path.startsWith('/ext/')) path = '/b/' + path.substring(5);
      // Alias: /b/deployments → /b/projects (frontend uses old name)
      path = path.replace('/b/deployments', '/b/projects');
      path = path.replace('/admin/b/deployments', '/admin/b/projects');
      message.meta[idx].value = path;
    }
  }

  return { message };
}

function isApiRoute(pathname: string): boolean {
  const apiPrefixes = [
    '/health', '/nav', '/debug/',
    '/auth/', '/admin/', '/storage/',
    '/b/', '/ext/', '/profile/', '/settings/',
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
    // Only allow the platform subdomain and the exact marketing domain.
    // Project subdomains are NOT allowed as CORS origins for other projects
    // — each project should only access its own API (same-origin).
    if (u.hostname === 'cloud.solobase.dev') return origin;
    if (u.hostname === 'solobase.dev') return origin;
    // Allow same-origin: the request's own host is always allowed
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
  r.headers.set('Strict-Transport-Security', 'max-age=63072000; includeSubDomains; preload');
  r.headers.set('Referrer-Policy', 'strict-origin-when-cross-origin');
  r.headers.set('Permissions-Policy', 'camera=(), microphone=(), geolocation=()');
  return r;
}

function isDev(env: Env): boolean {
  return (env.ENVIRONMENT as string) === 'development';
}

function jsonError(code: string, message: string, status: number): Response {
  return new Response(JSON.stringify({ error: code, message }), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}
