// Block routing and dispatch for the TypeScript Cloudflare Worker.
//
// Replaces the Rust SolobaseRouterBlock + solobase-core pipeline.
// Handles JWT validation, feature gates, and routing to the correct
// jco-transpiled WASM block's handle() function.

import type {
  Env,
  TenantConfig,
  Message,
  BlockResult,
  ErrorCode,
  Block,
} from './types';
import type { RuntimeHost } from './host';
import { META, metaGet, metaSet } from './convert';
import { isFeatureEnabled } from './tenant';

// ---------------------------------------------------------------------------
// Block identifiers (mirrors solobase-core/src/routing.rs BlockId)
// ---------------------------------------------------------------------------

type BlockId =
  | 'system'
  | 'auth'
  | 'admin'
  | 'files'
  | 'legalpages'
  | 'products'
  | 'deployments'
  | 'userportal'
  | 'profile';

// ---------------------------------------------------------------------------
// Route table (mirrors solobase-core/src/routing.rs ROUTES)
// ---------------------------------------------------------------------------

interface Route {
  prefix: string;
  requiresAdmin: boolean;
  blockId: BlockId;
  /** The feature name checked against TenantAppConfig. */
  feature: string;
}

const ROUTES: Route[] = [
  // System (always enabled)
  { prefix: '/health', requiresAdmin: false, blockId: 'system', feature: 'system' },
  { prefix: '/nav', requiresAdmin: false, blockId: 'system', feature: 'system' },
  { prefix: '/debug/', requiresAdmin: false, blockId: 'system', feature: 'system' },
  // Auth
  { prefix: '/auth/', requiresAdmin: false, blockId: 'auth', feature: 'auth' },
  { prefix: '/internal/oauth/', requiresAdmin: false, blockId: 'auth', feature: 'auth' },
  // Admin sub-routes (more specific before general)
  { prefix: '/admin/settings/', requiresAdmin: true, blockId: 'admin', feature: 'admin' },
  { prefix: '/settings/', requiresAdmin: true, blockId: 'admin', feature: 'admin' },
  { prefix: '/admin/storage/', requiresAdmin: true, blockId: 'files', feature: 'files' },
  { prefix: '/admin/b/cloudstorage/', requiresAdmin: true, blockId: 'files', feature: 'files' },
  { prefix: '/admin/legalpages/', requiresAdmin: true, blockId: 'legalpages', feature: 'legalpages' },
  { prefix: '/admin/b/products', requiresAdmin: true, blockId: 'products', feature: 'products' },
  { prefix: '/admin/b/deployments', requiresAdmin: true, blockId: 'deployments', feature: 'deployments' },
  { prefix: '/admin/', requiresAdmin: true, blockId: 'admin', feature: 'admin' },
  // Non-admin feature routes
  { prefix: '/storage/', requiresAdmin: false, blockId: 'files', feature: 'files' },
  { prefix: '/b/cloudstorage/', requiresAdmin: false, blockId: 'files', feature: 'files' },
  { prefix: '/b/products', requiresAdmin: false, blockId: 'products', feature: 'products' },
  { prefix: '/b/legalpages', requiresAdmin: false, blockId: 'legalpages', feature: 'legalpages' },
  { prefix: '/b/deployments', requiresAdmin: false, blockId: 'deployments', feature: 'deployments' },
  { prefix: '/b/userportal', requiresAdmin: false, blockId: 'userportal', feature: 'userportal' },
  { prefix: '/b/usage', requiresAdmin: false, blockId: 'system', feature: 'system' },
  { prefix: '/profile', requiresAdmin: false, blockId: 'profile', feature: 'profile' },
];

// ---------------------------------------------------------------------------
// Public routes (no JWT required)
// ---------------------------------------------------------------------------

const PUBLIC_PREFIXES: string[] = [
  '/health',
  '/nav',
  '/auth/login',
  '/auth/signup',
  '/auth/refresh',
  '/auth/oauth/',
  '/internal/oauth/',
  '/b/legalpages/documents/',
  '/b/legalpages/terms',
  '/b/legalpages/privacy',
  '/storage/direct/',
  '/b/products/webhooks',
  '/auth/forgot-password',
  '/auth/reset-password',
  '/auth/verify-email',
  // /debug/* is NOT public — requires authentication in production
];

