// Project resolution and feature gate logic for the TypeScript Cloudflare Worker.

import type { Env, ProjectConfig, ProjectAppConfig } from './types';

// ---------------------------------------------------------------------------
// Reserved subdomains — never resolve as a project
// ---------------------------------------------------------------------------

export const RESERVED_SUBDOMAINS = new Set([
  'app', 'admin', 'console', 'cloud', 'www', 'api', 'mail', 'smtp', 'ftp',
  'dashboard', 'billing', 'support', 'help', 'docs', 'blog', 'status',
  'staging', 'dev', 'test', 'demo', 'internal', 'private',
  'ns1', 'ns2', 'mx', 'autoconfig', 'autodiscover',
]);

// ---------------------------------------------------------------------------
// Project resolution
// ---------------------------------------------------------------------------

/**
 * Resolve a ProjectConfig from the request hostname.
 *
 * - Extracts the subdomain (first label before the first dot).
 * - localhost / 127.0.0.1 returns a hardcoded dev project with all features on.
 * - Otherwise looks up `project:{subdomain}:config` in the PROJECTS KV namespace.
 */
export async function resolveProject(
  hostname: string,
  env: Env,
): Promise<ProjectConfig | null> {
  // Strip port if present (e.g. "localhost:8787")
  const hostNoPort = hostname.split(':')[0];
  const subdomain = hostNoPort.split('.')[0];
  if (!subdomain) return null;

  // Reserved subdomains are the platform — never resolve as a project.
  const isDev = (env.ENVIRONMENT as string) === 'development';
  if (RESERVED_SUBDOMAINS.has(subdomain)) return null;
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

  // Look up project config in KV
  const kv = env.PROJECTS;
  if (!kv) return null;

  const key = `project:${subdomain}:config`;
  const raw = await kv.get(key, 'json');
  if (!raw) return null;

  return raw as ProjectConfig;
}

// ---------------------------------------------------------------------------
// Feature gates
// ---------------------------------------------------------------------------

/**
 * Check if a feature is enabled for the project.
 *
 * A feature is considered enabled when its field is present and truthy:
 * - `{}`, `true`, `{...}` -> enabled
 * - `undefined`, `null`, `false` -> disabled
 *
 * System and Profile are always enabled and do not need a feature gate.
 */
export function isFeatureEnabled(
  config: ProjectAppConfig,
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
    case 'projects':
      return isEnabled(config.projects);
    case 'legalpages':
      return isEnabled(config.legalpages);
    case 'userportal':
      return isEnabled(config.userportal);
    default:
      return false;
  }
}

function isEnabled(val: unknown): boolean {
  if (val === undefined || val === null || val === false) return false;
  return true;
}

// ---------------------------------------------------------------------------
// D1 binding resolution
// ---------------------------------------------------------------------------

/**
 * Get the D1Database binding for a project.
 *
 * Uses the project's `db_binding` field if set (e.g. `"DB_myapp"`),
 * otherwise falls back to the shared `DB` binding.
 */
export function getD1ForProject(env: Env, project: ProjectConfig): D1Database {
  if (project.db_binding) {
    const db = env[project.db_binding] as D1Database | undefined;
    if (db) return db;
  }
  return env.DB;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Create a ProjectAppConfig with all features enabled (dev / new project default). */
function allEnabledConfig(): ProjectAppConfig {
  return {
    version: 0,
    auth: {},
    admin: {},
    files: {},
    products: {},
    projects: {},
    legalpages: {},
    userportal: {},
  };
}
