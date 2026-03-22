// Crypto service handler — password hashing (PBKDF2), JWT (HMAC-SHA256), random bytes.
// Uses only the Web Crypto API available in Cloudflare Workers.

import type { Message, BlockResult } from '../types';

// ---------------------------------------------------------------------------
// Request / response types (wire format)
// ---------------------------------------------------------------------------

interface HashReq {
  password: string;
}

interface CompareHashReq {
  password: string;
  hash: string;
}

interface SignReq {
  claims: Record<string, unknown>;
  expiry_secs?: number;
}

interface VerifyReq {
  token: string;
}

interface RandomBytesReq {
  n?: number;
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

function hexEncode(buf: ArrayBuffer | Uint8Array): string {
  return Array.from(buf instanceof Uint8Array ? buf : new Uint8Array(buf))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

function hexDecode(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

/** Base64url encode (no padding). */
function b64urlEncode(data: Uint8Array): string {
  const binStr = Array.from(data)
    .map((b) => String.fromCharCode(b))
    .join('');
  return btoa(binStr).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

/** Base64url decode. */
function b64urlDecode(str: string): Uint8Array {
  let base64 = str.replace(/-/g, '+').replace(/_/g, '/');
  // Pad to multiple of 4
  while (base64.length % 4 !== 0) base64 += '=';
  const binStr = atob(base64);
  return Uint8Array.from(binStr, (c) => c.charCodeAt(0));
}

// ---------------------------------------------------------------------------
// PBKDF2 password hashing
// ---------------------------------------------------------------------------

const PBKDF2_ITERATIONS = 100_000;

async function pbkdf2Hash(password: string): Promise<string> {
  const salt = crypto.getRandomValues(new Uint8Array(16));
  const keyMaterial = await crypto.subtle.importKey(
    'raw',
    encoder.encode(password),
    'PBKDF2',
    false,
    ['deriveBits'],
  );
  const derived = await crypto.subtle.deriveBits(
    { name: 'PBKDF2', salt, iterations: PBKDF2_ITERATIONS, hash: 'SHA-256' },
    keyMaterial,
    256,
  );
  return `pbkdf2:${PBKDF2_ITERATIONS}:${hexEncode(salt)}:${hexEncode(derived)}`;
}

async function pbkdf2Verify(password: string, hashStr: string): Promise<boolean> {
  const parts = hashStr.split(':');
  if (parts.length !== 4 || parts[0] !== 'pbkdf2') return false;

  const iterations = parseInt(parts[1], 10);
  const salt = hexDecode(parts[2]);
  const expected = parts[3];

  const keyMaterial = await crypto.subtle.importKey(
    'raw',
    encoder.encode(password),
    'PBKDF2',
    false,
    ['deriveBits'],
  );
  const derived = await crypto.subtle.deriveBits(
    { name: 'PBKDF2', salt, iterations, hash: 'SHA-256' },
    keyMaterial,
    256,
  );

  // Constant-time comparison
  const actual = hexEncode(derived);
  if (actual.length !== expected.length) return false;
  let diff = 0;
  for (let i = 0; i < actual.length; i++) {
    diff |= actual.charCodeAt(i) ^ expected.charCodeAt(i);
  }
  return diff === 0;
}

// ---------------------------------------------------------------------------
// JWT (HMAC-SHA256)
// ---------------------------------------------------------------------------

async function getHmacKey(secret: string): Promise<CryptoKey> {
  return crypto.subtle.importKey(
    'raw',
    encoder.encode(secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign', 'verify'],
  );
}

async function jwtSign(
  claims: Record<string, unknown>,
  expirySecs: number,
  secret: string,
): Promise<string> {
  const header = { alg: 'HS256', typ: 'JWT' };
  const now = Math.floor(Date.now() / 1000);
  const payload = { ...claims, iat: now, exp: now + expirySecs };

  const headerB64 = b64urlEncode(encoder.encode(JSON.stringify(header)));
  const payloadB64 = b64urlEncode(encoder.encode(JSON.stringify(payload)));
  const signingInput = `${headerB64}.${payloadB64}`;

  const key = await getHmacKey(secret);
  const sig = await crypto.subtle.sign('HMAC', key, encoder.encode(signingInput));
  const sigB64 = b64urlEncode(new Uint8Array(sig));

  return `${signingInput}.${sigB64}`;
}

async function jwtVerify(
  token: string,
  secret: string,
): Promise<Record<string, unknown>> {
  const parts = token.split('.');
  if (parts.length !== 3) throw new Error('invalid JWT format');

  const [headerB64, payloadB64, sigB64] = parts;
  const signingInput = `${headerB64}.${payloadB64}`;

  const key = await getHmacKey(secret);
  const sig = b64urlDecode(sigB64);
  const valid = await crypto.subtle.verify('HMAC', key, sig, encoder.encode(signingInput));
  if (!valid) throw new Error('invalid JWT signature');

  const payload = JSON.parse(decoder.decode(b64urlDecode(payloadB64)));

  // Check expiry
  if (payload.exp && typeof payload.exp === 'number') {
    const now = Math.floor(Date.now() / 1000);
    if (now >= payload.exp) throw new Error('JWT expired');
  }

  return payload;
}

// ---------------------------------------------------------------------------
// Main handler
// ---------------------------------------------------------------------------

export async function cryptoHandler(
  jwtSecret: string,
  msg: Message,
): Promise<BlockResult> {
  let req: any;
  try {
    req = msg.data.length > 0 ? JSON.parse(decoder.decode(msg.data)) : {};
  } catch (e) {
    return error('invalid-argument', `failed to parse request: ${e}`);
  }

  try {
    switch (msg.kind) {
      // ----- HASH -----
      case 'crypto.hash': {
        const { password } = req as HashReq;
        const hash = await pbkdf2Hash(password);
        return success({ hash });
      }

      // ----- COMPARE_HASH -----
      case 'crypto.compare_hash': {
        const { password, hash } = req as CompareHashReq;
        const matches = await pbkdf2Verify(password, hash);
        return success({ match: matches });
      }

      // ----- SIGN (JWT) -----
      case 'crypto.sign': {
        const { claims, expiry_secs = 3600 } = req as SignReq;
        const token = await jwtSign(claims, expiry_secs, jwtSecret);
        return success({ token });
      }

      // ----- VERIFY (JWT) -----
      case 'crypto.verify': {
        const { token } = req as VerifyReq;
        try {
          const claims = await jwtVerify(token, jwtSecret);
          return success({ claims });
        } catch (e: any) {
          return error('unauthenticated', e?.message ?? 'invalid token');
        }
      }

      // ----- RANDOM_BYTES -----
      case 'crypto.random_bytes': {
        const { n = 32 } = req as RandomBytesReq;
        if (n > 1_048_576) {
          return error('invalid-argument', 'random_bytes n exceeds 1 MiB limit');
        }
        const bytes = new Uint8Array(n);
        crypto.getRandomValues(bytes);
        return success({ bytes: Array.from(bytes) });
      }

      default:
        return error('unimplemented', `unknown crypto operation: ${msg.kind}`);
    }
  } catch (e: any) {
    console.error('Crypto error:', e);
    return error('internal', 'crypto operation failed');
  }
}
