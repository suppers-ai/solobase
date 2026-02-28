---
title: "Cloud Storage"
description: "Advanced storage capabilities with usage tracking and limits"
weight: 30
---

# Cloud Storage Extension

The Cloud Storage extension enhances Solobase's storage capabilities with advanced features like usage tracking, bandwidth monitoring, capacity limits, and extensible sharing permissions. Perfect for applications that need sophisticated file management and storage control.

## Features

- **Usage Tracking** - Monitor storage consumption per user/organization
- **Capacity Limits** - Set storage quotas and enforce limits
- **Bandwidth Management** - Track and limit upload/download bandwidth
- **Extensible Sharing** - Granular permission controls for file sharing
- **Multi-Provider Support** - Works with S3, GCS, Azure, and local storage
- **File Versioning** - Keep track of file versions automatically
- **Analytics Dashboard** - Visualize storage usage and trends

## Installation

```bash
solobase extension install cloud-storage
```

## Configuration

Add to your `solobase.config.yml`:

```yaml
extensions:
  cloud-storage:
    enabled: true
    provider: "s3" # Options: s3, gcs, azure, local
    default_quota_gb: 10
    max_file_size_mb: 100
    allowed_extensions: [".jpg", ".png", ".pdf", ".doc", ".docx"]
    enable_versioning: true
    enable_public_links: true
    link_expiry_hours: 24
    
    # Provider specific config
    s3:
      bucket: "my-bucket"
      region: "us-east-1"
      access_key: "${S3_ACCESS_KEY}"
      secret_key: "${S3_SECRET_KEY}"
```

## API Endpoints

### Upload File

```http
POST /api/extensions/cloud-storage/upload
Content-Type: multipart/form-data

file: [binary data]
folder: /documents
permissions: private
```

Response:
```json
{
  "file_id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "document.pdf",
  "size": 1048576,
  "mime_type": "application/pdf",
  "url": "https://storage.example.com/files/550e8400-e29b-41d4-a716-446655440000",
  "created_at": "2024-01-15T10:30:00Z"
}
```

### Get Storage Usage

```http
GET /api/extensions/cloud-storage/usage
```

Response:
```json
{
  "used_bytes": 5368709120,
  "used_gb": 5.0,
  "quota_bytes": 10737418240,
  "quota_gb": 10.0,
  "percentage_used": 50,
  "file_count": 1234,
  "bandwidth": {
    "download_this_month_gb": 25.5,
    "upload_this_month_gb": 8.3,
    "limit_gb": 100
  }
}
```

### Share File

```http
POST /api/extensions/cloud-storage/share
Content-Type: application/json

{
  "file_id": "550e8400-e29b-41d4-a716-446655440000",
  "permissions": {
    "type": "link", // or "user", "group"
    "access": "view", // or "edit", "download"
    "expires_at": "2024-01-20T00:00:00Z",
    "password": "optional-password"
  }
}
```

## Storage Quotas

### User Quotas

```javascript
// Set user quota
await setUserQuota({
  user_id: "user-123",
  quota_gb: 50,
  bandwidth_limit_gb: 500
});

// Check quota before upload
const canUpload = await checkQuota({
  user_id: "user-123",
  file_size_bytes: 104857600 // 100MB
});
```

### Organization Quotas

```javascript
// Set organization-wide quota
await setOrgQuota({
  org_id: "org-456",
  total_quota_gb: 1000,
  per_user_quota_gb: 20
});
```

## File Operations

### File Versioning

```javascript
// Upload new version
const version = await uploadVersion({
  file_id: "original-file-id",
  file: newFileData,
  comment: "Updated formatting"
});

// List versions
const versions = await getVersions("file-id");

// Restore version
await restoreVersion({
  file_id: "file-id",
  version_id: "version-2"
});
```

### Batch Operations

```javascript
// Bulk upload
const files = await bulkUpload({
  files: [file1, file2, file3],
  folder: "/bulk-upload",
  process_async: true
});

// Bulk delete
await bulkDelete({
  file_ids: ["id1", "id2", "id3"]
});

// Bulk move
await bulkMove({
  file_ids: ["id1", "id2"],
  destination: "/new-folder"
});
```

## Sharing & Permissions

### Permission Levels

