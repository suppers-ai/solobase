// TypeScript-native system block handler.

import type { Message, BlockResult, TenantConfig } from '../types';
import type { RuntimeHost } from '../host';
import { metaGet } from '../convert';
import { getUsageSummary } from '../usage';

export function handle(msg: Message, host?: RuntimeHost): BlockResult | Promise<BlockResult> {
  const path = metaGet(msg.meta, 'req.resource') ?? '/';

  if (path === '/health') {
    return jsonRespond(msg, { status: 'ok' });
  }

  if (path === '/debug/time') {
    const now = new Date();
    return jsonRespond(msg, {
      utc: now.toISOString(),
      unix: Math.floor(now.getTime() / 1000),
      unix_ms: now.getTime(),
    });
  }

  if (path === '/nav') {
    return jsonRespond(msg, [
      { id: 'dashboard', title: 'Dashboard', href: '/admin', icon: 'LayoutDashboard' },
      { id: 'users', title: 'Users', href: '/admin/users', icon: 'Users' },
      { id: 'database', title: 'Database', href: '/admin/database', icon: 'Database' },
      { id: 'iam', title: 'IAM', href: '/admin/iam', icon: 'Shield' },
      { id: 'logs', title: 'Logs', href: '/admin/logs', icon: 'FileText' },
      { id: 'settings', title: 'Settings', href: '/admin/settings', icon: 'Settings' },
    ]);
  }

  if (path === '/b/usage') {
    return handleUsage(msg, host);
  }

  return { action: 'error', error: { code: 'not-found', message: 'not found', meta: [] } };
}

async function handleUsage(msg: Message, host?: RuntimeHost): Promise<BlockResult> {
  // Usage is only available for tenant API requests (not platform)
  // We need access to the D1 database via the host
  if (!host) {
    return jsonRespond(msg, { error: 'usage not available' });
  }

  // Get tenant info from dispatch context — passed via meta by the worker
  // For now, return a basic response; the actual usage query needs the DB directly
  // which we'll access through a special service call
  try {
    const result = await host.callBlock('wafer-run/database', {
      kind: 'database.query_raw',
      data: new TextEncoder().encode(JSON.stringify({
        query: 'SELECT requests, addon_requests, addon_r2_bytes, addon_d1_bytes FROM tenant_usage WHERE month = ? ORDER BY requests DESC LIMIT 1',
        args: [currentMonth()],
      })),
      meta: [],
    });

    let usage = { requests: 0, addon_requests: 0, addon_r2_bytes: 0, addon_d1_bytes: 0 };
    if (result.action === 'respond' && result.response) {
      const records = JSON.parse(new TextDecoder().decode(result.response.data));
      if (Array.isArray(records) && records.length > 0) {
        const r = records[0].data || records[0];
        usage = {
          requests: r.requests ?? 0,
          addon_requests: r.addon_requests ?? 0,
          addon_r2_bytes: r.addon_r2_bytes ?? 0,
          addon_d1_bytes: r.addon_d1_bytes ?? 0,
        };
      }
    }

    // Get storage usage
    const storageResult = await host.callBlock('wafer-run/database', {
      kind: 'database.query_raw',
      data: new TextEncoder().encode(JSON.stringify({
        query: 'SELECT COALESCE(SUM(size), 0) as total FROM storage_objects',
        args: [],
      })),
      meta: [],
    });

    let storageBytes = 0;
    if (storageResult.action === 'respond' && storageResult.response) {
      const records = JSON.parse(new TextDecoder().decode(storageResult.response.data));
      if (Array.isArray(records) && records.length > 0) {
        storageBytes = (records[0].data || records[0]).total ?? 0;
      }
    }

    return jsonRespond(msg, {
      month: currentMonth(),
      requests: {
        used: usage.requests,
        addon: usage.addon_requests,
      },
      storage: {
        r2_bytes: storageBytes,
        r2_addon_bytes: usage.addon_r2_bytes,
        d1_addon_bytes: usage.addon_d1_bytes,
      },
    });
  } catch {
    return jsonRespond(msg, { error: 'failed to fetch usage' });
  }
}

function currentMonth(): string {
  const d = new Date();
  return `${d.getUTCFullYear()}-${String(d.getUTCMonth() + 1).padStart(2, '0')}`;
}

function jsonRespond(msg: Message, data: unknown): BlockResult {
  return {
    action: 'respond',
    response: {
      data: new TextEncoder().encode(JSON.stringify(data)),
      meta: [{ key: 'resp.content_type', value: 'application/json' }],
    },
    message: msg,
  };
}
