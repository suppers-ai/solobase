# Solobase API Documentation

## Base URL

```
http://localhost:8090/api
```

## Authentication

All authenticated endpoints require a Bearer token in the Authorization header:

```http
Authorization: Bearer <token>
```

Tokens are now stored in httpOnly cookies for enhanced security.

## Endpoints

### Authentication

#### Login
```http
POST /auth/login
```

**Request Body:**
```json
{
  "email": "user@example.com",
  "password": "password123"
}
```

**Response:**
```json
{
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    "roles": ["user", "admin"]
  },
  "message": "Login successful"
}
```

**Note:** Token is set as httpOnly cookie, not returned in response body.

#### Register
```http
POST /auth/register
```

**Request Body:**
```json
{
  "email": "user@example.com",
  "password": "password123",
  "username": "username"
}
```

#### OAuth Login
```http
GET /auth/oauth/:provider
```

**Providers:** `google`, `github`, `facebook`, `microsoft`

#### OAuth Callback
```http
GET /auth/oauth/:provider/callback
```

#### Logout
```http
POST /auth/logout
```

### User Management

#### Get Current User
```http
GET /auth/me
```

**Response:**
```json
{
  "id": "uuid",
  "email": "user@example.com",
  "username": "username",
  "roles": ["user"],
  "created_at": "2024-01-01T00:00:00Z"
}
```

#### Update Profile
```http
PATCH /users/profile
```

**Request Body:**
```json
{
  "username": "newusername",
  "first_name": "John",
  "last_name": "Doe"
}
```

#### List Users (Admin)
```http
GET /users?page=1&limit=20
```

#### Get User by ID (Admin)
```http
GET /users/:id
```

#### Update User (Admin)
```http
PATCH /users/:id
```

#### Delete User (Admin)
```http
DELETE /users/:id
```

### IAM (Identity & Access Management)

#### List Roles
```http
GET /iam/roles
```

#### Create Role
```http
POST /iam/roles
```

**Request Body:**
```json
{
  "name": "editor",
  "description": "Can edit content",
  "permissions": ["read", "write"]
}
```

#### Assign Role to User
```http
POST /iam/users/:userId/roles
```

**Request Body:**
```json
{
  "role_id": "role-uuid"
}
```

### Storage

#### List Buckets
```http
GET /storage/buckets
```

#### Create Bucket
```http
POST /storage/buckets
```

**Request Body:**
```json
{
  "name": "my-bucket",
  "public": false
}
```

#### List Objects
```http
GET /storage/buckets/:bucket/objects?parent_folder_id=<folder_id>
```

#### Upload File
```http
POST /storage/buckets/:bucket/upload
```

**Form Data:**
- `file`: File to upload
- `parent_folder_id`: Optional parent folder ID

#### Download File
```http
GET /storage/buckets/:bucket/objects/:id/download
```

#### Delete Object
```http
DELETE /storage/buckets/:bucket/objects/:id
```

#### Rename Object
```http
PATCH /storage/buckets/:bucket/objects/:id/rename
```

**Request Body:**
```json
{
  "newName": "new-filename.txt"
}
```

### Database

#### List Tables
```http
GET /database/tables
```

#### Get Table Data
```http
GET /database/tables/:table?page=1&limit=25
```

#### Execute Query
```http
POST /database/query
```

**Request Body:**
```json
{
  "query": "SELECT * FROM users LIMIT 10"
}
```

**Response:**
```json
{
  "rows": [...],
  "columns": [...],
  "rowCount": 10,
  "executionTime": 15
}
```

### Extensions

#### Products & Pricing

##### List Products
```http
GET /extensions/products
```

##### Create Product
```http
POST /extensions/products
```

**Request Body:**
```json
{
  "name": "Pro Plan",
  "description": "Advanced features",
  "price": 29.99,
  "currency": "USD",
  "billing_cycle": "monthly"
}
```

##### Create Subscription
```http
POST /extensions/products/:productId/subscribe
```

#### Analytics

##### Track Event
```http
POST /extensions/analytics/events
```

**Request Body:**
```json
{
  "event_name": "page_view",
  "properties": {
    "page": "/dashboard",
    "referrer": "google.com"
  }
}
```

##### Get Analytics
```http
GET /extensions/analytics?start_date=2024-01-01&end_date=2024-01-31
```

### Settings

#### Get App Settings
```http
GET /settings
```

#### Update Settings (Admin)
```http
PATCH /settings
```

**Request Body:**
```json
{
  "app_name": "My App",
  "notification": "System maintenance scheduled",
  "maintenance_mode": false
}
```

## Error Responses

All errors follow this format:

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human readable message",
    "details": {},
    "field": "email"
  },
  "status": 400
}
```

### Common Error Codes

- `AUTH_INVALID_CREDENTIALS`: Invalid login credentials
- `AUTH_SESSION_EXPIRED`: Session has expired
- `AUTH_INSUFFICIENT_PERMISSIONS`: Missing required permissions
- `VALIDATION_REQUIRED_FIELD`: Required field missing
- `VALIDATION_INVALID_EMAIL`: Invalid email format
- `NOT_FOUND`: Resource not found
- `CONFLICT`: Resource already exists
- `DATABASE_CONSTRAINT_ERROR`: Database constraint violation
- `STORAGE_QUOTA_EXCEEDED`: Storage limit reached

## Rate Limiting

API endpoints are rate-limited:

- Authentication: 5 requests/minute
- General API: 100 requests/minute
- File uploads: 10 requests/minute

Rate limit headers:
```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1609459200
```

## Pagination

Paginated endpoints support these query parameters:

- `page`: Page number (default: 1)
- `limit`: Items per page (default: 20, max: 100)
- `sort`: Sort field
- `order`: Sort order (asc/desc)

Paginated responses include metadata:

```json
{
  "data": [...],
  "page": 1,
  "per_page": 20,
  "total": 100,
  "total_pages": 5,
  "has_next": true,
  "has_prev": false
}
```

## WebSocket Events

Connect to WebSocket for real-time updates:

```javascript
const ws = new WebSocket('ws://localhost:8090/ws');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Event:', data.type, data.payload);
};
```

### Event Types

- `user.updated`: User profile updated
- `storage.uploaded`: File uploaded
- `storage.deleted`: File deleted
- `database.changed`: Database table modified
- `system.notification`: System notification