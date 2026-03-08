#!/usr/bin/env node
/**
 * Standalone Stripe mock server for local development.
 * Run directly: node scripts/stripe-mock.mjs
 */
import * as http from 'node:http';
import * as crypto from 'node:crypto';
import * as querystring from 'node:querystring';

const PORT = parseInt(process.env.STRIPE_MOCK_PORT || '12111', 10);
const sessions = new Map();

function handler(req, res) {
  const url = req.url || '';
  const method = req.method || 'GET';
  const chunks = [];

  req.on('data', (chunk) => chunks.push(chunk));
  req.on('end', () => {
    const body = Buffer.concat(chunks).toString();
    const auth = req.headers['authorization'] || '';

    // Require Bearer auth (like real Stripe)
    if (!auth.startsWith('Bearer ')) {
      res.writeHead(401, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: { message: 'Missing API key', type: 'authentication_error' } }));
      return;
    }

    // POST /v1/checkout/sessions — create a checkout session
    if (method === 'POST' && url === '/v1/checkout/sessions') {
      const params = querystring.parse(body);
      const sessionId = `cs_test_${crypto.randomBytes(16).toString('hex')}`;
      const paymentIntent = `pi_test_${crypto.randomBytes(16).toString('hex')}`;

      const session = {
        id: sessionId,
        object: 'checkout.session',
        url: `http://127.0.0.1:${PORT}/checkout/${sessionId}`,
        payment_intent: paymentIntent,
        payment_status: 'unpaid',
        status: 'open',
        mode: params['mode'] || 'payment',
        metadata: { purchase_id: params['metadata[purchase_id]'] || '' },
        amount_total: parseInt(params['line_items[0][price_data][unit_amount]'] || '0', 10),
        currency: params['line_items[0][price_data][currency]'] || 'usd',
        success_url: params['success_url'] || '',
        cancel_url: params['cancel_url'] || '',
        created: Math.floor(Date.now() / 1000),
      };
      sessions.set(sessionId, { ...session, paymentIntent });

      console.log(`  checkout session created: ${sessionId} (purchase: ${session.metadata.purchase_id})`);
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(session));
      return;
    }

    // GET /v1/checkout/sessions/:id — retrieve a session
    if (method === 'GET' && url?.startsWith('/v1/checkout/sessions/')) {
      const id = url.split('/').pop() || '';
      const session = sessions.get(id);
      if (!session) {
        res.writeHead(404, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: { message: 'No such session', type: 'invalid_request_error' } }));
        return;
      }
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(session));
      return;
    }

    // GET /checkout/:id — fake checkout page (for browser testing)
    if (method === 'GET' && url?.startsWith('/checkout/')) {
      const id = url.split('/').pop() || '';
      const session = sessions.get(id);
      if (!session) {
        res.writeHead(404, { 'Content-Type': 'text/html' });
        res.end('<h1>Session not found</h1>');
        return;
      }
      res.writeHead(200, { 'Content-Type': 'text/html' });
      res.end(`<!DOCTYPE html>
<html><head><title>Test Checkout</title></head>
<body style="font-family:system-ui;max-width:500px;margin:40px auto;padding:20px">
  <h1>Mock Stripe Checkout</h1>
  <p>Session: <code>${session.id}</code></p>
  <p>Amount: ${(session.amount_total / 100).toFixed(2)} ${session.currency.toUpperCase()}</p>
  <p>Purchase ID: <code>${session.metadata.purchase_id}</code></p>
  <hr>
  <p>This is a mock checkout page. Click below to simulate payment:</p>
  <button onclick="simulatePayment()" style="padding:12px 24px;font-size:16px;background:#635bff;color:white;border:none;border-radius:6px;cursor:pointer">
    Pay Now (Test)
  </button>
  <p id="status" style="margin-top:16px"></p>
  <script>
    async function simulatePayment() {
      document.getElementById('status').textContent = 'Processing...';
      try {
        const res = await fetch('/simulate-payment/${session.id}', { method: 'POST' });
        const data = await res.json();
        if (data.success) {
          document.getElementById('status').innerHTML = '<b style="color:green">Payment successful!</b> Redirecting...';
          setTimeout(() => window.location.href = '${session.success_url}', 1500);
        } else {
          document.getElementById('status').innerHTML = '<b style="color:red">Payment failed: ' + (data.error || 'unknown') + '</b>';
        }
      } catch(e) {
        document.getElementById('status').innerHTML = '<b style="color:red">Error: ' + e.message + '</b>';
      }
    }
  </script>
</body></html>`);
      return;
    }

    // POST /simulate-payment/:id — simulate successful payment + send webhook
    if (method === 'POST' && url?.startsWith('/simulate-payment/')) {
      const id = url.split('/').pop() || '';
      const session = sessions.get(id);
      if (!session) {
        res.writeHead(404, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ success: false, error: 'session not found' }));
        return;
      }

      // Send webhook to solobase
      const solobaseUrl = process.env.SOLOBASE_URL || 'http://127.0.0.1:8090';
      const webhookSecret = process.env.STRIPE_WEBHOOK_SECRET || 'whsec_test_mock_secret_for_e2e';

      const event = {
        id: `evt_test_${crypto.randomBytes(8).toString('hex')}`,
        object: 'event',
        type: 'checkout.session.completed',
        data: {
          object: {
            id: session.id,
            payment_intent: session.paymentIntent,
            payment_status: 'paid',
            status: 'complete',
            metadata: { purchase_id: session.metadata.purchase_id },
          },
        },
        created: Math.floor(Date.now() / 1000),
      };
      const payload = JSON.stringify(event);
      const timestamp = Math.floor(Date.now() / 1000).toString();
      const sig = crypto.createHmac('sha256', webhookSecret)
        .update(`${timestamp}.${payload}`)
        .digest('hex');

      fetch(`${solobaseUrl}/ext/products/webhooks`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Stripe-Signature': `t=${timestamp},v1=${sig}`,
        },
        body: payload,
      })
        .then((r) => {
          console.log(`  webhook sent for ${id} -> ${r.status}`);
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ success: true, webhook_status: r.status }));
        })
        .catch((e) => {
          console.error(`  webhook failed for ${id}: ${e.message}`);
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ success: false, error: e.message }));
        });
      return;
    }

    res.writeHead(404, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: { message: `Not found: ${method} ${url}`, type: 'invalid_request_error' } }));
  });
}

const server = http.createServer(handler);
server.listen(PORT, '127.0.0.1', () => {
  console.log(`Stripe mock server running on http://127.0.0.1:${PORT}`);
  console.log(`  Checkout sessions: POST http://127.0.0.1:${PORT}/v1/checkout/sessions`);
  console.log(`  Fake checkout UI:  http://127.0.0.1:${PORT}/checkout/:session_id`);
  console.log('');
});

process.on('SIGINT', () => { server.close(); process.exit(0); });
process.on('SIGTERM', () => { server.close(); process.exit(0); });
