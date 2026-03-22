// Network service handler — outbound HTTP requests via fetch().

import type { Message, BlockResult } from '../types';

// ---------------------------------------------------------------------------
// Request / response types (wire format)
// ---------------------------------------------------------------------------

interface DoReq {
  method: string;
  url: string;
  headers?: Record<string, string>;
  body?: number[] | null;  // byte array serialized as JSON array
}

interface DoResp {
  status_code: number;
  headers: Record<string, string[]>;
  body: number[];
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

function error(code: string, message: string): BlockResult {
  return {
    action: 'error',
    error: { code: code as any, message, meta: [] },
  };
}

// ---------------------------------------------------------------------------
// Main handler
// ---------------------------------------------------------------------------

export async function networkHandler(msg: Message): Promise<BlockResult> {
  let req: any;
  try {
    req = msg.data.length > 0 ? JSON.parse(decoder.decode(msg.data)) : {};
  } catch (e) {
    return error('invalid-argument', `failed to parse request: ${e}`);
  }

  try {
    switch (msg.kind) {
      case 'network.do': {
        const { method, url, headers: reqHeaders = {}, body: reqBody } = req as DoReq;

        // SSRF protection: block requests to private/internal networks
        if (!isAllowedUrl(url)) {
          return error('permission-denied', 'requests to private/internal networks are not allowed');
        }

        const init: RequestInit = { method: method.toUpperCase(), redirect: 'manual' };

        // Set headers
        const fetchHeaders = new Headers();
        for (const [k, v] of Object.entries(reqHeaders)) {
          fetchHeaders.set(k, v);
        }
        init.headers = fetchHeaders;

        // Set body (not for GET/HEAD)
        if (reqBody && reqBody.length > 0) {
          init.body = new Uint8Array(reqBody);
        }

        const resp = await fetch(url, init);

        const statusCode = resp.status;
        const respBody = await resp.arrayBuffer();

        // Collect response headers (values as arrays to match wire format)
        const respHeaders: Record<string, string[]> = {};
        resp.headers.forEach((v, k) => {
          if (!respHeaders[k]) respHeaders[k] = [];
          respHeaders[k].push(v);
        });

        const result: DoResp = {
          status_code: statusCode,
          headers: respHeaders,
          body: Array.from(new Uint8Array(respBody)),
        };

        return success(result);
      }

      default:
        return error('unimplemented', `unknown network operation: ${msg.kind}`);
    }
  } catch (e: any) {
    return error('unavailable', 'network request failed');
  }
}

/** Block requests to private/internal/localhost networks (SSRF protection). */
function isAllowedUrl(url: string): boolean {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    return false;
  }

  // Must be http or https
  if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') return false;

  const hostname = parsed.hostname.toLowerCase();

  // Block localhost
  if (hostname === 'localhost' || hostname === '127.0.0.1' || hostname === '[::1]') return false;

  // Block private IP ranges
  const parts = hostname.split('.');
  if (parts.length === 4 && parts.every(p => /^\d+$/.test(p))) {
    const octets = parts.map(Number);
    if (octets[0] === 10) return false;                                         // 10.0.0.0/8
    if (octets[0] === 172 && octets[1] >= 16 && octets[1] <= 31) return false;  // 172.16.0.0/12
    if (octets[0] === 192 && octets[1] === 168) return false;                   // 192.168.0.0/16
    if (octets[0] === 169 && octets[1] === 254) return false;                   // 169.254.0.0/16 (link-local / metadata)
    if (octets[0] === 0) return false;                                          // 0.0.0.0/8
  }

  // Block IPv6 link-local and loopback
  if (hostname.startsWith('[fe80:') || hostname.startsWith('[::1]')) return false;

  return true;
}
