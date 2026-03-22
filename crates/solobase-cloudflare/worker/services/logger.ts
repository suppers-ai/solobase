// Logger service handler — forwards log messages to the console.

import type { Message, BlockResult } from '../types';

// ---------------------------------------------------------------------------
// Request type (wire format)
// ---------------------------------------------------------------------------

interface LogReq {
  message: string;
  fields?: Record<string, unknown>;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const decoder = new TextDecoder();

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

export function loggerHandler(msg: Message): BlockResult {
  let req: LogReq;
  try {
    req = msg.data.length > 0 ? JSON.parse(decoder.decode(msg.data)) : { message: '' };
  } catch {
    // If we cannot parse the body, log what we can and move on.
    req = { message: '(unparseable log data)' };
  }

  const text = req.message;
  const fields = req.fields;
  const extra = fields && Object.keys(fields).length > 0 ? ` ${JSON.stringify(fields)}` : '';

  switch (msg.kind) {
    case 'logger.debug':
      console.debug(`[debug] ${text}${extra}`);
      break;
    case 'logger.info':
      console.log(`[info] ${text}${extra}`);
      break;
    case 'logger.warn':
      console.warn(`[warn] ${text}${extra}`);
      break;
    case 'logger.error':
      console.error(`[error] ${text}${extra}`);
      break;
    default:
      return error('unimplemented', `unknown logger operation: ${msg.kind}`);
  }

  return successEmpty();
}
