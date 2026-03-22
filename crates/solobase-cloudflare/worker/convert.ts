// HTTP <-> WAFER Message conversion for the TypeScript Cloudflare Worker.
//
// Ports the logic from src/convert.rs to TypeScript. Converts between
// standard Web API Request/Response and WAFER Message/BlockResult types.

import type { MetaEntry, Message, BlockResult, ErrorCode } from './types';

// ---------------------------------------------------------------------------
// Meta key constants (from wafer-run/common/definitions/meta_keys.toml)
// ---------------------------------------------------------------------------

export const META = {
  REQ_ACTION: 'req.action',
  REQ_RESOURCE: 'req.resource',
  REQ_CLIENT_IP: 'req.client.ip',
  REQ_CONTENT_TYPE: 'req.content_type',
  REQ_METHOD: 'req.method',
  REQ_PATH: 'req.path',
  REQ_QUERY_PREFIX: 'req.query.',
  RESP_STATUS: 'resp.status',
  RESP_CONTENT_TYPE: 'resp.content_type',
  RESP_HEADER_PREFIX: 'resp.header.',
  RESP_COOKIE_PREFIX: 'resp.set_cookie.',
  AUTH_USER_ID: 'auth.user_id',
  AUTH_USER_EMAIL: 'auth.user_email',
  AUTH_USER_ROLES: 'auth.user_roles',
} as const;

// ---------------------------------------------------------------------------
// Request -> Message
// ---------------------------------------------------------------------------

/**
 * Convert a standard Web Request into a WAFER Message.
 *
 * Extracts method, path, query params, headers, body, and remote address.
 * Maps HTTP method to a canonical action (retrieve, create, update, delete).
 */
export function requestToMessage(
  request: Request,
  url: URL,
  body: Uint8Array,
  remoteAddr: string,
): Message {
  const method = request.method;
  const path = url.pathname;
  const query = url.search.replace(/^\?/, '');

  const meta: MetaEntry[] = [];

  // HTTP-specific meta
  meta.push({ key: 'http.method', value: method });
  meta.push({ key: 'http.path', value: path });
  meta.push({ key: 'http.raw_query', value: query });
  meta.push({ key: 'http.remote_addr', value: remoteAddr });

  const contentType = request.headers.get('content-type') ?? '';
  meta.push({ key: 'http.content_type', value: contentType });

  const host = request.headers.get('host') ?? '';
  meta.push({ key: 'http.host', value: host });

  // Normalized request meta
  const action = methodToAction(method);
  meta.push({ key: META.REQ_ACTION, value: action });
  meta.push({ key: META.REQ_RESOURCE, value: path });
  meta.push({ key: META.REQ_CLIENT_IP, value: remoteAddr });
  meta.push({ key: META.REQ_CONTENT_TYPE, value: contentType });

  // Copy all headers to meta
  for (const [name, value] of request.headers.entries()) {
    meta.push({ key: `http.header.${name}`, value });
  }

  // Parse query params into meta entries
  if (query) {
    for (const pair of query.split('&')) {
      const eqIdx = pair.indexOf('=');
      if (eqIdx === -1) continue;
      const key = pair.substring(0, eqIdx);
      const val = decodeURIComponentSafe(pair.substring(eqIdx + 1));
      meta.push({ key: `http.query.${key}`, value: val });
      meta.push({ key: `${META.REQ_QUERY_PREFIX}${key}`, value: val });
    }
  }

  return {
    kind: `${method}:${path}`,
    data: body,
    meta,
  };
}

// ---------------------------------------------------------------------------
// BlockResult -> Response
// ---------------------------------------------------------------------------

/**
 * Convert a WAFER BlockResult into a standard Web Response.
 */
