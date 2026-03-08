/**
 * Minimal Stripe API mock server for E2E testing.
 *
 * Implements just enough of the Stripe API to test the checkout + webhook flow:
 *   POST /v1/checkout/sessions  — returns a fake session with a checkout URL
 *
 * Usage:
 *   import { startStripeMock, stopStripeMock, STRIPE_MOCK_PORT, WEBHOOK_SECRET } from './stripe-mock';
 *   beforeAll(() => startStripeMock());
 *   afterAll(() => stopStripeMock());
 */
import * as http from 'http';
import * as crypto from 'crypto';
import * as querystring from 'querystring';

export const STRIPE_MOCK_PORT = 12111;
export const STRIPE_TEST_KEY = 'sk_test_mock_key_for_e2e';
export const WEBHOOK_SECRET = 'whsec_test_mock_secret_for_e2e';

let server: http.Server | null = null;
const sessions: Map<string, any> = new Map();

function handler(req: http.IncomingMessage, res: http.ServerResponse) {
  const url = req.url || '';
  const method = req.method || 'GET';

  // Collect body
  const chunks: Buffer[] = [];
  req.on('data', (chunk: Buffer) => chunks.push(chunk));
  req.on('end', () => {
    const body = Buffer.concat(chunks).toString();
    const auth = req.headers['authorization'] || '';

    // Require Bearer auth
    if (!auth.startsWith('Bearer ')) {
      res.writeHead(401, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: { message: 'Missing API key', type: 'authentication_error' } }));
      return;
    }

    if (method === 'POST' && url === '/v1/checkout/sessions') {
      return handleCheckoutSession(body, res);
    }

    if (method === 'GET' && url?.startsWith('/v1/checkout/sessions/')) {
      const id = url.split('/').pop() || '';
      return handleGetSession(id, res);
    }

    res.writeHead(404, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: { message: `Not found: ${method} ${url}`, type: 'invalid_request_error' } }));
  });
}

function handleCheckoutSession(body: string, res: http.ServerResponse) {
  const params = querystring.parse(body);
  const sessionId = `cs_test_${crypto.randomBytes(16).toString('hex')}`;
  const paymentIntent = `pi_test_${crypto.randomBytes(16).toString('hex')}`;

  const session = {
    id: sessionId,
    object: 'checkout.session',
    url: `http://127.0.0.1:${STRIPE_MOCK_PORT}/checkout/${sessionId}`,
    payment_intent: paymentIntent,
    payment_status: 'unpaid',
    status: 'open',
    mode: params['mode'] || 'payment',
    metadata: {
      purchase_id: params['metadata[purchase_id]'] || '',
    },
    amount_total: parseInt(params['line_items[0][price_data][unit_amount]'] as string || '0', 10),
    currency: params['line_items[0][price_data][currency]'] || 'usd',
    success_url: params['success_url'] || '',
    cancel_url: params['cancel_url'] || '',
    created: Math.floor(Date.now() / 1000),
  };

  sessions.set(sessionId, { ...session, paymentIntent });

  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify(session));
}

function handleGetSession(id: string, res: http.ServerResponse) {
  const session = sessions.get(id);
  if (!session) {
    res.writeHead(404, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: { message: 'No such session', type: 'invalid_request_error' } }));
    return;
  }
  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify(session));
}

/**
 * Build a signed Stripe webhook event payload.
 * Returns { payload, signature } ready to POST to the webhook endpoint.
 */
export function buildWebhookEvent(
  type: string,
  data: Record<string, any>,
  secret: string = WEBHOOK_SECRET,
): { payload: string; signature: string } {
  const event = {
    id: `evt_test_${crypto.randomBytes(8).toString('hex')}`,
    object: 'event',
    type,
    data: { object: data },
    created: Math.floor(Date.now() / 1000),
  };
  const payload = JSON.stringify(event);
  const timestamp = Math.floor(Date.now() / 1000).toString();
  const signedPayload = `${timestamp}.${payload}`;
  const sig = crypto.createHmac('sha256', secret).update(signedPayload).digest('hex');
  const signature = `t=${timestamp},v1=${sig}`;

  return { payload, signature };
}

/**
 * Simulate a completed checkout: sends a checkout.session.completed webhook
 * event to the Solobase webhook endpoint.
 */
export async function simulateCheckoutComplete(
  baseUrl: string,
  sessionId: string,
  purchaseId: string,
) {
  const session = sessions.get(sessionId);
  const { payload, signature } = buildWebhookEvent('checkout.session.completed', {
    id: sessionId,
    payment_intent: session?.paymentIntent || `pi_test_${crypto.randomBytes(8).toString('hex')}`,
    payment_status: 'paid',
    status: 'complete',
    metadata: { purchase_id: purchaseId },
  });

  const res = await fetch(`${baseUrl}/ext/products/webhooks`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Stripe-Signature': signature,
    },
    body: payload,
  });
  return { status: res.status, body: await res.json().catch(() => null) };
}

export function startStripeMock(): Promise<void> {
  return new Promise((resolve, reject) => {
    server = http.createServer(handler);
    server.listen(STRIPE_MOCK_PORT, '127.0.0.1', () => {
      console.log(`Stripe mock listening on http://127.0.0.1:${STRIPE_MOCK_PORT}`);
      resolve();
    });
    server.on('error', reject);
  });
}

export function stopStripeMock(): Promise<void> {
  return new Promise((resolve) => {
    if (server) {
      server.close(() => resolve());
      server = null;
    } else {
      resolve();
    }
  });
}
