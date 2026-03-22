// TypeScript-native admin block handler (for testing without WASM).
// Calls host.callBlock() for database operations — same pattern as WASM blocks.
//
// This is the largest block, handling: users, database introspection, IAM,
// audit logs, settings, custom tables, and wafer info.

import type { Message, BlockResult } from '../types';
import type { RuntimeHost } from '../host';
import { metaGet } from '../convert';

// ---------------------------------------------------------------------------
// Collection names
// ---------------------------------------------------------------------------

const USERS = 'auth_users';
const ROLES = 'iam_roles';
const PERMISSIONS = 'iam_permissions';
const USER_ROLES = 'iam_user_roles';
const SETTINGS = 'settings';
const AUDIT_LOGS = 'audit_logs';

// ---------------------------------------------------------------------------
// Main handler
// ---------------------------------------------------------------------------

export async function handle(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const action = metaGet(msg.meta, 'req.action') ?? '';
  const path = metaGet(msg.meta, 'req.resource') ?? '';

  if (path.startsWith('/admin/users')) return handleUsers(action, path, msg, host);
  if (path.startsWith('/admin/database')) return handleDatabase(action, path, msg, host);
  if (path.startsWith('/admin/iam')) return handleIam(action, path, msg, host);
  if (path.startsWith('/admin/logs')) return handleLogs(action, path, msg, host);
  if (path.startsWith('/admin/settings') || path.startsWith('/settings')) return handleSettings(action, path, msg, host);
  if (path.startsWith('/admin/wafer')) return handleWaferInfo(action, path);
  if (path.startsWith('/admin/custom-tables')) return handleCustomTables(action, path, msg, host);

  return errResult('not-found', 'not found');
}

// ===========================================================================
// USERS — /admin/users/*
// ===========================================================================

async function handleUsers(action: string, path: string, msg: Message, host: RuntimeHost): Promise<BlockResult> {
  switch (true) {
    case action === 'retrieve' && path === '/admin/users':
      return usersHandleList(msg, host);
    case action === 'retrieve' && path.startsWith('/admin/users/'):
      return usersHandleGet(path, host);
    case action === 'update' && path.startsWith('/admin/users/'):
      return usersHandleUpdate(path, msg, host);
    case action === 'delete' && path.startsWith('/admin/users/'):
      return usersHandleDelete(path, host);
    default:
      return errResult('not-found', 'not found');
  }
}

async function usersHandleList(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const { page, pageSize } = paginationParams(msg, 20);
  const search = queryParam(msg, 'search');

  const filters: Filter[] = [
    { field: 'deleted_at', operator: 'is_null' },
  ];
  if (search) {
    filters.push({ field: 'email', operator: 'like', value: `%${search}%` });
  }

  const result = await dbList(host, USERS, {
    filters,
    sort: [{ field: 'created_at', desc: true }],
    limit: pageSize,
    offset: (page - 1) * pageSize,
  });
  if (!result) return errResult('internal', 'Database error');

  // Strip password hashes
  if (result.records) {
    for (const record of result.records) {
      delete record.data.password_hash;
    }
  }

  return jsonRespond(result);
}

async function usersHandleGet(path: string, host: RuntimeHost): Promise<BlockResult> {
  const id = path.replace('/admin/users/', '');
  if (!id) return errResult('invalid-argument', 'Missing user ID');

  const user = await dbGet(host, USERS, id);
  if (!user) return errResult('not-found', 'User not found');

  delete user.data.password_hash;

  // Get roles for this user
  const rolesResult = await dbList(host, USER_ROLES, {
    filters: [{ field: 'user_id', operator: 'eq', value: id }],
    limit: 100,
    offset: 0,
  });
  const roles = (rolesResult?.records ?? [])
    .map((r: any) => r.data.role as string)
    .filter(Boolean);

  return jsonRespond({ ...user, roles });
}

