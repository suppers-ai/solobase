# Design Document

## Overview

This design outlines the refactoring of the DirectAuthClient from a monolithic 1000+ line file in the auth-client package to a modular, well-organized structure within the profile package. The refactoring will maintain all existing functionality while improving code organization, maintainability, and reducing package coupling.

## Architecture

### Current State
- Single file: `packages/auth-client/src/direct-auth-client.ts` (1142 lines)
- Monolithic class with mixed responsibilities
- Located in separate auth-client package despite only being used by profile package

### Target State
- Modular structure in `packages/profile/lib/auth-client/`
- Separated concerns with focused modules
- Main DirectAuthClient class as a facade pattern
- Co-located with its only consumer (profile package)

## Components and Interfaces

### 1. Main DirectAuthClient Class
**Location:** `packages/profile/lib/auth-client/direct-auth-client.ts`
**Responsibility:** Facade pattern that composes all auth modules
**Size:** ~100-150 lines

```typescript
export class DirectAuthClient {
  private sessionManager: SessionManager;
  private authMethods: AuthMethods;
  private userManager: UserManager;
  private storageManager: StorageManager;
  private eventManager: EventManager;
  
  constructor(supabaseUrl: string, supabaseAnonKey: string) {
    // Initialize all managers
  }
  
  // Delegate methods to appropriate managers
}
```

### 2. Session Management Module
**Location:** `packages/profile/lib/auth-client/session-manager.ts`
**Responsibility:** Handle session state, storage, and validation
**Size:** ~200-250 lines

**Key Methods:**
- `getSession()`
- `getSessionStatus()`
- `quickAuthCheck()`
- `hasExistingSession()`
- `isAuthenticated()`
- `getAccessToken()`
- `saveUserIdToStorage()`
- `getUserIdFromStorage()`
- `clearUserIdFromStorage()`

### 3. Authentication Methods Module
**Location:** `packages/profile/lib/auth-client/auth-methods.ts`
**Responsibility:** Handle sign in, sign up, password reset, OAuth
**Size:** ~200-250 lines

**Key Methods:**
- `signIn()`
- `signUp()`
- `signOut()`
- `resetPassword()`
- `signInWithOAuth()`

### 4. User Management Module
**Location:** `packages/profile/lib/auth-client/user-manager.ts`
**Responsibility:** Handle user profile operations and data
**Size:** ~200-250 lines

**Key Methods:**
- `getUser()`
- `updateUser()`
- `ensureUserProfile()`
- `createUserProfileIfNeeded()`

### 5. Storage Operations Module
**Location:** `packages/profile/lib/auth-client/storage-manager.ts`
**Responsibility:** Handle file upload, download, and storage operations
**Size:** ~200-250 lines

**Key Methods:**
- `uploadFile()`
- `uploadContent()`
- `downloadFile()`
- `listFiles()`
- `getFileInfo()`
- `deleteFile()`

### 6. Event Management Module
**Location:** `packages/profile/lib/auth-client/event-manager.ts`
**Responsibility:** Handle event listeners and callbacks
**Size:** ~100-150 lines

**Key Methods:**
- `addEventListener()`
- `removeEventListener()`
- `emitEvent()`
- Event callback management

### 7. Types and Interfaces
**Location:** `packages/profile/lib/auth-client/types.ts`
**Responsibility:** All type definitions and interfaces
**Size:** ~100 lines

**Exports:**
- `SignInData`
- `SignUpData`
- `UpdateUserData`
- `ResetPasswordData`
- Internal interfaces for managers

### 8. Utilities and Helpers
**Location:** `packages/profile/lib/auth-client/utils.ts`
**Responsibility:** Shared utilities and helper functions
**Size:** ~100 lines

**Functions:**
- Timeout promise helpers
- Error handling utilities
- Common validation functions

### 9. Main Export File
**Location:** `packages/profile/lib/auth-client/index.ts`
**Responsibility:** Central export point for the auth client
**Size:** ~20-30 lines

## Data Models

### Manager Dependencies
```typescript
interface ManagerDependencies {
  supabase: SupabaseClient;
  storageKey: string;
  eventCallbacks: Map<AuthEventType, AuthEventCallback[]>;
}
```

### Manager Interfaces
Each manager will implement a consistent interface pattern:
```typescript
interface BaseManager {
  initialize?(dependencies: ManagerDependencies): void;
  destroy?(): void;
}
```

## Error Handling

### Consistent Error Patterns
- All managers will use consistent error handling patterns
- Timeout handling for all async operations (5-10 second timeouts)
- Graceful degradation when Supabase client is unavailable
- Proper error logging with context

### Error Types
- Network/timeout errors
- Authentication errors
- Permission/RLS errors
- Storage errors

## Testing Strategy

### Unit Testing
- Each manager module will have its own test file
- Mock Supabase client for isolated testing
- Test error conditions and edge cases
- Test timeout scenarios

### Integration Testing
- Test the composed DirectAuthClient class
- Test manager interactions
- Test with real Supabase client in development

### Migration Testing
- Verify all existing functionality works after refactoring
- Test profile package integration
- Verify no breaking changes

## Migration Strategy

### Phase 1: Create New Structure
1. Create new directory structure in profile package
2. Split DirectAuthClient into manager modules
3. Create facade DirectAuthClient class
4. Add comprehensive tests

### Phase 2: Update Profile Package
1. Update profile package imports
2. Update auth.ts to use new location
3. Test all profile package functionality

### Phase 3: Clean Up Auth-Client Package
1. Remove old DirectAuthClient file
2. Update auth-client exports
3. Update oauth-auth-client imports if needed
4. Update documentation and examples

### Phase 4: Verification
1. Run all tests
2. Verify profile package works correctly
3. Check for any remaining references
4. Update any documentation

## File Structure

```
packages/profile/lib/auth-client/
├── index.ts                 # Main exports
├── direct-auth-client.ts    # Facade class
├── session-manager.ts       # Session handling
├── auth-methods.ts          # Authentication methods
├── user-manager.ts          # User profile operations
├── storage-manager.ts       # File/storage operations
├── event-manager.ts         # Event handling
├── types.ts                 # Type definitions
└── utils.ts                 # Shared utilities
```

## Dependencies

### Internal Dependencies
- All managers depend on SupabaseClient
- Managers may share utilities from utils.ts
- Event manager is used by other managers for notifications

### External Dependencies
- `@supabase/supabase-js` (existing)
- Shared types from `@suppers/shared` (existing)
- Config from root config.ts (existing)

## Performance Considerations

### Module Loading
- Lazy loading of managers where possible
- Minimal initialization overhead
- Efficient memory usage

### Caching
- Maintain existing session caching behavior
- Cache user data appropriately
- Efficient storage of user ID in localStorage

## Security Considerations

### Token Handling
- Secure token storage and retrieval
- Proper token timeout handling
- No token logging or exposure

### User Data
- Maintain existing RLS policy compliance
- Secure user profile creation
- Proper error handling for permission issues