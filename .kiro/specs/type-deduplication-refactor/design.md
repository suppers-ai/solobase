# Design Document

## Overview

This design establishes a unified type system that eliminates duplication across packages by creating a single source of truth based on database schema types. The approach uses the database as the canonical definition since it represents the actual stored data structure, with utility functions to transform types for different contexts (API responses, auth flows, etc.).

## Architecture

### Type Hierarchy

```
Database Schema (Source of Truth)
├── Core Types (packages/shared/types/database.ts)
├── API Types (derived from database types)
├── Auth Types (derived from database types)
└── Package-specific Type Aliases (deprecated → canonical)
```

### Package Structure

- **packages/shared/types/**: Canonical type definitions
  - `database.ts` - Database schema types (source of truth)
  - `api.ts` - API-specific derived types and utilities
  - `auth.ts` - Auth-specific derived types and utilities
- **packages/shared/utils/**: Type transformation utilities
  - `type-mappers.ts` - Functions to convert between type representations
- **Migration files**: Temporary compatibility layers during transition

## Components and Interfaces

### 1. Database Types (Source of Truth)

The `packages/shared/types/database.ts` file contains the canonical definitions:

```typescript
// Core database table types
export interface UsersTable extends BaseTable {
  id: string;
  email: string;
  first_name?: string;
  middle_names?: string;
  last_name?: string;
  display_name?: string;
  avatar_url?: string;
  role: "user" | "admin" | "moderator";
  created_at: string;
  updated_at: string;
}

// Helper types for operations
export type User = UsersTable;
export type UserInsert = TablesInsert<"users">;
export type UserUpdate = TablesUpdate<"users">;
```

### 2. API Response Types

Derived types for API responses in `packages/shared/types/api.ts`:

```typescript
import type { UsersTable } from './database.ts';

// API response wrapper
export interface ApiResponse<T = any> {
  data?: T;
  error?: string;
  success: boolean;
  status: number;
}

// User API response (derived from database type)
export type UserResponse = Pick<UsersTable, 'id' | 'email' | 'display_name' | 'avatar_url' | 'created_at'>;

// Extended user response with computed fields
export interface UserResponseExtended extends UserResponse {
  full_name: string;
  initials: string;
}
```

### 3. Auth Types

Auth-specific types in `packages/shared/types/auth.ts`:

```typescript
import type { UsersTable } from './database.ts';
import type { Session, User as SupabaseUser } from "@supabase/supabase-js";

// Auth user (derived from database type)
export type AuthUser = Pick<UsersTable, 'id' | 'email' | 'first_name' | 'last_name' | 'display_name' | 'avatar_url'>;

// Auth session with our user type
export interface AuthSession {
  user: AuthUser;
  session: Session;
  supabaseUser: SupabaseUser;
}

// Auth state
export interface AuthState {
  user: AuthUser | null;
  session: Session | null;
  loading: boolean;
}
```

### 4. Type Mapping Utilities

Utility functions in `packages/shared/utils/type-mappers.ts`:

```typescript
import type { UsersTable, User } from '../types/database.ts';
import type { UserResponse, UserResponseExtended } from '../types/api.ts';
import type { AuthUser } from '../types/auth.ts';
import type { User as SupabaseUser } from "@supabase/supabase-js";

export class TypeMappers {
  /**
   * Convert database user to API response format
   */
  static userToApiResponse(user: User): UserResponse {
    return {
      id: user.id,
      email: user.email,
      display_name: user.display_name,
      avatar_url: user.avatar_url,
      created_at: user.created_at,
    };
  }

  /**
   * Convert database user to extended API response
   */
  static userToExtendedApiResponse(user: User): UserResponseExtended {
    const baseResponse = this.userToApiResponse(user);
    return {
      ...baseResponse,
      full_name: this.getFullName(user),
      initials: this.getInitials(user),
    };
  }

  /**
   * Convert database user to auth user format
   */
  static userToAuthUser(user: User): AuthUser {
    return {
      id: user.id,
      email: user.email,
      first_name: user.first_name,
      last_name: user.last_name,
      display_name: user.display_name,
      avatar_url: user.avatar_url,
    };
  }

  /**
   * Convert Supabase user to our database user format
   */
  static supabaseUserToUser(supabaseUser: SupabaseUser, dbUser?: Partial<User>): User {
    return {
      id: supabaseUser.id,
      email: supabaseUser.email || '',
      first_name: dbUser?.first_name || supabaseUser.user_metadata?.first_name,
      middle_names: dbUser?.middle_names,
      last_name: dbUser?.last_name || supabaseUser.user_metadata?.last_name,
      display_name: dbUser?.display_name || supabaseUser.user_metadata?.display_name || supabaseUser.user_metadata?.full_name,
      avatar_url: dbUser?.avatar_url || supabaseUser.user_metadata?.avatar_url,
      role: dbUser?.role || 'user',
      created_at: dbUser?.created_at || supabaseUser.created_at,
      updated_at: dbUser?.updated_at || supabaseUser.updated_at || supabaseUser.created_at,
    };
  }

  /**
   * Helper: Get full name from user
   */
  private static getFullName(user: User): string {
    if (user.display_name) return user.display_name;
    const parts = [user.first_name, user.middle_names, user.last_name].filter(Boolean);
    return parts.length > 0 ? parts.join(' ') : user.email;
  }

  /**
   * Helper: Get initials from user
   */
  private static getInitials(user: User): string {
    const name = this.getFullName(user);
    return name
      .split(' ')
      .map(word => word.charAt(0).toUpperCase())
      .slice(0, 2)
      .join('');
  }
}
```

### 5. Migration Strategy

#### Phase 1: Create Canonical Types
- Establish database types as source of truth
- Create type mapping utilities
- Add derived types for API and auth contexts

#### Phase 2: Add Compatibility Layer
- Create deprecated type aliases pointing to canonical types
- Add migration warnings/comments
- Update imports gradually

#### Phase 3: Update Package Implementations
- Replace duplicate type definitions with imports from shared
- Update function signatures to use canonical types
- Use type mappers for transformations

#### Phase 4: Remove Deprecated Types
- Remove compatibility aliases
- Clean up unused type definitions
- Update documentation

## Data Models

### Current Duplicated Types

| Concept | Current Locations | Canonical Location |
|---------|------------------|-------------------|
| User | `UserResponse` (sso), `AuthUser` (auth-client), `User` (shared/api), `UsersTable` (shared/database) | `UsersTable` → `User` (shared/database) |
| AuthState | `AuthState` (shared/auth), `AuthState` (store/auth), `AuthState` (ui-lib-website/auth-helpers) | `AuthState` (shared/auth) |
| Session | `AuthSession` (auth-client), `SessionData` (shared/auth) | `AuthSession` (shared/auth) |
| UpdateUserData | Multiple locations with slight variations | `UserUpdate` (shared/database) + mapper |

### Unified Data Model

```typescript
// Database layer (source of truth)
type User = UsersTable;

