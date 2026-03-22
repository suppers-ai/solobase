// TypeScript-native files block handler (for testing without WASM).
// Calls host.callBlock() for storage/database/crypto operations.

import type { Message, BlockResult } from '../types';
import type { RuntimeHost } from '../host';
import { metaGet } from '../convert';

// ---------------------------------------------------------------------------
// Collections
// ---------------------------------------------------------------------------

const BUCKETS = 'storage_buckets';
const OBJECTS = 'storage_objects';
const VIEWS = 'storage_views';
const SHARES = 'cloud_shares';
const ACCESS_LOGS = 'cloud_access_logs';
const QUOTAS = 'cloud_quotas';

// ---------------------------------------------------------------------------
// Main handler
// ---------------------------------------------------------------------------

export async function handle(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const action = metaGet(msg.meta, 'req.action') ?? '';
  const path = metaGet(msg.meta, 'req.resource') ?? '';

  // Direct share access (public, no auth)
  if (path.startsWith('/storage/direct/')) {
    return handleDirectAccess(msg, host, path);
  }

  // Cloud storage routes
  if (path.startsWith('/b/cloudstorage') || path.startsWith('/admin/b/cloudstorage')) {
    return handleCloud(action, path, msg, host);
  }

  // Admin storage routes
  if (path.startsWith('/admin/storage')) {
    return handleAdmin(action, path, msg, host);
  }

  // User storage routes
  if (path.startsWith('/storage')) {
    return handleStorage(action, path, msg, host);
  }

  return errResult('not-found', 'not found');
}

// ---------------------------------------------------------------------------
// Path validation
// ---------------------------------------------------------------------------

function isValidStorageKey(key: string): boolean {
  return key.length > 0
    && !key.includes('..')
    && !key.startsWith('/')
    && !key.includes('\0');
}

function isValidBucketName(name: string): boolean {
  return name.length > 0
    && !name.includes('..')
    && !name.includes('/')
    && !name.includes('\0');
}

// ---------------------------------------------------------------------------
// Path extraction helpers
// ---------------------------------------------------------------------------

function extractBucketName(path: string): string {
  const rest = stripPrefix(path, '/storage/buckets/')
    ?? stripPrefix(path, '/admin/storage/buckets/')
    ?? '';
  const idx = rest.indexOf('/');
  return idx >= 0 ? rest.substring(0, idx) : rest;
}

function extractObjectKey(path: string): string {
  const idx = path.indexOf('/objects/');
  return idx >= 0 ? path.substring(idx + 9) : '';
}

function stripPrefix(s: string, prefix: string): string | null {
  return s.startsWith(prefix) ? s.substring(prefix.length) : null;
}

// ---------------------------------------------------------------------------
// User storage routes (/storage/*)
// ---------------------------------------------------------------------------

async function handleStorage(action: string, path: string, msg: Message, host: RuntimeHost): Promise<BlockResult> {
  switch (true) {
    case action === 'retrieve' && path === '/storage/buckets':
      return handleListBuckets(msg, host);
    case action === 'create' && path === '/storage/buckets':
      return handleCreateBucket(msg, host);
    case action === 'retrieve' && path.startsWith('/storage/buckets/') && path.includes('/objects/'):
      return handleGetObject(msg, host, path);
    case action === 'retrieve' && path.startsWith('/storage/buckets/') && path.includes('/objects'):
      return handleListObjects(msg, host, path);
    case action === 'create' && path.startsWith('/storage/buckets/') && path.includes('/objects'):
      return handleUploadObject(msg, host, path);
    case action === 'delete' && path.startsWith('/storage/buckets/') && path.includes('/objects/'):
      return handleDeleteObject(msg, host, path);
    case action === 'delete' && path.startsWith('/storage/buckets/'):
      return handleDeleteBucket(msg, host, path);
    case action === 'retrieve' && path === '/storage/search':
      return handleSearch(msg, host);
    case action === 'retrieve' && path === '/storage/recent':
      return handleRecent(msg, host);
    default:
      return errResult('not-found', 'not found');
  }
}

// ---------------------------------------------------------------------------
// Admin storage routes (/admin/storage/*)
// ---------------------------------------------------------------------------

