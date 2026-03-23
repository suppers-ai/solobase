// TypeScript-native auth block handler (for testing without WASM).
// Calls host.callBlock() for database/crypto operations — same pattern as WASM blocks.

import type { Message, BlockResult, MetaEntry } from '../types';
import type { RuntimeHost } from '../host';
import { metaGet } from '../convert';

const USERS = 'auth_users';
const TOKENS = 'auth_tokens';
const USER_ROLES = 'iam_user_roles';
const API_KEYS = 'api_keys';
const USED_TOKENS = 'used_one_time_tokens';

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
    // API keys
    case action === 'retrieve' && path === '/auth/api-keys': return handleApiKeysList(msg, host);
    case action === 'create' && path === '/auth/api-keys': return handleApiKeysCreate(msg, host);
    case action === 'delete' && path.startsWith('/auth/api-keys/'): return handleApiKeysRevoke(msg, host);
    // OAuth
    case action === 'retrieve' && path === '/auth/oauth/providers': return handleOAuthProviders(msg, host);
    case action === 'retrieve' && path === '/auth/oauth/login': return handleOAuthLogin(msg, host);
    case action === 'retrieve' && path === '/auth/oauth/callback': return handleOAuthCallback(msg, host);
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

  const cookie = buildAuthCookie(tokens.access, 86400);
  return jsonRespondWithCookie(msg, {
    access_token: tokens.access, refresh_token: tokens.refresh,
    token_type: 'Bearer', expires_in: 86400,
    user: { id: user.id, email, roles: [role], name: body.name ?? '' },
  }, cookie, 201);
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

  const cookie = buildAuthCookie(tokens.access, 86400);
  return jsonRespondWithCookie(msg, {
    access_token: tokens.access, refresh_token: tokens.refresh,
    token_type: 'Bearer', expires_in: 86400,
    user: { id: user.id, email, roles, name: user.data.name ?? '' },
  }, cookie);
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

  const cookie = buildAuthCookie(tokens.access, 86400);
  return jsonRespondWithCookie(msg, {
    access_token: tokens.access, refresh_token: tokens.refresh,
    token_type: 'Bearer', expires_in: 86400,
  }, cookie);
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
  const cookie = buildAuthCookie('', 0);
  return jsonRespondWithCookie(msg, { message: 'logged out' }, cookie);
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
// API keys
// ---------------------------------------------------------------------------

async function handleApiKeysList(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  if (!userId) return errResult('unauthenticated', 'not authenticated');

  const result = await callService(host, 'wafer-run/database', 'database.list', {
    collection: API_KEYS,
    filters: [{ field: 'user_id', operator: 'eq', value: userId }],
    sort: [{ field: 'created_at', direction: 'desc' }],
    limit: 100,
    offset: 0,
  });
  const records: { id: string; data: Record<string, any> }[] = result?.records ?? [];

  // Strip key_hash from response
  for (const record of records) {
    delete record.data.key_hash;
  }

  return jsonRespond(msg, { records, total: records.length });
}

async function handleApiKeysCreate(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  if (!userId) return errResult('unauthenticated', 'not authenticated');

  const body = parseBody<{ name: string; expires_at?: string }>(msg);
  if (!body?.name) return errResult('invalid-argument', 'name is required');

  // Generate random key bytes
  const randomBytes = new Uint8Array(24);
  crypto.getRandomValues(randomBytes);
  const keyString = 'sb_' + Array.from(randomBytes).map(b => b.toString(16).padStart(2, '0')).join('');

  // SHA-256 hash for storage (deterministic, not argon2)
  const keyHash = await sha256hex(keyString);

  const keyPrefix = keyString.substring(0, 10);
  const now = new Date().toISOString();

  const data: Record<string, unknown> = {
    user_id: userId,
    name: body.name,
    key_hash: keyHash,
    key_prefix: keyPrefix,
    created_at: now,
  };
  if (body.expires_at) {
    data.expires_at = body.expires_at;
  }

  const record = await dbCreate(host, API_KEYS, data);
  if (!record) return errResult('internal', 'failed to create API key');

  return jsonRespond(msg, {
    id: record.id,
    key: keyString,
    name: record.data.name,
    key_prefix: record.data.key_prefix,
    message: "Save this key — it won't be shown again",
  });
}