async function usersHandleUpdate(path: string, msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const id = path.replace('/admin/users/', '');
  if (!id) return errResult('invalid-argument', 'Missing user ID');

  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  // Only allow safe fields
  const data: Record<string, unknown> = {};
  for (const key of ['name', 'disabled', 'avatar_url']) {
    if (body[key] !== undefined) data[key] = body[key];
  }
  data.updated_at = new Date().toISOString();

  const updated = await dbUpdate(host, USERS, id, data);
  if (!updated) return errResult('not-found', 'User not found');

  delete updated.data.password_hash;
  return jsonRespond(updated);
}

async function usersHandleDelete(path: string, host: RuntimeHost): Promise<BlockResult> {
  const id = path.replace('/admin/users/', '');
  if (!id) return errResult('invalid-argument', 'Missing user ID');

  // Soft delete: set deleted_at timestamp
  const now = new Date().toISOString();
  const updated = await dbUpdate(host, USERS, id, { deleted_at: now, updated_at: now });
  if (!updated) return errResult('not-found', 'User not found');

  return jsonRespond({ deleted: true });
}

// ===========================================================================
// DATABASE — /admin/database/*
// ===========================================================================

async function handleDatabase(action: string, path: string, msg: Message, host: RuntimeHost): Promise<BlockResult> {
  switch (true) {
    case action === 'retrieve' && path === '/admin/database/info':
      return dbInfoHandle(host);
    case action === 'retrieve' && path === '/admin/database/tables':
      return dbTablesHandle(host);
    case action === 'retrieve' && path.startsWith('/admin/database/tables/') && path.endsWith('/columns'):
      return dbColumnsHandle(path, host);
    case action === 'create' && path === '/admin/database/query':
      return dbQueryHandle(msg, host);
    default:
      return errResult('not-found', 'not found');
  }
}

async function dbInfoHandle(host: RuntimeHost): Promise<BlockResult> {
  const tables = await dbQueryRaw(host,
    "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    [],
  );
  if (!tables) return errResult('internal', 'Database error');

  const tableNames = tables
    .map((r: any) => r.data?.name ?? r.name)
    .filter(Boolean);

  return jsonRespond({
    type: 'sqlite',
    tables: tableNames,
    table_count: tableNames.length,
  });
}

async function dbTablesHandle(host: RuntimeHost): Promise<BlockResult> {
  const tables = await dbQueryRaw(host,
    "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    [],
  );
  if (!tables) return errResult('internal', 'Database error');

  const tableInfo: { name: string; row_count: number }[] = [];
  for (const table of tables) {
    const name = table.data?.name ?? table.name ?? '';
    if (!name) continue;
    const safeName = sanitizeIdent(name);
    const countResult = await dbQueryRaw(host, `SELECT COUNT(*) as cnt FROM "${safeName}"`, []);
    const count = countResult?.[0]?.data?.cnt ?? countResult?.[0]?.cnt ?? 0;
    tableInfo.push({ name, row_count: Number(count) });
  }

  return jsonRespond(tableInfo);
}

async function dbColumnsHandle(path: string, host: RuntimeHost): Promise<BlockResult> {
  // Extract table name from /admin/database/tables/{name}/columns
  const tableName = path
    .replace('/admin/database/tables/', '')
    .replace(/\/columns$/, '');
  if (!tableName) return errResult('invalid-argument', 'Missing table name');

  const safeName = sanitizeIdent(tableName);
  const columns = await dbQueryRaw(host, `PRAGMA table_info("${safeName}")`, []);
  if (!columns) return errResult('internal', 'Database error');

  const colInfo = columns.map((c: any) => {
    const d = c.data ?? c;
    return {
      name: d.name ?? '',
      type: d.type ?? '',
      notnull: Number(d.notnull ?? 0) === 1,
      pk: Number(d.pk ?? 0) === 1,
      default_value: d.dflt_value ?? null,
    };
  });

  return jsonRespond({ table: tableName, columns: colInfo });
}

async function dbQueryHandle(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<{ query: string; args?: unknown[] }>(msg);
  if (!body || !body.query) return errResult('invalid-argument', 'Invalid body');

  // Only allow read-only queries
  const trimmed = body.query.trim();
  const firstWord = trimmed.split(/\s+/)[0]?.toUpperCase() ?? '';
  if (!['SELECT', 'PRAGMA', 'EXPLAIN'].includes(firstWord)) {
    return errResult('permission-denied', 'Only SELECT, PRAGMA, and EXPLAIN queries are allowed');
  }

  const rows = await dbQueryRaw(host, body.query, body.args ?? []);
  if (!rows) return errResult('invalid-argument', 'Query error');

  return jsonRespond({ rows, row_count: rows.length });
}

