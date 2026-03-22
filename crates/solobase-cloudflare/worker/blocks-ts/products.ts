// TypeScript-native products block handler (for testing without WASM).
// Calls host.callBlock() for database/network/config/crypto operations.

import type { Message, BlockResult } from '../types';
import type { RuntimeHost } from '../host';
import { metaGet } from '../convert';

// ---------------------------------------------------------------------------
// Collections
// ---------------------------------------------------------------------------

const PRODUCTS = 'block_products_products';
const GROUPS = 'block_products_groups';
const TYPES = 'block_products_types';
const PRICING = 'block_products_pricing_templates';
const PURCHASES = 'block_products_purchases';
const LINE_ITEMS = 'block_products_line_items';
const GROUP_TEMPLATES = 'block_products_group_templates';
const PRODUCT_TEMPLATES = 'block_products_product_templates';
const VARIABLES = 'block_products_variables';

// ---------------------------------------------------------------------------
// Main handler
// ---------------------------------------------------------------------------

export async function handle(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const action = metaGet(msg.meta, 'req.action') ?? '';
  const path = metaGet(msg.meta, 'req.resource') ?? '';

  // Webhook (no auth)
  if (path === '/b/products/webhooks' || path.startsWith('/b/products/webhooks/')) {
    return handleWebhook(msg, host);
  }

  // Admin routes
  if (path.startsWith('/admin/b/products')) {
    return handleAdmin(action, path, msg, host);
  }

  // User-facing routes
  if (path.startsWith('/b/products')) {
    return handleUser(action, path, msg, host);
  }

  return errResult('not-found', 'not found');
}

// ---------------------------------------------------------------------------
// Admin route dispatch
// ---------------------------------------------------------------------------

async function handleAdmin(action: string, path: string, msg: Message, host: RuntimeHost): Promise<BlockResult> {
  switch (true) {
    // Products
    case action === 'retrieve' && path === '/admin/b/products/products':
      return handleListProducts(msg, host);
    case action === 'retrieve' && path.startsWith('/admin/b/products/products/'):
      return handleGetProduct(msg, host, path, '/admin/b/products/products/');
    case action === 'create' && path === '/admin/b/products/products':
      return handleCreateProduct(msg, host);
    case action === 'update' && path.startsWith('/admin/b/products/products/'):
      return handleUpdateProduct(msg, host, path, '/admin/b/products/products/');
    case action === 'delete' && path.startsWith('/admin/b/products/products/'):
      return handleDeleteProduct(msg, host, path, '/admin/b/products/products/');
    // Groups
    case action === 'retrieve' && path === '/admin/b/products/groups':
      return handleListGroups(msg, host);
    case action === 'create' && path === '/admin/b/products/groups':
      return handleCreateGroup(msg, host);
    case action === 'update' && path.startsWith('/admin/b/products/groups/'):
      return handleUpdateGroup(msg, host, path, '/admin/b/products/groups/');
    case action === 'delete' && path.startsWith('/admin/b/products/groups/'):
      return handleDeleteGroup(msg, host, path, '/admin/b/products/groups/');
    // Types
    case action === 'retrieve' && path === '/admin/b/products/types':
      return handleListTypes(msg, host);
    case action === 'create' && path === '/admin/b/products/types':
      return handleCreateType(msg, host);
    case action === 'delete' && path.startsWith('/admin/b/products/types/'):
      return handleDeleteType(msg, host, path);
    // Pricing templates
    case action === 'retrieve' && path === '/admin/b/products/pricing':
      return handleListPricing(msg, host);
    case action === 'create' && path === '/admin/b/products/pricing':
      return handleCreatePricing(msg, host);
    case action === 'update' && path.startsWith('/admin/b/products/pricing/'):
      return handleUpdatePricing(msg, host, path);
    case action === 'delete' && path.startsWith('/admin/b/products/pricing/'):
      return handleDeletePricing(msg, host, path);
    // Variables
    case action === 'retrieve' && path === '/admin/b/products/variables':
      return handleListVariables(msg, host);
    case action === 'create' && path === '/admin/b/products/variables':
      return handleCreateVariable(msg, host);
    case action === 'update' && path.startsWith('/admin/b/products/variables/'):
      return handleUpdateVariable(msg, host, path);
    case action === 'delete' && path.startsWith('/admin/b/products/variables/'):
      return handleDeleteVariable(msg, host, path);
    // Purchases (admin view)
    case action === 'retrieve' && path === '/admin/b/products/purchases':
      return handleListPurchasesAdmin(msg, host);
    case action === 'retrieve' && path.startsWith('/admin/b/products/purchases/') && !path.endsWith('/refund'):
      return handleGetPurchase(msg, host, path);
    case action === 'update' && path.startsWith('/admin/b/products/purchases/') && path.endsWith('/refund'):
      return handleRefund(msg, host, path);
    // Stats
    case action === 'retrieve' && path === '/admin/b/products/stats':
      return handleStats(msg, host);
    default:
      return errResult('not-found', 'not found');
  }
}

// ---------------------------------------------------------------------------
// User route dispatch
// ---------------------------------------------------------------------------