- **View** - Can view file only
- **Download** - Can view and download
- **Edit** - Can modify file
- **Delete** - Full control
- **Share** - Can share with others

### Sharing Examples

```javascript
// Share with specific users
await shareWithUsers({
  file_id: "file-id",
  users: [
    {email: "user1@example.com", permission: "view"},
    {email: "user2@example.com", permission: "edit"}
  ],
  message: "Please review this document"
});

// Create public link
const link = await createPublicLink({
  file_id: "file-id",
  expires_in_hours: 48,
  max_downloads: 10,
  password_protected: true
});

// Set folder permissions
await setFolderPermissions({
  folder_path: "/shared/documents",
  inherit: true,
  permissions: {
    "group:editors": "edit",
    "group:viewers": "view"
  }
});
```

## Analytics & Monitoring

### Usage Analytics

```javascript
// Get storage analytics
const analytics = await getAnalytics({
  period: "month",
  group_by: "file_type"
});

// Returns:
{
  "periods": [...],
  "data": {
    "images": {
      "count": 450,
      "size_gb": 2.3,
      "growth_percent": 15
    },
    "documents": {
      "count": 1200,
      "size_gb": 3.5,
      "growth_percent": 8
    }
  }
}
```

### Activity Logs

```javascript
// Get file activity
const activity = await getFileActivity({
  file_id: "file-id",
  limit: 50
});

// Returns:
[
  {
    "action": "download",
    "user": "john@example.com",
    "timestamp": "2024-01-15T14:30:00Z",
    "ip_address": "192.168.1.1"
  }
]
```

## Advanced Features

### Content Delivery

```yaml
extensions:
  cloud-storage:
    cdn:
      enabled: true
      provider: "cloudflare"
      cache_ttl: 3600
      optimize_images: true
```

### Encryption

```yaml
extensions:
  cloud-storage:
    encryption:
      at_rest: true
      algorithm: "AES-256"
      key_management: "kms" # or "local"
```

### Backup & Recovery

```javascript
// Schedule backups
await scheduleBackup({
  frequency: "daily",
  retention_days: 30,
  destination: "backup-bucket"
});

// Restore from backup
await restoreFromBackup({
  backup_id: "backup-123",
  restore_point: "2024-01-10T00:00:00Z"
});
```

## Storage Providers

### Amazon S3

```yaml
s3:
  bucket: "my-bucket"
  region: "us-east-1"
  storage_class: "STANDARD_IA"
  transfer_acceleration: true
```

### Google Cloud Storage

```yaml
gcs:
  bucket: "my-bucket"
  project_id: "my-project"
  location: "us-central1"
```

### Azure Blob Storage

```yaml
azure:
  container: "my-container"
  account_name: "mystorageaccount"
  tier: "Hot"
```

### Local Storage

```yaml
local:
  path: "/var/solobase/storage"
  max_size_gb: 100
```

## Best Practices

1. **Set Appropriate Quotas** - Balance user needs with available resources
2. **Enable Versioning** - Keep file history for important documents
3. **Monitor Usage** - Track storage trends to plan capacity
4. **Secure Sharing** - Use expiring links and passwords for sensitive files
5. **Regular Cleanup** - Implement retention policies for old files
6. **Optimize Storage** - Use appropriate storage classes for different file types

## Migration

### From Existing Storage

```javascript
// Migrate from existing storage system
const migration = await startMigration({
  source: {
    type: "filesystem",
    path: "/old/storage"
  },
  destination: {
    type: "s3",
    bucket: "new-bucket"
  },
  preserve_structure: true,
  preserve_permissions: true
});

// Monitor migration progress
const status = await getMigrationStatus(migration.id);
```

## Troubleshooting

### Common Issues

**Upload Failures**
- Check file size limits
- Verify allowed extensions
- Confirm user quota available

**Slow Performance**
- Enable CDN for better distribution
- Use multipart uploads for large files
- Optimize storage provider settings

**Permission Errors**
- Verify user authentication
- Check folder inheritance settings
- Review sharing permissions

## Support

For help with the Cloud Storage extension:

- [GitHub Issues](https://github.com/suppers-ai/solobase-extensions/issues)
- [Documentation](https://docs.solobase.ai/extensions/cloud-storage)
- [Discord Community](https://discord.gg/solobase)