function isPublicRoute(path: string): boolean {
  return PUBLIC_PREFIXES.some(
    (prefix) => path === prefix || path.startsWith(prefix),
  );
}

// ---------------------------------------------------------------------------
// Main dispatch
// ---------------------------------------------------------------------------

/**
 * Route a WAFER Message to the appropriate solobase block.
 *
 * Steps:
 * 1. Strip /api prefix (CF convention)
 * 2. Validate JWT from Authorization header or auth_token cookie
 * 3. Match path against route table
 * 4. Check feature gate
 * 5. Check admin gate
 * 6. Load and call the jco-transpiled WASM block
 */
export async function dispatchToBlock(
  msg: Message,
  tenant: TenantConfig,
  host: RuntimeHost,
  env: Env,
): Promise<BlockResult> {
  // 1. Get the resource path (already stripped of /api prefix by index.ts)
  const resource = getPath(msg);

  // 2. Validate JWT (unless public route)
  const authHeader = resolveAuthHeader(msg);
  if (authHeader) {
    await extractAuthMeta(authHeader, host, msg);
  }

  if (!isPublicRoute(resource) && !metaGet(msg.meta, META.AUTH_USER_ID)) {
    return errorResult('unauthenticated', 'authentication required');
  }

  // 3. Match route
  const route = matchRoute(resource);
  if (!route) {
    return errorResult('not-found', 'endpoint not found');
  }

  // 4. Feature gate
  if (!isFeatureEnabled(tenant.config, route.feature)) {
    return errorResult('not-found', 'endpoint not found');
  }

  // 5. Admin gate
  if (route.requiresAdmin) {
    const roles = metaGet(msg.meta, META.AUTH_USER_ROLES) ?? '';
    const isAdmin = roles.split(',').some((r) => r.trim() === 'admin');
    if (!isAdmin) {
      return errorResult('permission-denied', 'admin access required');
    }
  }

  // 6. Load and call the block
  return callBlock(route.blockId, msg, host, env);
}

// ---------------------------------------------------------------------------
// Route matching
// ---------------------------------------------------------------------------

function matchRoute(path: string): Route | null {
  for (const route of ROUTES) {
    if (path === route.prefix || path.startsWith(route.prefix)) {
      return route;
    }
  }
  return null;
}

// ---------------------------------------------------------------------------
// Auth resolution (mirrors SolobaseRouterBlock::handle in router.rs)
// ---------------------------------------------------------------------------

/**
 * Extract the Authorization header value from either:
 * - `Authorization: Bearer <token>` header
 * - `auth_token` cookie
 */
function resolveAuthHeader(msg: Message): string | null {
  // Check Authorization header
  const authHeader = metaGet(msg.meta, 'http.header.authorization');
  if (authHeader && authHeader.length > 0) {
    return authHeader;
  }

  // Check auth_token cookie
  const cookieHeader = metaGet(msg.meta, 'http.header.cookie') ?? '';
  const token = parseCookie(cookieHeader, 'auth_token');
  if (token) {
    return `Bearer ${token}`;
  }

  return null;
}

/**
 * Validate JWT and set auth meta fields on the message.
 *
 * Calls the crypto service's `crypto.verify` to validate the token,
 * then sets `auth.user_id`, `auth.user_email`, and `auth.user_roles`
 * on the message meta. Silently continues if the token is invalid
 * (the request proceeds as unauthenticated).
 */