async function handleUser(action: string, path: string, msg: Message, host: RuntimeHost): Promise<BlockResult> {
  switch (true) {
    // User's own products
    case action === 'retrieve' && path === '/b/products/products':
      return handleUserListProducts(msg, host);
    case action === 'retrieve' && path.startsWith('/b/products/products/'):
      return handleUserGetProduct(msg, host, path);
    case action === 'create' && path === '/b/products/products':
      return handleUserCreateProduct(msg, host);
    case action === 'update' && path.startsWith('/b/products/products/'):
      return handleUserUpdateProduct(msg, host, path);
    case action === 'delete' && path.startsWith('/b/products/products/'):
      return handleUserDeleteProduct(msg, host, path);
    // User's own groups
    case action === 'retrieve' && path === '/b/products/groups':
      return handleUserListGroups(msg, host);
    case action === 'retrieve' && path.startsWith('/b/products/groups/') && !path.endsWith('/products'):
      return handleUserGetGroup(msg, host, path);
    case action === 'create' && path === '/b/products/groups':
      return handleUserCreateGroup(msg, host);
    case action === 'update' && path.startsWith('/b/products/groups/') && !path.endsWith('/products'):
      return handleUserUpdateGroup(msg, host, path);
    case action === 'delete' && path.startsWith('/b/products/groups/') && !path.endsWith('/products'):
      return handleUserDeleteGroup(msg, host, path);
    // Products in a group
    case action === 'retrieve' && path.startsWith('/b/products/groups/') && path.endsWith('/products'):
      return handleUserGroupProducts(msg, host, path);
    // Read-only: types and group templates
    case action === 'retrieve' && path === '/b/products/types':
      return handleListTypes(msg, host);
    case action === 'retrieve' && path === '/b/products/group-templates':
      return handleUserListGroupTemplates(msg, host);
    // Catalog
    case action === 'retrieve' && path === '/b/products/catalog':
      return handleCatalog(msg, host);
    case action === 'retrieve' && path.startsWith('/b/products/catalog/'):
      return handleGetProductPublic(msg, host, path);
    // Pricing, purchases, checkout
    case action === 'create' && path === '/b/products/calculate-price':
      return handleCalculatePrice(msg, host);
    case action === 'create' && path === '/b/products/purchases':
      return handleCreatePurchase(msg, host);
    case action === 'retrieve' && path === '/b/products/purchases':
      return handleListPurchasesUser(msg, host);
    case action === 'retrieve' && path.startsWith('/b/products/purchases/'):
      return handleGetPurchase(msg, host, path);
    case action === 'create' && path === '/b/products/checkout':
      return handleCheckout(msg, host);
    default:
      return errResult('not-found', 'not found');
  }
}

// ---------------------------------------------------------------------------
// Admin: Product CRUD
// ---------------------------------------------------------------------------

async function handleListProducts(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const page = parseInt(queryParam(msg, 'page') ?? '1', 10);
  const pageSize = parseInt(queryParam(msg, 'page_size') ?? '20', 10);

  const filters: Filter[] = [];
  const groupId = queryParam(msg, 'group_id');
  if (groupId) filters.push({ field: 'group_id', operator: 'eq', value: groupId });
  const status = queryParam(msg, 'status');
  if (status) filters.push({ field: 'status', operator: 'eq', value: status });
  const search = queryParam(msg, 'search');
  if (search) filters.push({ field: 'name', operator: 'like', value: `%${search}%` });

  return dbPaginatedList(host, msg, PRODUCTS, page, pageSize, filters,
    [{ field: 'created_at', desc: true }]);
}

async function handleGetProduct(msg: Message, host: RuntimeHost, path: string, prefix: string): Promise<BlockResult> {
  const id = path.substring(prefix.length);
  if (!id) return errResult('invalid-argument', 'Missing product ID');
  const record = await dbGet(host, PRODUCTS, id);
  if (!record) return errResult('not-found', 'Product not found');
  return jsonRespond(msg, record);
}

async function handleCreateProduct(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  const now = new Date().toISOString();
  if (!body.status) body.status = 'draft';
  body.created_at = now;
  body.updated_at = now;
  body.created_by = metaGet(msg.meta, 'auth.user_id') ?? '';

  const record = await dbCreate(host, PRODUCTS, body);
  if (!record) return errResult('internal', 'Database error');
  return jsonRespond(msg, record);
}

async function handleUpdateProduct(msg: Message, host: RuntimeHost, path: string, prefix: string): Promise<BlockResult> {
  const id = path.substring(prefix.length);
  if (!id) return errResult('invalid-argument', 'Missing product ID');

  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');
  body.updated_at = new Date().toISOString();

  const record = await dbUpdate(host, PRODUCTS, id, body);
  if (!record) return errResult('not-found', 'Product not found');
  return jsonRespond(msg, record);
}

async function handleDeleteProduct(msg: Message, host: RuntimeHost, path: string, prefix: string): Promise<BlockResult> {
  const id = path.substring(prefix.length);
  if (!id) return errResult('invalid-argument', 'Missing product ID');

  const result = await callService(host, 'wafer-run/database', 'database.delete', {
    collection: PRODUCTS, id,
  });
  if (!result) return errResult('not-found', 'Product not found');
  return jsonRespond(msg, { deleted: true });
}

// ---------------------------------------------------------------------------
// Admin: Groups
// ---------------------------------------------------------------------------

async function handleListGroups(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const result = await dbList(host, GROUPS, {
    filters: [],
    sort: [{ field: 'name', desc: false }],
    limit: 1000,
    offset: 0,
  });
  if (!result) return errResult('internal', 'Database error');
  return jsonRespond(msg, result);
}

async function handleCreateGroup(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');
  body.created_at = new Date().toISOString();
  if (!body.user_id) body.user_id = metaGet(msg.meta, 'auth.user_id') ?? '';

  const record = await dbCreate(host, GROUPS, body);
  if (!record) return errResult('internal', 'Database error');
  return jsonRespond(msg, record);
}

async function handleUpdateGroup(msg: Message, host: RuntimeHost, path: string, prefix: string): Promise<BlockResult> {
  const id = path.substring(prefix.length);
  if (!id) return errResult('invalid-argument', 'Missing group ID');

  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  const record = await dbUpdate(host, GROUPS, id, body);
  if (!record) return errResult('not-found', 'Group not found');
  return jsonRespond(msg, record);
}

async function handleDeleteGroup(msg: Message, host: RuntimeHost, path: string, prefix: string): Promise<BlockResult> {
  const id = path.substring(prefix.length);
  if (!id) return errResult('invalid-argument', 'Missing group ID');

  const result = await callService(host, 'wafer-run/database', 'database.delete', {
    collection: GROUPS, id,
  });
  if (!result) return errResult('not-found', 'Group not found');
  return jsonRespond(msg, { deleted: true });
}

// ---------------------------------------------------------------------------
// Admin: Types
// ---------------------------------------------------------------------------

