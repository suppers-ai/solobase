// TypeScript-native deployments block handler (for testing without WASM).
// Calls host.callBlock() for database operations — same pattern as WASM blocks.

import type { Message, BlockResult } from '../types';
import type { RuntimeHost } from '../host';
import { metaGet } from '../convert';

const COLLECTION = 'block_deployments';

export async function handle(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const action = metaGet(msg.meta, 'req.action') ?? '';
  const path = metaGet(msg.meta, 'req.resource') ?? '';

  // Admin routes
  if (path.startsWith('/admin/b/deployments')) {
    return handleAdmin(msg, host, action, path);
  }

  // User-facing routes
  if (path.startsWith('/b/deployments')) {
    return handleUser(msg, host, action, path);
  }

  return errResult('not-found', 'not found');
}

// ---------------------------------------------------------------------------
// User routes
// ---------------------------------------------------------------------------

async function handleUser(msg: Message, host: RuntimeHost, action: string, path: string): Promise<BlockResult> {
  switch (true) {
    case action === 'retrieve' && path === '/b/deployments':
      return handleList(msg, host);
    case action === 'retrieve' && path.startsWith('/b/deployments/'):
      return handleGet(msg, host);
    case action === 'create' && path === '/b/deployments':
      return handleCreate(msg, host);
    case action === 'update' && path.startsWith('/b/deployments/'):
      return handleUpdate(msg, host);
    case action === 'delete' && path.startsWith('/b/deployments/'):
      return handleDelete(msg, host);
    default:
      return errResult('not-found', 'not found');
  }
}

// ---------------------------------------------------------------------------
// User: list own deployments
// ---------------------------------------------------------------------------

async function handleList(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  if (!userId) return errResult('permission-denied', 'Authentication required');

  const { page, limit, offset } = paginationParams(msg, 20);

  const result = await callService(host, 'wafer-run/database', 'database.paginated_list', {
    collection: COLLECTION,
    page,
    page_size: limit,
    filters: [{ field: 'user_id', operator: 'eq', value: userId }],
    sort: [{ field: 'created_at', desc: true }],
  });

  if (!result) return errResult('internal', 'Database error');
  return jsonRespond(msg, result);
}

// ---------------------------------------------------------------------------
// User: get own deployment
// ---------------------------------------------------------------------------

async function handleGet(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  if (!userId) return errResult('permission-denied', 'Authentication required');

  const id = extractId(msg, '/b/deployments/');
  if (!id) return errResult('invalid-argument', 'Missing deployment ID');

  const record = await dbGet(host, COLLECTION, id);
  if (!record) return errResult('not-found', 'Deployment not found');

  // Verify ownership
  if (record.data?.user_id !== userId) {
    return errResult('not-found', 'Deployment not found');
  }

  return jsonRespond(msg, record);
}

// ---------------------------------------------------------------------------
// User: create deployment
// ---------------------------------------------------------------------------

async function handleCreate(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  if (!userId) return errResult('permission-denied', 'Authentication required');

  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  const name = typeof body.name === 'string' ? body.name.trim() : '';
  if (!name) return errResult('invalid-argument', 'Name is required');
  if (name.length > 100) return errResult('invalid-argument', 'Name must be 100 characters or fewer');

  const slug = name.toLowerCase().replace(/[^a-z0-9]/g, '-');
  const now = new Date().toISOString();

  const data: Record<string, unknown> = {
    user_id: userId,
    name,
    slug,
    status: 'pending',
    created_at: now,
    updated_at: now,
  };
  if (body.config !== undefined) data.config = body.config;
  if (body.plan_id !== undefined) data.plan_id = body.plan_id;
  if (body.purchase_id !== undefined) data.purchase_id = body.purchase_id;

  const record = await dbCreate(host, COLLECTION, data);
  if (!record) return errResult('internal', 'Database error');

  // Provision tenant on the control plane (best-effort)
  const plan = typeof body.plan_id === 'string' ? body.plan_id : 'hobby';
  const provisionResult = await callService(host, 'wafer-run/network', 'network.control_plane_request', {
    method: 'POST',
    path: '/_control/tenants',
    body: { subdomain: slug, plan },
  });

  if (provisionResult && provisionResult.status_code && provisionResult.status_code < 300) {
    const updateData: Record<string, unknown> = {
      status: 'active',
      updated_at: new Date().toISOString(),
    };
    if (provisionResult.body?.id) updateData.tenant_id = provisionResult.body.id;
    if (provisionResult.body?.subdomain) updateData.subdomain = provisionResult.body.subdomain;

    const updated = await dbUpdate(host, COLLECTION, record.id, updateData);
    return jsonRespond(msg, updated ?? record);
  } else if (provisionResult && provisionResult.status_code && provisionResult.status_code >= 300) {
    const errorMsg = provisionResult.body?.error ?? 'Provisioning failed';
    await dbUpdate(host, COLLECTION, record.id, {
      status: 'failed',
      provision_error: `HTTP ${provisionResult.status_code}: ${errorMsg}`,
      updated_at: new Date().toISOString(),
    });
    return errResult('internal', `Provisioning failed: ${errorMsg}`);
  }

  // Control plane unreachable — return record in pending status
  return jsonRespond(msg, record);
}