async function handleAdmin(action: string, path: string, msg: Message, host: RuntimeHost): Promise<BlockResult> {
  switch (true) {
    case action === 'retrieve' && path === '/admin/storage/buckets':
      return handleListBuckets(msg, host);
    case action === 'retrieve' && path === '/admin/storage/stats':
      return handleStats(msg, host);
    default:
      return errResult('not-found', 'not found');
  }
}

// ---------------------------------------------------------------------------
// Cloud sharing routes (/b/cloudstorage/*, /admin/b/cloudstorage/*)
// ---------------------------------------------------------------------------

async function handleCloud(action: string, path: string, msg: Message, host: RuntimeHost): Promise<BlockResult> {
  switch (true) {
    // User-facing
    case action === 'retrieve' && path === '/b/cloudstorage/shares':
      return handleListShares(msg, host);
    case action === 'create' && path === '/b/cloudstorage/shares':
      return handleCreateShare(msg, host);
    case action === 'delete' && path.startsWith('/b/cloudstorage/shares/'):
      return handleDeleteShare(msg, host, path);
    case action === 'retrieve' && path === '/b/cloudstorage/quota':
      return handleGetQuota(msg, host);
    // Admin
    case action === 'retrieve' && path === '/admin/b/cloudstorage/shares':
      return handleAdminListShares(msg, host);
    case action === 'retrieve' && path === '/admin/b/cloudstorage/access-logs':
      return handleAccessLogs(msg, host);
    case action === 'retrieve' && path === '/admin/b/cloudstorage/quotas':
      return handleAdminQuotas(msg, host);
    case action === 'update' && path.startsWith('/admin/b/cloudstorage/quotas/'):
      return handleUpdateQuota(msg, host, path);
    default:
      return errResult('not-found', 'not found');
  }
}

// ---------------------------------------------------------------------------
// Storage handlers
// ---------------------------------------------------------------------------

async function handleListBuckets(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const result = await callStorage(host, 'storage.list_folders', {});
  if (!result) return errResult('internal', 'Storage error');
  return jsonRespond(msg, { buckets: result.folders ?? result });
}

async function handleCreateBucket(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<{ name: string; public?: boolean }>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');
  if (!body.name) return errResult('invalid-argument', 'Bucket name is required');
  if (!isValidBucketName(body.name)) return errResult('invalid-argument', 'Invalid bucket name');

  const result = await callStorage(host, 'storage.create_folder', {
    name: body.name,
    public: body.public ?? false,
  });
  if (!result) return errResult('internal', 'Failed to create bucket');

  // Track in DB
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  await dbCreate(host, BUCKETS, {
    name: body.name,
    public: body.public ?? false,
    created_by: userId,
    created_at: new Date().toISOString(),
  });

  return jsonRespond(msg, { name: body.name, created: true });
}

async function handleDeleteBucket(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const bucket = extractBucketName(path);
  if (!bucket) return errResult('invalid-argument', 'Missing bucket name');

  const result = await callStorage(host, 'storage.delete_folder', { name: bucket });
  if (!result) return errResult('internal', 'Failed to delete bucket');

  return jsonRespond(msg, { deleted: true });
}

async function handleListObjects(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const bucket = extractBucketName(path);
  if (!bucket) return errResult('invalid-argument', 'Missing bucket name');

  const prefix = queryParam(msg, 'prefix') ?? '';
  const limit = parseInt(queryParam(msg, 'page_size') ?? '50', 10);
  const offset = parseInt(queryParam(msg, 'offset') ?? '0', 10);

  const result = await callStorage(host, 'storage.list', {
    folder: bucket,
    prefix,
    limit,
    offset,
  });
  if (!result) return errResult('internal', 'Storage error');

  return jsonRespond(msg, result);
}

async function handleGetObject(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const bucket = extractBucketName(path);
  const key = extractObjectKey(path);
  if (!bucket || !key) return errResult('invalid-argument', 'Missing bucket name or object key');
  if (!isValidStorageKey(key)) return errResult('invalid-argument', 'Invalid object key');

  // Track view in DB
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  await dbCreate(host, VIEWS, {
    bucket,
    key,
    user_id: userId,
    viewed_at: new Date().toISOString(),
  });

  const result = await callStorage(host, 'storage.get', { folder: bucket, key });
  if (!result) return errResult('not-found', 'Object not found');

  // The storage service returns { data: base64, content_type: string }
  const contentType = result.content_type ?? 'application/octet-stream';
  const data = result.data
    ? base64ToUint8Array(result.data)
    : new Uint8Array(0);

  return {
    action: 'respond',
    response: {
      data,
      meta: [{ key: 'resp.content_type', value: contentType }],
    },
    message: msg,
  };
}