// ===========================================================================
// IAM — /admin/iam/*
// ===========================================================================

async function handleIam(action: string, path: string, msg: Message, host: RuntimeHost): Promise<BlockResult> {
  switch (true) {
    // Roles
    case action === 'retrieve' && path === '/admin/iam/roles':
      return iamListRoles(host);
    case action === 'create' && path === '/admin/iam/roles':
      return iamCreateRole(msg, host);
    case action === 'update' && path.startsWith('/admin/iam/roles/'):
      return iamUpdateRole(path, msg, host);
    case action === 'delete' && path.startsWith('/admin/iam/roles/'):
      return iamDeleteRole(path, host);
    // Permissions
    case action === 'retrieve' && path === '/admin/iam/permissions':
      return iamListPermissions(host);
    case action === 'create' && path === '/admin/iam/permissions':
      return iamCreatePermission(msg, host);
    case action === 'delete' && path.startsWith('/admin/iam/permissions/'):
      return iamDeletePermission(path, host);
    // User-role assignments
    case action === 'retrieve' && path === '/admin/iam/user-roles':
      return iamListUserRoles(msg, host);
    case action === 'create' && path === '/admin/iam/user-roles':
      return iamAssignRole(msg, host);
    case action === 'delete' && path.startsWith('/admin/iam/user-roles/'):
      return iamRemoveRole(path, host);
    default:
      return errResult('not-found', 'not found');
  }
}

async function iamListRoles(host: RuntimeHost): Promise<BlockResult> {
  const result = await dbList(host, ROLES, {
    sort: [{ field: 'name', desc: false }],
    limit: 1000,
    offset: 0,
  });
  if (!result) return errResult('internal', 'Database error');
  return jsonRespond(result);
}

async function iamCreateRole(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<{ name: string; description?: string; permissions?: string[] }>(msg);
  if (!body || !body.name) return errResult('invalid-argument', 'Invalid body');

  const record = await dbCreate(host, ROLES, {
    name: body.name,
    description: body.description ?? '',
    permissions: JSON.stringify(body.permissions ?? []),
    is_system: 0,
  });
  if (!record) return errResult('internal', 'Database error');
  return jsonRespond(record);
}

async function iamUpdateRole(path: string, msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const id = path.replace('/admin/iam/roles/', '');
  if (!id) return errResult('invalid-argument', 'Missing role ID');

  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  const data: Record<string, unknown> = {};
  for (const key of ['name', 'description', 'permissions']) {
    if (body[key] !== undefined) {
      data[key] = key === 'permissions' && Array.isArray(body[key])
        ? JSON.stringify(body[key])
        : body[key];
    }
  }
  data.updated_at = new Date().toISOString();

  const updated = await dbUpdate(host, ROLES, id, data);
  if (!updated) return errResult('not-found', 'Role not found');
  return jsonRespond(updated);
}

async function iamDeleteRole(path: string, host: RuntimeHost): Promise<BlockResult> {
  const id = path.replace('/admin/iam/roles/', '');
  if (!id) return errResult('invalid-argument', 'Missing role ID');

  // Check if system role
  const role = await dbGet(host, ROLES, id);
  if (role && (role.data.is_system === 1 || role.data.is_system === true)) {
    return errResult('permission-denied', 'Cannot delete system role');
  }

  const deleted = await dbDelete(host, ROLES, id);
  if (!deleted) return errResult('not-found', 'Role not found');
  return jsonRespond({ deleted: true });
}

async function iamListPermissions(host: RuntimeHost): Promise<BlockResult> {
  const result = await dbList(host, PERMISSIONS, { limit: 1000, offset: 0 });
  if (!result) return errResult('internal', 'Database error');
  return jsonRespond(result);
}

