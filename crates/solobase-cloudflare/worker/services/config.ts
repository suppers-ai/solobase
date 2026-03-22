// Config service handler — reads configuration from environment variables.

import type { Message, BlockResult } from '../types';

// ---------------------------------------------------------------------------
// Request types (wire format)
// ---------------------------------------------------------------------------

interface GetReq {
  key: string;
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

// ---------------------------------------------------------------------------
// Main handler
// ---------------------------------------------------------------------------

export function configHandler(
  envVars: Record<string, string>,
  msg: Message,
): BlockResult {
  let req: any;
  try {
    req = msg.data.length > 0 ? JSON.parse(decoder.decode(msg.data)) : {};
  } catch (e) {
    return error('invalid-argument', `failed to parse request: ${e}`);
  }

  switch (msg.kind) {
    case 'config.get': {
      let key: string | undefined;

      // Try parsing key from body first, then fall back to meta.
      if (req && typeof req.key === 'string') {
        key = req.key;
      } else {
        const metaEntry = msg.meta.find((m) => m.key === 'key');
        if (metaEntry) key = metaEntry.value;
      }

      if (!key) {
        return error('invalid-argument', "config.get requires a 'key'");
      }

      const value = envVars[key];
      if (value === undefined) {
        return error('not-found', `config key not found: ${key}`);
      }

      return success({ value });
    }

    case 'config.set': {
      // Environment variables are immutable at runtime in CF Workers — no-op.
      return successEmpty();
    }

    default:
      return error('unimplemented', `unknown config operation: ${msg.kind}`);
  }
}
