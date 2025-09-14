# Solobase Demo Deployment Guide

This guide explains how to deploy a secure public demo of Solobase using Fly.io with the integrated IAM system.

## Features

### IAM System with Casbin
- **Role-Based Access Control (RBAC)** with flexible policies
- **Dynamic quotas** per role (storage, bandwidth, upload size)
- **Rate limiting** based on user roles
- **Policy-based permissions** that can be changed without code modifications
- **Audit logging** for all permission checks

### Default Roles

1. **Admin** - Full system access
   - 100GB storage, 1TB bandwidth
   - 5GB max upload, 1000 req/min

2. **Manager** - User and content management
   - 50GB storage, 500GB bandwidth
   - 2GB max upload, 500 req/min

3. **Editor** - Content creation and editing
   - 10GB storage, 100GB bandwidth
   - 1GB max upload, 200 req/min

4. **Viewer** - Read-only access
   - 1GB storage, 10GB bandwidth
   - No uploads allowed, 100 req/min

5. **Admin Viewer** - Read-only administrative access (perfect for demos)
   - Read access to all endpoints
   - Cannot delete or modify resources
   - Cannot execute database queries
   - Cannot manage users or settings
   - 30 req/min rate limit
   - 30-minute session timeout

## Demo Deployment

### 1. Deploy to Fly.io

```bash
# Deploy the demo
fly deploy --config fly-demo.toml --dockerfile Dockerfile.fly

# The demo will be available with:
#
# Admin Account (Full Access):
# Email: admin@solobase.demo
# Password: [Auto-generated secure password - check container logs]
#
# To create a viewer account after deployment:
# 1. Login as admin
# 2. Create user: viewer@solobase.demo with password: demo123
# 3. Assign role: admin_viewer (read-only access)
```

### 2. Demo Security Configuration

The demo deployment features:
- **Admin Account**: Auto-generated secure password for administration
- **Viewer Account**: Can be created with `admin_viewer` role for safe demo access
- **Security Features**:
  - Read-only access for viewer accounts
  - No delete, modify, or execute operations for viewers
  - Rate limiting (30 requests per minute)
  - Session timeout (30 minutes)
  - Persistent SQLite database with data isolation
- **Auto-stop**: Machine stops when idle to save resources

### 3. Periodic Restart

The demo uses Fly.io's auto-stop feature:
- Machine stops after inactivity
- Automatically restarts on new requests
- Memory database ensures clean state

For scheduled restarts, use:
```bash
# Manual restart
fly apps restart solobase-demo

# Or use external scheduler (e.g., GitHub Actions)
# to call this command periodically
```

## IAM Management UI

Access the IAM management interface at `/admin/iam` to:
- Create and manage roles
- Define custom policies
- Assign roles to users
- Test permissions
- View audit logs

### Creating Custom Roles

```javascript
// Example: Create a "trial" role
{
  "name": "trial",
  "display_name": "Trial User",
  "description": "Limited trial access",
  "metadata": {
    "storage_quota": 268435456,      // 256MB
    "bandwidth_quota": 1073741824,   // 1GB
    "max_upload_size": 10485760,     // 10MB
    "max_requests_per_min": 60,
    "session_timeout": 3600,
    "disabled_features": ["webhooks"]
  }
}
```

### Policy Examples

```javascript
// Allow read access to specific resources
{
  "subject": "trial",
  "resource": "/api/storage/*",
  "action": "read",
  "effect": "allow"
}

// Deny delete operations
{
  "subject": "trial", 
  "resource": "/api/*/delete",
  "action": "*",
  "effect": "deny"
}
```

## Security Features

1. **No Hardcoded Demo Mode** - Everything is policy-driven
2. **Dynamic Quotas** - CloudStorage extension enforces IAM quotas
3. **Granular Permissions** - Control access to specific endpoints
4. **Audit Trail** - All permission checks are logged
5. **Rate Limiting** - Per-role request limits
6. **Session Management** - Configurable timeouts per role

## Monitoring

View metrics and usage:
- Storage usage: `/api/storage/quota`
- Request logs: `/api/iam/audit`
- System metrics: `/api/system/metrics`

## Customization

### Different Demo Scenarios

1. **Read-Only Demo**
   ```bash
   # Set environment variables
   DEFAULT_ADMIN_EMAIL=admin@demo.com
   VIEWER_EMAIL=viewer@demo.com
   VIEWER_PASSWORD=demo123
   # Viewer gets admin_viewer role for read-only access
   ```

2. **Trial with Uploads**
   ```javascript
   // Create custom "trial" role with specific limits
   ```

3. **Time-Limited Access**
   ```javascript
   // Set expiry on role assignment
   "expires_at": "2024-12-31T23:59:59Z"
   ```

## Production Deployment

For production, remove demo restrictions:

1. Use persistent database (PostgreSQL)
2. Set proper admin credentials
3. Configure appropriate roles and policies
4. Enable full features as needed
5. Set up proper backup and monitoring

## API Testing

Test permissions using the Policy Tester:

```bash
curl -X POST https://your-demo.fly.dev/api/iam/evaluate \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "user_id": "demo-user-id",
    "resource": "/api/storage/upload",
    "action": "POST",
    "context": {"size": 5242880}
  }'
```

## Troubleshooting

1. **Quota Exceeded**: Check `/api/storage/quota` for current usage
2. **Permission Denied**: Review policies in `/api/iam/policies`
3. **Rate Limited**: Check role metadata for limits
4. **Session Expired**: Configurable per role in metadata

## Benefits

- **Safe Public Demo**: No risk of abuse or data persistence
- **Flexible Permissions**: Easily adjust for different demo scenarios
- **Production-Ready IAM**: Same system works for real deployments
- **No Special Code**: Everything configured through policies
- **Cost-Effective**: Auto-stop saves resources when idle