async function handleListTypes(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const result = await dbList(host, TYPES, {
    filters: [], sort: [], limit: 1000, offset: 0,
  });
  if (!result) return errResult('internal', 'Database error');
  return jsonRespond(msg, result);
}

async function handleCreateType(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  const record = await dbCreate(host, TYPES, body);
  if (!record) return errResult('internal', 'Database error');
  return jsonRespond(msg, record);
}

async function handleDeleteType(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const id = path.replace('/admin/b/products/types/', '');
  if (!id) return errResult('invalid-argument', 'Missing type ID');

  const result = await callService(host, 'wafer-run/database', 'database.delete', {
    collection: TYPES, id,
  });
  if (!result) return errResult('not-found', 'Type not found');
  return jsonRespond(msg, { deleted: true });
}

// ---------------------------------------------------------------------------
// Admin: Pricing Templates
// ---------------------------------------------------------------------------

async function handleListPricing(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const result = await dbList(host, PRICING, {
    filters: [],
    sort: [{ field: 'name', desc: false }],
    limit: 1000,
    offset: 0,
  });
  if (!result) return errResult('internal', 'Database error');
  return jsonRespond(msg, result);
}

async function handleCreatePricing(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');
  body.created_at = new Date().toISOString();

  const record = await dbCreate(host, PRICING, body);
  if (!record) return errResult('internal', 'Database error');
  return jsonRespond(msg, record);
}

async function handleUpdatePricing(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const id = path.replace('/admin/b/products/pricing/', '');
  if (!id) return errResult('invalid-argument', 'Missing pricing template ID');

  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  const record = await dbUpdate(host, PRICING, id, body);
  if (!record) return errResult('not-found', 'Pricing template not found');
  return jsonRespond(msg, record);
}

async function handleDeletePricing(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const id = path.replace('/admin/b/products/pricing/', '');
  if (!id) return errResult('invalid-argument', 'Missing pricing template ID');

  const result = await callService(host, 'wafer-run/database', 'database.delete', {
    collection: PRICING, id,
  });
  if (!result) return errResult('not-found', 'Pricing template not found');
  return jsonRespond(msg, { deleted: true });
}

// ---------------------------------------------------------------------------
// Admin: Variables
// ---------------------------------------------------------------------------

async function handleListVariables(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const filters: Filter[] = [];
  const scope = queryParam(msg, 'scope');
  if (scope) filters.push({ field: 'scope', operator: 'eq', value: scope });
  const productId = queryParam(msg, 'product_id');
  if (productId) filters.push({ field: 'product_id', operator: 'eq', value: productId });

  const result = await dbList(host, VARIABLES, {
    filters,
    sort: [{ field: 'name', desc: false }],
    limit: 1000,
    offset: 0,
  });
  if (!result) return errResult('internal', 'Database error');
  return jsonRespond(msg, result);
}

async function handleCreateVariable(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<{
    name: string;
    var_type?: string;
    default_value?: unknown;
    scope?: string;
    product_id?: string;
  }>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  const data: Record<string, unknown> = {
    name: body.name,
    var_type: body.var_type ?? 'number',
    scope: body.scope ?? 'system',
    created_at: new Date().toISOString(),
  };
  if (body.default_value !== undefined) data.default_value = body.default_value;
  if (body.product_id) data.product_id = body.product_id;

  const record = await dbCreate(host, VARIABLES, data);
  if (!record) return errResult('internal', 'Database error');
  return jsonRespond(msg, record);
}

async function handleUpdateVariable(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const id = path.replace('/admin/b/products/variables/', '');
  if (!id) return errResult('invalid-argument', 'Missing variable ID');

  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');
  body.updated_at = new Date().toISOString();

  const record = await dbUpdate(host, VARIABLES, id, body);
  if (!record) return errResult('not-found', 'Variable not found');
  return jsonRespond(msg, record);
}

async function handleDeleteVariable(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const id = path.replace('/admin/b/products/variables/', '');
  if (!id) return errResult('invalid-argument', 'Missing variable ID');

  const result = await callService(host, 'wafer-run/database', 'database.delete', {
    collection: VARIABLES, id,
  });
  if (!result) return errResult('not-found', 'Variable not found');
  return jsonRespond(msg, { deleted: true });
}

// ---------------------------------------------------------------------------
// Admin: Purchases
// ---------------------------------------------------------------------------

async function handleListPurchasesAdmin(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const page = parseInt(queryParam(msg, 'page') ?? '1', 10);
  const pageSize = parseInt(queryParam(msg, 'page_size') ?? '20', 10);

  const filters: Filter[] = [];
  const status = queryParam(msg, 'status');
  if (status) filters.push({ field: 'status', operator: 'eq', value: status });
  const userId = queryParam(msg, 'user_id');
  if (userId) filters.push({ field: 'user_id', operator: 'eq', value: userId });

  return dbPaginatedList(host, msg, PURCHASES, page, pageSize, filters,
    [{ field: 'created_at', desc: true }]);
}

async function handleRefund(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  // /admin/b/products/purchases/{id}/refund
  const rest = path.replace('/admin/b/products/purchases/', '');
  const id = rest.replace('/refund', '');
  if (!id) return errResult('invalid-argument', 'Missing purchase ID');

  const body = parseBody<{ reason?: string }>(msg) ?? {};

  const purchase = await dbGet(host, PURCHASES, id);
  if (!purchase) return errResult('not-found', 'Purchase not found');

  const currentStatus = (purchase.data?.status ?? '') as string;
  if (currentStatus !== 'completed') {
    return errResult('invalid-argument', `Can only refund completed purchases (current status: ${currentStatus})`);
  }

  const now = new Date().toISOString();
  const updateData: Record<string, unknown> = {
    status: 'refunded',
    refunded_at: now,
    refunded_by: metaGet(msg.meta, 'auth.user_id') ?? '',
    updated_at: now,
  };
  if (body.reason) updateData.refund_reason = body.reason;

  const record = await dbUpdate(host, PURCHASES, id, updateData);
  if (!record) return errResult('internal', 'Database error');
  return jsonRespond(msg, record);
}

// ---------------------------------------------------------------------------
// Admin: Stats
// ---------------------------------------------------------------------------