async function iamCreatePermission(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<{ name: string; resource: string; actions: string[] }>(msg);
  if (!body || !body.name) return errResult('invalid-argument', 'Invalid body');

  const record = await dbCreate(host, PERMISSIONS, {
    name: body.name,
    resource: body.resource ?? '',
    actions: JSON.stringify(body.actions ?? []),
  });
  if (!record) return errResult('internal', 'Database error');
  return jsonRespond(record);
}

async function iamDeletePermission(path: string, host: RuntimeHost): Promise<BlockResult> {
  const id = path.replace('/admin/iam/permissions/', '');
  if (!id) return errResult('invalid-argument', 'Missing permission ID');

  const deleted = await dbDelete(host, PERMISSIONS, id);
  if (!deleted) return errResult('not-found', 'Permission not found');
  return jsonRespond({ deleted: true });
}

async function iamListUserRoles(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = queryParam(msg, 'user_id');
  const filters: Filter[] = [];
  if (userId) {
    filters.push({ field: 'user_id', operator: 'eq', value: userId });
  }

  const result = await dbList(host, USER_ROLES, { filters, limit: 1000, offset: 0 });
  if (!result) return errResult('internal', 'Database error');
  return jsonRespond(result);
}

async function iamAssignRole(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<{ user_id: string; role: string }>(msg);
  if (!body || !body.user_id || !body.role) return errResult('invalid-argument', 'Invalid body');

  // Check if already assigned
  const existing = await dbList(host, USER_ROLES, {
    filters: [
      { field: 'user_id', operator: 'eq', value: body.user_id },
      { field: 'role', operator: 'eq', value: body.role },
    ],
    limit: 1,
    offset: 0,
  });
  if (existing && existing.records && existing.records.length > 0) {
    return errResult('already-exists', 'Role already assigned to user');
  }

  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  const record = await dbCreate(host, USER_ROLES, {
    user_id: body.user_id,
    role: body.role,
    assigned_at: new Date().toISOString(),
    assigned_by: userId,
  });
  if (!record) return errResult('internal', 'Database error');
  return jsonRespond(record);
}

async function iamRemoveRole(path: string, host: RuntimeHost): Promise<BlockResult> {
  const id = path.replace('/admin/iam/user-roles/', '');
  if (!id) return errResult('invalid-argument', 'Missing user-role ID');

  const deleted = await dbDelete(host, USER_ROLES, id);
  if (!deleted) return errResult('not-found', 'User-role assignment not found');
  return jsonRespond({ deleted: true });
}

// ===========================================================================
// LOGS — /admin/logs
// ===========================================================================

async function handleLogs(action: string, path: string, msg: Message, host: RuntimeHost): Promise<BlockResult> {
  if (action === 'retrieve' && path === '/admin/logs') {
    return logsHandleList(msg, host);
  }
  return errResult('not-found', 'not found');
}

async function logsHandleList(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const { page, pageSize } = paginationParams(msg, 50);

  const filters: Filter[] = [];
  const userId = queryParam(msg, 'user_id');
  if (userId) {
    filters.push({ field: 'user_id', operator: 'eq', value: userId });
  }
  const actionFilter = queryParam(msg, 'action');
  if (actionFilter) {
    filters.push({ field: 'action', operator: 'eq', value: actionFilter });
  }
  const resource = queryParam(msg, 'resource');
  if (resource) {
    filters.push({ field: 'resource', operator: 'like', value: `%${resource}%` });
  }

  const result = await dbList(host, AUDIT_LOGS, {
    filters,
    sort: [{ field: 'created_at', desc: true }],
    limit: pageSize,
    offset: (page - 1) * pageSize,
  });
  if (!result) return errResult('internal', 'Database error');
  return jsonRespond(result);
}

// ===========================================================================
// SETTINGS — /admin/settings/* and /settings/*
// ===========================================================================

async function handleSettings(action: string, path: string, msg: Message, host: RuntimeHost): Promise<BlockResult> {
  switch (true) {
    case action === 'retrieve' && (path === '/admin/settings' || path === '/settings'):
      return settingsHandleList(host);
    case action === 'retrieve' && (path.startsWith('/admin/settings/') || path.startsWith('/settings/')):
      return settingsHandleGet(path, host);
    case action === 'update' && path.startsWith('/admin/settings/'):
      return settingsHandleSet(path, msg, host);
    case action === 'create' && path === '/admin/settings':
      return settingsHandleBatch(msg, host);
    default:
      return errResult('not-found', 'not found');
  }
}