async function extractAuthMeta(
  authHeader: string,
  host: RuntimeHost,
  msg: Message,
): Promise<void> {
  const token = authHeader.startsWith('Bearer ')
    ? authHeader.substring(7)
    : null;
  if (!token) return;

  // Call crypto.verify via the runtime host
  const verifyMsg: Message = {
    kind: 'crypto.verify',
    data: new TextEncoder().encode(JSON.stringify({ token })),
    meta: [],
  };

  const result = await host.callBlock('wafer-run/crypto', verifyMsg);

  if (result.action !== 'respond' || !result.response) return;

  // Parse claims from response
  let claims: Record<string, unknown>;
  try {
    claims = JSON.parse(new TextDecoder().decode(result.response.data));
    // The crypto service returns { claims: {...} }
    if (claims.claims && typeof claims.claims === 'object') {
      claims = claims.claims as Record<string, unknown>;
    }
  } catch {
    return;
  }

  // Set auth meta
  if (typeof claims.sub === 'string') {
    metaSet(msg.meta, META.AUTH_USER_ID, claims.sub);
  }
  if (typeof claims.email === 'string') {
    metaSet(msg.meta, META.AUTH_USER_EMAIL, claims.email);
  }

  // Roles: check for "roles" array or legacy "role" string
  if (Array.isArray(claims.roles)) {
    const rolesStr = claims.roles
      .filter((r): r is string => typeof r === 'string')
      .join(',');
    metaSet(msg.meta, META.AUTH_USER_ROLES, rolesStr);
  } else if (typeof claims.role === 'string') {
    metaSet(msg.meta, META.AUTH_USER_ROLES, claims.role);
  }
}

// ---------------------------------------------------------------------------
// Block loading and execution
// ---------------------------------------------------------------------------

/**
 * Block module cache. jco-transpiled blocks are lazily loaded and cached
 * for the lifetime of the Worker isolate.
 */
const blockCache = new Map<string, Block>();

/**
 * Load and call a solobase block.
 *
 * For the initial implementation, blocks are loaded as jco-transpiled
 * ES modules from the worker bundle. Each block must export:
 * - `info(): BlockInfo`
 * - `handle(msg: Message): BlockResult`
 * - `lifecycle(event: LifecycleEvent): void`
 *
 * TODO: Support loading blocks from KV (tenant.blocks) for custom WASM blocks.
 */
async function callBlock(
  blockId: BlockId,
  msg: Message,
  host: RuntimeHost,
  _env: Env,
): Promise<BlockResult> {
  // TypeScript-native block handlers (testing mode).
  // These call host.callBlock() for service operations (D1, crypto, etc.),
  // exactly mirroring what the jco-transpiled WASM blocks would do.
  //
  // Once the WASM async bridge (JSPI/Asyncify) is wired, these will be
  // replaced by jco-transpiled block loading:
  //
  //   const mod = await import(`./blocks/${blockId}.js`);
  //   const instance = await mod.instantiate(compileCore, {
  //     'wafer:block-world/runtime@0.2.0': {
  //       callBlock: (name: string, msg: Message) => host.callBlock(name, msg),
  //     },
  //   });
  //   return instance.block.handle(msg);

  switch (blockId) {
    case 'system': {
      const { handle } = await import('./blocks-ts/system');
      return handle(msg, host);
    }
    case 'auth': {
      const { handle } = await import('./blocks-ts/auth');
      return handle(msg, host);
    }
    case 'admin': {
      const { handle } = await import('./blocks-ts/admin');
      return handle(msg, host);
    }
    case 'profile': {
      const { handle } = await import('./blocks-ts/profile');
      return handle(msg);
    }
    case 'userportal': {
      const { handle } = await import('./blocks-ts/userportal');
      return handle(msg, host);
    }
    case 'legalpages': {
      const { handle } = await import('./blocks-ts/legalpages');
      return handle(msg, host);
    }
    case 'deployments': {
      const { handle } = await import('./blocks-ts/deployments');
      return handle(msg, host);
    }
    case 'files': {
      const { handle } = await import('./blocks-ts/files');
      return handle(msg, host);
    }
    case 'products': {
      const { handle } = await import('./blocks-ts/products');
      return handle(msg, host);
    }
    default:
      return errorResult(
        'unimplemented',
        `block '${blockId}' matched but no TS handler is implemented yet`,
      );
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function getPath(msg: Message): string {
  return metaGet(msg.meta, META.REQ_RESOURCE) ?? '/';
}

function parseCookie(cookieHeader: string, name: string): string | null {
  if (!cookieHeader) return null;
  for (const part of cookieHeader.split(';')) {
    const trimmed = part.trim();
    const eqIdx = trimmed.indexOf('=');
    if (eqIdx === -1) continue;
    if (trimmed.substring(0, eqIdx).trim() === name) {
      return trimmed.substring(eqIdx + 1).trim();
    }
  }
  return null;
}

function errorResult(code: ErrorCode, message: string): BlockResult {
  return {
    action: 'error',
    error: { code, message, meta: [] },
  };
}