async function handleStats(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const totalProducts = await dbCount(host, PRODUCTS, []);
  const activeProducts = await dbCount(host, PRODUCTS, [
    { field: 'status', operator: 'eq', value: 'active' },
  ]);
  const totalPurchases = await dbCount(host, PURCHASES, []);
  const totalRevenue = await dbSum(host, PURCHASES, 'total_cents', [
    { field: 'status', operator: 'eq', value: 'completed' },
  ]);
  const totalGroups = await dbCount(host, GROUPS, []);

  return jsonRespond(msg, {
    total_products: totalProducts,
    active_products: activeProducts,
    total_purchases: totalPurchases,
    total_revenue: totalRevenue,
    total_groups: totalGroups,
  });
}

// ---------------------------------------------------------------------------
// User: Products
// ---------------------------------------------------------------------------

async function handleUserListProducts(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  if (!userId) return errResult('unauthenticated', 'Not authenticated');

  const page = parseInt(queryParam(msg, 'page') ?? '1', 10);
  const pageSize = parseInt(queryParam(msg, 'page_size') ?? '20', 10);

  const filters: Filter[] = [{ field: 'created_by', operator: 'eq', value: userId }];
  const groupId = queryParam(msg, 'group_id');
  if (groupId) filters.push({ field: 'group_id', operator: 'eq', value: groupId });
  const status = queryParam(msg, 'status');
  if (status) filters.push({ field: 'status', operator: 'eq', value: status });
  const search = queryParam(msg, 'search');
  if (search) filters.push({ field: 'name', operator: 'like', value: `%${search}%` });

  return dbPaginatedList(host, msg, PRODUCTS, page, pageSize, filters,
    [{ field: 'created_at', desc: true }]);
}

async function handleUserGetProduct(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  const id = path.replace('/b/products/products/', '');
  if (!id) return errResult('invalid-argument', 'Missing product ID');

  const record = await dbGet(host, PRODUCTS, id);
  if (!record) return errResult('not-found', 'Product not found');
  if (fieldStr(record, 'created_by') !== userId) return errResult('not-found', 'Product not found');
  return jsonRespond(msg, record);
}

async function handleUserCreateProduct(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  if (!userId) return errResult('unauthenticated', 'Not authenticated');

  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  // Verify user owns the group (if provided)
  const groupId = asString(body.group_id);
  if (groupId) {
    const group = await dbGet(host, GROUPS, groupId);
    if (!group) return errResult('invalid-argument', 'Group not found');
    if (fieldStr(group, 'user_id') !== userId) return errResult('invalid-argument', "You don't own this group");
  }

  const now = new Date().toISOString();
  if (!body.status) body.status = 'draft';
  body.created_at = now;
  body.updated_at = now;
  body.created_by = userId;
  if (!body.product_template_id) body.product_template_id = 1;

  const record = await dbCreate(host, PRODUCTS, body);
  if (!record) return errResult('internal', 'Database error');
  return jsonRespond(msg, record);
}

async function handleUserUpdateProduct(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  const id = path.replace('/b/products/products/', '');
  if (!id) return errResult('invalid-argument', 'Missing product ID');

  // Verify ownership
  const existing = await dbGet(host, PRODUCTS, id);
  if (!existing) return errResult('not-found', 'Product not found');
  if (fieldStr(existing, 'created_by') !== userId) return errResult('not-found', 'Product not found');

  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');
  delete body.created_by; // prevent ownership change
  body.updated_at = new Date().toISOString();

  const record = await dbUpdate(host, PRODUCTS, id, body);
  if (!record) return errResult('internal', 'Database error');
  return jsonRespond(msg, record);
}

async function handleUserDeleteProduct(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  const id = path.replace('/b/products/products/', '');
  if (!id) return errResult('invalid-argument', 'Missing product ID');

  const existing = await dbGet(host, PRODUCTS, id);
  if (!existing) return errResult('not-found', 'Product not found');
  if (fieldStr(existing, 'created_by') !== userId) return errResult('not-found', 'Product not found');

  await callService(host, 'wafer-run/database', 'database.delete', { collection: PRODUCTS, id });
  return jsonRespond(msg, { deleted: true });
}

// ---------------------------------------------------------------------------
// User: Groups
// ---------------------------------------------------------------------------

async function handleUserListGroups(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  if (!userId) return errResult('unauthenticated', 'Not authenticated');

  const result = await dbList(host, GROUPS, {
    filters: [{ field: 'user_id', operator: 'eq', value: userId }],
    sort: [{ field: 'name', desc: false }],
    limit: 1000,
    offset: 0,
  });
  if (!result) return errResult('internal', 'Database error');
  return jsonRespond(msg, result);
}

async function handleUserGetGroup(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  const id = path.replace('/b/products/groups/', '');
  if (!id) return errResult('invalid-argument', 'Missing group ID');

  const record = await dbGet(host, GROUPS, id);
  if (!record) return errResult('not-found', 'Group not found');
  if (fieldStr(record, 'user_id') !== userId) return errResult('not-found', 'Group not found');
  return jsonRespond(msg, record);
}

async function handleUserCreateGroup(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  if (!userId) return errResult('unauthenticated', 'Not authenticated');

  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');
  body.created_at = new Date().toISOString();
  body.user_id = userId;
  if (!body.group_template_id) body.group_template_id = 1;

  const record = await dbCreate(host, GROUPS, body);
  if (!record) return errResult('internal', 'Database error');
  return jsonRespond(msg, record);
}

async function handleUserUpdateGroup(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  const id = path.replace('/b/products/groups/', '');
  if (!id) return errResult('invalid-argument', 'Missing group ID');

  const existing = await dbGet(host, GROUPS, id);
  if (!existing) return errResult('not-found', 'Group not found');
  if (fieldStr(existing, 'user_id') !== userId) return errResult('not-found', 'Group not found');

  const body = parseBody<Record<string, unknown>>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');
  delete body.user_id; // prevent ownership change

  const record = await dbUpdate(host, GROUPS, id, body);
  if (!record) return errResult('internal', 'Database error');
  return jsonRespond(msg, record);
}

