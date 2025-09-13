# Admin Routes Refactoring Plan

## Benefits of New Structure

### 1. **Clear Authorization Boundaries**
- All admin endpoints under `/admin/*` - single IAM rule
- Extension admin endpoints under `/admin/ext/{extension}/*`
- User endpoints remain in their logical locations

### 2. **Simplified IAM Policies**

#### Before (Complex):
```go
{"user", "/api/users", "*", "deny"}
{"user", "/api/users/*", "*", "deny"}
{"user", "/api/database/*", "*", "deny"}
{"user", "/api/logs/*", "*", "deny"}
{"user", "/api/system/*", "*", "deny"}
{"user", "/api/storage/admin/*", "*", "deny"}
// ... many more deny rules
```

#### After (Simple):
```go
{"user", "/api/admin/*", "*", "deny"}        // Block all admin endpoints
{"admin", "/api/admin/*", "*", "allow"}      // Allow admin role everything
{"admin_viewer", "/api/admin/*", "GET", "allow"} // Read-only admin
```

### 3. **Middleware Benefits**
- Single admin middleware check on `/admin` subrouter
- No need to check admin role in individual handlers
- Cleaner separation of concerns

## Endpoint Mapping

### Core Admin Endpoints

| Old Path | New Path | Description |
|----------|----------|-------------|
| `/users/*` | `/admin/users/*` | User management |
| `/database/*` | `/admin/database/*` | Database operations |
| `/logs/*` | `/admin/logs/*` | Log viewing |
| `/system/metrics` | `/admin/system/metrics` | System metrics |
| `/storage/admin/stats` | `/admin/storage/stats` | Storage admin stats |
| `/settings` (POST/PATCH) | `/admin/settings` | Settings management |
| `/iam/*` | `/admin/iam/*` | IAM management |

### Extension Admin Endpoints

| Old Path | New Path | Description |
|----------|----------|-------------|
| `/ext/products/variables` | `/admin/ext/products/variables` | Product variables config |
| `/ext/products/group-types` | `/admin/ext/products/group-types` | Group types config |
| `/ext/products/product-types` | `/admin/ext/products/product-types` | Product types config |
| `/ext/products/pricing-templates` | `/admin/ext/products/pricing-templates` | Pricing config |
| `/ext/webhooks/webhooks` (POST/DELETE) | `/admin/ext/webhooks/webhooks` | Webhook management |
| `/ext/hugo/*` | `/admin/ext/hugo/*` | All Hugo operations |
| `/ext/cloudstorage/providers` | `/admin/ext/cloudstorage/providers` | Provider management |
| `/ext/cloudstorage/activity` | `/admin/ext/cloudstorage/activity` | Activity logs |

### User-Accessible Endpoints (No Change)

- `/auth/*` - Authentication
- `/me` - Current user profile
- `/storage/buckets/*` - User storage
- `/dashboard/stats` - Dashboard
- `/settings` (GET only) - View settings
- `/ext/products/products` (GET) - View products
- `/ext/products/groups/*` - User groups
- `/ext/analytics/*` - Analytics tracking
- `/ext/cloudstorage/shares/*` - User shares

## Implementation Steps

1. **Update Router**
   - Replace `setupRoutesClean()` with `setupRoutesWithAdmin()`
   - Add admin role middleware to `/admin` subrouter

2. **Update IAM Policies**
   - Simplify to use `/admin/*` pattern
   - Remove individual deny rules

3. **Update UI/Frontend**
   - Update all admin API calls to use `/admin/` prefix
   - Update SDK to handle admin routes

4. **Update Documentation**
   - Document new admin API structure
   - Update API documentation

## Security Benefits

1. **Defense in Depth**: Multiple layers (auth + admin middleware + IAM)
2. **Fail Secure**: Default deny on `/admin/*` for non-admins
3. **Audit Trail**: Easier to log/monitor admin actions
4. **Principle of Least Privilege**: Clear separation of user vs admin operations