async function handleUploadObject(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const bucket = extractBucketName(path);
  if (!bucket) return errResult('invalid-argument', 'Missing bucket name');

  const key = queryParam(msg, 'key') ?? '';
  if (!key) return errResult('invalid-argument', 'Missing object key (pass as ?key=filename)');
  if (!isValidStorageKey(key)) return errResult('invalid-argument', 'Invalid object key');

  const contentType = metaGet(msg.meta, 'req.content_type') || 'application/octet-stream';
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';

  // Check quota
  const quotaErr = await checkQuota(host, userId, msg.data.length);
  if (quotaErr) return quotaErr;

  // Upload via storage service
  const dataBase64 = uint8ArrayToBase64(msg.data);
  const result = await callStorage(host, 'storage.put', {
    folder: bucket,
    key,
    data: dataBase64,
    content_type: contentType,
  });
  if (!result) return errResult('internal', 'Upload failed');

  // Track metadata in DB
  await dbCreate(host, OBJECTS, {
    bucket,
    key,
    size: msg.data.length,
    content_type: contentType,
    uploaded_by: userId,
    uploaded_at: new Date().toISOString(),
  });

  return jsonRespond(msg, { bucket, key, uploaded: true });
}

async function handleDeleteObject(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const bucket = extractBucketName(path);
  const key = extractObjectKey(path);
  if (!bucket || !key) return errResult('invalid-argument', 'Missing bucket name or object key');
  if (!isValidStorageKey(key)) return errResult('invalid-argument', 'Invalid object key');

  const result = await callStorage(host, 'storage.delete', { folder: bucket, key });
  if (!result) return errResult('not-found', 'Object not found');

  // Clean up metadata
  await dbDeleteByFilters(host, OBJECTS, [
    { field: 'bucket', operator: 'eq', value: bucket },
    { field: 'key', operator: 'eq', value: key },
  ]);

  return jsonRespond(msg, { deleted: true });
}

async function handleSearch(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const query = queryParam(msg, 'q') ?? '';
  if (!query) return errResult('invalid-argument', 'Missing search query');

  const pageSize = parseInt(queryParam(msg, 'page_size') ?? '20', 10);
  const offset = parseInt(queryParam(msg, 'offset') ?? '0', 10);

  const result = await dbList(host, OBJECTS, {
    filters: [{ field: 'key', operator: 'like', value: `%${query}%` }],
    sort: [{ field: 'uploaded_at', desc: true }],
    limit: pageSize,
    offset,
  });
  if (!result) return errResult('internal', 'Search failed');

  return jsonRespond(msg, result);
}

async function handleRecent(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';

  const result = await dbList(host, VIEWS, {
    filters: [{ field: 'user_id', operator: 'eq', value: userId }],
    sort: [{ field: 'viewed_at', desc: true }],
    limit: 20,
    offset: 0,
  });
  if (!result) return errResult('internal', 'Database error');

  return jsonRespond(msg, result);
}

async function handleStats(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const totalObjects = await dbCount(host, OBJECTS, []);
  const totalSize = await dbSum(host, OBJECTS, 'size', []);
  const bucketsResult = await callStorage(host, 'storage.list_folders', {});
  const bucketCount = bucketsResult?.folders?.length ?? 0;

  return jsonRespond(msg, {
    total_objects: totalObjects,
    total_size_bytes: totalSize,
    bucket_count: bucketCount,
  });
}

// ---------------------------------------------------------------------------
// Cloud sharing handlers
// ---------------------------------------------------------------------------

async function handleListShares(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';

  const result = await dbList(host, SHARES, {
    filters: [{ field: 'created_by', operator: 'eq', value: userId }],
    sort: [{ field: 'created_at', desc: true }],
    limit: 100,
    offset: 0,
  });
  if (!result) return errResult('internal', 'Database error');

  return jsonRespond(msg, result);
}