async function handleUserDeleteGroup(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  const id = path.replace('/b/products/groups/', '');
  if (!id) return errResult('invalid-argument', 'Missing group ID');

  const existing = await dbGet(host, GROUPS, id);
  if (!existing) return errResult('not-found', 'Group not found');
  if (fieldStr(existing, 'user_id') !== userId) return errResult('not-found', 'Group not found');

  await callService(host, 'wafer-run/database', 'database.delete', { collection: GROUPS, id });
  return jsonRespond(msg, { deleted: true });
}

async function handleUserGroupProducts(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  // /b/products/groups/{id}/products
  const rest = path.replace('/b/products/groups/', '');
  const groupId = rest.replace('/products', '');
  if (!groupId) return errResult('invalid-argument', 'Missing group ID');

  // Verify group ownership
  const group = await dbGet(host, GROUPS, groupId);
  if (!group) return errResult('not-found', 'Group not found');
  if (fieldStr(group, 'user_id') !== userId) return errResult('not-found', 'Group not found');

  const page = parseInt(queryParam(msg, 'page') ?? '1', 10);
  const pageSize = parseInt(queryParam(msg, 'page_size') ?? '20', 10);

  return dbPaginatedList(host, msg, PRODUCTS, page, pageSize,
    [{ field: 'group_id', operator: 'eq', value: groupId }],
    [{ field: 'created_at', desc: true }]);
}

async function handleUserListGroupTemplates(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const result = await dbList(host, GROUP_TEMPLATES, {
    filters: [], sort: [], limit: 1000, offset: 0,
  });
  if (!result) return errResult('internal', 'Database error');
  return jsonRespond(msg, result);
}

// ---------------------------------------------------------------------------
// Catalog (public active products)
// ---------------------------------------------------------------------------

async function handleCatalog(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const page = parseInt(queryParam(msg, 'page') ?? '1', 10);
  const pageSize = parseInt(queryParam(msg, 'page_size') ?? '20', 10);

  return dbPaginatedList(host, msg, PRODUCTS, page, pageSize,
    [{ field: 'status', operator: 'eq', value: 'active' }],
    [{ field: 'name', desc: false }]);
}

async function handleGetProductPublic(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  const id = path.replace('/b/products/catalog/', '');
  if (!id) return errResult('invalid-argument', 'Missing product ID');

  const record = await dbGet(host, PRODUCTS, id);
  if (!record) return errResult('not-found', 'Product not found');
  if (fieldStr(record, 'status') !== 'active') return errResult('not-found', 'Product not found');
  return jsonRespond(msg, record);
}

// ---------------------------------------------------------------------------
// Pricing calculation
// ---------------------------------------------------------------------------

async function handleCalculatePrice(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<{
    product_id: string;
    variables?: Record<string, number>;
    quantity?: number;
  }>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  const variables = body.variables ?? {};
  const quantity = body.quantity ?? 1;

  const product = await dbGet(host, PRODUCTS, body.product_id);
  if (!product) return errResult('not-found', 'Product not found');

  const templateId = fieldStr(product, 'pricing_template_id');
  if (!templateId) {
    // Direct price from product
    const basePrice = asNumber(product.data?.base_price, 0);
    const total = basePrice * quantity;
    return jsonRespond(msg, {
      unit_price: basePrice,
      quantity,
      total,
      currency: fieldStr(product, 'currency') || 'USD',
    });
  }

  const template = await dbGet(host, PRICING, templateId);
  if (!template) return errResult('internal', 'Pricing template not found');

  const formula = fieldStr(template, 'price_formula');
  if (!formula) return errResult('internal', 'Empty pricing formula');

  let unitPrice: number;
  try {
    unitPrice = evaluateFormula(formula, variables);
  } catch (e: any) {
    return errResult('invalid-argument', `Formula evaluation error: ${e.message ?? e}`);
  }

  // Check conditions
  const conditions = template.data?.conditions;
  let finalPrice = unitPrice;
  if (Array.isArray(conditions)) {
    for (const cond of conditions) {
      if (cond && typeof cond === 'object' && evaluateCondition(cond, variables)) {
        if (typeof cond.formula === 'string') {
          try {
            finalPrice = evaluateFormula(cond.formula, variables);
          } catch { /* keep current price */ }
        }
      }
    }
  }

  const total = finalPrice * quantity;

  return jsonRespond(msg, {
    unit_price: finalPrice,
    quantity,
    total,
    currency: fieldStr(product, 'currency') || 'USD',
    formula,
    variables_used: variables,
  });
}

// ---------------------------------------------------------------------------
// Purchases
// ---------------------------------------------------------------------------