// ---------------------------------------------------------------------------
// User: update own deployment
// ---------------------------------------------------------------------------

async function handleUpdate(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  if (!userId) return errResult('permission-denied', 'Authentication required');

  const id = extractId(msg, '/b/deployments/');
  if (!id) return errResult('invalid-argument', 'Missing deployment ID');

  // Verify ownership
  const existing = await dbGet(host, COLLECTION, id);
  if (!existing) return errResult('not-found', 'Deployment not found');
  if (existing.data?.user_id !== userId) return errResult('not-found', 'Deployment not found');

  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  // Users cannot change status or user_id directly
  delete body.status;
  delete body.user_id;
  body.updated_at = new Date().toISOString();

  const record = await dbUpdate(host, COLLECTION, id, body);
  if (!record) return errResult('not-found', 'Deployment not found');
  return jsonRespond(msg, record);
}

// ---------------------------------------------------------------------------
// User: delete own deployment (soft delete)
// ---------------------------------------------------------------------------

async function handleDelete(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  if (!userId) return errResult('permission-denied', 'Authentication required');

  const id = extractId(msg, '/b/deployments/');
  if (!id) return errResult('invalid-argument', 'Missing deployment ID');

  // Verify ownership
  const existing = await dbGet(host, COLLECTION, id);
  if (!existing) return errResult('not-found', 'Deployment not found');
  if (existing.data?.user_id !== userId) return errResult('not-found', 'Deployment not found');

  // Deprovision tenant on the control plane (best-effort)
  const subdomain = existing.data?.subdomain || existing.data?.slug || '';
  if (subdomain) {
    const deprovisionResult = await callService(host, 'wafer-run/network', 'network.control_plane_request', {
      method: 'DELETE',
      path: `/_control/tenants/${subdomain}`,
    });
    if (!deprovisionResult || (deprovisionResult.status_code && deprovisionResult.status_code >= 300 && deprovisionResult.status_code !== 404)) {
      // Log error but don't block deletion
      await dbUpdate(host, COLLECTION, id, {
        deprovision_error: deprovisionResult?.body?.error ?? 'Deprovision request failed',
        updated_at: new Date().toISOString(),
      });
    }
  }

  // Soft delete
  const now = new Date().toISOString();
  const record = await dbUpdate(host, COLLECTION, id, {
    status: 'deleted',
    deleted_at: now,
    updated_at: now,
  });

  if (!record) return errResult('internal', 'Database error');
  return jsonRespond(msg, record);
}

// ---------------------------------------------------------------------------
// Admin routes
// ---------------------------------------------------------------------------

async function handleAdmin(msg: Message, host: RuntimeHost, action: string, path: string): Promise<BlockResult> {
  switch (true) {
    case action === 'retrieve' && path === '/admin/b/deployments':
      return handleAdminList(msg, host);
    case action === 'retrieve' && path.startsWith('/admin/b/deployments/'):
      return handleAdminGet(msg, host);
    case action === 'update' && path.startsWith('/admin/b/deployments/'):
      return handleAdminUpdate(msg, host);
    default:
      return errResult('not-found', 'not found');
  }
}

// ---------------------------------------------------------------------------
// Admin: list all deployments (with optional filters)
// ---------------------------------------------------------------------------