async function settingsHandleList(host: RuntimeHost): Promise<BlockResult> {
  const result = await dbList(host, SETTINGS, { limit: 1000, offset: 0 });
  if (!result) return errResult('internal', 'Database error');

  // Convert to key-value map
  const settings: Record<string, unknown> = {};
  for (const record of (result.records ?? [])) {
    const key = record.data.key as string;
    if (key) {
      settings[key] = record.data.value;
    }
  }
  return jsonRespond(settings);
}

async function settingsHandleGet(path: string, host: RuntimeHost): Promise<BlockResult> {
  const key = path.replace('/admin/settings/', '').replace('/settings/', '');
  if (!key) return errResult('invalid-argument', 'Missing setting key');

  // Find by key field
  const result = await dbList(host, SETTINGS, {
    filters: [{ field: 'key', operator: 'eq', value: key }],
    limit: 1,
    offset: 0,
  });
  if (!result || !result.records || result.records.length === 0) {
    return errResult('not-found', 'Setting not found');
  }
  return jsonRespond(result.records[0]);
}

async function settingsHandleSet(path: string, msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const key = path.replace('/admin/settings/', '');
  if (!key) return errResult('invalid-argument', 'Missing setting key');

  const body = parseBody<{ value: unknown }>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';

  // Upsert: try to find existing, then update or create
  const existing = await dbList(host, SETTINGS, {
    filters: [{ field: 'key', operator: 'eq', value: key }],
    limit: 1,
    offset: 0,
  });

  if (existing && existing.records && existing.records.length > 0) {
    const record = existing.records[0];
    const updated = await dbUpdate(host, SETTINGS, record.id, {
      value: body.value,
      updated_by: userId,
      updated_at: new Date().toISOString(),
    });
    if (!updated) return errResult('internal', 'Failed to update setting');
    return jsonRespond(updated);
  } else {
    const created = await dbCreate(host, SETTINGS, {
      key,
      value: body.value,
      updated_by: userId,
    });
    if (!created) return errResult('internal', 'Failed to create setting');
    return jsonRespond(created);
  }
}

async function settingsHandleBatch(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  const now = new Date().toISOString();
  let count = 0;

  for (const [key, value] of Object.entries(body)) {
    const existing = await dbList(host, SETTINGS, {
      filters: [{ field: 'key', operator: 'eq', value: key }],
      limit: 1,
      offset: 0,
    });

    if (existing && existing.records && existing.records.length > 0) {
      await dbUpdate(host, SETTINGS, existing.records[0].id, {
        value,
        updated_by: userId,
        updated_at: now,
      });
    } else {
      await dbCreate(host, SETTINGS, {
        key,
        value,
        updated_by: userId,
      });
    }
    count++;
  }

  return jsonRespond({ updated: count });
}

// ===========================================================================
// CUSTOM TABLES — /admin/custom-tables/*
// ===========================================================================

async function handleCustomTables(action: string, path: string, msg: Message, host: RuntimeHost): Promise<BlockResult> {
  switch (true) {
    case action === 'retrieve' && path === '/admin/custom-tables':
      return ctListTables(host);
    case action === 'create' && path === '/admin/custom-tables':
      return ctCreateTable(msg, host);
    case action === 'delete' && path.startsWith('/admin/custom-tables/') && !path.includes('/records'):
      return ctDropTable(path, host);
    // Record CRUD
    case action === 'retrieve' && path.includes('/records'):
      return ctListRecords(path, msg, host);
    case action === 'create' && path.includes('/records'):
      return ctCreateRecord(path, msg, host);
    case action === 'update' && path.includes('/records/'):
      return ctUpdateRecord(path, msg, host);
    case action === 'delete' && path.includes('/records/'):
      return ctDeleteRecord(path, host);
    default:
      return errResult('not-found', 'not found');
  }
}

function ctExtractTableName(path: string): string {
  const rest = path.replace('/admin/custom-tables/', '');
  const slashIdx = rest.indexOf('/');
  return slashIdx >= 0 ? rest.substring(0, slashIdx) : rest;
}

