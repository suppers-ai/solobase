// R2 Storage service handler — implements storage.* operations against Cloudflare R2.

import type { Message, BlockResult, TenantConfig } from '../types';

// ---------------------------------------------------------------------------
// Request / response types (wire format)
// ---------------------------------------------------------------------------

interface PutReq {
  folder: string;
  key: string;
  data: number[];  // byte array serialized as JSON array
  content_type?: string;
}

interface GetReq {
  folder: string;
  key: string;
}

interface DeleteReq {
  folder: string;
  key: string;
}

interface ListReq {
  folder: string;
  prefix?: string;
  limit?: number;
  offset?: number;
}

interface CreateFolderReq {
  name: string;
  public?: boolean;
}

interface DeleteFolderReq {
  name: string;
}

interface ObjectInfo {
  key: string;
  size: number;
  content_type: string;
  last_modified: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const encoder = new TextEncoder();
const decoder = new TextDecoder();

function success(data: unknown): BlockResult {
  return {
    action: 'respond',
    response: {
      data: encoder.encode(JSON.stringify(data)),
      meta: [],
    },
  };
}

function successEmpty(): BlockResult {
  return {
    action: 'respond',
    response: { data: new Uint8Array(0), meta: [] },
  };
}

function error(code: string, message: string): BlockResult {
  return {
    action: 'error',
    error: { code: code as any, message, meta: [] },
  };
}

function validatePathComponent(name: string): boolean {
  return !!name && !name.includes('..') && !name.startsWith('/') && !name.includes('\0') && !name.includes('\\');
}

function prefixedKey(tenantId: string, folder: string, key: string): string {
  if (!validatePathComponent(folder) || !validatePathComponent(key)) {
    throw new Error('invalid path component');
  }
  return `${tenantId}/${folder}/${key}`;
}

function folderPrefix(tenantId: string, folder: string): string {
  if (!validatePathComponent(folder)) {
    throw new Error('invalid folder name');
  }
  return `${tenantId}/${folder}/`;
}

// ---------------------------------------------------------------------------
// Main handler
// ---------------------------------------------------------------------------

export async function r2Handler(
  bucket: R2Bucket,
  tenantId: string,
  msg: Message,
  db?: D1Database,
  tenant?: TenantConfig,
): Promise<BlockResult> {
  let req: any;
  try {
    req = msg.data.length > 0 ? JSON.parse(decoder.decode(msg.data)) : {};
  } catch (e) {
    return error('invalid-argument', `failed to parse request: ${e}`);
  }

  try {
    switch (msg.kind) {
      // ----- PUT -----
      case 'storage.put': {
        const { folder, key, data, content_type } = req as PutReq;
        // Check storage limit before upload
        if (db && tenant && tenant.plan !== 'platform') {
          const { checkStorageLimit } = await import('../usage');
          const overLimit = await checkStorageLimit(db, tenant, data?.length ?? 0);
          if (overLimit) return error('resource-exhausted', overLimit);
        }
        const r2Key = prefixedKey(tenantId, folder, key);
        const body = new Uint8Array(data);
        await bucket.put(r2Key, body, {
          httpMetadata: {
            contentType: content_type ?? 'application/octet-stream',
          },
        });
        return successEmpty();
      }

      // ----- GET -----
      case 'storage.get': {
        const { folder, key } = req as GetReq;
        const r2Key = prefixedKey(tenantId, folder, key);
        const obj = await bucket.get(r2Key);
        if (!obj) return error('not-found', 'object not found');

        const bytes = await obj.arrayBuffer();
        const info: ObjectInfo = {
          key,
          size: bytes.byteLength,
          content_type: obj.httpMetadata?.contentType ?? 'application/octet-stream',
          last_modified: (obj.uploaded ?? new Date()).toISOString(),
        };

        return success({
          data: Array.from(new Uint8Array(bytes)),
          info,
        });
      }

      // ----- DELETE -----
      case 'storage.delete': {
        const { folder, key } = req as DeleteReq;
        const r2Key = prefixedKey(tenantId, folder, key);
        await bucket.delete(r2Key);
        return successEmpty();
      }

      // ----- LIST -----
      case 'storage.list': {
        const { folder, prefix: extraPrefix = '', limit: rawLimit, offset: _offset } = req as ListReq;
        const fullPrefix = folderPrefix(tenantId, folder) + extraPrefix;
        const limit = rawLimit && rawLimit > 0 ? rawLimit : 100;
        const fpLen = folderPrefix(tenantId, folder).length;

        const listed = await bucket.list({ prefix: fullPrefix, limit });

        const objects: ObjectInfo[] = listed.objects.map((obj) => {
          const relKey = obj.key.length > fpLen ? obj.key.slice(fpLen) : obj.key;
          return {
            key: relKey,
            size: obj.size,
            content_type: obj.httpMetadata?.contentType ?? 'application/octet-stream',
            last_modified: (obj.uploaded ?? new Date()).toISOString(),
          };
        });

        return success({
          objects,
          total_count: objects.length,
        });
      }

      // ----- CREATE_FOLDER -----
      case 'storage.create_folder': {
        // R2 has no native folder concept — prefix-based, so this is a no-op.
        return successEmpty();
      }

      // ----- DELETE_FOLDER -----
      case 'storage.delete_folder': {
        const { name } = req as DeleteFolderReq;
        const prefix = folderPrefix(tenantId, name);

        // List and delete all objects under the folder prefix.
        let cursor: string | undefined;
        do {
          const listed = await bucket.list({
            prefix,
            limit: 1000,
            ...(cursor ? { cursor } : {}),
          });
          if (listed.objects.length > 0) {
            const keys = listed.objects.map((o) => o.key);
            await bucket.delete(keys);
          }
          cursor = listed.truncated ? listed.cursor : undefined;
        } while (cursor);

        return successEmpty();
      }

      // ----- LIST_FOLDERS -----
      case 'storage.list_folders': {
        // List distinct folder prefixes under the tenant.
        const tenantPrefix = `${tenantId}/`;
        const listed = await bucket.list({ prefix: tenantPrefix, delimiter: '/' });
        const folders = (listed.delimitedPrefixes ?? []).map((dp) => {
          const name = dp.endsWith('/') ? dp.slice(tenantPrefix.length, -1) : dp.slice(tenantPrefix.length);
          return {
            name,
            public: false,
            created_at: new Date().toISOString(),
          };
        });
        return success(folders);
      }

      default:
        return error('unimplemented', `unknown storage operation: ${msg.kind}`);
    }
  } catch (e: any) {
    console.error('R2 error:', e);
    return error('internal', 'storage operation failed');
  }
}
