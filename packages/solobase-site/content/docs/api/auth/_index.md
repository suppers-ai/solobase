---
title: "Authentication API"
description: "User authentication and authorization endpoints"
weight: 10
tags: ["api", "authentication", "jwt", "security"]
---

# Authentication API

The Authentication API provides endpoints for user login, registration, password management, and token validation. All authentication uses JWT tokens for stateless authentication.

## Base URL

All authentication endpoints are available under:

```
https://your-solobase-instance.com/api/auth
```

## Authentication Flow

1. **Login**: Exchange credentials for JWT token
2. **Use Token**: Include token in `Authorization` header for API requests
3. **Refresh**: Get new token before expiration
4. **Logout**: Invalidate token (optional)

## Endpoints

### POST /login

Authenticate a user and receive a JWT token.

#### Request

```http
POST /api/auth/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "password123"
}
```

#### Response

**Success (200 OK)**

```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refresh_token": "rt_1234567890abcdef",
    "expires_in": 3600,
    "user": {
      "id": 1,
      "email": "user@example.com",
      "name": "John Doe",
      "role": "user",
      "created_at": "2024-01-15T10:30:00Z",
      "last_login": "2024-01-15T14:30:00Z"
    }
  }
}
```

**Error (401 Unauthorized)**

```json
{
  "success": false,
  "error": {
    "code": "INVALID_CREDENTIALS",
    "message": "Invalid email or password"
  }
}
```

**Error (429 Too Many Requests)**

```json
{
  "success": false,
  "error": {
    "code": "RATE_LIMITED",
    "message": "Too many login attempts. Try again in 15 minutes.",
    "retry_after": 900
  }
}
```

#### Example

```bash
curl -X POST https://api.solobase.dev/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "secure123"
  }'
```

```javascript
// JavaScript/Node.js
const response = await fetch('/api/auth/login', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    email: 'user@example.com',
    password: 'password123'
  })
});

const data = await response.json();
if (data.success) {
  localStorage.setItem('token', data.data.token);
}
```

```python
# Python
import requests

response = requests.post('https://api.solobase.dev/api/auth/login', json={
    'email': 'user@example.com',
    'password': 'password123'
})

if response.status_code == 200:
    data = response.json()
    token = data['data']['token']
```

### POST /register

Register a new user account.

#### Request

```http
POST /api/auth/register
Content-Type: application/json

{
  "name": "John Doe",
  "email": "john@example.com",
  "password": "secure_password_123",
  "password_confirmation": "secure_password_123"
}
```

#### Response

**Success (201 Created)**

```json
{
  "success": true,
  "data": {
    "user": {
      "id": 2,
      "name": "John Doe",
      "email": "john@example.com",
      "role": "user",
      "created_at": "2024-01-15T15:30:00Z",
      "email_verified": false
    },
    "message": "Registration successful. Please check your email to verify your account."
  }
}
```

**Error (422 Unprocessable Entity)**

```json
{
  "success": false,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Validation failed",
    "details": {
      "email": ["Email already exists"],
      "password": ["Password must be at least 8 characters"]
    }
  }
}
```

#### Example

```bash
curl -X POST https://api.solobase.dev/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Jane Smith",
    "email": "jane@example.com",
    "password": "secure123",
    "password_confirmation": "secure123"
  }'
```

### POST /refresh

Refresh an expired or expiring JWT token.

#### Request

```http
POST /api/auth/refresh
Content-Type: application/json

{
  "refresh_token": "rt_1234567890abcdef"
}
```

#### Response

**Success (200 OK)**

```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_in": 3600
  }
}
```

### GET /me

Get current user information.

#### Request

```http
GET /api/auth/me
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

#### Response

**Success (200 OK)**

```json
{
  "success": true,
  "data": {
    "user": {
      "id": 1,
      "name": "John Doe",
      "email": "john@example.com",
      "role": "admin",
      "permissions": ["read", "write", "delete", "admin"],
      "created_at": "2024-01-01T00:00:00Z",
      "last_login": "2024-01-15T14:30:00Z",
      "email_verified": true
    }
  }
}
```

### PUT /me

Update current user profile.

#### Request

```http
PUT /api/auth/me
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json

{
  "name": "John Smith",
  "email": "johnsmith@example.com"
}
```

#### Response

**Success (200 OK)**

```json
{
  "success": true,
  "data": {
    "user": {
      "id": 1,
      "name": "John Smith",
      "email": "johnsmith@example.com",
      "role": "admin",
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-15T15:45:00Z"
    }
  }
}
```

### POST /change-password

Change user password.

#### Request

```http
POST /api/auth/change-password
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json

{
  "current_password": "old_password",
  "new_password": "new_secure_password",
  "new_password_confirmation": "new_secure_password"
}
```

#### Response

**Success (200 OK)**

```json
{
  "success": true,
  "data": {
    "message": "Password changed successfully"
  }
}
```

### POST /forgot-password

Request password reset email.

#### Request

```http
POST /api/auth/forgot-password
Content-Type: application/json

{
  "email": "user@example.com"
}
```

#### Response

**Success (200 OK)**

```json
{
  "success": true,
  "data": {
    "message": "Password reset email sent if account exists"
  }
}
```

### POST /reset-password

Reset password using token from email.

#### Request

```http
POST /api/auth/reset-password
Content-Type: application/json

