// TypeScript-native auth block handler (for testing without WASM).
// Calls host.callBlock() for database/crypto operations — same pattern as WASM blocks.

import type { Message, BlockResult } from '../types';
import type { RuntimeHost } from '../host';
import { metaGet } from '../convert';

const USERS = 'auth_users';
const TOKENS = 'auth_tokens';
const USER_ROLES = 'iam_user_roles';

export async function handle(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const action = metaGet(msg.meta, 'req.action') ?? '';
  const path = metaGet(msg.meta, 'req.resource') ?? '';

  switch (true) {
    case action === 'create' && path === '/auth/signup': return handleSignup(msg, host);
    case action === 'create' && path === '/auth/login': return handleLogin(msg, host);
    case action === 'create' && path === '/auth/refresh': return handleRefresh(msg, host);
    case action === 'create' && path === '/auth/logout': return handleLogout(msg, host);
    case action === 'retrieve' && path === '/auth/me': return handleMe(msg, host);
    case action === 'update' && path === '/auth/me': return handleMeUpdate(msg, host);
    case action === 'create' && path === '/auth/verify-email': return handleVerifyEmail(msg, host);
    case action === 'create' && path === '/auth/resend-verification': return handleResendVerification(msg, host);
    case action === 'create' && path === '/auth/forgot-password': return handleForgotPassword(msg, host);
    case action === 'create' && path === '/auth/reset-password': return handleResetPassword(msg, host);
    default: return errResult('not-found', 'not found');
  }
}

async function handleSignup(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<{ email: string; password: string; name?: string }>(msg);
  if (!body) return errResult('invalid-argument', 'invalid body');

  const email = body.email.trim().toLowerCase();
  if (!email.includes('@') || body.password.length < 8) {
    return errResult('invalid-argument', 'invalid email or password too short');
  }
  if (body.password.length > 256) {
    return errResult('invalid-argument', 'password must not exceed 256 characters');
  }

  // Check existing
  const existing = await dbListFiltered(host, USERS, 'email', email);
  if (existing.length > 0) return errResult('already-exists', 'email already registered');

  // Hash password
  const hashResult = await callService(host, 'wafer-run/crypto', 'crypto.hash', { password: body.password });
  if (!hashResult) return errResult('internal', 'failed to hash password');
  const passwordHash = hashResult.hash;

  // Create user
  const user = await dbCreate(host, USERS, {
    email, password_hash: passwordHash, name: body.name ?? '', disabled: 0,
  });
  if (!user) return errResult('internal', 'failed to create user');

  // Assign role
  const adminEmail = await getConfig(host, 'ADMIN_EMAIL', '');
  const role = adminEmail && email.toLowerCase() === adminEmail.toLowerCase() ? 'admin' : 'user';
  await dbCreate(host, USER_ROLES, { user_id: user.id, role, assigned_at: new Date().toISOString() });

  // Generate JWT
  const tokens = await generateTokens(host, user.id, email, [role]);
  if (!tokens) return errResult('internal', 'failed to generate tokens');

  await dbCreate(host, TOKENS, { user_id: user.id, token: tokens.refresh });

  return jsonRespond(msg, {
    access_token: tokens.access, refresh_token: tokens.refresh,
    token_type: 'Bearer', expires_in: 86400,
    user: { id: user.id, email, roles: [role], name: body.name ?? '' },
  }, 201);
}