async function handleApiKeysRevoke(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const path = metaGet(msg.meta, 'req.resource') ?? '';
  const id = path.replace('/auth/api-keys/', '');
  if (!id) return errResult('invalid-argument', 'missing key ID');

  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  if (!userId) return errResult('unauthenticated', 'not authenticated');

  // Verify ownership
  const key = await dbGet(host, API_KEYS, id);
  if (!key) return errResult('not-found', 'API key not found');

  const keyOwner = (key.data.user_id ?? '') as string;
  const userRoles = metaGet(msg.meta, 'auth.user_roles') ?? '';
  if (keyOwner !== userId && !userRoles.split(',').some(r => r.trim() === 'admin')) {
    return errResult('permission-denied', "cannot revoke another user's API key");
  }

  const updated = await dbUpdate(host, API_KEYS, id, { revoked_at: new Date().toISOString() });
  if (!updated) return errResult('internal', 'failed to revoke API key');

  return jsonRespond(msg, { message: 'API key revoked' });
}

// ---------------------------------------------------------------------------
// OAuth
// ---------------------------------------------------------------------------

async function handleOAuthProviders(_msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const providers: { name: string; enabled: boolean }[] = [];

  for (const providerName of ['google', 'github', 'microsoft']) {
    const clientIdKey = `OAUTH_${providerName.toUpperCase()}_CLIENT_ID`;
    const clientId = await getConfig(host, clientIdKey, '');
    if (clientId) {
      providers.push({ name: providerName, enabled: true });
    }
  }

  return jsonRespond(_msg, { providers });
}

async function handleOAuthLogin(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const provider = metaGet(msg.meta, 'req.query.provider') ?? '';
  if (!provider) return errResult('invalid-argument', 'missing provider parameter');

  const clientIdKey = `OAUTH_${provider.toUpperCase()}_CLIENT_ID`;
  const clientId = await getConfig(host, clientIdKey, '');
  if (!clientId) return errResult('invalid-argument', `OAuth provider '${provider}' not configured`);

  const redirectUri = await getConfig(host, 'OAUTH_REDIRECT_URI', 'http://localhost:8090/auth/oauth/callback');

  // Generate CSRF state token (signed JWT containing the provider name)
  const stateResult = await callService(host, 'wafer-run/crypto', 'crypto.sign', {
    claims: { provider, type: 'oauth_state' },
    expiry_secs: 600,
  });
  if (!stateResult?.token) return errResult('internal', 'failed to generate state');
  const state = stateResult.token;

  let authUrl: string;
  switch (provider) {
    case 'google':
      authUrl = `https://accounts.google.com/o/oauth2/v2/auth?client_id=${encURI(clientId)}&redirect_uri=${encURI(redirectUri)}&response_type=code&scope=openid%20email%20profile&state=${encURI(state)}`;
      break;
    case 'github':
      authUrl = `https://github.com/login/oauth/authorize?client_id=${encURI(clientId)}&redirect_uri=${encURI(redirectUri)}&scope=user:email&state=${encURI(state)}`;
      break;
    case 'microsoft':
      authUrl = `https://login.microsoftonline.com/common/oauth2/v2.0/authorize?client_id=${encURI(clientId)}&redirect_uri=${encURI(redirectUri)}&response_type=code&scope=openid%20email%20profile&state=${encURI(state)}`;
      break;
    default:
      return errResult('invalid-argument', `unsupported provider: ${provider}`);
  }

  return jsonRespond(msg, { auth_url: authUrl, provider });
}