async function handleAdminList(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const { page, limit } = paginationParams(msg, 20);

  const filters: { field: string; operator: string; value: string }[] = [];
  const filterUserId = metaGet(msg.meta, 'req.query.user_id') ?? '';
  if (filterUserId) {
    filters.push({ field: 'user_id', operator: 'eq', value: filterUserId });
  }
  const filterStatus = metaGet(msg.meta, 'req.query.status') ?? '';
  if (filterStatus) {
    filters.push({ field: 'status', operator: 'eq', value: filterStatus });
  }

  const result = await callService(host, 'wafer-run/database', 'database.paginated_list', {
    collection: COLLECTION,
    page,
    page_size: limit,
    filters,
    sort: [{ field: 'created_at', desc: true }],
  });

  if (!result) return errResult('internal', 'Database error');
  return jsonRespond(msg, result);
}

// ---------------------------------------------------------------------------
// Admin: get any deployment
// ---------------------------------------------------------------------------

async function handleAdminGet(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const id = extractId(msg, '/admin/b/deployments/');
  if (!id) return errResult('invalid-argument', 'Missing deployment ID');

  const record = await dbGet(host, COLLECTION, id);
  if (!record) return errResult('not-found', 'Deployment not found');
  return jsonRespond(msg, record);
}

// ---------------------------------------------------------------------------
// Admin: update any deployment
// ---------------------------------------------------------------------------

async function handleAdminUpdate(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const id = extractId(msg, '/admin/b/deployments/');
  if (!id) return errResult('invalid-argument', 'Missing deployment ID');

  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  body.updated_at = new Date().toISOString();

  const record = await dbUpdate(host, COLLECTION, id, body);
  if (!record) return errResult('not-found', 'Deployment not found');
  return jsonRespond(msg, record);
}

// ---------------------------------------------------------------------------
// Service call helpers
// ---------------------------------------------------------------------------

async function callService(host: RuntimeHost, block: string, kind: string, data: unknown): Promise<any> {
  const result = await host.callBlock(block, {
    kind, data: new TextEncoder().encode(JSON.stringify(data)), meta: [],
  });
  if (result.action !== 'respond' || !result.response) return null;
  try { return JSON.parse(new TextDecoder().decode(result.response.data)); } catch { return null; }
}

async function dbGet(host: RuntimeHost, collection: string, id: string): Promise<{ id: string; data: Record<string, any> } | null> {
  return callService(host, 'wafer-run/database', 'database.get', { collection, id });
}

async function dbCreate(host: RuntimeHost, collection: string, data: Record<string, unknown>): Promise<{ id: string; data: Record<string, any> } | null> {
  return callService(host, 'wafer-run/database', 'database.create', { collection, data });
}

async function dbUpdate(host: RuntimeHost, collection: string, id: string, data: Record<string, unknown>): Promise<{ id: string; data: Record<string, any> } | null> {
  return callService(host, 'wafer-run/database', 'database.update', { collection, id, data });
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function extractId(msg: Message, prefix: string): string {
  const path = metaGet(msg.meta, 'req.resource') ?? '';
  if (!path.startsWith(prefix)) return '';
  return path.substring(prefix.length).split('/')[0] || '';
}

function paginationParams(msg: Message, defaultLimit: number): { page: number; limit: number; offset: number } {
  const page = parseInt(metaGet(msg.meta, 'req.query.page') ?? '1', 10) || 1;
  const limit = parseInt(metaGet(msg.meta, 'req.query.page_size') ?? String(defaultLimit), 10) || defaultLimit;
  const offset = (page - 1) * limit;
  return { page, limit, offset };
}

function parseBody<T>(msg: Message): T | null {
  try { return JSON.parse(new TextDecoder().decode(msg.data)) as T; } catch { return null; }
}

function jsonRespond(msg: Message, data: unknown, status?: number): BlockResult {
  const meta = [{ key: 'resp.content_type', value: 'application/json' }];
  if (status) meta.push({ key: 'resp.status', value: String(status) });
  return {
    action: 'respond',
    response: { data: new TextEncoder().encode(JSON.stringify(data)), meta },
    message: msg,
  };
}

function errResult(code: string, message: string): BlockResult {
  return { action: 'error', error: { code: code as any, message, meta: [] } };
}
