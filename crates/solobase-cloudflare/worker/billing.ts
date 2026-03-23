// Stripe billing module — handles checkout, webhooks, subscription status, and portal.
//
// Uses raw fetch() against the Stripe API (no SDK). JWT verification for
// authenticated routes uses Web Crypto HMAC-SHA256 directly.

import type { Env } from './types';
import { getPlanLimits } from './types';

// ---------------------------------------------------------------------------
// Env extensions — Stripe-specific bindings
// ---------------------------------------------------------------------------

interface BillingEnv extends Env {
  STRIPE_SECRET_KEY: string;
  STRIPE_WEBHOOK_SECRET: string;
  STRIPE_PRICE_STARTER: string;
  STRIPE_PRICE_PRO: string;
}

// ---------------------------------------------------------------------------
// Helpers — JSON responses
// ---------------------------------------------------------------------------

function jsonOk(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function jsonError(code: string, message: string, status: number): Response {
  return new Response(JSON.stringify({ error: code, message }), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

// ---------------------------------------------------------------------------
// Stripe API helper
// ---------------------------------------------------------------------------

async function stripeRequest(
  secretKey: string,
  method: string,
  path: string,
  body?: Record<string, string>,
): Promise<any> {
  const resp = await fetch(`https://api.stripe.com/v1${path}`, {
    method,
    headers: {
      'Authorization': `Bearer ${secretKey}`,
      'Content-Type': 'application/x-www-form-urlencoded',
    },
    body: body ? new URLSearchParams(body).toString() : undefined,
  });
  return resp.json();
}

// ---------------------------------------------------------------------------
// JWT verification (Web Crypto HMAC-SHA256)
// ---------------------------------------------------------------------------

const encoder = new TextEncoder();
const decoder = new TextDecoder();

function b64urlDecode(str: string): Uint8Array {
  let base64 = str.replace(/-/g, '+').replace(/_/g, '/');
  while (base64.length % 4 !== 0) base64 += '=';
  const binStr = atob(base64);
  return Uint8Array.from(binStr, (c) => c.charCodeAt(0));
}

async function verifyJwt(
  token: string,
  secret: string,
): Promise<Record<string, unknown> | null> {
  const parts = token.split('.');
  if (parts.length !== 3) return null;

  const [headerB64, payloadB64, sigB64] = parts;
  const signingInput = `${headerB64}.${payloadB64}`;

  try {
    const key = await crypto.subtle.importKey(
      'raw',
      encoder.encode(secret),
      { name: 'HMAC', hash: 'SHA-256' },
      false,
      ['verify'],
    );

    const sig = b64urlDecode(sigB64);
    const valid = await crypto.subtle.verify(
      'HMAC',
      key,
      sig,
      encoder.encode(signingInput),
    );
    if (!valid) return null;

    const payload = JSON.parse(decoder.decode(b64urlDecode(payloadB64)));

    // Check expiry
    if (payload.exp && typeof payload.exp === 'number') {
      const now = Math.floor(Date.now() / 1000);
      if (now >= payload.exp) return null;
    }

    return payload;
  } catch {
    return null;
  }
}

/**
 * Extract and verify JWT from Authorization header.
 * Returns the claims on success, or a Response (error) on failure.
 */
async function authenticate(
  request: Request,
  env: BillingEnv,
): Promise<Record<string, unknown> | Response> {
  const authHeader = request.headers.get('Authorization');
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return jsonError('unauthenticated', 'missing or invalid Authorization header', 401);
  }

  const token = authHeader.slice(7);
  const jwtSecret = env.JWT_SECRET;
  if (!jwtSecret) {
    console.error('JWT_SECRET not configured');
    return jsonError('internal', 'server configuration error', 500);
  }

  const claims = await verifyJwt(token, jwtSecret);
  if (!claims) {
    return jsonError('unauthenticated', 'invalid or expired token', 401);
  }

  return claims;
}

// ---------------------------------------------------------------------------
// Webhook signature verification
// ---------------------------------------------------------------------------

function hexEncode(buf: ArrayBuffer): string {
  return Array.from(new Uint8Array(buf))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

/** Constant-time comparison via SHA-256 to prevent timing and length leaks. */
async function timingSafeEqualAsync(a: string, b: string): Promise<boolean> {
  const enc = new TextEncoder();
  const [ha, hb] = await Promise.all([
    crypto.subtle.digest('SHA-256', enc.encode(a)),
    crypto.subtle.digest('SHA-256', enc.encode(b)),
  ]);
  const ua = new Uint8Array(ha);
  const ub = new Uint8Array(hb);
  let result = 0;
  for (let i = 0; i < ua.length; i++) {
    result |= ua[i] ^ ub[i];
  }
  return result === 0;
}

async function verifyStripeSignature(
  rawBody: string,
  signatureHeader: string,
  webhookSecret: string,
): Promise<boolean> {
  // Parse the Stripe-Signature header: t=timestamp,v1=signature
  const parts: Record<string, string> = {};
  for (const item of signatureHeader.split(',')) {
    const [key, ...valueParts] = item.split('=');
    parts[key.trim()] = valueParts.join('=');
  }

  const timestamp = parts['t'];
  const expectedSig = parts['v1'];
  if (!timestamp || !expectedSig) return false;

  // Check timestamp is within 5 minutes
  const ts = parseInt(timestamp, 10);
  const now = Math.floor(Date.now() / 1000);
  if (Math.abs(now - ts) > 300) return false;

  // Compute expected signature
  const payload = `${timestamp}.${rawBody}`;
  const key = await crypto.subtle.importKey(
    'raw',
    encoder.encode(webhookSecret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign'],
  );
  const sig = await crypto.subtle.sign('HMAC', key, encoder.encode(payload));
  const computedSig = hexEncode(sig);

  return await timingSafeEqualAsync(computedSig, expectedSig);
}

// ---------------------------------------------------------------------------
// Route: POST /api/billing/checkout
// ---------------------------------------------------------------------------

async function handleCheckout(
  request: Request,
  env: BillingEnv,
  claims: Record<string, unknown>,
): Promise<Response> {
  const userId = claims.sub as string | undefined;
  if (!userId) {
    return jsonError('unauthenticated', 'missing user id in token', 401);
  }

  let body: { plan: string; name?: string };
  try {
    body = await request.json();
  } catch {
    return jsonError('invalid-argument', 'invalid JSON body', 400);
  }

  const { plan, name } = body;
  if (plan !== 'starter' && plan !== 'pro') {
    return jsonError('invalid-argument', 'plan must be "starter" or "pro"', 400);
  }

  const db = env.DB;

  // Check if user already has an active subscription
  const existing = await db.prepare(
    `SELECT id, status FROM subscriptions WHERE user_id = ? AND status IN ('active', 'trialing')`,
  ).bind(userId).first<{ id: string; status: string }>();

  if (existing) {
    return jsonError('already-exists', 'you already have an active subscription', 409);
  }

  // Check project count vs plan limit
  const limits = getPlanLimits(plan);
  const projectCount = await db.prepare(
    `SELECT COUNT(*) as cnt FROM projects WHERE user_id = ? AND deleted_at IS NULL`,
  ).bind(userId).first<{ cnt: number }>();

  const currentProjects = projectCount?.cnt ?? 0;
  if (currentProjects >= limits.maxProjects && limits.maxProjects !== Infinity) {
    return jsonError(
      'resource-exhausted',
      `plan "${plan}" allows ${limits.maxProjects} projects, you already have ${currentProjects}`,
      400,
    );
  }

  // Look up or create Stripe customer
  let stripeCustomerId: string | undefined;
  const sub = await db.prepare(
    `SELECT stripe_customer_id FROM subscriptions WHERE user_id = ? ORDER BY created_at DESC LIMIT 1`,
  ).bind(userId).first<{ stripe_customer_id: string }>();

  if (sub?.stripe_customer_id) {
    stripeCustomerId = sub.stripe_customer_id;
  } else {
    // Look up user email for customer creation
    const user = await db.prepare(
      `SELECT email FROM users WHERE id = ?`,
    ).bind(userId).first<{ email: string }>();

    const customerData: Record<string, string> = {
      'metadata[user_id]': userId,
    };
    if (user?.email) {
      customerData['email'] = user.email;
    }

    const customer = await stripeRequest(env.STRIPE_SECRET_KEY, 'POST', '/customers', customerData);
    if (customer.error) {
      console.error('Stripe create customer error:', customer.error);
      return jsonError('internal', 'failed to create customer', 500);
    }
    stripeCustomerId = customer.id;
  }

  // Select price based on plan
  const priceId = plan === 'starter' ? env.STRIPE_PRICE_STARTER : env.STRIPE_PRICE_PRO;

  // Create checkout session
  const sessionParams: Record<string, string> = {
    'mode': 'subscription',
    'customer': stripeCustomerId!,
    'line_items[0][price]': priceId,
    'line_items[0][quantity]': '1',
    'success_url': 'https://cloud.solobase.dev/blocks/dashboard/?checkout=success',
    'cancel_url': 'https://solobase.dev/pricing/?checkout=cancelled',
    'metadata[user_id]': userId,
    'metadata[plan]': plan,
  };
  if (name) {
    sessionParams['metadata[project_name]'] = name;
  }

  const session = await stripeRequest(
    env.STRIPE_SECRET_KEY,
    'POST',
    '/checkout/sessions',
    sessionParams,
  );

  if (session.error) {
    console.error('Stripe create checkout session error:', session.error);
    return jsonError('internal', 'failed to create checkout session', 500);
  }

  return jsonOk({ url: session.url });
}

// ---------------------------------------------------------------------------
// Route: POST /api/billing/webhook
// ---------------------------------------------------------------------------

async function handleWebhook(
  request: Request,
  env: BillingEnv,
): Promise<Response> {
  const signatureHeader = request.headers.get('Stripe-Signature');
  if (!signatureHeader) {
    return jsonError('invalid-argument', 'missing Stripe-Signature header', 400);
  }

  const rawBody = await request.text();

  const valid = await verifyStripeSignature(rawBody, signatureHeader, env.STRIPE_WEBHOOK_SECRET);
  if (!valid) {
    return jsonError('unauthenticated', 'invalid webhook signature', 401);
  }

  let event: any;
  try {
    event = JSON.parse(rawBody);
  } catch {
    return jsonError('invalid-argument', 'invalid JSON body', 400);
  }

  const db = env.DB;
  const kv = env.PROJECTS;

  try {
    switch (event.type) {
      case 'checkout.session.completed':
        await onCheckoutCompleted(event.data.object, db, kv, env);
        break;

      case 'customer.subscription.updated':
        await onSubscriptionUpdated(event.data.object, db, kv);
        break;

      case 'invoice.payment_failed':
        await onPaymentFailed(event.data.object, db);
        break;

      case 'customer.subscription.deleted':
        await onSubscriptionDeleted(event.data.object, db);
        break;

      default:
        // Unhandled event type — acknowledge silently
        break;
    }
  } catch (err) {
    console.error(`Webhook handler error for ${event.type}:`, err);
    return jsonError('internal', 'webhook processing failed', 500);
  }

  return jsonOk({ received: true });
}

// ---------------------------------------------------------------------------
// Webhook event handlers
// ---------------------------------------------------------------------------

async function onCheckoutCompleted(
  session: any,
  db: D1Database,
  kv: KVNamespace,
  env: BillingEnv,
): Promise<void> {
  const userId = session.metadata?.user_id;
  const plan = session.metadata?.plan ?? 'starter';
  const projectName = session.metadata?.project_name;
  const stripeCustomerId = session.customer;
  const stripeSubscriptionId = session.subscription;

  if (!userId) {
    console.error('checkout.session.completed missing user_id in metadata');
    return;
  }

  const now = new Date().toISOString();
  const subscriptionId = `sub_${userId}_${Date.now()}`;

  // Upsert subscription row (idempotent)
  await db.prepare(
    `INSERT INTO subscriptions (id, user_id, stripe_customer_id, stripe_subscription_id, plan, status, created_at, updated_at)
     VALUES (?, ?, ?, ?, ?, 'active', ?, ?)
     ON CONFLICT (user_id) DO UPDATE SET
       stripe_customer_id = excluded.stripe_customer_id,
       stripe_subscription_id = excluded.stripe_subscription_id,
       plan = excluded.plan,
       status = 'active',
       updated_at = excluded.updated_at`,
  ).bind(
    subscriptionId, userId, stripeCustomerId, stripeSubscriptionId, plan, now, now,
  ).run();

  // Create project if project_name provided
  if (projectName) {
    const subdomain = projectName
      .toLowerCase()
      .replace(/[^a-z0-9-]/g, '-')
      .replace(/-+/g, '-')
      .replace(/^-|-$/g, '');

    const projectId = `dep_${userId}_${Date.now()}`;

    await db.prepare(
      `INSERT INTO projects (id, user_id, name, subdomain, plan, status, created_at, updated_at)
       VALUES (?, ?, ?, ?, ?, 'active', ?, ?)
       ON CONFLICT (subdomain) DO UPDATE SET
         plan = excluded.plan,
         status = 'active',
         updated_at = excluded.updated_at`,
    ).bind(
      projectId, userId, projectName, subdomain, plan, now, now,
    ).run();

    // Create project config in KV
    const projectConfig = {
      id: projectId,
      subdomain,
      plan,
      config: {
        version: 1,
        auth: {},
        admin: {},
        files: {},
        products: {},
      },
      blocks: [],
    };
    await kv.put(`project:${subdomain}:config`, JSON.stringify(projectConfig));
  }
}

async function onSubscriptionUpdated(
  subscription: any,
  db: D1Database,
  kv: KVNamespace,
): Promise<void> {
  const stripeSubscriptionId = subscription.id;
  const status = subscription.status; // active, past_due, canceled, etc.

  // Determine plan from the first price's product metadata or lookup_key
  let plan: string | undefined;
  if (subscription.items?.data?.length > 0) {
    const item = subscription.items.data[0];
    plan = item.price?.lookup_key ?? item.price?.metadata?.plan;
  }

  const now = new Date().toISOString();

  // Build update query
  const updates: string[] = ['status = ?', 'updated_at = ?'];
  const binds: unknown[] = [status, now];

  if (plan) {
    updates.push('plan = ?');
    binds.push(plan);
  }

  binds.push(stripeSubscriptionId);

  await db.prepare(
    `UPDATE subscriptions SET ${updates.join(', ')} WHERE stripe_subscription_id = ?`,
  ).bind(...binds).run();

  // Update project plan in KV if we have a plan change
  if (plan) {
    const sub = await db.prepare(
      `SELECT user_id FROM subscriptions WHERE stripe_subscription_id = ?`,
    ).bind(stripeSubscriptionId).first<{ user_id: string }>();

    if (sub) {
      // Find projects for this user and update their KV configs
      const projects = await db.prepare(
        `SELECT subdomain FROM projects WHERE user_id = ? AND deleted_at IS NULL`,
      ).bind(sub.user_id).all<{ subdomain: string }>();

      for (const dep of projects.results ?? []) {
        const key = `project:${dep.subdomain}:config`;
        const raw = await kv.get(key, 'json') as any;
        if (raw) {
          raw.plan = plan;
          await kv.put(key, JSON.stringify(raw));
        }
      }
    }
  }
}

async function onPaymentFailed(
  invoice: any,
  db: D1Database,
): Promise<void> {
  const stripeSubscriptionId = invoice.subscription;
  if (!stripeSubscriptionId) return;

  const now = new Date().toISOString();
  const gracePeriodEnd = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString();

  await db.prepare(
    `UPDATE subscriptions
     SET status = 'past_due', grace_period_end = ?, updated_at = ?
     WHERE stripe_subscription_id = ?`,
  ).bind(gracePeriodEnd, now, stripeSubscriptionId).run();
}

async function onSubscriptionDeleted(
  subscription: any,
  db: D1Database,
): Promise<void> {
  const stripeSubscriptionId = subscription.id;
  const now = new Date().toISOString();

  await db.prepare(
    `UPDATE subscriptions
     SET status = 'cancelled', updated_at = ?
     WHERE stripe_subscription_id = ?`,
  ).bind(now, stripeSubscriptionId).run();
}

// ---------------------------------------------------------------------------
// Route: GET /api/billing/subscription
// ---------------------------------------------------------------------------

async function handleSubscription(
  env: BillingEnv,
  claims: Record<string, unknown>,
): Promise<Response> {
  const userId = claims.sub as string | undefined;
  if (!userId) {
    return jsonError('unauthenticated', 'missing user id in token', 401);
  }

  const db = env.DB;

  const sub = await db.prepare(
    `SELECT id, plan, status, stripe_subscription_id, grace_period_end, created_at, updated_at
     FROM subscriptions WHERE user_id = ?`,
  ).bind(userId).first<{
    id: string;
    plan: string;
    status: string;
    stripe_subscription_id: string;
    grace_period_end: string | null;
    created_at: string;
    updated_at: string;
  }>();

  if (!sub) {
    return jsonOk({
      subscription: null,
      usage: null,
    });
  }

  // Get current month usage
  const month = currentMonth();
  const usage = await db.prepare(
    `SELECT requests, addon_requests, r2_bytes, addon_r2_bytes
     FROM project_usage WHERE project_id = ? AND month = ?`,
  ).bind(userId, month).first<{
    requests: number;
    addon_requests: number;
    r2_bytes: number;
    addon_r2_bytes: number;
  }>();

  const limits = getPlanLimits(sub.plan);

  return jsonOk({
    subscription: {
      id: sub.id,
      plan: sub.plan,
      status: sub.status,
      gracePeriodEnd: sub.grace_period_end,
      createdAt: sub.created_at,
      updatedAt: sub.updated_at,
    },
    usage: {
      month,
      requests: {
        used: usage?.requests ?? 0,
        limit: limits.maxRequestsPerMonth + (usage?.addon_requests ?? 0),
      },
      r2Storage: {
        usedBytes: usage?.r2_bytes ?? 0,
        limitBytes: limits.maxR2StorageBytes + (usage?.addon_r2_bytes ?? 0),
      },
    },
  });
}

// ---------------------------------------------------------------------------
// Route: POST /api/billing/portal
// ---------------------------------------------------------------------------

async function handlePortal(
  env: BillingEnv,
  claims: Record<string, unknown>,
): Promise<Response> {
  const userId = claims.sub as string | undefined;
  if (!userId) {
    return jsonError('unauthenticated', 'missing user id in token', 401);
  }

  const db = env.DB;

  const sub = await db.prepare(
    `SELECT stripe_customer_id FROM subscriptions WHERE user_id = ?`,
  ).bind(userId).first<{ stripe_customer_id: string }>();

  if (!sub?.stripe_customer_id) {
    return jsonError('not-found', 'no subscription found', 404);
  }

  const session = await stripeRequest(
    env.STRIPE_SECRET_KEY,
    'POST',
    '/billing_portal/sessions',
    {
      customer: sub.stripe_customer_id,
      return_url: 'https://cloud.solobase.dev/blocks/dashboard/',
    },
  );

  if (session.error) {
    console.error('Stripe create portal session error:', session.error);
    return jsonError('internal', 'failed to create portal session', 500);
  }

  return jsonOk({ url: session.url });
}

// ---------------------------------------------------------------------------
// Main router
// ---------------------------------------------------------------------------

export async function handleBillingRoute(
  request: Request,
  env: Env,
  url: URL,
): Promise<Response> {
  const billingEnv = env as BillingEnv;
  const path = url.pathname;

  // POST /api/billing/webhook — unauthenticated (verified by Stripe signature)
  if (path === '/api/billing/webhook' && request.method === 'POST') {
    return handleWebhook(request, billingEnv);
  }

  // All other routes require authentication
  const authResult = await authenticate(request, billingEnv);
  if (authResult instanceof Response) {
    return authResult;
  }
  const claims = authResult;

  // POST /api/billing/checkout
  if (path === '/api/billing/checkout' && request.method === 'POST') {
    return handleCheckout(request, billingEnv, claims);
  }

  // GET /api/billing/subscription
  if (path === '/api/billing/subscription' && request.method === 'GET') {
    return handleSubscription(billingEnv, claims);
  }

  // POST /api/billing/portal
  if (path === '/api/billing/portal' && request.method === 'POST') {
    return handlePortal(billingEnv, claims);
  }

  return jsonError('not-found', `unknown billing route: ${request.method} ${path}`, 404);
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

function currentMonth(): string {
  const d = new Date();
  return `${d.getUTCFullYear()}-${String(d.getUTCMonth() + 1).padStart(2, '0')}`;
}
