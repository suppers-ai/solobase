// TypeScript-native profile block handler.
// Returns empty profile sections array.

import type { Message, BlockResult } from '../types';

export function handle(msg: Message): BlockResult {
  // GET /profile/sections -> empty array
  const empty: never[] = [];
  return {
    action: 'respond',
    response: {
      data: new TextEncoder().encode(JSON.stringify(empty)),
      meta: [{ key: 'resp.content_type', value: 'application/json' }],
    },
    message: msg,
  };
}
