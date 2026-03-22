// TypeScript-native legalpages block handler (for testing without WASM).
// Calls host.callBlock() for database operations — same pattern as WASM blocks.

import type { Message, BlockResult } from '../types';
import type { RuntimeHost } from '../host';
import { metaGet } from '../convert';

const COLLECTION = 'block_legalpages_legal_documents';

export async function handle(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const action = metaGet(msg.meta, 'req.action') ?? '';
  const path = metaGet(msg.meta, 'req.resource') ?? '';

  switch (true) {
    // Public endpoints
    case action === 'retrieve' && path === '/b/legalpages/terms':
      return handleGetPublic(msg, host, 'terms');
    case action === 'retrieve' && path === '/b/legalpages/privacy':
      return handleGetPublic(msg, host, 'privacy');

    // Admin API
    case action === 'retrieve' && path === '/admin/legalpages/documents':
      return handleAdminList(msg, host);
    case action === 'retrieve' && path.startsWith('/admin/legalpages/documents/'):
      return handleAdminGet(msg, host);
    case action === 'create' && path === '/admin/legalpages/documents':
      return handleAdminCreate(msg, host);
    case action === 'update' && path.startsWith('/admin/legalpages/documents/') && path.endsWith('/publish'):
      return handleAdminPublish(msg, host);
    case action === 'update' && path.startsWith('/admin/legalpages/documents/'):
      return handleAdminUpdate(msg, host);
    case action === 'delete' && path.startsWith('/admin/legalpages/documents/'):
      return handleAdminDelete(msg, host);

    // Ext API aliases (same as admin, routed through admin-pipe)
    case action === 'retrieve' && path === '/b/legalpages/documents':
      return handleAdminList(msg, host);
    case action === 'create' && path === '/b/legalpages/documents':
      return handleAdminCreate(msg, host);

    default:
      return errResult('not-found', 'not found');
  }
}

// ---------------------------------------------------------------------------
// Public endpoint: serve published legal document as HTML
// ---------------------------------------------------------------------------

async function handleGetPublic(msg: Message, host: RuntimeHost, docType: string): Promise<BlockResult> {
  const result = await callService(host, 'wafer-run/database', 'database.list', {
    collection: COLLECTION,
    filters: [
      { field: 'doc_type', operator: 'eq', value: docType },
      { field: 'status', operator: 'eq', value: 'published' },
    ],
    sort: [{ field: 'version', desc: true }],
    limit: 1,
    offset: 0,
  });

  const records = result?.records ?? [];
  if (records.length === 0) {
    const title = docType === 'terms' ? 'Terms of Service' : 'Privacy Policy';
    const html = `<html><body><h1>${title}</h1><p>No ${docType} document has been published yet.</p></body></html>`;
    return htmlRespond(msg, html);
  }

  const record = records[0];
  const rawContent = record.data?.content ?? '';
  const content = sanitizeHtml(rawContent);
  const title = escapeHtml(record.data?.title ?? docType);

  const html = `<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>${title}</title>
<style>body{font-family:system-ui,sans-serif;max-width:800px;margin:40px auto;padding:0 20px;line-height:1.6;color:#333}h1{color:#111}</style>
</head><body><h1>${title}</h1><div>${content}</div></body></html>`;

  return htmlRespond(msg, html);
}

// ---------------------------------------------------------------------------
// Admin: list documents (paginated, optional type filter)
// ---------------------------------------------------------------------------

async function handleAdminList(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const { limit, offset } = paginationParams(msg, 20);
  const docType = metaGet(msg.meta, 'req.query.type') ?? '';

  const filters: { field: string; operator: string; value: string }[] = [];
  if (docType) {
    filters.push({ field: 'doc_type', operator: 'eq', value: docType });
  }

  const result = await callService(host, 'wafer-run/database', 'database.list', {
    collection: COLLECTION,
    filters,
    sort: [{ field: 'updated_at', desc: true }],
    limit,
    offset,
  });

  if (!result) return errResult('internal', 'Database error');
  return jsonRespond(msg, result);
}

// ---------------------------------------------------------------------------
// Admin: get single document
// ---------------------------------------------------------------------------

async function handleAdminGet(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const id = extractDocId(msg);
  if (!id) return errResult('invalid-argument', 'Missing document ID');

  const record = await dbGet(host, COLLECTION, id);
  if (!record) return errResult('not-found', 'Document not found');
  return jsonRespond(msg, record);
}

