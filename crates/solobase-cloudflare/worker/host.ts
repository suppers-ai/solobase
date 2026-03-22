// WIT runtime host implementation for the TypeScript Cloudflare Worker.
//
// Implements the wafer:block-world/runtime@0.2.0 interface that jco-transpiled
// WASM blocks import. Each service (D1, R2, crypto, config, network, logger)
// is handled by a dedicated handler in ./services/.
//
// IMPORTANT — Async bridge:
// The WIT `call-block` function is synchronous from the WASM block's
// perspective. However, D1/R2/fetch are all async in JavaScript.
//
// For the initial implementation, we use a two-phase approach:
// 1. WASM blocks call `callBlock()` which is synchronous.
// 2. For Cloudflare Workers with JSPI support, the jco transpile uses
//    `--async-mode=jspi` to bridge sync WASM <-> async JS imports.
// 3. If JSPI is not available, an alternative is to pre-compute all
//    needed data before entering WASM, or use Asyncify-compiled blocks.
//
// The service handlers are written as async functions. The `createHost()`
// function returns an `AsyncRuntimeHost` — the caller (dispatch.ts) is
// responsible for wiring this into the jco instantiation with the
// appropriate async bridge.

import type {
  Env,
  TenantConfig,
  Message,
  BlockResult,
} from './types';

// ---------------------------------------------------------------------------
// Host interface
// ---------------------------------------------------------------------------

/**
 * The runtime host provides the `callBlock` function that WASM blocks
 * use to invoke service blocks (database, storage, crypto, etc.).
 *
 * The async variant is used at the TS layer; the sync variant is what
 * the WASM import signature expects (bridged via JSPI/Asyncify).
 */
export interface RuntimeHost {
  callBlock(blockName: string, msg: Message): Promise<BlockResult>;
}

// ---------------------------------------------------------------------------
// Host factory
// ---------------------------------------------------------------------------

/**
 * Create a RuntimeHost wired to the given tenant's Cloudflare bindings.
 *
 * @param env       Cloudflare Worker environment bindings
 * @param tenant    Resolved tenant configuration
 * @param db        The tenant's D1Database binding
 */
export function createHost(
  env: Env,
  tenant: TenantConfig,
  db: D1Database,
): RuntimeHost {
  const envVars = buildEnvVars(env, tenant);
  const jwtSecret = envVars['JWT_SECRET'] ?? '';

  return {
    async callBlock(blockName: string, msg: Message): Promise<BlockResult> {
      // Normalize block name: strip leading '@' and org prefix variations
      const name = normalizeBlockName(blockName);

      switch (name) {
        case 'wafer-run/database':
        case 'wafer-run/d1':
        case 'db': {
          const { d1Handler } = await import('./services/d1');
          return d1Handler(db, msg);
        }

        case 'wafer-run/storage':
        case 'wafer-run/r2':
        case 'storage': {
          const { r2Handler } = await import('./services/r2');
          return r2Handler(env.STORAGE, tenant.id, msg, db, tenant);
        }

        case 'wafer-run/crypto': {
          const { cryptoHandler } = await import('./services/crypto');
          return cryptoHandler(jwtSecret, msg);
        }

        case 'wafer-run/config': {
          const { configHandler } = await import('./services/config');
          return configHandler(envVars, msg);
        }

        case 'wafer-run/network': {
          const { networkHandler } = await import('./services/network');
          return networkHandler(msg);
        }

        case 'wafer-run/logger': {
          const { loggerHandler } = await import('./services/logger');
          return loggerHandler(msg);
        }

        default:
          return {
            action: 'error',
            error: {
              code: 'not-found',
              message: `block '${blockName}' not found`,
              meta: [],
            },
          };
      }
    },
  };
}

// ---------------------------------------------------------------------------
// Env vars builder (mirrors Rust lib.rs env_vars map)
// ---------------------------------------------------------------------------

const ENV_KEYS = [
  'JWT_SECRET',
  'STRIPE_SECRET_KEY',
  'STRIPE_WEBHOOK_SECRET',
  'STRIPE_PRICE_STARTER',
  'STRIPE_PRICE_PRO',
  'SITE_NAME',
  'SITE_URL',
  'ADMIN_EMAIL',
  'STORAGE_MAX_FILE_SIZE',
  'STORAGE_QUOTA_MB',
  'CONTROL_PLANE_URL',
  'CONTROL_PLANE_SECRET',
] as const;

function buildEnvVars(env: Env, _tenant: TenantConfig): Record<string, string> {
  const vars: Record<string, string> = {};

  for (const key of ENV_KEYS) {
    const val = env[key];
    if (typeof val === 'string' && val.length > 0) {
      vars[key] = val;
    }
  }

  return vars;
}

// ---------------------------------------------------------------------------
// Block name normalization
// ---------------------------------------------------------------------------

/**
 * Normalize block names to their canonical short form.
 *
 * Input formats:
 *   "@wafer-run/wafer-run/database" -> "wafer-run/database"
 *   "wafer-run/database"            -> "wafer-run/database"
 *   "db"                            -> "db"
 */
function normalizeBlockName(name: string): string {
  // Strip leading '@'
  let n = name.startsWith('@') ? name.substring(1) : name;

  // "@wafer-run/wafer-run/X" -> "wafer-run/X"
  if (n.startsWith('wafer-run/wafer-run/')) {
    n = 'wafer-run/' + n.substring('wafer-run/wafer-run/'.length);
  }

  // "@suppers-ai/solobase/X" -> "suppers-ai/X"
  if (n.startsWith('suppers-ai/solobase/')) {
    n = 'suppers-ai/' + n.substring('suppers-ai/solobase/'.length);
  }

  return n;
}