async function handleCreateShare(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<{
    bucket: string;
    key: string;
    expires_in_hours?: number;
    max_access_count?: number;
  }>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  // Generate share token via crypto service
  const token = await generateShareToken(host, body.bucket, body.key);
  if (!token) return errResult('internal', 'Token generation failed');

  const now = new Date().toISOString();
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';

  const data: Record<string, unknown> = {
    token,
    bucket: body.bucket,
    key: body.key,
    created_by: userId,
    created_at: now,
    access_count: 0,
  };

  if (body.expires_in_hours != null) {
    const expiresAt = new Date(Date.now() + body.expires_in_hours * 3600 * 1000).toISOString();
    data.expires_at = expiresAt;
  }
  if (body.max_access_count != null) {
    data.max_access_count = body.max_access_count;
  }

  const record = await dbCreate(host, SHARES, data);
  if (!record) return errResult('internal', 'Database error');

  return jsonRespond(msg, {
    id: record.id,
    token,
    direct_url: `/storage/direct/${token}`,
  });
}

async function handleDeleteShare(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const id = path.replace('/b/cloudstorage/shares/', '');
  if (!id) return errResult('invalid-argument', 'Missing share ID');

  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  const roles = metaGet(msg.meta, 'auth.user_roles') ?? '';
  const isAdmin = roles.split(',').some(r => r.trim() === 'admin');

  // Verify ownership
  const share = await dbGet(host, SHARES, id);
  if (share) {
    const owner = (share.data?.created_by ?? '') as string;
    if (owner !== userId && !isAdmin) {
      return errResult('permission-denied', "Cannot delete another user's share");
    }
  }

  const deleteResult = await callService(host, 'wafer-run/database', 'database.delete', {
    collection: SHARES,
    id,
  });
  if (!deleteResult) return errResult('not-found', 'Share not found');

  return jsonRespond(msg, { deleted: true });
}

async function handleGetQuota(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  const quota = await getUserQuota(host, userId);
  const usage = await getUserUsage(host, userId);

  return jsonRespond(msg, { quota, usage });
}

async function handleAdminListShares(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const page = parseInt(queryParam(msg, 'page') ?? '1', 10);
  const pageSize = parseInt(queryParam(msg, 'page_size') ?? '20', 10);
  const offset = (page - 1) * pageSize;

  const result = await dbList(host, SHARES, {
    filters: [],
    sort: [{ field: 'created_at', desc: true }],
    limit: pageSize,
    offset,
  });
  if (!result) return errResult('internal', 'Database error');

  return jsonRespond(msg, result);
}

async function handleAccessLogs(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const page = parseInt(queryParam(msg, 'page') ?? '1', 10);
  const pageSize = parseInt(queryParam(msg, 'page_size') ?? '50', 10);
  const offset = (page - 1) * pageSize;

  const filters: Filter[] = [];
  const shareId = queryParam(msg, 'share_id') ?? '';
  if (shareId) {
    filters.push({ field: 'share_id', operator: 'eq', value: shareId });
  }

  const result = await dbList(host, ACCESS_LOGS, {
    filters,
    sort: [{ field: 'accessed_at', desc: true }],
    limit: pageSize,
    offset,
  });
  if (!result) return errResult('internal', 'Database error');

  return jsonRespond(msg, result);
}

async function handleAdminQuotas(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const result = await dbList(host, QUOTAS, {
    filters: [],
    sort: [],
    limit: 1000,
    offset: 0,
  });
  if (!result) return errResult('internal', 'Database error');

  return jsonRespond(msg, result);
}

async function handleUpdateQuota(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const userId = path.replace('/admin/b/cloudstorage/quotas/', '');
  if (!userId) return errResult('invalid-argument', 'Missing user ID');

  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  body.user_id = userId;
  body.updated_at = new Date().toISOString();

  // Upsert: try to find existing, then create or update
  const existing = await dbListFiltered(host, QUOTAS, 'user_id', userId);
  let record;
  if (existing.length > 0) {
    record = await dbUpdate(host, QUOTAS, existing[0].id, body);
  } else {
    record = await dbCreate(host, QUOTAS, body);
  }
  if (!record) return errResult('internal', 'Database error');

  return jsonRespond(msg, record);
}

