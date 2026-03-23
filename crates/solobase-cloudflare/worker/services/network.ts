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

  // Block userinfo in URL (bypass technique)
  if (parsed.username || parsed.password) return false;

  const hostname = parsed.hostname.toLowerCase();

  // Block localhost variants
  if (hostname === 'localhost' || hostname === '127.0.0.1' || hostname === '[::1]' || hostname === '0.0.0.0') return false;

  // Block cloud metadata hostnames
  if (hostname.includes('metadata') || hostname === 'instance' || hostname.endsWith('.internal')) return false;

  // Block IPv6 addresses that map to private/loopback IPv4
  if (hostname.startsWith('[')) {
    const inner = hostname.slice(1, -1).toLowerCase();
    // Block loopback
    if (inner === '::1' || inner === '0:0:0:0:0:0:0:1') return false;
    // Block link-local (fe80::/10)
    if (inner.startsWith('fe80:')) return false;
    // Block unique local (fc00::/7 = fc00:: and fd00::)
    if (inner.startsWith('fc') || inner.startsWith('fd')) return false;
    // Block IPv4-mapped IPv6 (::ffff:x.x.x.x)
    const v4Mapped = inner.match(/^::ffff:(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/);
    if (v4Mapped) {
      const octets = [Number(v4Mapped[1]), Number(v4Mapped[2]), Number(v4Mapped[3]), Number(v4Mapped[4])];
      if (isPrivateIPv4(octets)) return false;
    }
    // Block IPv4-compatible IPv6 (::x.x.x.x)
    const v4Compat = inner.match(/^::(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/);
    if (v4Compat) {
      const octets = [Number(v4Compat[1]), Number(v4Compat[2]), Number(v4Compat[3]), Number(v4Compat[4])];
      if (isPrivateIPv4(octets)) return false;
    }
    return true;
  }

  // Parse as IPv4
  const parts = hostname.split('.');
  if (parts.length === 4 && parts.every(p => /^\d+$/.test(p))) {
    const octets = parts.map(Number);
    if (octets.some(o => o > 255)) return false;
    if (isPrivateIPv4(octets)) return false;
  }

  // Block decimal IP notation (e.g., 2130706433 = 127.0.0.1)
  if (/^\d+$/.test(hostname)) {
    const num = Number(hostname);
    if (num >= 0 && num <= 0xFFFFFFFF) {
      const octets = [(num >> 24) & 0xFF, (num >> 16) & 0xFF, (num >> 8) & 0xFF, num & 0xFF];
      if (isPrivateIPv4(octets)) return false;
    }
  }

  // Block octal IP notation (e.g., 0177.0.0.1 = 127.0.0.1)
  if (parts.length === 4 && parts.every(p => /^0\d+$/.test(p) || /^\d+$/.test(p))) {
    const octets = parts.map(p => p.startsWith('0') && p.length > 1 ? parseInt(p, 8) : parseInt(p, 10));
    if (octets.every(o => !isNaN(o) && o >= 0 && o <= 255) && isPrivateIPv4(octets)) return false;
  }

  return true;
}

/** Check if an IPv4 address (as 4 octets) is in a private/reserved range. */
function isPrivateIPv4(octets: number[]): boolean {
  if (octets[0] === 0) return true;                                            // 0.0.0.0/8
  if (octets[0] === 10) return true;                                           // 10.0.0.0/8
  if (octets[0] === 127) return true;                                          // 127.0.0.0/8
  if (octets[0] === 169 && octets[1] === 254) return true;                    // 169.254.0.0/16 (link-local / cloud metadata)
  if (octets[0] === 172 && octets[1] >= 16 && octets[1] <= 31) return true;   // 172.16.0.0/12
  if (octets[0] === 192 && octets[1] === 168) return true;                    // 192.168.0.0/16
  return false;
}