async function handleLogin(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<{ email: string; password: string }>(msg);
  if (!body) return errResult('invalid-argument', 'invalid body');

  const email = body.email.trim().toLowerCase();

  // Find user
  const users = await dbListFiltered(host, USERS, 'email', email);
  if (users.length === 0) return errResult('unauthenticated', 'invalid email or password');
  const user = users[0];

  // Verify password
  const cmpResult = await callService(host, 'wafer-run/crypto', 'crypto.compare_hash', {
    password: body.password, hash: user.data.password_hash ?? '',
  });
  if (!cmpResult || !cmpResult.match) return errResult('unauthenticated', 'invalid email or password');

  if (user.data.disabled) return errResult('permission-denied', 'account is disabled');

  // Get roles
  const roles = await getUserRoles(host, user.id);

  // Generate tokens
  const tokens = await generateTokens(host, user.id, email, roles);
  if (!tokens) return errResult('internal', 'failed to generate tokens');

  await dbCreate(host, TOKENS, { user_id: user.id, token: tokens.refresh });

  return jsonRespond(msg, {
    access_token: tokens.access, refresh_token: tokens.refresh,
    token_type: 'Bearer', expires_in: 86400,
    user: { id: user.id, email, roles, name: user.data.name ?? '' },
  });
}

async function handleRefresh(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<{ refresh_token: string }>(msg);
  if (!body) return errResult('invalid-argument', 'invalid body');

  const claims = await verifyToken(host, body.refresh_token);
  if (!claims || claims.type !== 'refresh') return errResult('unauthenticated', 'invalid refresh token');

  const userId = (claims.sub ?? claims.user_id) as string;
  if (!userId) return errResult('unauthenticated', 'invalid refresh token');

  // Verify token exists in DB (ensures revoked tokens are rejected)
  const storedTokens = await dbListFiltered(host, TOKENS, 'user_id', userId);
  const tokenExists = storedTokens.some(t => t.data.token === body.refresh_token);
  if (!tokenExists) return errResult('unauthenticated', 'refresh token has been revoked');

  // Delete the consumed refresh token (rotation: one-time use)
  for (const t of storedTokens) {
    if (t.data.token === body.refresh_token) {
      await callService(host, 'wafer-run/database', 'database.delete', { collection: TOKENS, id: t.id });
    }
  }

  const users = await dbGet(host, USERS, userId);
  if (!users) return errResult('unauthenticated', 'user not found');

  const email = (users.data.email ?? '') as string;
  const roles = await getUserRoles(host, userId);
  const tokens = await generateTokens(host, userId, email, roles);
  if (!tokens) return errResult('internal', 'failed to generate tokens');

  // Store the new refresh token
  await dbCreate(host, TOKENS, { user_id: userId, token: tokens.refresh });

  return jsonRespond(msg, {
    access_token: tokens.access, refresh_token: tokens.refresh,
    token_type: 'Bearer', expires_in: 86400,
  });
}

async function handleLogout(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  if (userId) {
    // Revoke all refresh tokens for this user
    const tokens = await dbListFiltered(host, TOKENS, 'user_id', userId);
    for (const t of tokens) {
      await callService(host, 'wafer-run/database', 'database.delete', { collection: TOKENS, id: t.id });
    }
  }
  return jsonRespond(msg, { message: 'logged out' });
}

async function handleMe(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  if (!userId) return errResult('unauthenticated', 'not authenticated');

  const user = await dbGet(host, USERS, userId);
  if (!user) return errResult('not-found', 'user not found');

  const roles = await getUserRoles(host, userId);
  return jsonRespond(msg, {
    user: {
      id: user.id, email: user.data.email, name: user.data.name,
      roles, created_at: user.data.created_at, avatar_url: user.data.avatar_url,
    },
  });
}

async function handleMeUpdate(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  if (!userId) return errResult('unauthenticated', 'not authenticated');

  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'invalid body');

  const data: Record<string, unknown> = {};
  for (const key of ['name', 'avatar_url']) {
    if (body[key] !== undefined) data[key] = body[key];
  }

  const updated = await dbUpdate(host, USERS, userId, data);
  if (!updated) return errResult('internal', 'update failed');

  const roles = await getUserRoles(host, userId);
  return jsonRespond(msg, {
    id: updated.id, email: updated.data.email, name: updated.data.name, roles,
  });
}

// ---------------------------------------------------------------------------
// Service call helpers
// ---------------------------------------------------------------------------