// ---------------------------------------------------------------------------
// Direct access handler (public, no auth)
// ---------------------------------------------------------------------------

async function handleDirectAccess(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const token = path.replace('/storage/direct/', '');
  if (!token) return errResult('invalid-argument', 'Missing share token');

  // Look up share by token
  const shares = await dbListFiltered(host, SHARES, 'token', token);
  if (shares.length === 0) return errResult('not-found', 'Share not found or expired');
  const share = shares[0];

  // Check expiry
  const expiresAt = share.data?.expires_at as string | undefined;
  if (expiresAt && expiresAt.length > 0) {
    const expTime = new Date(expiresAt).getTime();
    if (expTime < Date.now()) {
      return errResult('permission-denied', 'Share link has expired');
    }
  }

  // Check access count
  const accessCount = (share.data?.access_count as number) ?? 0;
  const maxAccess = share.data?.max_access_count as number | undefined;
  if (maxAccess != null && maxAccess > 0 && accessCount >= maxAccess) {
    return errResult('permission-denied', 'Share link access limit reached');
  }

  const bucket = (share.data?.bucket ?? '') as string;
  const key = (share.data?.key ?? '') as string;
  if (!bucket || !key) return errResult('internal', 'Invalid share data');

  // Increment access count
  await dbUpdate(host, SHARES, share.id, { access_count: accessCount + 1 });

  // Log access
  const ipAddress = metaGet(msg.meta, 'req.client.ip') ?? metaGet(msg.meta, 'http.remote_addr') ?? '';
  const userAgent = metaGet(msg.meta, 'http.header.user-agent') ?? '';
  await dbCreate(host, ACCESS_LOGS, {
    share_id: share.id,
    accessed_at: new Date().toISOString(),
    ip_address: ipAddress,
    user_agent: userAgent,
  });

  // Serve the file
  const result = await callStorage(host, 'storage.get', { folder: bucket, key });
  if (!result) return errResult('not-found', 'File not found');

  const contentType = result.content_type ?? 'application/octet-stream';
  const data = result.data
    ? base64ToUint8Array(result.data)
    : new Uint8Array(0);

  return {
    action: 'respond',
    response: {
      data,
      meta: [
        { key: 'resp.content_type', value: contentType },
        { key: 'resp.header.Content-Disposition', value: `inline; filename="${key}"` },
        { key: 'resp.header.Cache-Control', value: 'private, max-age=3600' },
      ],
    },
    message: msg,
  };
}

// ---------------------------------------------------------------------------
// Quota helpers
// ---------------------------------------------------------------------------

interface QuotaConfig {
  max_storage_bytes: number;
  max_file_size_bytes: number;
  max_files_per_bucket: number;
  reset_period_days: number;
}

const DEFAULT_QUOTA: QuotaConfig = {
  max_storage_bytes: 1_073_741_824,   // 1 GB
  max_file_size_bytes: 104_857_600,    // 100 MB
  max_files_per_bucket: 10_000,
  reset_period_days: 0,
};

async function getUserQuota(host: RuntimeHost, userId: string): Promise<QuotaConfig> {
  const records = await dbListFiltered(host, QUOTAS, 'user_id', userId);
  if (records.length > 0) {
    const d = records[0].data ?? {};
    return {
      max_storage_bytes: asNumber(d.max_storage_bytes, DEFAULT_QUOTA.max_storage_bytes),
      max_file_size_bytes: asNumber(d.max_file_size_bytes, DEFAULT_QUOTA.max_file_size_bytes),
      max_files_per_bucket: asNumber(d.max_files_per_bucket, DEFAULT_QUOTA.max_files_per_bucket),
      reset_period_days: asNumber(d.reset_period_days, DEFAULT_QUOTA.reset_period_days),
    };
  }
  return DEFAULT_QUOTA;
}

async function getUserUsage(host: RuntimeHost, userId: string): Promise<{ total_bytes: number; file_count: number }> {
  const totalBytes = await dbSum(host, OBJECTS, 'size', [
    { field: 'uploaded_by', operator: 'eq', value: userId },
  ]);
  const fileCount = await dbCount(host, OBJECTS, [
    { field: 'uploaded_by', operator: 'eq', value: userId },
  ]);
  return { total_bytes: totalBytes, file_count: fileCount };
}

