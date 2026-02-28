---
title: "Database API"
description: "Database operations and query endpoints"
weight: 20
tags: ["api", "database", "sql", "crud"]
---

# Database API

The Database API provides endpoints for performing CRUD operations, executing queries, and managing database schema. All endpoints require authentication.

## Base URL

```text
https://your-solobase-instance.com/api/database
```

## Authentication

Include your JWT token in the Authorization header:

```http
Authorization: Bearer your-jwt-token-here
```

## Tables

### GET /tables

List all available database tables.

#### Response

```json
{
  "success": true,
  "data": {
    "tables": [
      {
        "name": "users",
        "columns": 8,
        "rows": 1247,
        "size": "2.1MB",
        "created_at": "2024-01-01T00:00:00Z"
      },
      {
        "name": "products",
        "columns": 12,
        "rows": 5892,
        "size": "8.7MB",
        "created_at": "2024-01-15T10:30:00Z"
      }
    ]
  }
}
```

### GET /tables/{table}

Get table schema and metadata.

#### Response

```json
{
  "success": true,
  "data": {
    "table": {
      "name": "users",
      "columns": [
        {
          "name": "id",
          "type": "INTEGER",
          "nullable": false,
          "primary_key": true,
          "auto_increment": true
        },
        {
          "name": "email",
          "type": "VARCHAR(255)",
          "nullable": false,
          "unique": true
        },
        {
          "name": "name",
          "type": "VARCHAR(255)",
          "nullable": false
        },
        {
          "name": "created_at",
          "type": "TIMESTAMP",
          "nullable": false,
          "default": "CURRENT_TIMESTAMP"
        }
      ],
      "indexes": [
        {
          "name": "idx_users_email",
          "columns": ["email"],
          "unique": true
        }
      ],
      "row_count": 1247,
      "size": "2.1MB"
    }
  }
}
```

## Records

### GET /tables/{table}/records

Get records from a table with pagination and filtering.

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `page` | integer | Page number (default: 1) |
| `limit` | integer | Records per page (default: 50, max: 1000) |
| `sort` | string | Sort column |
| `order` | string | Sort order: `asc` or `desc` |
| `filter` | string | Filter expression |
| `search` | string | Search term |

#### Examples

```bash
# Get first page of users
curl -H "Authorization: Bearer $TOKEN" \
  "https://api.solobase.dev/api/database/tables/users/records"

# Get users sorted by name
curl -H "Authorization: Bearer $TOKEN" \
  "https://api.solobase.dev/api/database/tables/users/records?sort=name&order=asc"

# Search for users
curl -H "Authorization: Bearer $TOKEN" \
  "https://api.solobase.dev/api/database/tables/users/records?search=john"

# Filter users by role
curl -H "Authorization: Bearer $TOKEN" \
  "https://api.solobase.dev/api/database/tables/users/records?filter=role=admin"
```

#### Response

```json
{
  "success": true,
  "data": {
    "records": [
      {
        "id": 1,
        "email": "admin@example.com",
        "name": "Admin User",
        "role": "admin",
        "created_at": "2024-01-01T00:00:00Z"
      },
      {
        "id": 2,
        "email": "john@example.com",
        "name": "John Doe",
        "role": "user",
        "created_at": "2024-01-02T10:30:00Z"
      }
    ],
    "pagination": {
      "page": 1,
      "limit": 50,
      "total": 1247,
      "pages": 25
    }
  }
}
```

### GET /tables/{table}/records/{id}

Get a specific record by ID.

#### Response

```json
{
  "success": true,
  "data": {
    "record": {
      "id": 1,
      "email": "admin@example.com",
      "name": "Admin User",
      "role": "admin",
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-15T14:30:00Z"
    }
  }
}
```

### POST /tables/{table}/records

Create a new record.

#### Request

```json
{
  "email": "newuser@example.com",
  "name": "New User",
  "role": "user"
}
```

#### Response

```json
{
  "success": true,
  "data": {
    "record": {
      "id": 1248,
      "email": "newuser@example.com",
      "name": "New User",
      "role": "user",
      "created_at": "2024-01-15T15:45:00Z"
    }
  }
}
```

### PUT /tables/{table}/records/{id}

Update an existing record.

#### Request

```json
{
  "name": "Updated Name",
  "role": "admin"
}
```

#### Response

```json
{
  "success": true,
  "data": {
    "record": {
      "id": 1248,
      "email": "newuser@example.com",
      "name": "Updated Name",
      "role": "admin",
      "created_at": "2024-01-15T15:45:00Z",
      "updated_at": "2024-01-15T16:00:00Z"
    }
  }
}
```

### DELETE /tables/{table}/records/{id}

Delete a record.

#### Response

```json
{
  "success": true,
  "data": {
    "message": "Record deleted successfully"
  }
}
```

## Custom Queries

### POST /query

Execute a custom SQL query.

#### Request

```json
{
  "query": "SELECT COUNT(*) as total_users FROM users WHERE role = ?",
  "params": ["admin"]
}
```

#### Response

```json
{
  "success": true,
  "data": {
    "results": [
      {
        "total_users": 15
      }
    ],
    "columns": ["total_users"],
    "rows_affected": 0,
    "execution_time": "2.3ms"
  }
}
```

### POST /query/explain

Get query execution plan.

#### Request

```json
{
  "query": "SELECT * FROM users WHERE email = ?",
  "params": ["admin@example.com"]
}
```