async function handleOAuthCallback(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const code = metaGet(msg.meta, 'req.query.code') ?? '';
  const state = metaGet(msg.meta, 'req.query.state') ?? '';
  if (!code || !state) return errResult('invalid-argument', 'missing code or state parameter');

  // Verify CSRF state token and extract provider name
  const stateClaims = await verifyToken(host, state);
  if (!stateClaims) return errResult('invalid-argument', 'invalid or expired OAuth state');

  const stateType = stateClaims.type as string;
  if (stateType !== 'oauth_state') return errResult('invalid-argument', 'invalid OAuth state token');

  const provider = (stateClaims.provider as string) ?? '';
  if (!provider) return errResult('invalid-argument', 'missing provider in OAuth state');

  const clientId = await getConfig(host, `OAUTH_${provider.toUpperCase()}_CLIENT_ID`, '');
  const clientSecret = await getConfig(host, `OAUTH_${provider.toUpperCase()}_CLIENT_SECRET`, '');
  const redirectUri = await getConfig(host, 'OAUTH_REDIRECT_URI', 'http://localhost:8090/auth/oauth/callback');

  if (!clientId || !clientSecret) return errResult('internal', 'OAuth provider not fully configured');

  // Exchange code for token
  let tokenUrl: string;
  let tokenBody: string;
  switch (provider) {
    case 'google':
      tokenUrl = 'https://oauth2.googleapis.com/token';
      tokenBody = `code=${encURI(code)}&client_id=${encURI(clientId)}&client_secret=${encURI(clientSecret)}&redirect_uri=${encURI(redirectUri)}&grant_type=authorization_code`;
      break;
    case 'github':
      tokenUrl = 'https://github.com/login/oauth/access_token';
      tokenBody = `code=${encURI(code)}&client_id=${encURI(clientId)}&client_secret=${encURI(clientSecret)}&redirect_uri=${encURI(redirectUri)}`;
      break;
    default:
      return errResult('invalid-argument', 'unsupported OAuth provider');
  }

  const tokenResp = await networkRequest(host, 'POST', tokenUrl, {
    'Content-Type': 'application/x-www-form-urlencoded',
    'Accept': 'application/json',
  }, new TextEncoder().encode(tokenBody));
  if (!tokenResp) return errResult('internal', 'token exchange failed');

  const tokenData = tokenResp as Record<string, unknown>;
  const oauthAccessToken = (tokenData.access_token as string) ?? '';
  if (!oauthAccessToken) return errResult('internal', 'no access token in OAuth response');

  // Get user info
  let userinfoUrl: string;
  let authHeader: string;
  switch (provider) {
    case 'google':
      userinfoUrl = 'https://www.googleapis.com/oauth2/v2/userinfo';
      authHeader = `Bearer ${oauthAccessToken}`;
      break;
    case 'github':
      userinfoUrl = 'https://api.github.com/user';
      authHeader = `token ${oauthAccessToken}`;
      break;
    default:
      return errResult('internal', 'unsupported provider');
  }

  const userInfo = await networkRequest(host, 'GET', userinfoUrl, {
    'Authorization': authHeader,
    'Accept': 'application/json',
  });
  if (!userInfo) return errResult('internal', 'user info request failed');

  const info = userInfo as Record<string, unknown>;
  const email = ((info.email as string) ?? '').toLowerCase();
  const name = (info.name as string) ?? '';
  const avatar = (info.picture as string) ?? (info.avatar_url as string) ?? '';

  if (!email) return errResult('internal', 'no email returned by OAuth provider');

  // Upsert user
  const existingUsers = await dbListFiltered(host, USERS, 'email', email);
  let user: { id: string; data: Record<string, any> };

  if (existingUsers.length > 0) {
    user = existingUsers[0];
    // Update existing user profile
    const updateData: Record<string, unknown> = {
      last_login_at: new Date().toISOString(),
      oauth_provider: provider,
    };
    if (name) updateData.name = name;
    if (avatar) updateData.avatar_url = avatar;
    await dbUpdate(host, USERS, user.id, updateData);
  } else {
    // Create new user
    const newUser = await dbCreate(host, USERS, {
      email,
      name,
      avatar_url: avatar,
      oauth_provider: provider,
      disabled: 0,
    });
    if (!newUser) return errResult('internal', 'failed to create user');
    user = newUser;

    // Assign role
    const adminEmail = await getConfig(host, 'ADMIN_EMAIL', '');
    const role = adminEmail && email.toLowerCase() === adminEmail.toLowerCase() ? 'admin' : 'user';
    await dbCreate(host, USER_ROLES, { user_id: user.id, role, assigned_at: new Date().toISOString() });
  }

  const roles = await getUserRoles(host, user.id);
  const tokens = await generateTokens(host, user.id, email, roles);
  if (!tokens) return errResult('internal', 'failed to generate tokens');

  await dbCreate(host, TOKENS, { user_id: user.id, token: tokens.refresh });

  // Redirect to frontend with token
  const frontendUrl = await getConfig(host, 'FRONTEND_URL', 'http://localhost:5173');
  const redirectUrl = `${frontendUrl}/?token=${tokens.access}`;

  const cookie = buildAuthCookie(tokens.access, 86400);
  return redirectWithCookie(msg, redirectUrl, cookie);
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

async function networkRequest(
  host: RuntimeHost,
  method: string,
  url: string,
  headers: Record<string, string>,
  body?: Uint8Array,
): Promise<Record<string, unknown> | null> {
  const reqData: { method: string; url: string; headers: Record<string, string>; body?: number[] } = {
    method,
    url,
    headers,
  };
  if (body) {
    reqData.body = Array.from(body);
  }
  const result = await callService(host, 'wafer-run/network', 'network.do', reqData);
  if (!result || !result.body) return null;
  try {
    const bodyBytes = new Uint8Array(result.body as number[]);
    return JSON.parse(new TextDecoder().decode(bodyBytes));
  } catch {
    return null;
  }
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

function buildAuthCookie(token: string, maxAge: number): string {
  return `auth_token=${token}; HttpOnly; Path=/; SameSite=Lax; Max-Age=${maxAge}`;
}

function jsonRespond(msg: Message, data: unknown, status?: number): BlockResult {
  const meta: MetaEntry[] = [{ key: 'resp.content_type', value: 'application/json' }];
  if (status) meta.push({ key: 'resp.status', value: String(status) });
  return {
    action: 'respond',
    response: { data: new TextEncoder().encode(JSON.stringify(data)), meta },
    message: msg,
  };
}

function jsonRespondWithCookie(msg: Message, data: unknown, cookie: string, status?: number): BlockResult {
  const meta: MetaEntry[] = [
    { key: 'resp.content_type', value: 'application/json' },
    { key: 'resp.set_cookie.0', value: cookie },
  ];
  if (status) meta.push({ key: 'resp.status', value: String(status) });
  return {
    action: 'respond',
    response: { data: new TextEncoder().encode(JSON.stringify(data)), meta },
    message: msg,
  };
}

function redirectWithCookie(msg: Message, location: string, cookie: string): BlockResult {
  const meta: MetaEntry[] = [
    { key: 'resp.status', value: '302' },
    { key: 'resp.content_type', value: 'application/json' },
    { key: 'resp.header.Location', value: location },
    { key: 'resp.set_cookie.0', value: cookie },
  ];
  return {
    action: 'respond',
    response: {
      data: new TextEncoder().encode(JSON.stringify({ redirect: location })),
      meta,
    },
    message: msg,
  };
}

function errResult(code: string, message: string): BlockResult {
  return { action: 'error', error: { code: code as any, message, meta: [] } };
}

async function sha256hex(data: string): Promise<string> {
  const hash = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(data));
  return Array.from(new Uint8Array(hash)).map(b => b.toString(16).padStart(2, '0')).join('');
}

function encURI(s: string): string {
  return encodeURIComponent(s);
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
  const jti = claims.jti as string;
  if (!userId || !jti) return errResult('invalid-argument', 'invalid token');

  // Ensure token is single-use
  const usedTokens = await dbListFiltered(host, USED_TOKENS, 'jti', jti);
  if (usedTokens.length > 0) {
    return errResult('invalid-argument', 'verification token has already been used');
  }
  await dbCreate(host, USED_TOKENS, { jti, type: 'email_verification', consumed_at: new Date().toISOString() });

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

  // TODO: Send verification email via network service (Mailgun).
  // The email module needs env which isn't available here. Wire through
  // a service call (e.g. host.callBlock('wafer-run/network', ...)) to
  // POST to Mailgun API with the token embedded in the verification link.
  return jsonRespond(msg, {
    message: 'Verification email sent',
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
  const resetJti = crypto.randomUUID();
  const tokenResult = await callService(host, 'wafer-run/crypto', 'crypto.sign', {
    claims: { sub: user.id, email, type: 'password_reset', jti: resetJti },
    expiry_secs: 3600,
  });
  if (!tokenResult?.token) return successResponse;

  // TODO: Send reset email via network service (Mailgun).
  // POST to Mailgun API with the token embedded in the reset link.
  return successResponse;
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
  const jti = claims.jti as string;
  if (!userId || !jti) return errResult('invalid-argument', 'invalid token');

  // Ensure token is single-use: check if jti has already been consumed
  const usedTokens = await dbListFiltered(host, USED_TOKENS, 'jti', jti);
  if (usedTokens.length > 0) {
    return errResult('invalid-argument', 'reset token has already been used');
  }
  // Mark token as consumed
  await dbCreate(host, USED_TOKENS, { jti, type: 'password_reset', consumed_at: new Date().toISOString() });

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