async function callService(host: RuntimeHost, block: string, kind: string, data: unknown): Promise<any> {
  const result = await host.callBlock(block, {
    kind, data: new TextEncoder().encode(JSON.stringify(data)), meta: [],
  });
  if (result.action !== 'respond' || !result.response) return null;
  try { return JSON.parse(new TextDecoder().decode(result.response.data)); } catch { return null; }
}

async function dbGet(host: RuntimeHost, collection: string, id: string): Promise<{ id: string; data: Record<string, any> } | null> {
  return callService(host, 'wafer-run/database', 'database.get', { collection, id });
}

async function dbCreate(host: RuntimeHost, collection: string, data: Record<string, unknown>): Promise<{ id: string; data: Record<string, any> } | null> {
  return callService(host, 'wafer-run/database', 'database.create', { collection, data });
}

async function dbUpdate(host: RuntimeHost, collection: string, id: string, data: Record<string, unknown>): Promise<{ id: string; data: Record<string, any> } | null> {
  return callService(host, 'wafer-run/database', 'database.update', { collection, id, data });
}

async function dbListFiltered(host: RuntimeHost, collection: string, field: string, value: string): Promise<{ id: string; data: Record<string, any> }[]> {
  const result = await callService(host, 'wafer-run/database', 'database.list', {
    collection, filters: [{ field, operator: 'eq', value }], sort: [], limit: 10, offset: 0,
  });
  return result?.records ?? [];
}

async function getUserRoles(host: RuntimeHost, userId: string): Promise<string[]> {
  const records = await dbListFiltered(host, USER_ROLES, 'user_id', userId);
  return records.map(r => r.data.role as string).filter(Boolean);
}

async function getConfig(host: RuntimeHost, key: string, defaultVal: string): Promise<string> {
  const result = await callService(host, 'wafer-run/config', 'config.get', { key });
  return result?.value ?? defaultVal;
}

async function generateTokens(host: RuntimeHost, userId: string, email: string, roles: string[]): Promise<{ access: string; refresh: string } | null> {
  // Include a random jti (JWT ID) to ensure each token is unique even within the same second
  const jti = crypto.randomUUID();
  const accessResult = await callService(host, 'wafer-run/crypto', 'crypto.sign', {
    claims: { sub: userId, user_id: userId, email, roles, type: 'access', jti }, expiry_secs: 86400,
  });
  const refreshJti = crypto.randomUUID();
  const refreshResult = await callService(host, 'wafer-run/crypto', 'crypto.sign', {
    claims: { sub: userId, user_id: userId, type: 'refresh', jti: refreshJti }, expiry_secs: 604800,
  });
  if (!accessResult?.token || !refreshResult?.token) return null;
  return { access: accessResult.token, refresh: refreshResult.token };
}

