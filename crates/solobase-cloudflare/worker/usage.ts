/**
 * Tenant usage tracking and plan limit enforcement.
 *
 * Tracks API request counts per tenant per month in D1.
 * Checks limits before dispatching to blocks.
 *
 * Table: tenant_usage
 *   - tenant_id TEXT
 *   - month TEXT (YYYY-MM)
 *   - requests INTEGER
 *   - r2_bytes INTEGER
 *   - addon_requests INTEGER (purchased add-on capacity)
 *   - addon_r2_bytes INTEGER
 *   - addon_d1_bytes INTEGER
 */

import type { TenantConfig, PlanLimits } from './types';
import { getPlanLimits } from './types';

/**
 * Result of a usage check. Either:
 * - allowed (no error, optional warning)
 * - blocked (error message, request should be rejected)
 */
export interface UsageCheckResult {
  /** If set, the request should be rejected with 429. */
  error: string | null;
  /** If set, add as X-Solobase-Warning header (request still succeeds). */
  warning: string | null;
}

/**
 * Check if the tenant is within their plan limits and increment the request counter.
 */
export async function checkAndIncrementUsage(
  db: D1Database,
  tenant: TenantConfig,
): Promise<UsageCheckResult> {
  const ok: UsageCheckResult = { error: null, warning: null };

  // Skip checks for platform
  if (tenant.plan === 'platform') return ok;

  const limits = getPlanLimits(tenant.plan);
  const month = currentMonth();

  // Check subscription status if tenant has an owner
  if (tenant.owner_user_id) {
    const sub = await db.prepare(
      'SELECT status, grace_period_end FROM subscriptions WHERE user_id = ? LIMIT 1'
    ).bind(tenant.owner_user_id).first<{ status: string; grace_period_end: string | null }>();

    if (sub) {
      if (sub.status === 'cancelled' || sub.status === 'suspended') {
        return { error: 'Subscription inactive. Please resubscribe at solobase.dev/pricing/', warning: null };
      }
      if (sub.status === 'past_due') {
        if (sub.grace_period_end && new Date(sub.grace_period_end) < new Date()) {
          return { error: 'Payment overdue. Service suspended. Please update payment at app.solobase.dev/blocks/dashboard/', warning: null };
        }
        // Within grace period — allow but warn
        const daysLeft = sub.grace_period_end
          ? Math.max(0, Math.ceil((new Date(sub.grace_period_end).getTime() - Date.now()) / 86400000))
          : 0;
        ok.warning = `Payment failed. ${daysLeft} days remaining before service suspension. Update payment at app.solobase.dev/blocks/dashboard/`;
      }
    }
  }

  // Upsert and increment in a single query
  await db.prepare(
    `INSERT INTO tenant_usage (id, tenant_id, month, requests, r2_bytes, addon_requests, addon_r2_bytes, addon_d1_bytes)
     VALUES (?, ?, ?, 1, 0, 0, 0, 0)
     ON CONFLICT (tenant_id, month) DO UPDATE SET requests = requests + 1`
  ).bind(
    `${tenant.id}:${month}`, tenant.id, month
  ).run();

  // Read the current count
  const row = await db.prepare(
    `SELECT requests, addon_requests FROM tenant_usage WHERE tenant_id = ? AND month = ?`
  ).bind(tenant.id, month).first<{ requests: number; addon_requests: number }>();

  if (!row) return ok;

  const maxRequests = limits.maxRequestsPerMonth + (row.addon_requests ?? 0);
  if (row.requests > maxRequests) {
    return {
      error: `Plan limit exceeded: ${row.requests.toLocaleString()} / ${maxRequests.toLocaleString()} requests this month. Upgrade your plan or add more requests.`,
      warning: null,
    };
  }

  // Warn at 80% usage
  const usagePct = row.requests / maxRequests;
  if (usagePct >= 0.8 && !ok.warning) {
    const pct = Math.round(usagePct * 100);
    ok.warning = `${pct}% of monthly API requests used (${row.requests.toLocaleString()} / ${maxRequests.toLocaleString()})`;
  }

  return ok;
}

/**
 * Check R2 storage usage before allowing an upload.
 */
export async function checkStorageLimit(
  db: D1Database,
  tenant: TenantConfig,
  additionalBytes: number,
): Promise<string | null> {
  const limits = getPlanLimits(tenant.plan);
  const month = currentMonth();

  // Get current R2 usage from storage_objects table
  const row = await db.prepare(
    `SELECT COALESCE(SUM(size), 0) as total_bytes FROM storage_objects`
  ).first<{ total_bytes: number }>();

  const usageRow = await db.prepare(
    `SELECT addon_r2_bytes FROM tenant_usage WHERE tenant_id = ? AND month = ?`
  ).bind(tenant.id, month).first<{ addon_r2_bytes: number }>();

  const currentBytes = row?.total_bytes ?? 0;
  const addonBytes = usageRow?.addon_r2_bytes ?? 0;
  const maxBytes = limits.maxR2StorageBytes + addonBytes;

  if (currentBytes + additionalBytes > maxBytes) {
    const usedMB = Math.round(currentBytes / 1024 / 1024);
    const maxMB = Math.round(maxBytes / 1024 / 1024);
    return `Storage limit exceeded: ${usedMB}MB / ${maxMB}MB. Upgrade your plan or add more storage.`;
  }

  return null;
}

/**
 * Get current usage summary for a tenant (for dashboard display).
 */
export async function getUsageSummary(
  db: D1Database,
  tenant: TenantConfig,
): Promise<{
  plan: string;
  month: string;
  requests: { used: number; limit: number };
  r2Storage: { usedBytes: number; limitBytes: number };
  d1Storage: { usedBytes: number; limitBytes: number };
}> {
  const limits = getPlanLimits(tenant.plan);
  const month = currentMonth();

  const usage = await db.prepare(
    `SELECT requests, addon_requests, addon_r2_bytes, addon_d1_bytes
     FROM tenant_usage WHERE tenant_id = ? AND month = ?`
  ).bind(tenant.id, month).first<{
    requests: number;
    addon_requests: number;
    addon_r2_bytes: number;
    addon_d1_bytes: number;
  }>();

  const r2 = await db.prepare(
    `SELECT COALESCE(SUM(size), 0) as total FROM storage_objects`
  ).first<{ total: number }>();

  return {
    plan: tenant.plan,
    month,
    requests: {
      used: usage?.requests ?? 0,
      limit: limits.maxRequestsPerMonth + (usage?.addon_requests ?? 0),
    },
    r2Storage: {
      usedBytes: r2?.total ?? 0,
      limitBytes: limits.maxR2StorageBytes + (usage?.addon_r2_bytes ?? 0),
    },
    d1Storage: {
      usedBytes: 0, // D1 doesn't expose per-database size easily
      limitBytes: limits.maxD1StorageBytes + (usage?.addon_d1_bytes ?? 0),
    },
  };
}

function currentMonth(): string {
  const d = new Date();
  return `${d.getUTCFullYear()}-${String(d.getUTCMonth() + 1).padStart(2, '0')}`;
}