#### Response

```json
{
  "success": true,
  "data": {
    "plan": [
      {
        "id": 0,
        "select_type": "SIMPLE",
        "table": "users",
        "type": "const",
        "possible_keys": "idx_users_email",
        "key": "idx_users_email",
        "rows": 1,
        "extra": "Using index"
      }
    ],
    "estimated_cost": 1.0,
    "estimated_rows": 1
  }
}
```

## Bulk Operations

### POST /tables/{table}/records/bulk

Create multiple records at once.

#### Request

```json
{
  "records": [
    {
      "email": "user1@example.com",
      "name": "User One",
      "role": "user"
    },
    {
      "email": "user2@example.com",
      "name": "User Two",
      "role": "user"
    }
  ]
}
```

#### Response

```json
{
  "success": true,
  "data": {
    "created": 2,
    "records": [
      {
        "id": 1249,
        "email": "user1@example.com",
        "name": "User One",
        "role": "user",
        "created_at": "2024-01-15T16:15:00Z"
      },
      {
        "id": 1250,
        "email": "user2@example.com",
        "name": "User Two",
        "role": "user",
        "created_at": "2024-01-15T16:15:00Z"
      }
    ]
  }
}
```

### PUT /tables/{table}/records/bulk

Update multiple records.

#### Request

```json
{
  "updates": [
    {
      "id": 1249,
      "data": {
        "role": "editor"
      }
    },
    {
      "id": 1250,
      "data": {
        "role": "editor"
      }
    }
  ]
}
```

### DELETE /tables/{table}/records/bulk

Delete multiple records.

#### Request

```json
{
  "ids": [1249, 1250]
}
```

## Data Export/Import

### GET /tables/{table}/export

Export table data in various formats.

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `format` | string | Export format: `csv`, `json`, `xlsx` |
| `filter` | string | Filter expression |
| `columns` | string | Comma-separated column names |

#### Example

```bash
# Export users as CSV
curl -H "Authorization: Bearer $TOKEN" \
  "https://api.solobase.dev/api/database/tables/users/export?format=csv" \
  -o users.csv

# Export filtered data
curl -H "Authorization: Bearer $TOKEN" \
  "https://api.solobase.dev/api/database/tables/users/export?format=json&filter=role=admin" \
  -o admin_users.json
```

### POST /tables/{table}/import

Import data from file.

#### Request (multipart/form-data)

```bash
curl -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@users.csv" \
  -F "format=csv" \
  -F "update_existing=true" \
  "https://api.solobase.dev/api/database/tables/users/import"
```

#### Response

```json
{
  "success": true,
  "data": {
    "imported": 150,
    "updated": 25,
    "errors": 2,
    "error_details": [
      {
        "row": 15,
        "error": "Duplicate email address"
      },
      {
        "row": 32,
        "error": "Invalid email format"
      }
    ]
  }
}
```

## Error Handling

### Common Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `TABLE_NOT_FOUND` | 404 | Table doesn't exist |
| `RECORD_NOT_FOUND` | 404 | Record doesn't exist |
| `INVALID_QUERY` | 400 | SQL syntax error |
| `PERMISSION_DENIED` | 403 | Insufficient permissions |
| `VALIDATION_ERROR` | 422 | Data validation failed |
| `CONSTRAINT_VIOLATION` | 409 | Database constraint violated |

### Error Response Format

```json
{
  "success": false,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Validation failed",
    "details": {
      "email": ["Email already exists"],
      "name": ["Name is required"]
    }
  }
}
```

## Rate Limiting

Database API endpoints are rate limited:
- **Read operations**: 1000 requests per minute
- **Write operations**: 100 requests per minute
- **Query operations**: 50 requests per minute

## Best Practices

1. **Use pagination** for large datasets
2. **Filter data** at the API level rather than client-side
3. **Use bulk operations** for multiple records
4. **Cache frequently accessed data**
5. **Use prepared statements** for custom queries
6. **Monitor query performance** with explain plans
7. **Implement proper error handling**
8. **Use transactions** for related operations

## SDK Examples

### JavaScript

```javascript
class SolobaseDB {
  constructor(baseUrl, token) {
    this.baseUrl = baseUrl;
    this.token = token;
  }

  async getRecords(table, options = {}) {
    const params = new URLSearchParams(options);
    const response = await fetch(
      `${this.baseUrl}/api/database/tables/${table}/records?${params}`,
      {
        headers: {
          'Authorization': `Bearer ${this.token}`
        }
      }
    );
    return response.json();
  }

  async createRecord(table, data) {
    const response = await fetch(
      `${this.baseUrl}/api/database/tables/${table}/records`,
      {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${this.token}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(data)
      }
    );
    return response.json();
  }
}
```

### Python

```python
import requests

class SolobaseDB:
    def __init__(self, base_url, token):
        self.base_url = base_url
        self.token = token
        self.headers = {'Authorization': f'Bearer {token}'}

    def get_records(self, table, **params):
        response = requests.get(
            f"{self.base_url}/api/database/tables/{table}/records",
            headers=self.headers,
            params=params
        )
        return response.json()

    def create_record(self, table, data):
        response = requests.post(
            f"{self.base_url}/api/database/tables/{table}/records",
            headers={**self.headers, 'Content-Type': 'application/json'},
            json=data
        )
        return response.json()
```