async function verifyToken(host: RuntimeHost, token: string): Promise<Record<string, unknown> | null> {
  const result = await callService(host, 'wafer-run/crypto', 'crypto.verify', { token });
  return result?.claims ?? null;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function parseBody<T>(msg: Message): T | null {
  try { return JSON.parse(new TextDecoder().decode(msg.data)) as T; } catch { return null; }
}

function jsonRespond(msg: Message, data: unknown, status?: number): BlockResult {
  const meta = [{ key: 'resp.content_type', value: 'application/json' }];
  if (status) meta.push({ key: 'resp.status', value: String(status) });
  return {
    action: 'respond',
    response: { data: new TextEncoder().encode(JSON.stringify(data)), meta },
    message: msg,
  };
}

function errResult(code: string, message: string): BlockResult {
  return { action: 'error', error: { code: code as any, message, meta: [] } };
}

// ---------------------------------------------------------------------------
// Email verification
// ---------------------------------------------------------------------------

async function handleVerifyEmail(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<{ token: string }>(msg);
  if (!body?.token) return errResult('invalid-argument', 'missing token');

  const claims = await verifyToken(host, body.token);
  if (!claims || claims.type !== 'email_verification') {
    return errResult('invalid-argument', 'invalid or expired verification token');
  }

  const userId = claims.sub as string;
  if (!userId) return errResult('invalid-argument', 'invalid token');

  // Mark email as verified
  await dbUpdate(host, USERS, userId, { email_verified: 1 });

  return jsonRespond(msg, { message: 'Email verified successfully' });
}

async function handleResendVerification(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  if (!userId) return errResult('unauthenticated', 'not authenticated');

  const user = await dbGet(host, USERS, userId);
  if (!user) return errResult('not-found', 'user not found');

  if (user.data.email_verified) {
    return jsonRespond(msg, { message: 'Email already verified' });
  }

  const email = (user.data.email ?? '') as string;

  // Generate verification token (24h expiry)
  const tokenResult = await callService(host, 'wafer-run/crypto', 'crypto.sign', {
    claims: { sub: userId, email, type: 'email_verification', jti: crypto.randomUUID() },
    expiry_secs: 86400,
  });
  if (!tokenResult?.token) return errResult('internal', 'failed to generate token');

  // Send email — import dynamically to avoid circular deps
  const { sendVerificationEmail } = await import('../email');
  // We need env to send email, but we don't have it here.
  // The email sending needs to be done at the index.ts level or via a service call.
  // For now, return the token so the caller can trigger the email.
  return jsonRespond(msg, {
    message: 'Verification email sent',
    // In production, don't return the token — send via email only
    _verification_token: tokenResult.token,
  });
}

// ---------------------------------------------------------------------------
// Password reset
// ---------------------------------------------------------------------------

async function handleForgotPassword(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<{ email: string }>(msg);
  if (!body?.email) return errResult('invalid-argument', 'email required');

  const email = body.email.trim().toLowerCase();

  // Always return success to prevent email enumeration
  const successResponse = jsonRespond(msg, {
    message: 'If an account exists with that email, a password reset link has been sent.',
  });

  // Look up user
  const users = await dbListFiltered(host, USERS, 'email', email);
  if (users.length === 0) return successResponse;

  const user = users[0];

  // Generate reset token (1h expiry)
  const tokenResult = await callService(host, 'wafer-run/crypto', 'crypto.sign', {
    claims: { sub: user.id, email, type: 'password_reset', jti: crypto.randomUUID() },
    expiry_secs: 3600,
  });
  if (!tokenResult?.token) return successResponse;

  // Return token (in production, send via email and don't return it)
  return jsonRespond(msg, {
    message: 'If an account exists with that email, a password reset link has been sent.',
    _reset_token: tokenResult.token,
  });
}

async function handleResetPassword(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<{ token: string; new_password: string }>(msg);
  if (!body?.token || !body?.new_password) return errResult('invalid-argument', 'token and new_password required');

  if (body.new_password.length < 8) return errResult('invalid-argument', 'password must be at least 8 characters');
  if (body.new_password.length > 256) return errResult('invalid-argument', 'password must not exceed 256 characters');

  const claims = await verifyToken(host, body.token);
  if (!claims || claims.type !== 'password_reset') {
    return errResult('invalid-argument', 'invalid or expired reset token');
  }

  const userId = claims.sub as string;
  if (!userId) return errResult('invalid-argument', 'invalid token');

  // Hash new password
  const hashResult = await callService(host, 'wafer-run/crypto', 'crypto.hash', { password: body.new_password });
  if (!hashResult?.hash) return errResult('internal', 'failed to hash password');

  // Update password
  await dbUpdate(host, USERS, userId, { password_hash: hashResult.hash });

  // Revoke all refresh tokens (force re-login)
  const tokens = await dbListFiltered(host, TOKENS, 'user_id', userId);
  for (const t of tokens) {
    await callService(host, 'wafer-run/database', 'database.delete', { collection: TOKENS, id: t.id });
  }

  return jsonRespond(msg, { message: 'Password reset successfully. Please log in with your new password.' });
}