// ---------------------------------------------------------------------------
// Admin: create document
// ---------------------------------------------------------------------------

async function handleAdminCreate(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<{ doc_type: string; title: string; content: string }>(msg);
  if (!body || !body.doc_type || !body.title || body.content === undefined) {
    return errResult('invalid-argument', 'Invalid body: doc_type, title, and content are required');
  }

  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  const now = new Date().toISOString();

  const record = await dbCreate(host, COLLECTION, {
    doc_type: body.doc_type,
    title: body.title,
    content: body.content,
    status: 'draft',
    version: 1,
    created_by: userId,
    created_at: now,
    updated_at: now,
  });

  if (!record) return errResult('internal', 'Database error');
  return jsonRespond(msg, record);
}

// ---------------------------------------------------------------------------
// Admin: update document
// ---------------------------------------------------------------------------

async function handleAdminUpdate(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const id = extractDocId(msg);
  if (!id) return errResult('invalid-argument', 'Missing document ID');

  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  const data: Record<string, unknown> = { ...body, updated_at: new Date().toISOString() };

  const record = await dbUpdate(host, COLLECTION, id, data);
  if (!record) return errResult('not-found', 'Document not found');
  return jsonRespond(msg, record);
}

// ---------------------------------------------------------------------------
// Admin: publish document
// ---------------------------------------------------------------------------

async function handleAdminPublish(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const path = metaGet(msg.meta, 'req.resource') ?? '';
  // Extract ID from /admin/legalpages/documents/{id}/publish
  const stripped = path.replace('/admin/legalpages/documents/', '').replace('/publish', '');
  const id = stripped;
  if (!id) return errResult('invalid-argument', 'Missing document ID');

  // Get current document
  const doc = await dbGet(host, COLLECTION, id);
  if (!doc) return errResult('not-found', 'Document not found');

  const docType = doc.data?.doc_type ?? '';

  // Unpublish other documents of same type
  const existing = await callService(host, 'wafer-run/database', 'database.list', {
    collection: COLLECTION,
    filters: [
      { field: 'doc_type', operator: 'eq', value: docType },
      { field: 'status', operator: 'eq', value: 'published' },
    ],
    sort: [],
    limit: 100,
    offset: 0,
  });

  if (existing?.records) {
    for (const r of existing.records) {
      await dbUpdate(host, COLLECTION, r.id, { status: 'archived' });
    }
  }

  // Publish this one
  const now = new Date().toISOString();
  const record = await dbUpdate(host, COLLECTION, id, {
    status: 'published',
    published_at: now,
    updated_at: now,
  });

  if (!record) return errResult('internal', 'Database error');
  return jsonRespond(msg, record);
}

// ---------------------------------------------------------------------------
// Admin: delete document
// ---------------------------------------------------------------------------

async function handleAdminDelete(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const id = extractDocId(msg);
  if (!id) return errResult('invalid-argument', 'Missing document ID');

  const result = await callService(host, 'wafer-run/database', 'database.delete', {
    collection: COLLECTION,
    id,
  });

  if (!result) return errResult('not-found', 'Document not found');
  return jsonRespond(msg, { deleted: true });
}

// ---------------------------------------------------------------------------
// HTML sanitization (port of Rust sanitize_html)
// ---------------------------------------------------------------------------

const DANGEROUS_TAGS = [
  'script', 'iframe', 'object', 'embed', 'style', 'form',
  'input', 'textarea', 'select', 'button', 'meta', 'link',
  'base', 'svg', 'math', 'applet',
];

function sanitizeHtml(input: string): string {
  let s = input;

  // Strip dangerous tags and their contents
  for (const tag of DANGEROUS_TAGS) {
    let iterations = 0;
    while (iterations++ < 100) {
      const lower = s.toLowerCase();
      const open = `<${tag}`;
      const startIdx = lower.indexOf(open);
      if (startIdx === -1) break;

      const close = `</${tag}>`;
      const closeIdx = lower.indexOf(close, startIdx);
      let endIdx: number;
      if (closeIdx !== -1) {
        endIdx = closeIdx + close.length;
      } else {
        const gtIdx = s.indexOf('>', startIdx);
        endIdx = gtIdx !== -1 ? gtIdx + 1 : s.length;
      }
      s = s.substring(0, startIdx) + s.substring(endIdx);
    }
  }

  // Strip event handler attributes and dangerous URIs from remaining tags
  let result = '';
  let i = 0;
  while (i < s.length) {
    if (s[i] === '<') {
      const endIdx = s.indexOf('>', i);
      if (endIdx !== -1) {
        const tagContent = s.substring(i, endIdx + 1);
        result += removeDangerousAttrs(tagContent);
        i = endIdx + 1;
      } else {
        result += s[i];
        i++;
      }
    } else {
      result += s[i];
      i++;
    }
  }

  return result;
}

