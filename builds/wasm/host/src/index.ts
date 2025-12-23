// Solobase WASM Host - Cloudflare Workers with Hono
//
// This example demonstrates hosting Solobase WASM on Cloudflare Workers using:
// - Hono as the web framework
// - D1 (SQLite) as the database
// - R2 for file storage (optional)
//
// Note: Auth endpoints are handled directly by the host (not WASM) because
// D1 is async but WASM imports must be synchronous.
//
// Deploy:
//   npm install
//   npx wrangler deploy
//
// Local dev:
//   npx wrangler dev

import { Hono } from 'hono';
import { sign, verify } from 'hono/jwt';

// Import the WASM module (copied to dist/ during build)
import wasmModule from '../dist/solobase.wasm';

interface Env {
  DB: D1Database;
  STORAGE?: R2Bucket;
  JWT_SECRET: string;
  DEFAULT_ADMIN_EMAIL?: string;
  DEFAULT_ADMIN_PASSWORD?: string;
}

// Database initialization flag
let dbInitialized = false;

interface QueryResult {
  columns: string[];
  rows: unknown[][];
  error?: string;
}

interface ExecResult {
  rows_affected: number;
  error?: string;
}

interface TransactionResult {
  tx_id?: number;
  error?: string;
}

interface HTTPResponse {
  status: number;
  headers: Record<string, string[]>;
  body?: string; // Base64-encoded by Go's encoding/json for []byte
}

// Decode base64 string to Uint8Array
function base64ToBytes(base64: string): Uint8Array {
  const binaryString = atob(base64);
  const bytes = new Uint8Array(binaryString.length);
  for (let i = 0; i < binaryString.length; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }
  return bytes;
}