async function checkQuota(host: RuntimeHost, userId: string, fileSize: number): Promise<BlockResult | null> {
  const quota = await getUserQuota(host, userId);
  const usage = await getUserUsage(host, userId);

  if (fileSize > quota.max_file_size_bytes) {
    return errResult('invalid-argument', `File exceeds maximum size of ${quota.max_file_size_bytes} bytes`);
  }
  if (usage.total_bytes + fileSize > quota.max_storage_bytes) {
    return errResult('invalid-argument', 'Storage quota exceeded');
  }
  return null;
}

// ---------------------------------------------------------------------------
// Share token generation
// ---------------------------------------------------------------------------

async function generateShareToken(host: RuntimeHost, bucket: string, key: string): Promise<string | null> {
  const result = await callService(host, 'wafer-run/crypto', 'crypto.sign', {
    claims: { bucket, key, type: 'share' },
    expiry_secs: 365 * 24 * 3600,
  });
  return result?.token ?? null;
}

// ---------------------------------------------------------------------------
// Service call helpers
// ---------------------------------------------------------------------------

interface Filter {
  field: string;
  operator: string;
  value: unknown;
}

interface SortField {
  field: string;
  desc: boolean;
}

interface ListOpts {
  filters: Filter[];
  sort: SortField[];
  limit: number;
  offset: number;
}

async function callService(host: RuntimeHost, block: string, kind: string, data: unknown): Promise<any> {
  const result = await host.callBlock(block, {
    kind, data: new TextEncoder().encode(JSON.stringify(data)), meta: [],
  });
  if (result.action !== 'respond' || !result.response) return null;
  try { return JSON.parse(new TextDecoder().decode(result.response.data)); } catch { return null; }
}

async function callStorage(host: RuntimeHost, kind: string, data: unknown): Promise<any> {
  return callService(host, 'wafer-run/storage', kind, data);
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

async function dbList(host: RuntimeHost, collection: string, opts: ListOpts): Promise<any> {
  return callService(host, 'wafer-run/database', 'database.list', {
    collection,
    filters: opts.filters,
    sort: opts.sort,
    limit: opts.limit,
    offset: opts.offset,
  });
}

async function dbListFiltered(host: RuntimeHost, collection: string, field: string, value: string): Promise<{ id: string; data: Record<string, any> }[]> {
  const result = await callService(host, 'wafer-run/database', 'database.list', {
    collection, filters: [{ field, operator: 'eq', value }], sort: [], limit: 100, offset: 0,
  });
  return result?.records ?? [];
}

async function dbDeleteByFilters(host: RuntimeHost, collection: string, filters: Filter[]): Promise<void> {
  await callService(host, 'wafer-run/database', 'database.delete_by_filters', {
    collection, filters,
  });
}

async function dbCount(host: RuntimeHost, collection: string, filters: Filter[]): Promise<number> {
  const result = await callService(host, 'wafer-run/database', 'database.count', {
    collection, filters,
  });
  return result?.count ?? 0;
}

async function dbSum(host: RuntimeHost, collection: string, field: string, filters: Filter[]): Promise<number> {
  const result = await callService(host, 'wafer-run/database', 'database.sum', {
    collection, field, filters,
  });
  return result?.sum ?? 0;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function queryParam(msg: Message, key: string): string | undefined {
  return metaGet(msg.meta, `req.query.${key}`);
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

function asNumber(v: unknown, fallback: number): number {
  if (typeof v === 'number') return v;
  if (typeof v === 'string') {
    const n = parseFloat(v);
    return isNaN(n) ? fallback : n;
  }
  return fallback;
}

function uint8ArrayToBase64(arr: Uint8Array): string {
  let binary = '';
  for (let i = 0; i < arr.length; i++) {
    binary += String.fromCharCode(arr[i]);
  }
  return btoa(binary);
}

function base64ToUint8Array(b64: string): Uint8Array {
  const binary = atob(b64);
  const arr = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    arr[i] = binary.charCodeAt(i);
  }
  return arr;
}