export function blockResultToResponse(result: BlockResult): Response {
  switch (result.action) {
    case 'respond': {
      const respData = result.response?.data ?? new Uint8Array(0);
      const respMeta = result.response?.meta ?? [];

      const status = getStatus(respMeta, 200);
      const contentType =
        metaGet(respMeta, META.RESP_CONTENT_TYPE) ??
        metaGet(respMeta, 'Content-Type') ??
        'application/json';

      const headers = new Headers({ 'Content-Type': contentType });
      applyMetaHeaders(headers, respMeta);
      if (result.message) {
        applyMetaHeaders(headers, result.message.meta);
      }

      return new Response(respData, { status, headers });
    }

    case 'error': {
      const errMeta = result.error?.meta ?? [];
      const status = getErrorStatus(result.error ?? undefined, errMeta);

      const body = result.error
        ? JSON.stringify({
            error: result.error.code,
            message: result.error.message,
          })
        : '{}';

      const headers = new Headers({ 'Content-Type': 'application/json' });
      applyMetaHeaders(headers, errMeta);
      if (result.message) {
        applyMetaHeaders(headers, result.message.meta);
      }

      return new Response(body, { status, headers });
    }

    case 'drop': {
      const headers = new Headers();
      if (result.message) {
        applyMetaHeaders(headers, result.message.meta);
      }
      return new Response(null, { status: 204, headers });
    }

    case 'continue': {
      const body = result.message?.data ?? new Uint8Array(0);
      const headers = new Headers({ 'Content-Type': 'application/json' });
      if (result.message) {
        applyMetaHeaders(headers, result.message.meta);
      }
      return new Response(body, { status: 200, headers });
    }

    default:
      return new Response(
        JSON.stringify({ error: 'internal', message: 'unknown action' }),
        { status: 500, headers: { 'Content-Type': 'application/json' } },
      );
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function methodToAction(method: string): string {
  switch (method) {
    case 'GET':
    case 'HEAD':
      return 'retrieve';
    case 'POST':
      return 'create';
    case 'PUT':
    case 'PATCH':
      return 'update';
    case 'DELETE':
      return 'delete';
    default:
      return 'execute';
  }
}

/** Find the first meta entry matching a key. */
export function metaGet(meta: MetaEntry[], key: string): string | undefined {
  return meta.find((e) => e.key === key)?.value;
}

/** Set or overwrite a meta entry by key. */
export function metaSet(meta: MetaEntry[], key: string, value: string): void {
  const idx = meta.findIndex((e) => e.key === key);
  if (idx >= 0) {
    meta[idx].value = value;
  } else {
    meta.push({ key, value });
  }
}

function getStatus(meta: MetaEntry[], fallback: number): number {
  const raw =
    metaGet(meta, META.RESP_STATUS) ?? metaGet(meta, 'http.status');
  if (raw) {
    const parsed = parseInt(raw, 10);
    if (!isNaN(parsed)) return parsed;
  }
  return fallback;
}

function getErrorStatus(
  error: { code: ErrorCode; meta: MetaEntry[] } | undefined,
  meta: MetaEntry[],
): number {
  const fromMeta = getStatus(meta, 0);
  if (fromMeta > 0) return fromMeta;
  if (error) return errorCodeToHttpStatus(error.code);
  return 500;
}

/** Map WAFER error codes to HTTP status codes. */
export function errorCodeToHttpStatus(code: ErrorCode): number {
  switch (code) {
    case 'ok':
      return 200;
    case 'cancelled':
      return 499;
    case 'invalid-argument':
      return 400;
    case 'deadline-exceeded':
      return 504;
    case 'not-found':
      return 404;
    case 'already-exists':
      return 409;
    case 'permission-denied':
      return 403;
    case 'resource-exhausted':
      return 429;
    case 'failed-precondition':
      return 412;
    case 'aborted':
      return 409;
    case 'out-of-range':
      return 400;
    case 'unimplemented':
      return 501;
    case 'internal':
      return 500;
    case 'unavailable':
      return 503;
    case 'data-loss':
      return 500;
    case 'unauthenticated':
      return 401;
    default:
      return 500;
  }
}

/** Apply response headers and Set-Cookie from meta entries to a Headers object. */
function applyMetaHeaders(headers: Headers, meta: MetaEntry[]): void {
  for (const entry of meta) {
    if (
      entry.key.startsWith(META.RESP_COOKIE_PREFIX) ||
      entry.key.startsWith('http.resp.set-cookie.')
    ) {
      headers.append('Set-Cookie', entry.value);
    } else {
      const name =
        stripPrefix(entry.key, META.RESP_HEADER_PREFIX) ??
        stripPrefix(entry.key, 'http.resp.header.');
      if (name) {
        headers.set(name, entry.value);
      }
    }
  }
}

function stripPrefix(s: string, prefix: string): string | null {
  return s.startsWith(prefix) ? s.substring(prefix.length) : null;
}

function decodeURIComponentSafe(s: string): string {
  try {
    return decodeURIComponent(s.replace(/\+/g, ' '));
  } catch {
    return s;
  }
}