// API layer (derived)
type UserResponse = Pick<User, 'id' | 'email' | 'display_name' | 'avatar_url' | 'created_at'>;

// Auth layer (derived)  
type AuthUser = Pick<User, 'id' | 'email' | 'first_name' | 'last_name' | 'display_name' | 'avatar_url'>;

// Update operations (derived)
type UserUpdate = TablesUpdate<"users">;
```

## Error Handling

### Type Safety During Migration

1. **Gradual Migration**: Use TypeScript's type system to catch breaking changes
2. **Compatibility Aliases**: Provide temporary aliases with deprecation warnings
3. **Runtime Validation**: Add runtime checks for critical type transformations
4. **Testing**: Comprehensive tests for type mappers and transformations

### Error Recovery

```typescript
// Safe type conversion with fallbacks
export function safeUserToApiResponse(user: unknown): UserResponse | null {
  try {
    if (!user || typeof user !== 'object') return null;
    
    const u = user as User;
    return TypeMappers.userToApiResponse(u);
  } catch (error) {
    console.error('Failed to convert user to API response:', error);
    return null;
  }
}
```

## Testing Strategy

### Unit Tests

1. **Type Mapper Tests**: Verify all transformation functions work correctly
2. **Compatibility Tests**: Ensure deprecated aliases still work during migration
3. **Edge Case Tests**: Handle null/undefined values, missing properties

### Integration Tests

1. **API Response Tests**: Verify API endpoints return correctly typed responses
2. **Auth Flow Tests**: Ensure auth operations work with unified types
3. **Database Operation Tests**: Verify CRUD operations with canonical types

### Migration Tests

1. **Before/After Comparison**: Ensure functionality remains identical
2. **Performance Tests**: Verify no performance regression from type transformations
3. **Backward Compatibility**: Test that existing code continues to work

### Test Structure

```typescript
describe('TypeMappers', () => {
  describe('userToApiResponse', () => {
    it('should convert database user to API response format', () => {
      const dbUser: User = {
        id: '123',
        email: 'test@example.com',
        first_name: 'John',
        last_name: 'Doe',
        display_name: 'John Doe',
        avatar_url: 'https://example.com/avatar.jpg',
        role: 'user',
        created_at: '2023-01-01T00:00:00Z',
        updated_at: '2023-01-01T00:00:00Z',
      };

      const apiResponse = TypeMappers.userToApiResponse(dbUser);
      
      expect(apiResponse).toEqual({
        id: '123',
        email: 'test@example.com',
        display_name: 'John Doe',
        avatar_url: 'https://example.com/avatar.jpg',
        created_at: '2023-01-01T00:00:00Z',
      });
    });
  });
});
```