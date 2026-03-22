// D1 Database service handler — implements database.* operations against Cloudflare D1.

import type { Message, BlockResult } from '../types';

// ---------------------------------------------------------------------------
// Request / response types (wire format)
// ---------------------------------------------------------------------------

interface GetReq {
  collection: string;
  id: string;
}

interface ListReq {
  collection: string;
  filters?: FilterDef[];
  sort?: SortDef[];
  limit?: number;
  offset?: number;
}

interface CreateReq {
  collection: string;
  data: Record<string, unknown>;
}

interface UpdateReq {
  collection: string;
  id: string;
  data: Record<string, unknown>;
}

interface DeleteReq {
  collection: string;
  id: string;
}

interface CountReq {
  collection: string;
  filters?: FilterDef[];
}

interface SumReq {
  collection: string;
  field: string;
  filters?: FilterDef[];
}

interface QueryRawReq {
  query: string;
  args?: unknown[];
}

interface ExecRawReq {
  query: string;
  args?: unknown[];
}

interface FilterDef {
  field: string;
  operator?: string;
  value?: unknown;
}

interface SortDef {
  field: string;
  desc?: boolean;
}

interface DbRecord {
  id: string;
  data: Record<string, unknown>;
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

/** Sanitize a SQL identifier (table/column name) — only allow alphanumeric + underscore. */
function sanitize(name: string): string {
  const clean = name.replace(/[^a-zA-Z0-9_]/g, '');
  if (!clean) throw new Error('empty identifier');
  return clean;
}

/** Maximum number of rows returned by a single list query. */
const MAX_LIMIT = 1000;

/** Fields that are system-managed and must not be set by callers in create/update. */
const SYSTEM_FIELDS = new Set(['id']);

/** Map a filter operator string to its SQL equivalent. */
function opToSql(op: string): string {
  switch (op) {
    case 'eq': return '=';
    case 'neq': return '!=';
    case 'gt': return '>';
    case 'gte': return '>=';
    case 'lt': return '<';
    case 'lte': return '<=';
    case 'like': return 'LIKE';
    case 'in': return 'IN';
    case 'is_null': return 'IS NULL';
    case 'is_not_null': return 'IS NOT NULL';
    default: return '=';
  }
}

/** Convert a D1 result row into a Record (extracts `id`, rest goes into `data`). */
function rowToRecord(row: Record<string, unknown>): DbRecord {
  const { id, ...rest } = row;
  return { id: String(id ?? ''), data: rest };
}

/** Build WHERE clause and params from filter definitions. */
function buildWhere(filters: FilterDef[]): { sql: string; params: unknown[] } {
  const clauses: string[] = [];
  const params: unknown[] = [];

  for (const f of filters) {
    const col = sanitize(f.field);
    const op = f.operator ?? 'eq';

    if (op === 'is_null') {
      clauses.push(`${col} IS NULL`);
    } else if (op === 'is_not_null') {
      clauses.push(`${col} IS NOT NULL`);
    } else if (op === 'in') {
      // `value` should be an array — expand to (?, ?, ...)
      const arr = Array.isArray(f.value) ? f.value : [f.value];
      const placeholders = arr.map(() => '?').join(', ');
      clauses.push(`${col} IN (${placeholders})`);
      params.push(...arr);
    } else {
      clauses.push(`${col} ${opToSql(op)} ?`);
      params.push(f.value);
    }
  }

  const sql = clauses.length > 0 ? clauses.join(' AND ') : '1=1';
  return { sql, params };
}

// ---------------------------------------------------------------------------
// Main handler
// ---------------------------------------------------------------------------

export async function d1Handler(db: D1Database, msg: Message): Promise<BlockResult> {
  let req: any;
  try {
    req = msg.data.length > 0 ? JSON.parse(decoder.decode(msg.data)) : {};
  } catch (e) {
    return error('invalid-argument', `failed to parse request: ${e}`);
  }

  try {
    switch (msg.kind) {
      // ----- GET -----
      case 'database.get': {
        const { collection, id } = req as GetReq;
        const table = sanitize(collection);
        const stmt = db.prepare(`SELECT * FROM ${table} WHERE id = ?`).bind(id);
        const row = await stmt.first<Record<string, unknown>>();
        if (!row) return error('not-found', 'record not found');
        return success(rowToRecord(row));
      }

      // ----- LIST -----
      case 'database.list': {
        const { collection, filters = [], sort = [], limit: rawLimit, offset: rawOffset } = req as ListReq;
        const table = sanitize(collection);
        const where = buildWhere(filters);
        const limit = Math.min(rawLimit && rawLimit > 0 ? rawLimit : 100, MAX_LIMIT);
        const offset = rawOffset ?? 0;

        // Count query
        const countStmt = db
          .prepare(`SELECT COUNT(*) as cnt FROM ${table} WHERE ${where.sql}`)
          .bind(...where.params);
        const countRow = await countStmt.first<{ cnt: number }>();
        const totalCount = countRow?.cnt ?? 0;

        // Data query
        let sql = `SELECT * FROM ${table} WHERE ${where.sql}`;

        if (sort.length > 0) {
          const orderParts = sort.map((s) => {
            const col = sanitize(s.field);
            return s.desc ? `${col} DESC` : `${col} ASC`;
          });
          sql += ` ORDER BY ${orderParts.join(', ')}`;
        }

        sql += ` LIMIT ? OFFSET ?`;
        const dataParams = [...where.params, limit, offset];

        const results = await db.prepare(sql).bind(...dataParams).all<Record<string, unknown>>();
        const records = (results.results ?? []).map(rowToRecord);

        const page = limit > 0 ? Math.floor(offset / limit) + 1 : 1;

        return success({
          records,
          total_count: totalCount,
          page,
          page_size: limit,
        });
      }

      // ----- CREATE -----
      case 'database.create': {
        const { collection, data } = req as CreateReq;
        const table = sanitize(collection);
        const id = crypto.randomUUID();
        const now = new Date().toISOString();

        const fields: Record<string, unknown> = {};
        for (const [k, v] of Object.entries(data)) {
          if (!SYSTEM_FIELDS.has(k)) fields[k] = v;
        }
        if (!fields.created_at) fields.created_at = now;
        if (!fields.updated_at) fields.updated_at = now;

        const columns = ['id', ...Object.keys(fields).map(sanitize)];
        const placeholders = columns.map(() => '?').join(', ');
        const params = [id, ...Object.values(fields)];

        const sql = `INSERT INTO ${table} (${columns.join(', ')}) VALUES (${placeholders})`;
        await db.prepare(sql).bind(...params).run();

        return success({ id, data: fields });
      }

      // ----- UPDATE -----
      case 'database.update': {
        const { collection, id, data } = req as UpdateReq;
        const table = sanitize(collection);
        const now = new Date().toISOString();

        const fields: Record<string, unknown> = { ...data, updated_at: now };

        const sets = Object.keys(fields).map((k) => `${sanitize(k)} = ?`);
        const params = [...Object.values(fields), id];

        const sql = `UPDATE ${table} SET ${sets.join(', ')} WHERE id = ?`;
        await db.prepare(sql).bind(...params).run();

        // Re-read the updated record
        const row = await db
          .prepare(`SELECT * FROM ${table} WHERE id = ?`)
          .bind(id)
          .first<Record<string, unknown>>();
        if (!row) return error('not-found', 'record not found after update');
        return success(rowToRecord(row));
      }

      // ----- DELETE -----
      case 'database.delete': {
        const { collection, id } = req as DeleteReq;
        const table = sanitize(collection);
        await db.prepare(`DELETE FROM ${table} WHERE id = ?`).bind(id).run();
        return successEmpty();
      }

      // ----- COUNT -----
      case 'database.count': {
        const { collection, filters = [] } = req as CountReq;
        const table = sanitize(collection);
        const where = buildWhere(filters);
        const row = await db
          .prepare(`SELECT COUNT(*) as cnt FROM ${table} WHERE ${where.sql}`)
          .bind(...where.params)
          .first<{ cnt: number }>();
        return success({ count: row?.cnt ?? 0 });
      }

      // ----- SUM -----
      case 'database.sum': {
        const { collection, field, filters = [] } = req as SumReq;
        const table = sanitize(collection);
        const col = sanitize(field);
        const where = buildWhere(filters);
        const row = await db
          .prepare(`SELECT COALESCE(SUM(${col}), 0) as s FROM ${table} WHERE ${where.sql}`)
          .bind(...where.params)
          .first<{ s: number }>();
        return success({ sum: row?.s ?? 0 });
      }

      // ----- QUERY_RAW (read-only) -----
      case 'database.query_raw': {
        const { query, args = [] } = req as QueryRawReq;
        if (!isReadOnlyQuery(query)) {
          return error('permission-denied', 'query_raw only allows SELECT, PRAGMA, and EXPLAIN statements');
        }
        const results = await db.prepare(query).bind(...args).all<Record<string, unknown>>();
        const records = (results.results ?? []).map(rowToRecord);
        return success(records);
      }

      // ----- EXEC_RAW (restricted) -----
      case 'database.exec_raw': {
        const { query, args = [] } = req as ExecRawReq;
        if (!isSafeMutationQuery(query)) {
          return error('permission-denied', 'only INSERT, UPDATE, and DELETE statements are allowed in exec_raw');
        }
        const result = await db.prepare(query).bind(...args).run();
        return success({ rows_affected: result.meta?.changes ?? 0 });
      }

      default:
        return error('unimplemented', `unknown database operation: ${msg.kind}`);
    }
  } catch (e: any) {
    console.error('D1 error:', e);
    return error('internal', 'database operation failed');
  }
}

/** Reject multi-statement queries (semicolons outside string literals). */
function containsMultipleStatements(sql: string): boolean {
  // Strip string literals to avoid false positives on semicolons inside strings
  const stripped = sql.replace(/'[^']*'/g, '').replace(/"[^"]*"/g, '');
  return stripped.includes(';');
}

/** Only allow SELECT, PRAGMA, and EXPLAIN for query_raw (no mutations). */
function isReadOnlyQuery(sql: string): boolean {
  if (containsMultipleStatements(sql)) return false;
  const trimmed = sql.trim().toUpperCase();
  // Block WITH CTEs that contain mutations (WITH ... DELETE/INSERT/UPDATE)
  if (trimmed.startsWith('WITH')) {
    const mutationKeywords = /\b(INSERT|UPDATE|DELETE|DROP|ALTER|TRUNCATE|CREATE)\b/i;
    return !mutationKeywords.test(sql);
  }
  return (
    trimmed.startsWith('SELECT') ||
    trimmed.startsWith('PRAGMA') ||
    trimmed.startsWith('EXPLAIN')
  );
}

/** Allow INSERT, UPDATE, DELETE + CREATE/DROP TABLE for custom tables. No ALTER/TRUNCATE. */
function isSafeMutationQuery(sql: string): boolean {
  if (containsMultipleStatements(sql)) return false;
  const trimmed = sql.trim().toUpperCase();
  return (
    trimmed.startsWith('INSERT') ||
    trimmed.startsWith('UPDATE') ||
    trimmed.startsWith('DELETE') ||
    trimmed.startsWith('CREATE TABLE') ||
    trimmed.startsWith('DROP TABLE')
  );
}