// Simple SHA-256 password hashing (for demo - use argon2 in production)
async function hashPassword(password: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(password);
  const hashBuffer = await crypto.subtle.digest('SHA-256', data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  const hashHex = hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
  return `$sha256$${hashHex}`;
}

async function verifyPassword(password: string, hash: string): Promise<boolean> {
  if (hash.startsWith('$sha256$')) {
    const expected = await hashPassword(password);
    return expected === hash;
  }
  // For argon2 hashes, we can't verify in Workers without additional libs
  // Return false to indicate migration needed
  return false;
}

// Initialize database schema and default admin
async function initDatabase(env: Env): Promise<void> {
  if (dbInitialized) return;

  try {
    // Create tables if they don't exist (D1 requires separate statements, single-line)
    await env.DB.exec('CREATE TABLE IF NOT EXISTS auth_users (id TEXT PRIMARY KEY, email TEXT UNIQUE NOT NULL, password TEXT NOT NULL, username TEXT, confirmed INTEGER DEFAULT 0, first_name TEXT, last_name TEXT, display_name TEXT, phone TEXT, location TEXT, metadata TEXT, last_login TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT)');

    await env.DB.exec('CREATE TABLE IF NOT EXISTS auth_tokens (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, type TEXT NOT NULL, provider TEXT, provider_uid TEXT, access_token TEXT, refresh_token TEXT, expires_at TEXT, oauth_expiry TEXT, revoked_at TEXT, created_at TEXT NOT NULL, FOREIGN KEY (user_id) REFERENCES auth_users(id))');

    await env.DB.exec('CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT, updated_at TEXT)');

    // Check if admin exists
    const adminEmail = env.DEFAULT_ADMIN_EMAIL || 'admin@example.com';
    const adminPassword = env.DEFAULT_ADMIN_PASSWORD || 'password123';

    const existing = await env.DB.prepare(
      'SELECT id FROM auth_users WHERE email = ? AND deleted_at IS NULL'
    ).bind(adminEmail).first();

    if (!existing) {
      const hashedPassword = await hashPassword(adminPassword);
      const now = new Date().toISOString();
      const id = crypto.randomUUID();

      await env.DB.prepare(`
        INSERT INTO auth_users (id, email, password, confirmed, created_at, updated_at)
        VALUES (?, ?, ?, 1, ?, ?)
      `).bind(id, adminEmail, hashedPassword, now, now).run();

      console.log(`Created default admin: ${adminEmail}`);
    }

    dbInitialized = true;
  } catch (error) {
    console.error('Database initialization error:', error);
  }
}

const app = new Hono<{ Bindings: Env }>();

// ==================================
// AUTH ROUTES (handled by host, not WASM)
// ==================================

// Health endpoint
app.get('/api/health', (c) => {
  return c.json({ status: 'ok', message: 'API is running' });
});

// Login endpoint
app.post('/api/auth/login', async (c) => {
  const env = c.env;
  await initDatabase(env);

  try {
    const body = await c.req.json<{ email: string; password: string }>();
    const { email, password } = body;

    if (!email || !password) {
      return c.json({ error: 'Email and password required' }, 400);
    }

    // Find user
    const user = await env.DB.prepare(
      'SELECT id, email, password, username, confirmed, first_name, last_name, display_name, phone, location, metadata, last_login, created_at, updated_at FROM auth_users WHERE email = ? AND deleted_at IS NULL'
    ).bind(email).first<{
      id: string;
      email: string;
      password: string;
      username: string | null;
      confirmed: number;
      first_name: string | null;
      last_name: string | null;
      display_name: string | null;
      phone: string | null;
      location: string | null;
      metadata: string | null;
      last_login: string | null;
      created_at: string;
      updated_at: string;
    }>();

    if (!user) {
      return c.json({ error: 'Invalid credentials' }, 401);
    }

    // Verify password
    const valid = await verifyPassword(password, user.password);
    if (!valid) {
      return c.json({ error: 'Invalid credentials' }, 401);
    }

    // Generate JWT
    const now = Math.floor(Date.now() / 1000);
    const payload = {
      user_id: user.id,
      email: user.email,
      roles: ['admin'], // TODO: fetch from IAM
      exp: now + 86400, // 24 hours
      iat: now,
    };

    const token = await sign(payload, env.JWT_SECRET);

    // Update last login
    const lastLoginTime = new Date().toISOString();
    await env.DB.prepare(
      'UPDATE auth_users SET last_login = ?, updated_at = ? WHERE id = ?'
    ).bind(lastLoginTime, lastLoginTime, user.id).run();

    // Set httpOnly cookie for authentication (24 hours)
    const cookieValue = `auth_token=${token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400`;
    c.header('Set-Cookie', cookieValue);

    // Return format expected by frontend: { data: UserResponse, message: string }
    return c.json({
      data: {
        user: {
          id: user.id,
          email: user.email,
          username: user.username || '',
          confirmed: user.confirmed === 1,
          firstName: user.first_name || '',
          lastName: user.last_name || '',
          displayName: user.display_name || '',
          phone: user.phone || '',
          location: user.location || '',
          lastLogin: lastLoginTime,
          metadata: user.metadata || '',
          createdAt: user.created_at,
          updatedAt: lastLoginTime,
        },
        roles: ['admin'],
        permissions: [],
      },
      message: 'Login successful',
    });
  } catch (error) {
    console.error('Login error:', error);
    return c.json({ error: 'Login failed' }, 500);
  }
});

// Get current user
app.get('/api/auth/me', async (c) => {
  const env = c.env;
  const authHeader = c.req.header('Authorization');
  const cookieHeader = c.req.header('Cookie');

  // Try to get token from Bearer header or auth_token cookie
  let token: string | null = null;
  if (authHeader?.startsWith('Bearer ')) {
    token = authHeader.slice(7);
  } else if (cookieHeader) {
    const cookies = cookieHeader.split(';').map(c => c.trim());
    const authCookie = cookies.find(c => c.startsWith('auth_token='));
    if (authCookie) {
      token = authCookie.split('=')[1];
    }
  }

  if (!token) {
    return c.json({ error: 'Unauthorized' }, 401);
  }

  try {
    const payload = await verify(token, env.JWT_SECRET) as { user_id: string; email: string; roles: string[] };

    const user = await env.DB.prepare(
      'SELECT id, email, username, confirmed, first_name, last_name, display_name, phone, location, metadata, last_login, created_at, updated_at FROM auth_users WHERE id = ? AND deleted_at IS NULL'
    ).bind(payload.user_id).first<{
      id: string;
      email: string;
      username: string | null;
      confirmed: number;
      first_name: string | null;
      last_name: string | null;
      display_name: string | null;
      phone: string | null;
      location: string | null;
      metadata: string | null;
      last_login: string | null;
      created_at: string;
      updated_at: string;
    }>();

    if (!user) {
      return c.json({ error: 'User not found' }, 404);
    }

    // Return format expected by frontend: UserResponse
    return c.json({
      user: {
        id: user.id,
        email: user.email,
        username: user.username || '',
        confirmed: user.confirmed === 1,
        firstName: user.first_name || '',
        lastName: user.last_name || '',
        displayName: user.display_name || '',
        phone: user.phone || '',
        location: user.location || '',
        lastLogin: user.last_login,
        metadata: user.metadata || '',
        createdAt: user.created_at,
        updatedAt: user.updated_at,
      },
      roles: payload.roles,
      permissions: [],
    });
  } catch (error) {
    return c.json({ error: 'Invalid token' }, 401);
  }
});

// OAuth providers (return empty for now)
app.get('/api/auth/oauth/providers', (c) => {
  return c.json({ providers: [] });
});

// Logout endpoint - clear the auth cookie
app.post('/api/auth/logout', (c) => {
  c.header('Set-Cookie', 'auth_token=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0');
  return c.json({ message: 'Logged out successfully' });
});

// Dashboard stats
app.get('/api/dashboard/stats', async (c) => {
  const env = c.env;
  try {
    // Get user count
    const userResult = await env.DB.prepare('SELECT COUNT(*) as count FROM auth_users WHERE deleted_at IS NULL').first<{ count: number }>();
    const userCount = userResult?.count || 0;

    // Get table count
    const tableResult = await env.DB.prepare("SELECT COUNT(*) as count FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_cf_%'").first<{ count: number }>();
    const tableCount = tableResult?.count || 0;

    return c.json({
      users: userCount,
      tables: tableCount,
      storage: 0,
      extensions: 0,
      apiCalls: 0,
      activeConnections: 0,
    });
  } catch (error) {
    console.error('Dashboard stats error:', error);
    return c.json({
      users: 0,
      tables: 0,
      storage: 0,
      extensions: 0,
      apiCalls: 0,
      activeConnections: 0,
    });
  }
});

// Admin users list
app.get('/api/admin/users', async (c) => {
  const env = c.env;
  const page = parseInt(c.req.query('page') || '1');
  const pageSize = parseInt(c.req.query('pageSize') || '20');
  const offset = (page - 1) * pageSize;

  try {
    const users = await env.DB.prepare(
      'SELECT id, email, username, confirmed, first_name, last_name, display_name, phone, location, metadata, last_login, created_at, updated_at FROM auth_users WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT ? OFFSET ?'
    ).bind(pageSize, offset).all<{
      id: string;
      email: string;
      username: string | null;
      confirmed: number;
      first_name: string | null;
      last_name: string | null;
      display_name: string | null;
      phone: string | null;
      location: string | null;
      metadata: string | null;
      last_login: string | null;
      created_at: string;
      updated_at: string;
    }>();

    const countResult = await env.DB.prepare('SELECT COUNT(*) as count FROM auth_users WHERE deleted_at IS NULL').first<{ count: number }>();
    const total = countResult?.count || 0;

    const items = (users.results || []).map(u => ({
      id: u.id,
      email: u.email,
      username: u.username || '',
      confirmed: u.confirmed === 1,
      firstName: u.first_name || '',
      lastName: u.last_name || '',
      displayName: u.display_name || '',
      phone: u.phone || '',
      location: u.location || '',
      lastLogin: u.last_login,
      metadata: u.metadata || '',
      createdAt: u.created_at,
      updatedAt: u.updated_at,
    }));

    return c.json({
      items,
      total,
      page,
      pageSize,
      totalPages: Math.ceil(total / pageSize),
    });
  } catch (error) {
    console.error('Admin users error:', error);
    return c.json({ items: [], total: 0, page: 1, pageSize: 20, totalPages: 0 });
  }
});

// IAM roles
app.get('/api/admin/iam/roles', async (c) => {
  const env = c.env;
  try {
    const roles = await env.DB.prepare('SELECT * FROM iam_roles').all();
    return c.json(roles.results || []);
  } catch (error) {
    // Table might not exist yet
    return c.json([]);
  }
});

// IAM policies
app.get('/api/admin/iam/policies', async (c) => {
  const env = c.env;
  try {
    const policies = await env.DB.prepare('SELECT * FROM iam_policies').all();
    return c.json(policies.results || []);
  } catch (error) {
    return c.json([]);
  }
});

// IAM users (user-role mappings)
app.get('/api/admin/iam/users', async (c) => {
  const env = c.env;
  try {
    const users = await env.DB.prepare('SELECT * FROM iam_user_roles').all();
    return c.json(users.results || []);
  } catch (error) {
    return c.json([]);
  }
});

// Database tables
app.get('/api/admin/database/tables', async (c) => {
  const env = c.env;
  try {
    const tables = await env.DB.prepare(
      "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_cf_%' ORDER BY name"
    ).all<{ name: string }>();

    const tableList = await Promise.all((tables.results || []).map(async (t) => {
      const countResult = await env.DB.prepare(`SELECT COUNT(*) as count FROM "${t.name}"`).first<{ count: number }>();
      return {
        name: t.name,
        rowCount: countResult?.count || 0,
      };
    }));

    return c.json(tableList);
  } catch (error) {
    console.error('Database tables error:', error);
    return c.json([]);
  }
});

// Database info
app.get('/api/admin/database/info', async (c) => {
  return c.json({
    type: 'D1',
    version: 'Cloudflare D1',
    size: 0,
    path: 'remote',
  });
});

// Settings
app.get('/api/settings', async (c) => {
  const env = c.env;
  try {
    const settings = await env.DB.prepare('SELECT key, value FROM settings').all<{ key: string; value: string }>();
    const settingsObj: Record<string, string> = {};
    for (const s of settings.results || []) {
      settingsObj[s.key] = s.value;
    }
    return c.json(settingsObj);
  } catch (error) {
    // Table might not exist
    return c.json({});
  }
});

app.put('/api/settings', async (c) => {
  const env = c.env;
  try {
    const body = await c.req.json<Record<string, string>>();
    for (const [key, value] of Object.entries(body)) {
      await env.DB.prepare('INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)').bind(key, value).run();
    }
    return c.json({ success: true });
  } catch (error) {
    console.error('Settings update error:', error);
    return c.json({ error: 'Failed to update settings' }, 500);
  }
});

// TextEncoder/Decoder for string conversion
const encoder = new TextEncoder();
const decoder = new TextDecoder();

// Memory management
let wasmMemory: WebAssembly.Memory;
let wasmInstance: WebAssembly.Instance;

// Transaction state (simplified - single active tx per request)
let activeTx: D1PreparedStatement | null = null;

// Read string from WASM memory
function readString(ptr: number, len: number): string {
  const bytes = new Uint8Array(wasmMemory.buffer, ptr, len);
  return decoder.decode(bytes);
}

// Write bytes to WASM memory using solobase_alloc
function writeToMemory(data: Uint8Array): bigint {
  if (data.length === 0) return 0n;

  const alloc = wasmInstance.exports.solobase_alloc as (size: number) => number;
  const ptr = alloc(data.length);

  const view = new Uint8Array(wasmMemory.buffer, ptr, data.length);
  view.set(data);

  // Return packed result: (ptr << 32) | len
  return (BigInt(ptr) << 32n) | BigInt(data.length);
}

// Database host functions
async function dbQuery(
  env: Env,
  queryPtr: number,
  queryLen: number,
  paramsPtr: number,
  paramsLen: number
): Promise<bigint> {
  const query = readString(queryPtr, queryLen);

  let params: unknown[] = [];
  if (paramsLen > 0) {
    const paramsJson = readString(paramsPtr, paramsLen);
    params = JSON.parse(paramsJson);
  }

  try {
    const stmt = env.DB.prepare(query).bind(...params);
    const result = await stmt.all();

    const queryResult: QueryResult = {
      columns: result.results.length > 0 ? Object.keys(result.results[0]) : [],
      rows: result.results.map(row => Object.values(row)),
    };

    const json = encoder.encode(JSON.stringify(queryResult));
    return writeToMemory(json);
  } catch (error) {
    const queryResult: QueryResult = {
      columns: [],
      rows: [],
      error: (error as Error).message,
    };
    const json = encoder.encode(JSON.stringify(queryResult));
    return writeToMemory(json);
  }
}

async function dbExec(
  env: Env,
  queryPtr: number,
  queryLen: number,
  paramsPtr: number,
  paramsLen: number
): Promise<bigint> {
  const query = readString(queryPtr, queryLen);

  let params: unknown[] = [];
  if (paramsLen > 0) {
    const paramsJson = readString(paramsPtr, paramsLen);
    params = JSON.parse(paramsJson);
  }

  try {
    const stmt = env.DB.prepare(query).bind(...params);
    const result = await stmt.run();

    const execResult: ExecResult = {
      rows_affected: result.meta.changes || 0,
    };

    const json = encoder.encode(JSON.stringify(execResult));
    return writeToMemory(json);
  } catch (error) {
    const execResult: ExecResult = {
      rows_affected: 0,
      error: (error as Error).message,
    };
    const json = encoder.encode(JSON.stringify(execResult));
    return writeToMemory(json);
  }
}

// Transaction functions (D1 has limited transaction support)
function dbBeginTx(): bigint {
  // D1 doesn't support traditional transactions
  // Return a dummy tx_id
  const result: TransactionResult = { tx_id: 1 };
  const json = encoder.encode(JSON.stringify(result));
  return writeToMemory(json);
}

function dbCommitTx(_txId: number): bigint {
  const result: TransactionResult = {};
  const json = encoder.encode(JSON.stringify(result));
  return writeToMemory(json);
}

function dbRollbackTx(_txId: number): bigint {
  const result: TransactionResult = { error: 'D1 does not support rollback' };
  const json = encoder.encode(JSON.stringify(result));
  return writeToMemory(json);
}

async function dbQueryTx(
  env: Env,
  _txId: number,
  queryPtr: number,
  queryLen: number,
  paramsPtr: number,
  paramsLen: number
): Promise<bigint> {
  // D1 doesn't have real transactions, just run as normal query
  return dbQuery(env, queryPtr, queryLen, paramsPtr, paramsLen);
}

async function dbExecTx(
  env: Env,
  _txId: number,
  queryPtr: number,
  queryLen: number,
  paramsPtr: number,
  paramsLen: number
): Promise<bigint> {
  // D1 doesn't have real transactions, just run as normal exec
  return dbExec(env, queryPtr, queryLen, paramsPtr, paramsLen);
}

// Configuration values for the WASM module
function getConfigValue(env: Env, key: string): string {
  const config: Record<string, string> = {
    'DATABASE_TYPE': 'sqlite',
    'JWT_SECRET': env.JWT_SECRET || 'cloudflare-dev-secret-minimum-32-chars',
    'DEFAULT_ADMIN_EMAIL': env.DEFAULT_ADMIN_EMAIL || 'admin@example.com',
    'DEFAULT_ADMIN_PASSWORD': env.DEFAULT_ADMIN_PASSWORD || 'password123',
    'ENVIRONMENT': 'development',
    'LOG_LEVEL': 'INFO',
  };
  return config[key] || '';
}

// get_config host function - returns config value for a key
function getConfig(env: Env, keyPtr: number, keyLen: number): bigint {
  const key = readString(keyPtr, keyLen);
  const value = getConfigValue(env, key);

  if (!value) {
    return 0n;
  }

  const valueBytes = encoder.encode(value);
  return writeToMemory(valueBytes);
}

// Initialize WASM module with env bindings
async function initWasm(env: Env): Promise<WebAssembly.Instance> {
  const importObject = {
    env: {
      // Config function
      get_config: (keyPtr: number, keyLen: number) => getConfig(env, keyPtr, keyLen),
      // Database functions (must match wasmimport names in builds/wasm/database/host.go)
      db_query: (qPtr: number, qLen: number, pPtr: number, pLen: number) =>
        dbQuery(env, qPtr, qLen, pPtr, pLen),
      db_exec: (qPtr: number, qLen: number, pPtr: number, pLen: number) =>
        dbExec(env, qPtr, qLen, pPtr, pLen),
      db_begin: () => dbBeginTx(),
      db_commit: (txId: number) => dbCommitTx(txId),
      db_rollback: (txId: number) => dbRollbackTx(txId),
    },
    wasi_snapshot_preview1: {
      // WASI stubs for TinyGo
      args_get: () => 0,
      args_sizes_get: (_argcPtr: number, _argvBufSizePtr: number) => {
        const view = new DataView(wasmMemory.buffer);
        view.setUint32(_argcPtr, 0, true);
        view.setUint32(_argvBufSizePtr, 0, true);
        return 0;
      },
      environ_get: () => 0,
      environ_sizes_get: (_countPtr: number, _sizePtr: number) => {
        const view = new DataView(wasmMemory.buffer);
        view.setUint32(_countPtr, 0, true);
        view.setUint32(_sizePtr, 0, true);
        return 0;
      },
      clock_time_get: (_id: number, _precision: bigint, resultPtr: number) => {
        const view = new DataView(wasmMemory.buffer);
        view.setBigUint64(resultPtr, BigInt(Date.now()) * 1000000n, true);
        return 0;
      },
      fd_close: () => 0,
      fd_fdstat_get: (_fd: number, statPtr: number) => {
        // Return stats for stdout/stderr (character device)
        const view = new DataView(wasmMemory.buffer);
        view.setUint8(statPtr, 2); // filetype: character device
        view.setUint16(statPtr + 2, 0, true); // flags
        view.setBigUint64(statPtr + 8, 0n, true); // rights_base
        view.setBigUint64(statPtr + 16, 0n, true); // rights_inheriting
        return 0;
      },
      fd_prestat_get: () => 8, // EBADF - no preopened directories
      fd_prestat_dir_name: () => 8,
      fd_read: () => 0,
      fd_seek: () => 0,
      fd_filestat_get: (_fd: number, _statPtr: number) => 0, // Return success, stats are zeroed
      fd_fdstat_set_flags: () => 0,
      fd_sync: () => 0,
      fd_datasync: () => 0,
      fd_tell: () => 0,
      fd_readdir: () => 0,
      fd_renumber: () => 0,
      fd_allocate: () => 0,
      fd_advise: () => 0,
      fd_filestat_set_size: () => 0,
      fd_filestat_set_times: () => 0,
      fd_pread: () => 0,
      fd_pwrite: () => 0,
      path_create_directory: () => 28, // ENOSYS
      path_filestat_get: () => 28, // ENOSYS
      path_filestat_set_times: () => 28, // ENOSYS
      path_link: () => 28, // ENOSYS
      path_readlink: () => 28, // ENOSYS
      path_remove_directory: () => 28, // ENOSYS
      path_rename: () => 28, // ENOSYS
      path_symlink: () => 28, // ENOSYS
      path_unlink_file: () => 28, // ENOSYS
      sock_accept: () => 28, // ENOSYS
      sock_recv: () => 28, // ENOSYS
      sock_send: () => 28, // ENOSYS
      sock_shutdown: () => 28, // ENOSYS
      fd_write: (_fd: number, iovsPtr: number, iovsLen: number, nwrittenPtr: number) => {
        // Basic stdout/stderr support
        // fd 1 = stdout, fd 2 = stderr
        let written = 0;
        const view = new DataView(wasmMemory.buffer);
        for (let i = 0; i < iovsLen; i++) {
          const ptr = view.getUint32(iovsPtr + i * 8, true);
          const len = view.getUint32(iovsPtr + i * 8 + 4, true);
          const bytes = new Uint8Array(wasmMemory.buffer, ptr, len);
          const text = decoder.decode(bytes);
          if (_fd === 1) {
            console.log(text);
          } else if (_fd === 2) {
            console.error(text);
          } else {
            console.log(`[fd=${_fd}] ${text}`);
          }
          written += len;
        }
        view.setUint32(nwrittenPtr, written, true);
        return 0;
      },
      path_open: () => 28, // ENOSYS
      proc_exit: (_code: number) => {
        // Just log exit, don't actually exit
        console.log(`WASM proc_exit called with code: ${_code}`);
      },
      random_get: (bufPtr: number, bufLen: number) => {
        const buf = new Uint8Array(wasmMemory.buffer, bufPtr, bufLen);
        crypto.getRandomValues(buf);
        return 0;
      },
      // TinyGo scheduler functions
      poll_oneoff: (_inPtr: number, _outPtr: number, _nsubscriptions: number, _neventsPtr: number) => {
        // TinyGo uses this for time.Sleep and async operations
        // Return 0 events (no-op)
        const view = new DataView(wasmMemory.buffer);
        view.setUint32(_neventsPtr, 0, true);
        return 0;
      },
      sched_yield: () => 0,
    },
  };

  const instance = await WebAssembly.instantiate(wasmModule, importObject);
  wasmInstance = instance;
  wasmMemory = instance.exports.memory as WebAssembly.Memory;

  // Call _initialize or _start to set up the TinyGo runtime
  // _initialize is for reactor modules, _start is for command modules
  const initialize = instance.exports._initialize as (() => void) | undefined;
  const start = instance.exports._start as (() => void) | undefined;

  if (initialize) {
    console.log('Calling _initialize for TinyGo runtime...');
    initialize();
  } else if (start) {
    console.log('Calling _start for TinyGo runtime...');
    try {
      start();
    } catch (e) {
      // _start may throw if main() exits, which is expected for empty main()
      console.log('_start completed');
    }
  }

  return instance;
}

// Handle API requests through WASM
// Static assets (UI) are served by Cloudflare's asset serving
app.all('/api/*', async (c) => {
  const env = c.env;

  // Initialize WASM if needed
  if (!wasmInstance) {
    await initWasm(env);
  }

  const handleRequest = wasmInstance.exports.handle_request as (
    methodPtr: number, methodLen: number,
    pathPtr: number, pathLen: number,
    headersPtr: number, headersLen: number,
    bodyPtr: number, bodyLen: number,
  ) => bigint;

  // Prepare request data
  const method = c.req.method;
  const url = new URL(c.req.url);
  const path = url.pathname + url.search;

  // Convert headers to JSON
  const headers: Record<string, string[]> = {};
  c.req.raw.headers.forEach((value, key) => {
    if (!headers[key]) headers[key] = [];
    headers[key].push(value);
  });
  const headersJson = JSON.stringify(headers);

  // Read body
  const body = await c.req.arrayBuffer();

  // Write data to WASM memory
  const methodBytes = encoder.encode(method);
  const pathBytes = encoder.encode(path);
  const headersBytes = encoder.encode(headersJson);
  const bodyBytes = new Uint8Array(body);

  const methodPacked = writeToMemory(methodBytes);
  const pathPacked = writeToMemory(pathBytes);
  const headersPacked = writeToMemory(headersBytes);
  const bodyPacked = writeToMemory(bodyBytes);

  // Extract pointers (upper 32 bits)
  const methodPtr = Number(methodPacked >> 32n);
  const pathPtr = Number(pathPacked >> 32n);
  const headersPtr = Number(headersPacked >> 32n);
  const bodyPtr = Number(bodyPacked >> 32n);

  // Call handle_request
  const result = handleRequest(
    methodPtr, methodBytes.length,
    pathPtr, pathBytes.length,
    headersPtr, headersBytes.length,
    bodyPtr, bodyBytes.length,
  );

  // Unpack result
  const resultPtr = Number(result >> 32n);
  const resultLen = Number(result & 0xFFFFFFFFn);

  if (resultPtr === 0 || resultLen === 0) {
    return c.text('Empty response from WASM', 500);
  }

  // Read response JSON from memory
  const responseBytes = new Uint8Array(wasmMemory.buffer, resultPtr, resultLen);
  const responseJson = decoder.decode(responseBytes);
  const response: HTTPResponse = JSON.parse(responseJson);

  // Build response
  const responseHeaders = new Headers();
  for (const [key, values] of Object.entries(response.headers || {})) {
    for (const value of values) {
      responseHeaders.append(key, value);
    }
  }

  // Decode base64 body if present
  let responseBody: Uint8Array | undefined;
  if (response.body) {
    responseBody = base64ToBytes(response.body);
  }

  return new Response(responseBody, {
    status: response.status,
    headers: responseHeaders,
  });
});

export default app;