async function handleCreatePurchase(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const body = parseBody<{
    items: Array<{
      product_id: string;
      quantity: number;
      variables?: Record<string, number>;
    }>;
    currency?: string;
  }>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');
  if (!body.items || body.items.length === 0) return errResult('invalid-argument', 'No items in purchase');

  const currency = body.currency ?? 'USD';
  const now = new Date().toISOString();
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';

  let totalAmount = 0;
  const lineItemsData: Array<{
    productId: string;
    productName: string;
    quantity: number;
    unitPrice: number;
    totalPrice: number;
    variables: Record<string, number>;
  }> = [];

  for (const item of body.items) {
    if (item.quantity <= 0) return errResult('invalid-argument', 'Quantity must be positive');

    const product = await dbGet(host, PRODUCTS, item.product_id);
    if (!product) return errResult('not-found', `Product ${item.product_id} not found`);

    const productName = fieldStr(product, 'name') || 'Unknown';
    const variables = item.variables ?? {};

    // Calculate price
    let unitPrice: number;
    const templateId = fieldStr(product, 'pricing_template_id');
    if (templateId) {
      const template = await dbGet(host, PRICING, templateId);
      if (template) {
        const formula = fieldStr(template, 'price_formula') || '0';
        try {
          unitPrice = evaluateFormula(formula, variables);
        } catch {
          unitPrice = asNumber(product.data?.base_price, 0);
        }
      } else {
        unitPrice = asNumber(product.data?.base_price, 0);
      }
    } else {
      unitPrice = asNumber(product.data?.base_price, 0);
    }

    const lineTotal = unitPrice * item.quantity;
    totalAmount += lineTotal;

    lineItemsData.push({
      productId: item.product_id,
      productName,
      quantity: item.quantity,
      unitPrice,
      totalPrice: lineTotal,
      variables,
    });
  }

  const totalCents = Math.round(totalAmount * 100);

  // Create purchase
  const purchase = await dbCreate(host, PURCHASES, {
    user_id: userId,
    status: 'pending',
    total_cents: totalCents,
    amount_cents: totalCents,
    currency,
    provider: 'manual',
    created_at: now,
    updated_at: now,
  });
  if (!purchase) return errResult('internal', 'Failed to create purchase');

  // Create line items
  for (const li of lineItemsData) {
    const liResult = await dbCreate(host, LINE_ITEMS, {
      purchase_id: purchase.id,
      product_id: li.productId,
      product_name: li.productName,
      quantity: li.quantity,
      unit_price: li.unitPrice,
      total_price: li.totalPrice,
      variables: li.variables,
      created_at: now,
    });
    if (!liResult) {
      // Roll back
      await callService(host, 'wafer-run/database', 'database.delete', {
        collection: PURCHASES, id: purchase.id,
      });
      return errResult('internal', 'Failed to create line item');
    }
  }

  return jsonRespond(msg, {
    id: purchase.id,
    status: 'pending',
    total_cents: totalCents,
    item_count: lineItemsData.length,
  });
}

async function handleListPurchasesUser(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  const page = parseInt(queryParam(msg, 'page') ?? '1', 10);
  const pageSize = parseInt(queryParam(msg, 'page_size') ?? '20', 10);

  return dbPaginatedList(host, msg, PURCHASES, page, pageSize,
    [{ field: 'user_id', operator: 'eq', value: userId }],
    [{ field: 'created_at', desc: true }]);
}

async function handleGetPurchase(msg: Message, host: RuntimeHost, path: string): Promise<BlockResult> {
  // Works for both /b/products/purchases/{id} and /admin/b/products/purchases/{id}
  const id = path.split('/').pop() ?? '';
  if (!id || id === 'purchases') return errResult('invalid-argument', 'Missing purchase ID');

  const purchase = await dbGet(host, PURCHASES, id);
  if (!purchase) return errResult('not-found', 'Purchase not found');

  // Verify access
  const userId = metaGet(msg.meta, 'auth.user_id') ?? '';
  const roles = metaGet(msg.meta, 'auth.user_roles') ?? '';
  const isAdmin = roles.split(',').some(r => r.trim() === 'admin');
  const purchaseUser = fieldStr(purchase, 'user_id');
  if (purchaseUser !== userId && !isAdmin) {
    return errResult('permission-denied', 'Access denied');
  }

  // Get line items
  const lineItemsResult = await dbList(host, LINE_ITEMS, {
    filters: [{ field: 'purchase_id', operator: 'eq', value: id }],
    sort: [],
    limit: 100,
    offset: 0,
  });
  const lineItems = lineItemsResult?.records ?? [];

  return jsonRespond(msg, { purchase, line_items: lineItems });
}

// ---------------------------------------------------------------------------
// Stripe checkout
// ---------------------------------------------------------------------------

async function handleCheckout(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const stripeKey = await getConfig(host, 'STRIPE_SECRET_KEY', '');
  if (!stripeKey) return errResult('internal', 'Stripe is not configured');

  const body = parseBody<{
    purchase_id: string;
    success_url?: string;
    cancel_url?: string;
  }>(msg);
  if (!body) return errResult('invalid-argument', 'Invalid body');

  const purchase = await dbGet(host, PURCHASES, body.purchase_id);
  if (!purchase) return errResult('not-found', 'Purchase not found');

  const totalCents = asNumber(purchase.data?.total_cents, 0);
  const currency = (fieldStr(purchase, 'currency') || 'usd').toLowerCase();

  const baseUrl = await getConfig(host, 'FRONTEND_URL', 'http://localhost:5173');
  const successUrl = body.success_url ?? `${baseUrl}/checkout/success?session_id={CHECKOUT_SESSION_ID}`;
  const cancelUrl = body.cancel_url ?? `${baseUrl}/checkout/cancel`;

  const stripeBody = [
    'payment_method_types[]=card',
    `line_items[0][price_data][currency]=${currency}`,
    `line_items[0][price_data][unit_amount]=${totalCents}`,
    `line_items[0][price_data][product_data][name]=Order ${body.purchase_id}`,
    'line_items[0][quantity]=1',
    'mode=payment',
    `success_url=${urlEncode(successUrl)}`,
    `cancel_url=${urlEncode(cancelUrl)}`,
    `metadata[purchase_id]=${body.purchase_id}`,
  ].join('&');

  const stripeApiUrl = await getConfig(host, 'STRIPE_API_URL', 'https://api.stripe.com');
  const endpoint = `${stripeApiUrl}/v1/checkout/sessions`;

  const resp = await callNetwork(host, 'POST', endpoint, {
    'Authorization': `Bearer ${stripeKey}`,
    'Content-Type': 'application/x-www-form-urlencoded',
  }, stripeBody);

  if (!resp) return errResult('internal', 'Stripe API error');
  if (resp.status_code >= 400) {
    return errResult('internal', `Stripe error (${resp.status_code}): ${resp.body_text}`);
  }

  let session: Record<string, any>;
  try {
    session = JSON.parse(resp.body_text);
  } catch {
    return errResult('internal', 'Failed to parse Stripe response');
  }

  const sessionId = session.id ?? '';
  const checkoutUrl = session.url ?? '';

  // Update purchase with Stripe session ID
  await dbUpdate(host, PURCHASES, body.purchase_id, {
    provider: 'stripe',
    provider_session_id: sessionId,
    updated_at: new Date().toISOString(),
  });

  return jsonRespond(msg, { session_id: sessionId, checkout_url: checkoutUrl });
}