function ctExtractRecordId(path: string): string {
  const idx = path.lastIndexOf('/records/');
  return idx >= 0 ? path.substring(idx + 9) : '';
}

async function ctListTables(host: RuntimeHost): Promise<BlockResult> {
  const tables = await dbQueryRaw(host,
    "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'custom_%' ORDER BY name",
    [],
  );
  if (!tables) return errResult('internal', 'Database error');

  const names = tables
    .map((r: any) => r.data?.name ?? r.name)
    .filter(Boolean);

  return jsonRespond({ tables: names });
}

async function ctCreateTable(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<{ name: string; columns?: { name: string; type?: string }[] }>(msg);
  if (!body || !body.name) return errResult('invalid-argument', 'Invalid body');

  const tableName = 'custom_' + sanitizeIdent(body.name);

  const colDefs: string[] = ['id TEXT PRIMARY KEY'];
  for (const col of (body.columns ?? [])) {
    const safeName = sanitizeIdent(col.name);
    const safeType = ['TEXT', 'INTEGER', 'REAL', 'BLOB'].includes((col.type ?? 'TEXT').toUpperCase())
      ? (col.type ?? 'TEXT').toUpperCase()
      : 'TEXT';
    colDefs.push(`"${safeName}" ${safeType}`);
  }
  colDefs.push('created_at TEXT DEFAULT CURRENT_TIMESTAMP');
  colDefs.push('updated_at TEXT DEFAULT CURRENT_TIMESTAMP');

  const sql = `CREATE TABLE IF NOT EXISTS "${tableName}" (${colDefs.join(', ')})`;
  const result = await dbExecRaw(host, sql, []);
  if (result === null) return errResult('internal', 'Failed to create table');

  return jsonRespond({ table: tableName, created: true });
}

async function ctDropTable(path: string, host: RuntimeHost): Promise<BlockResult> {
  const rawName = ctExtractTableName(path);
  if (!rawName) return errResult('invalid-argument', 'Missing table name');

  const fullName = rawName.startsWith('custom_') ? rawName : `custom_${rawName}`;
  const safeName = sanitizeIdent(fullName);

  const result = await dbExecRaw(host, `DROP TABLE IF EXISTS "${safeName}"`, []);
  if (result === null) return errResult('internal', 'Failed to drop table');

  return jsonRespond({ deleted: true });
}

async function ctListRecords(path: string, msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const rawName = ctExtractTableName(path);
  if (!rawName) return errResult('invalid-argument', 'Missing table name');

  const fullName = rawName.startsWith('custom_') ? rawName : `custom_${rawName}`;
  const { pageSize } = paginationParams(msg, 20);
  const offset = Number(queryParam(msg, 'offset') ?? '0');

  const result = await dbList(host, fullName, {
    sort: [{ field: 'created_at', desc: true }],
    limit: pageSize,
    offset,
  });
  if (!result) return errResult('internal', 'Database error');
  return jsonRespond(result);
}

async function ctCreateRecord(path: string, msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const rawName = ctExtractTableName(path);
  if (!rawName) return errResult('invalid-argument', 'Missing table name');

  const fullName = rawName.startsWith('custom_') ? rawName : `custom_${rawName}`;
  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  const record = await dbCreate(host, fullName, body);
  if (!record) return errResult('internal', 'Database error');
  return jsonRespond(record);
}

async function ctUpdateRecord(path: string, msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const rawName = ctExtractTableName(path);
  const recordId = ctExtractRecordId(path);
  if (!rawName || !recordId) return errResult('invalid-argument', 'Missing table name or record ID');

  const fullName = rawName.startsWith('custom_') ? rawName : `custom_${rawName}`;
  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  const updated = await dbUpdate(host, fullName, recordId, body);
  if (!updated) return errResult('not-found', 'Record not found');
  return jsonRespond(updated);
}