function removeDangerousAttrs(tag: string): string {
  // Find end of tag name
  let nameEnd = 1;
  while (nameEnd < tag.length && tag[nameEnd] !== ' ' && tag[nameEnd] !== '>' && tag[nameEnd] !== '/') {
    nameEnd++;
  }

  if (nameEnd >= tag.length || tag[nameEnd] === '>') {
    return tag;
  }

  const tagName = tag.substring(0, nameEnd);
  const rest = tag.substring(nameEnd);
  let result = tagName;
  let pos = 0;

  while (pos < rest.length) {
    // Skip whitespace
    while (pos < rest.length && (rest[pos] === ' ' || rest[pos] === '\t' || rest[pos] === '\n')) {
      result += rest[pos];
      pos++;
    }
    if (pos >= rest.length) break;
    if (rest[pos] === '>' || (rest[pos] === '/' && pos + 1 < rest.length && rest[pos + 1] === '>')) {
      result += rest.substring(pos);
      break;
    }

    // Read attribute name
    const attrStart = pos;
    while (pos < rest.length && rest[pos] !== '=' && rest[pos] !== '>' && rest[pos] !== ' ' && rest[pos] !== '\t') {
      pos++;
    }
    const attrName = rest.substring(attrStart, pos).toLowerCase();

    const isDangerous = attrName.startsWith('on') || attrName === 'srcdoc' || attrName === 'formaction';

    // Read = and value if present
    let attrEnd = pos;
    if (pos < rest.length && rest[pos] === '=') {
      pos++; // skip =
      if (pos < rest.length && (rest[pos] === '"' || rest[pos] === "'")) {
        const quote = rest[pos];
        pos++;
        while (pos < rest.length && rest[pos] !== quote) pos++;
        if (pos < rest.length) pos++; // skip closing quote
      } else {
        while (pos < rest.length && rest[pos] !== ' ' && rest[pos] !== '>') pos++;
      }
      attrEnd = pos;

      const attrValue = rest.substring(attrStart, attrEnd).toLowerCase();
      const hasDangerousUri = attrValue.includes('javascript:') || attrValue.includes('data:text/html') || attrValue.includes('vbscript:');

      if (!isDangerous && !hasDangerousUri) {
        result += rest.substring(attrStart, attrEnd);
      }
    } else if (!isDangerous) {
      result += rest.substring(attrStart, attrEnd);
    }
  }

  return result;
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

function extractDocId(msg: Message): string {
  const path = metaGet(msg.meta, 'req.resource') ?? '';
  const suffix = path.startsWith('/admin/legalpages/documents/')
    ? path.substring('/admin/legalpages/documents/'.length)
    : path.startsWith('/b/legalpages/documents/')
      ? path.substring('/b/legalpages/documents/'.length)
      : '';
  // Strip trailing /publish or /
  return suffix.split('/')[0] || '';
}

function paginationParams(msg: Message, defaultLimit: number): { limit: number; offset: number } {
  const page = parseInt(metaGet(msg.meta, 'req.query.page') ?? '1', 10) || 1;
  const limit = parseInt(metaGet(msg.meta, 'req.query.page_size') ?? String(defaultLimit), 10) || defaultLimit;
  const offset = (page - 1) * limit;
  return { limit, offset };
}

function parseBody<T>(msg: Message): T | null {
  try { return JSON.parse(new TextDecoder().decode(msg.data)) as T; } catch { return null; }
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
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

function htmlRespond(msg: Message, html: string): BlockResult {
  return {
    action: 'respond',
    response: {
      data: new TextEncoder().encode(html),
      meta: [{ key: 'resp.content_type', value: 'text/html; charset=utf-8' }],
    },
    message: msg,
  };
}

function errResult(code: string, message: string): BlockResult {
  return { action: 'error', error: { code: code as any, message, meta: [] } };
}