// ---------------------------------------------------------------------------
// Stripe webhook
// ---------------------------------------------------------------------------

async function handleWebhook(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  // Verify signature
  const webhookSecret = await getConfig(host, 'STRIPE_WEBHOOK_SECRET', '');
  if (!webhookSecret) {
    return errResult('internal', 'STRIPE_WEBHOOK_SECRET not configured -- webhook processing disabled for security');
  }

  const sigHeader = metaGet(msg.meta, 'http.header.stripe-signature') ?? '';
  if (!sigHeader) return errResult('unauthenticated', 'Missing Stripe-Signature header');

  const isValid = await verifyStripeSignature(msg.data, sigHeader, webhookSecret);
  if (!isValid) return errResult('unauthenticated', 'Invalid webhook signature');

  const event = parseBody<Record<string, any>>(msg);
  if (!event) return errResult('invalid-argument', 'Invalid webhook body');

  const eventType = event.type ?? '';

  if (eventType === 'checkout.session.completed') {
    const session = event.data?.object;
    if (session) {
      const purchaseId = session.metadata?.purchase_id ?? '';
      if (purchaseId) {
        const paymentIntent = session.payment_intent ?? '';
        const now = new Date().toISOString();
        await dbUpdate(host, PURCHASES, purchaseId, {
          status: 'completed',
          provider_payment_intent_id: paymentIntent,
          approved_at: now,
          updated_at: now,
        });
      }
    }
  } else if (eventType === 'charge.refunded') {
    const charge = event.data?.object;
    if (charge) {
      const paymentIntent = charge.payment_intent ?? '';
      if (paymentIntent) {
        // Find purchase by provider_payment_intent_id
        const purchases = await dbListFiltered(host, PURCHASES, 'provider_payment_intent_id', paymentIntent);
        if (purchases.length > 0) {
          const now = new Date().toISOString();
          await dbUpdate(host, PURCHASES, purchases[0].id, {
            status: 'refunded',
            refunded_at: now,
            updated_at: now,
          });
        }
      }
    }
  }

  return jsonRespond(msg, { received: true });
}

// ---------------------------------------------------------------------------
// Stripe signature verification (HMAC-SHA256)
// ---------------------------------------------------------------------------

async function verifyStripeSignature(payload: Uint8Array, sigHeader: string, secret: string): Promise<boolean> {
  let timestamp = '';
  let expectedSig = '';

  for (const part of sigHeader.split(',')) {
    const trimmed = part.trim();
    if (trimmed.startsWith('t=')) timestamp = trimmed.substring(2);
    else if (trimmed.startsWith('v1=')) expectedSig = trimmed.substring(3);
  }

  if (!timestamp || !expectedSig) return false;

  // Replay protection: reject if older than 5 minutes
  const ts = parseInt(timestamp, 10);
  if (isNaN(ts)) return false;
  const now = Math.floor(Date.now() / 1000);
  if (Math.abs(now - ts) > 300) return false;

  // Compute HMAC-SHA256
  const payloadStr = new TextDecoder().decode(payload);
  const signedPayload = `${timestamp}.${payloadStr}`;

  const key = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign'],
  );
  const sig = await crypto.subtle.sign('HMAC', key, new TextEncoder().encode(signedPayload));
  const computedHex = arrayBufferToHex(sig);

  // Constant-time comparison
  return constantTimeEqual(computedHex, expectedSig);
}

function arrayBufferToHex(buf: ArrayBuffer): string {
  const arr = new Uint8Array(buf);
  let hex = '';
  for (let i = 0; i < arr.length; i++) {
    hex += arr[i].toString(16).padStart(2, '0');
  }
  return hex;
}

function constantTimeEqual(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  let result = 0;
  for (let i = 0; i < a.length; i++) {
    result |= a.charCodeAt(i) ^ b.charCodeAt(i);
  }
  return result === 0;
}

// ---------------------------------------------------------------------------
// Formula evaluator (ported from pricing.rs)
// ---------------------------------------------------------------------------

type Token =
  | { type: 'number'; value: number }
  | { type: 'ident'; value: string }
  | { type: 'plus' }
  | { type: 'minus' }
  | { type: 'star' }
  | { type: 'slash' }
  | { type: 'lparen' }
  | { type: 'rparen' };

function tokenize(input: string): Token[] {
  const tokens: Token[] = [];
  const chars = [...input];
  let i = 0;

  while (i < chars.length) {
    const c = chars[i];
    if (c === ' ' || c === '\t' || c === '\n') { i++; continue; }
    if (c === '+') { tokens.push({ type: 'plus' }); i++; continue; }
    if (c === '-') { tokens.push({ type: 'minus' }); i++; continue; }
    if (c === '*') { tokens.push({ type: 'star' }); i++; continue; }
    if (c === '/') { tokens.push({ type: 'slash' }); i++; continue; }
    if (c === '(') { tokens.push({ type: 'lparen' }); i++; continue; }
    if (c === ')') { tokens.push({ type: 'rparen' }); i++; continue; }
    if (/[0-9.]/.test(c)) {
      const start = i;
      while (i < chars.length && /[0-9.]/.test(chars[i])) i++;
      const numStr = chars.slice(start, i).join('');
      const num = parseFloat(numStr);
      if (isNaN(num)) throw new Error(`Invalid number: ${numStr}`);
      tokens.push({ type: 'number', value: num });
      continue;
    }
    if (/[a-zA-Z_]/.test(c)) {
      const start = i;
      while (i < chars.length && /[a-zA-Z0-9_]/.test(chars[i])) i++;
      tokens.push({ type: 'ident', value: chars.slice(start, i).join('') });
      continue;
    }
    throw new Error(`Unexpected character: ${c}`);
  }

  return tokens;
}

function parseExpression(tokens: Token[], pos: { v: number }, vars: Record<string, number>): number {
  let left = parseTerm(tokens, pos, vars);
  while (pos.v < tokens.length) {
    const t = tokens[pos.v];
    if (t.type === 'plus') { pos.v++; left += parseTerm(tokens, pos, vars); }
    else if (t.type === 'minus') { pos.v++; left -= parseTerm(tokens, pos, vars); }
    else break;
  }
  return left;
}

