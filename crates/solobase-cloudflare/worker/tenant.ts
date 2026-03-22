// Tenant resolution and feature gate logic for the TypeScript Cloudflare Worker.
//
// Ports the logic from src/tenant.rs and src/lib.rs (resolve_tenant) to TypeScript.

import type { Env, TenantConfig, TenantAppConfig } from './types';

// ---------------------------------------------------------------------------
// Tenant resolution
// ---------------------------------------------------------------------------

/**
 * Resolve a TenantConfig from the request hostname.
 *
 * - Extracts the subdomain (first label before the first dot).
 * - localhost / 127.0.0.1 returns a hardcoded dev tenant with all features on.
 * - Otherwise looks up `tenant:{subdomain}:config` in the TENANTS KV namespace.
 */
export async function resolveTenant(
  hostname: string,
  env: Env,
): Promise<TenantConfig | null> {
  // Strip port if present (e.g. "localhost:8787")
  const hostNoPort = hostname.split(':')[0];
  const subdomain = hostNoPort.split('.')[0];
  if (!subdomain) return null;

  // `app` subdomain is the platform — never resolve as a tenant.
  // In production, app.solobase.dev is handled by isMarketingHost() in static.ts.
  // In dev mode, localhost is the platform equivalent.
  const isDev = (env.ENVIRONMENT as string) === 'development';
  if (subdomain === 'app') return null;
  if (isDev && (subdomain === 'localhost' || subdomain === '127' || subdomain === 'www')) {
    return {
      id: 'dev',
      subdomain: 'localhost',
      plan: 'hobby',
      db_binding: 'DB',
      config: allEnabledConfig(),
      blocks: [],
    };
  }

  // Look up tenant config in KV
  const kv = env.TENANTS;
  if (!kv) return null;

  const key = `tenant:${subdomain}:config`;
  const raw = await kv.get(key, 'json');
  if (!raw) return null;

  return raw as TenantConfig;
}

// ---------------------------------------------------------------------------
// Feature gates
// ---------------------------------------------------------------------------

/**
 * Feature name to TenantAppConfig field mapping.
 *
 * A feature is considered enabled when its field is present and truthy:
 * - `{}`, `true`, `{...}` -> enabled
 * - `undefined`, `null`, `false` -> disabled
 *
 * System and Profile are always enabled and do not need a feature gate.
 */
export function isFeatureEnabled(
  config: TenantAppConfig,
  feature: string,
): boolean {
  switch (feature) {
    case 'system':
    case 'profile':
      return true;
    case 'auth':
      return isEnabled(config.auth);
    case 'admin':
      return isEnabled(config.admin);
    case 'files':
      return isEnabled(config.files);
    case 'products':
      return isEnabled(config.products);
    case 'deployments':
      return isEnabled(config.deployments);
    case 'legalpages':
      return isEnabled(config.legalpages);
    case 'userportal':
      return isEnabled(config.userportal);
    default:
      return false;
  }
}

/**
 * Same logic as Rust `features::is_feature_enabled`:
 * None / null / false -> disabled; anything else -> enabled.
 */
function isEnabled(val: unknown): boolean {
  if (val === undefined || val === null || val === false) return false;
  return true;
}

// ---------------------------------------------------------------------------
// D1 binding resolution
// ---------------------------------------------------------------------------

/**
 * Get the D1Database binding for a tenant.
 *
 * Uses the tenant's `db_binding` field if set (e.g. `"DB_myapp"`),
 * otherwise falls back to the shared `DB` binding.
 */
export function getD1ForTenant(env: Env, tenant: TenantConfig): D1Database {
  if (tenant.db_binding) {
    const db = env[tenant.db_binding] as D1Database | undefined;
    if (db) return db;
  }
  return env.DB;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Create a TenantAppConfig with all features enabled (dev / new tenant default). */
function allEnabledConfig(): TenantAppConfig {
  return {
    version: 0,
    auth: {},
    admin: {},
    files: {},
    products: {},
    deployments: {},
    legalpages: {},
    userportal: {},
  };
}
