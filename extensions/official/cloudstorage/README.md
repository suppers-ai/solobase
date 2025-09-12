# CloudStorage Extension

The CloudStorage extension provides comprehensive cloud storage capabilities for Solobase, using the storage provider configured in the main application.

## Features

### Core Features
- **Automatic Provider Detection**: Uses the storage provider configured in the main application (local or S3)
- **Bucket Management**: Create, list, and delete storage buckets
- **File Operations**: Upload, download, list, and delete files
- **Folder Support**: Organize files in hierarchical folder structures
- **Web Dashboard**: User-friendly interface for managing storage
- **REST API**: Complete API for programmatic access

### Advanced Features
- **File Sharing**: Create shareable links with expiration, password protection, and access limits
- **Storage Quotas**: Per-user storage limits with file size and count restrictions
- **Access Logging**: Track all storage operations with detailed audit logs
- **File Versioning**: Keep history of file changes with version restore capability
- **Tagging System**: Add metadata tags to objects for organization and search
- **Access Policies**: Define custom access rules for buckets
- **Webhooks**: Trigger events on storage operations

## Configuration

The extension uses the storage configuration from the main application. Extension-specific settings:

```go
type CloudStorageConfig struct {
    DefaultBucket      string // Default bucket for uploads
    MaxFileSize        int64  // Max file size in bytes (default: 100MB)
    MaxStoragePerUser  int64  // Max storage per user in bytes (default: 1GB)
    AllowPublicBuckets bool   // Allow creation of public buckets
    EnableVersioning   bool   // Enable file versioning
}
```

The actual storage provider (local or S3) is configured in the main application's configuration, not in the extension.

## API Endpoints

### Core Operations

#### Buckets
- `GET /ext/cloudstorage/api/buckets` - List all buckets
- `POST /ext/cloudstorage/api/buckets` - Create a new bucket
- `GET /ext/cloudstorage/api/buckets/{bucket}` - Get bucket details
- `DELETE /ext/cloudstorage/api/buckets/{bucket}` - Delete a bucket

#### Objects
- `GET /ext/cloudstorage/api/objects?bucket={name}` - List objects in a bucket
- `GET /ext/cloudstorage/api/objects/{id}` - Get object metadata
- `DELETE /ext/cloudstorage/api/objects/{id}` - Delete an object
- `POST /ext/cloudstorage/api/upload` - Upload a file
- `GET /ext/cloudstorage/api/download/{id}` - Download a file

#### Statistics
- `GET /ext/cloudstorage/api/stats` - Get storage statistics

### Advanced Operations

#### Sharing
- `GET /ext/cloudstorage/api/shares` - List user's shares
- `GET /ext/cloudstorage/api/shares/{token}` - Access shared link
- `DELETE /ext/cloudstorage/api/shares/{token}` - Delete share
- `POST /ext/cloudstorage/api/share/create` - Create shareable link

#### Quotas
- `GET /ext/cloudstorage/api/quotas` - List all quotas (admin)
- `POST /ext/cloudstorage/api/quotas` - Create quota
- `GET /ext/cloudstorage/api/quotas/{userId}` - Get user quota
- `PUT /ext/cloudstorage/api/quotas/{userId}` - Update user quota

#### Access Logs
- `GET /ext/cloudstorage/api/logs` - Get access logs with filters

#### Versioning
- `GET /ext/cloudstorage/api/versions/{id}` - Get file versions
- `POST /ext/cloudstorage/api/versions/{id}` - Create new version
- `POST /ext/cloudstorage/api/versions/restore` - Restore to version

#### Tags
- `GET /ext/cloudstorage/api/tags/{id}` - Get object tags
- `POST /ext/cloudstorage/api/tags/{id}` - Add tags
- `DELETE /ext/cloudstorage/api/tags/{id}?key={key}` - Remove tag
- `POST /ext/cloudstorage/api/tags/search` - Search by tags

## Permissions

The extension defines three permission levels:

- `cloudstorage.admin` - Full storage administration
- `cloudstorage.upload` - Upload files to storage
- `cloudstorage.download` - Download files from storage

## Usage Example

### Extension Configuration

```go
// The extension only needs to configure its own settings
// Storage provider is configured in the main application
config := &CloudStorageConfig{
    DefaultBucket:      "uploads",
    MaxFileSize:        100 * 1024 * 1024,  // 100MB
    MaxStoragePerUser:  10 * 1024 * 1024 * 1024, // 10GB
    AllowPublicBuckets: true,
    EnableVersioning:   false,
}

extension := cloudstorage.NewCloudStorageExtension(config)
```

### Main Application Storage Configuration

The storage provider is configured in the main application, not in the extension:

```go
// In main application configuration
storageConfig := storage.Config{
    Provider: "s3",  // or "local"
    // S3 settings (if using S3)
    S3Endpoint: "s3.amazonaws.com",
    S3AccessKeyID: "your-key",
    S3SecretAccessKey: "your-secret",
    S3Region: "us-east-1",
    // Local settings (if using local)
    BasePath: "./storage",
}
```

## Database Integration

The extension uses both base storage tables and its own extension tables:

### Base Tables (from packages/storage)
- `storage_buckets` - Storage bucket definitions
- `storage_objects` - File and folder objects

### Extension Tables (CloudStorage specific)
- `ext_cloudstorage_shares` - Shareable links and permissions
- `ext_cloudstorage_access_logs` - Access audit logs
- `ext_cloudstorage_quotas` - User storage quotas
- `ext_cloudstorage_versions` - File version history
- `ext_cloudstorage_tags` - Object metadata tags
- `ext_cloudstorage_policies` - Bucket access policies
- `ext_cloudstorage_webhooks` - Event webhooks

All tables are auto-migrated using GORM when the extension initializes.

## Dashboard

Access the CloudStorage dashboard at `/ext/cloudstorage` when the extension is enabled. The dashboard provides:

- Storage statistics overview
- Bucket management interface
- File browser with folder navigation
- Drag-and-drop file upload
- File download capabilities

## Development

The extension follows Solobase's extension architecture:

1. Implements the `Extension` interface
2. Uses GORM for database operations
3. Leverages the `packages/storage` module for storage operations
4. Provides a web dashboard and REST API
5. Integrates with Solobase's permission system

## License

Part of the Solobase platform - see main repository for license details.