function parseTerm(tokens: Token[], pos: { v: number }, vars: Record<string, number>): number {
  let left = parseFactor(tokens, pos, vars);
  while (pos.v < tokens.length) {
    const t = tokens[pos.v];
    if (t.type === 'star') { pos.v++; left *= parseFactor(tokens, pos, vars); }
    else if (t.type === 'slash') {
      pos.v++;
      const right = parseFactor(tokens, pos, vars);
      if (right === 0) throw new Error('Division by zero');
      left /= right;
    }
    else break;
  }
  return left;
}

function parseFactor(tokens: Token[], pos: { v: number }, vars: Record<string, number>): number {
  if (pos.v >= tokens.length) throw new Error('Unexpected end of expression');
  const t = tokens[pos.v];
  if (t.type === 'number') { pos.v++; return t.value; }
  if (t.type === 'ident') {
    pos.v++;
    if (!(t.value in vars)) throw new Error(`Unknown variable: ${t.value}`);
    return vars[t.value];
  }
  if (t.type === 'lparen') {
    pos.v++;
    const val = parseExpression(tokens, pos, vars);
    if (pos.v < tokens.length && tokens[pos.v].type === 'rparen') {
      pos.v++;
    } else {
      throw new Error('Expected closing parenthesis');
    }
    return val;
  }
  if (t.type === 'minus') {
    pos.v++;
    return -parseFactor(tokens, pos, vars);
  }
  throw new Error(`Unexpected token at position ${pos.v}`);
}

function evaluateFormula(formula: string, variables: Record<string, number>): number {
  const tokens = tokenize(formula);
  const pos = { v: 0 };
  return parseExpression(tokens, pos, variables);
}

function evaluateCondition(cond: Record<string, any>, variables: Record<string, number>): boolean {
  const field = cond.field ?? '';
  const operator = cond.operator ?? '';
  const value = typeof cond.value === 'number' ? cond.value : 0;
  const fieldValue = variables[field] ?? 0;

  switch (operator) {
    case '>': case 'gt': return fieldValue > value;
    case '>=': case 'gte': return fieldValue >= value;
    case '<': case 'lt': return fieldValue < value;
    case '<=': case 'lte': return fieldValue <= value;
    case '==': case 'eq': return Math.abs(fieldValue - value) < Number.EPSILON;
    case '!=': case 'neq': return Math.abs(fieldValue - value) >= Number.EPSILON;
    default: return false;
  }
}

// ---------------------------------------------------------------------------
// URL encoding
// ---------------------------------------------------------------------------

function urlEncode(s: string): string {
  return encodeURIComponent(s);
}

// ---------------------------------------------------------------------------
// Service call helpers
// ---------------------------------------------------------------------------

interface Filter {
  field: string;
  operator: string;
  value: unknown;
}

interface SortField {
  field: string;
  desc: boolean;
}

interface ListOpts {
  filters: Filter[];
  sort: SortField[];
  limit: number;
  offset: number;
}

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

async function dbList(host: RuntimeHost, collection: string, opts: ListOpts): Promise<any> {
  return callService(host, 'wafer-run/database', 'database.list', {
    collection,
    filters: opts.filters,
    sort: opts.sort,
    limit: opts.limit,
    offset: opts.offset,
  });
}

async function dbListFiltered(host: RuntimeHost, collection: string, field: string, value: string): Promise<{ id: string; data: Record<string, any> }[]> {
  const result = await callService(host, 'wafer-run/database', 'database.list', {
    collection, filters: [{ field, operator: 'eq', value }], sort: [], limit: 100, offset: 0,
  });
  return result?.records ?? [];
}

async function dbPaginatedList(host: RuntimeHost, msg: Message, collection: string, page: number, pageSize: number, filters: Filter[], sort: SortField[]): Promise<BlockResult> {
  const result = await callService(host, 'wafer-run/database', 'database.paginated_list', {
    collection, page, page_size: pageSize, filters, sort,
  });
  if (!result) return errResult('internal', 'Database error');
  return jsonRespond(msg, result);
}

async function dbCount(host: RuntimeHost, collection: string, filters: Filter[]): Promise<number> {
  const result = await callService(host, 'wafer-run/database', 'database.count', {
    collection, filters,
  });
  return result?.count ?? 0;
}

async function dbSum(host: RuntimeHost, collection: string, field: string, filters: Filter[]): Promise<number> {
  const result = await callService(host, 'wafer-run/database', 'database.sum', {
    collection, field, filters,
  });
  return result?.sum ?? 0;
}

async function getConfig(host: RuntimeHost, key: string, defaultVal: string): Promise<string> {
  const result = await callService(host, 'wafer-run/config', 'config.get', { key });
  return result?.value ?? defaultVal;
}

async function callNetwork(host: RuntimeHost, method: string, url: string, headers: Record<string, string>, body: string): Promise<{ status_code: number; body_text: string } | null> {
  const result = await callService(host, 'wafer-run/network', 'network.request', {
    method, url, headers, body: Array.from(new TextEncoder().encode(body)),
  });
  if (!result) return null;
  const bodyText = typeof result.body === 'string' ? result.body : new TextDecoder().decode(new Uint8Array(result.body ?? []));
  return { status_code: result.status_code ?? 500, body_text: bodyText };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function queryParam(msg: Message, key: string): string | undefined {
  return metaGet(msg.meta, `req.query.${key}`);
}

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

function fieldStr(record: { data?: Record<string, any> }, field: string): string {
  const v = record.data?.[field];
  if (typeof v === 'string') return v;
  if (v != null) return String(v);
  return '';
}

function asString(v: unknown): string {
  if (typeof v === 'string') return v;
  if (typeof v === 'number') return String(v);
  return '';
}

function asNumber(v: unknown, fallback: number): number {
  if (typeof v === 'number') return v;
  if (typeof v === 'string') {
    const n = parseFloat(v);
    return isNaN(n) ? fallback : n;
  }
  return fallback;
}