async function ctDeleteRecord(path: string, host: RuntimeHost): Promise<BlockResult> {
  const rawName = ctExtractTableName(path);
  const recordId = ctExtractRecordId(path);
  if (!rawName || !recordId) return errResult('invalid-argument', 'Missing table name or record ID');

  const fullName = rawName.startsWith('custom_') ? rawName : `custom_${rawName}`;
  const deleted = await dbDelete(host, fullName, recordId);
  if (!deleted) return errResult('not-found', 'Record not found');
  return jsonRespond({ deleted: true });
}

// ===========================================================================
// WAFER INFO — /admin/wafer/*
// ===========================================================================

function handleWaferInfo(action: string, path: string): BlockResult {
  switch (true) {
    case action === 'retrieve' && path === '/admin/wafer/blocks':
      return jsonRespond([]);
    case action === 'retrieve' && path === '/admin/wafer/flows':
      return jsonRespond([]);
    case action === 'retrieve' && path === '/admin/wafer/info':
      return jsonRespond({
        runtime: 'cloudflare-worker',
        version: '1.0.0',
        platform: 'solobase',
        block_mode: 'typescript-native',
        features: ['database', 'storage', 'crypto', 'network', 'config'],
      });
    default:
      return errResult('not-found', 'not found');
  }
}

// ===========================================================================
// Database service call helpers
// ===========================================================================

interface Filter {
  field: string;
  operator: string;
  value?: unknown;
}

interface SortDef {
  field: string;
  desc?: boolean;
}

interface ListOpts {
  filters?: Filter[];
  sort?: SortDef[];
  limit?: number;
  offset?: number;
}

async function callService(host: RuntimeHost, block: string, kind: string, data: unknown): Promise<any> {
  const result = await host.callBlock(block, {
    kind,
    data: new TextEncoder().encode(JSON.stringify(data)),
    meta: [],
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

async function dbDelete(host: RuntimeHost, collection: string, id: string): Promise<boolean> {
  const result = await host.callBlock('wafer-run/database', {
    kind: 'database.delete',
    data: new TextEncoder().encode(JSON.stringify({ collection, id })),
    meta: [],
  });
  return result.action === 'respond';
}

async function dbList(host: RuntimeHost, collection: string, opts: ListOpts): Promise<{ records: { id: string; data: Record<string, any> }[]; total_count: number; page: number; page_size: number } | null> {
  return callService(host, 'wafer-run/database', 'database.list', {
    collection,
    filters: opts.filters ?? [],
    sort: opts.sort ?? [],
    limit: opts.limit ?? 100,
    offset: opts.offset ?? 0,
  });
}

async function dbQueryRaw(host: RuntimeHost, query: string, args: unknown[]): Promise<any[] | null> {
  return callService(host, 'wafer-run/database', 'database.query_raw', { query, args });
}

async function dbExecRaw(host: RuntimeHost, query: string, args: unknown[]): Promise<any | null> {
  return callService(host, 'wafer-run/database', 'database.exec_raw', { query, args });
}

// ===========================================================================
// General helpers
// ===========================================================================

function parseBody<T>(msg: Message): T | null {
  try { return JSON.parse(new TextDecoder().decode(msg.data)) as T; } catch { return null; }
}

function jsonRespond(data: unknown, status?: number): BlockResult {
  const meta = [{ key: 'resp.content_type', value: 'application/json' }];
  if (status) meta.push({ key: 'resp.status', value: String(status) });
  return {
    action: 'respond',
    response: { data: new TextEncoder().encode(JSON.stringify(data)), meta },
  };
}

function errResult(code: string, message: string): BlockResult {
  return { action: 'error', error: { code: code as any, message, meta: [] } };
}

function queryParam(msg: Message, name: string): string {
  return metaGet(msg.meta, `req.query.${name}`) ?? '';
}

function paginationParams(msg: Message, defaultSize: number): { page: number; pageSize: number } {
  const page = Math.max(1, Number(queryParam(msg, 'page') || '1'));
  const pageSize = Math.min(100, Math.max(1, Number(queryParam(msg, 'page_size') || String(defaultSize))));
  return { page, pageSize };
}

/** Sanitize a SQL identifier — only allow alphanumeric + underscore. */
function sanitizeIdent(name: string): string {
  return name.replace(/[^a-zA-Z0-9_]/g, '');
}