{
  "token": "reset_token_from_email",
  "password": "new_password",
  "password_confirmation": "new_password"
}
```

### POST /logout

Logout and invalidate token (optional - tokens expire automatically).

#### Request

```http
POST /api/auth/logout
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

#### Response

**Success (200 OK)**

```json
{
  "success": true,
  "data": {
    "message": "Logged out successfully"
  }
}
```

## Using JWT Tokens

### Including Tokens in Requests

Include the JWT token in the `Authorization` header:

```http
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### Token Structure

JWT tokens contain three parts separated by dots:

```text
header.payload.signature
```

The payload contains:

```json
{
  "sub": "1",           // User ID
  "email": "user@example.com",
  "role": "admin",
  "permissions": ["read", "write", "admin"],
  "iat": 1642248600,    // Issued at
  "exp": 1642252200     // Expires at
}
```

### Token Expiration

- **Access tokens**: Expire in 1 hour by default
- **Refresh tokens**: Expire in 30 days by default
- Use refresh tokens to get new access tokens

## Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `INVALID_CREDENTIALS` | 401 | Wrong email or password |
| `TOKEN_EXPIRED` | 401 | JWT token has expired |
| `TOKEN_INVALID` | 401 | JWT token is malformed or invalid |
| `USER_NOT_FOUND` | 404 | User account doesn't exist |
| `EMAIL_NOT_VERIFIED` | 403 | Email address not verified |
| `ACCOUNT_LOCKED` | 423 | Account locked due to failed attempts |
| `RATE_LIMITED` | 429 | Too many requests |
| `VALIDATION_ERROR` | 422 | Request validation failed |

## Rate Limiting

Authentication endpoints are rate limited:

- **Login**: 5 attempts per 15 minutes per IP
- **Register**: 3 attempts per hour per IP
- **Password Reset**: 3 attempts per hour per email

Rate limit headers are included in responses:

```http
X-RateLimit-Limit: 5
X-RateLimit-Remaining: 3
X-RateLimit-Reset: 1642252200
```

## Security Best Practices

1. **Store tokens securely**: Use secure storage (not localStorage for sensitive apps)
2. **Handle token expiration**: Implement automatic token refresh
3. **Use HTTPS**: Always use HTTPS in production
4. **Validate on server**: Never trust client-side token validation
5. **Implement logout**: Clear tokens on logout
6. **Monitor failed attempts**: Watch for brute force attacks

## SDK Examples

### Go SDK

When using Solobase in your Go application, authentication is handled automatically:

```go
package main

import (
    "github.com/suppers-ai/solobase"
)

func main() {
    app := solobase.NewWithOptions(solobase.Options{
        DefaultAdminEmail: "admin@example.com",
        DefaultAdminPassword: "secure-password",
        JWTSecret: "your-secret-key",
    })

    // Add custom authentication hooks
    app.OnBeforeAPI("/api/auth/login", func(e *solobase.APIEvent) error {
        // Custom logic before login
        return e.Next()
    })

    app.OnAfterAPI("/api/auth/login", func(e *solobase.APIEvent) error {
        // Custom logic after successful login
        return e.Next()
    })

    app.Initialize()
    app.Start()
}
```

### JavaScript/TypeScript Client

```typescript
class SolobaseAuth {
  private baseUrl: string;
  private token: string | null = null;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
    this.token = localStorage.getItem('solobase_token');
  }

  async login(email: string, password: string): Promise<User> {
    const response = await fetch(`${this.baseUrl}/api/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email, password })
    });

    const data = await response.json();
    if (data.success) {
      this.token = data.data.token;
      localStorage.setItem('solobase_token', this.token);
      return data.data.user;
    }
    throw new Error(data.error.message);
  }

  async getCurrentUser(): Promise<User> {
    const response = await this.authenticatedRequest('/api/auth/me');
    return response.data.user;
  }

  private async authenticatedRequest(url: string, options: RequestInit = {}) {
    const response = await fetch(`${this.baseUrl}${url}`, {
      ...options,
      headers: {
        ...options.headers,
        'Authorization': `Bearer ${this.token}`
      }
    });
    return response.json();
  }
}
```

### Python

```python
import requests
from typing import Optional, Dict, Any

class SolobaseAuth:
    def __init__(self, base_url: str):
        self.base_url = base_url
        self.token: Optional[str] = None

    def login(self, email: str, password: str) -> Dict[str, Any]:
        response = requests.post(f"{self.base_url}/api/auth/login", json={
            "email": email,
            "password": password
        })
        
        data = response.json()
        if data["success"]:
            self.token = data["data"]["token"]
            return data["data"]["user"]
        
        raise Exception(data["error"]["message"])

    def get_current_user(self) -> Dict[str, Any]:
        response = self._authenticated_request("GET", "/api/auth/me")
        return response["data"]["user"]

    def _authenticated_request(self, method: str, url: str, **kwargs) -> Dict[str, Any]:
        headers = kwargs.get("headers", {})
        headers["Authorization"] = f"Bearer {self.token}"
        
        response = requests.request(
            method, 
            f"{self.base_url}{url}", 
            headers=headers, 
            **kwargs
        )
        return response.json()
```

## Next Steps

- [Database API](/docs/api/database/) - Database operations and queries
- [Storage API](/docs/api/storage/) - File upload and management
- [Admin API](/docs/api/admin/) - Administrative functions
- [User Management](/docs/users/) - Managing users in the dashboard