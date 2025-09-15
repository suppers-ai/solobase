# Implementation Plan

- [x] 1. Create new auth-client directory structure in profile package
  - Create the directory structure `packages/profile/lib/auth-client/`
  - Set up the basic file structure with placeholder files
  - _Requirements: 1.1, 2.1_

- [x] 2. Extract and implement types and utilities modules
- [x] 2.1 Create types module with all interface definitions
  - Extract all type definitions from DirectAuthClient
  - Create `packages/profile/lib/auth-client/types.ts` with SignInData, SignUpData, UpdateUserData, ResetPasswordData interfaces
  - Add internal manager interfaces and dependency types
  - _Requirements: 2.1, 3.4, 4.2_

- [x] 2.2 Create utilities module with shared helper functions
  - Extract timeout promise helpers and error handling utilities
  - Create `packages/profile/lib/auth-client/utils.ts` with common validation and helper functions
  - Implement consistent error handling patterns
  - _Requirements: 2.1, 4.3, 4.4_

- [x] 3. Implement session management module
- [x] 3.1 Create session manager with storage operations
  - Create `packages/profile/lib/auth-client/session-manager.ts`
  - Implement localStorage operations: saveUserIdToStorage, getUserIdFromStorage, clearUserIdFromStorage
  - Implement session validation and status checking methods
  - _Requirements: 2.5, 3.1, 4.3_

- [x] 3.2 Implement session retrieval and authentication checks
  - Add getSession, getSessionStatus, quickAuthCheck, hasExistingSession methods
  - Implement isAuthenticated and getAccessToken methods
  - Add proper timeout handling and error management
  - _Requirements: 2.5, 3.1, 4.3, 4.4_

- [x] 4. Implement authentication methods module
- [x] 4.1 Create authentication methods with sign in/up functionality
  - Create `packages/profile/lib/auth-client/auth-methods.ts`
  - Implement signIn, signUp, and resetPassword methods with timeout handling
  - Add proper error handling and validation
  - _Requirements: 2.2, 3.1, 4.3, 4.4_

- [x] 4.2 Implement OAuth and sign out functionality
  - Add signInWithOAuth method with provider support
  - Implement signOut method with proper cleanup
  - Ensure all methods maintain existing functionality and signatures
  - _Requirements: 2.2, 3.1, 3.2_

- [x] 5. Implement user management module
- [x] 5.1 Create user manager with profile operations
  - Create `packages/profile/lib/auth-client/user-manager.ts`
  - Implement getUser and updateUser methods
  - Add proper database interaction and error handling
  - _Requirements: 2.3, 3.1, 4.3_

- [x] 5.2 Implement user profile creation and management
  - Add ensureUserProfile and createUserProfileIfNeeded methods
  - Implement proper RLS policy handling and user data validation
  - Ensure backward compatibility with existing profile creation logic
  - _Requirements: 2.3, 3.1, 3.2_

- [x] 6. Implement storage operations module
- [x] 6.1 Create storage manager with file operations
  - Create `packages/profile/lib/auth-client/storage-manager.ts`
  - Implement uploadFile, uploadContent, and downloadFile methods
  - Add proper authentication token handling for storage operations
  - _Requirements: 2.4, 3.1, 4.3_

- [x] 6.2 Implement file management operations
  - Add listFiles, getFileInfo, and deleteFile methods
  - Implement proper error handling and response parsing
  - Ensure all storage operations maintain existing functionality
  - _Requirements: 2.4, 3.1, 3.2_

- [x] 7. Implement event management module
- [x] 7.1 Create event manager with callback handling
  - Create `packages/profile/lib/auth-client/event-manager.ts`
  - Implement addEventListener, removeEventListener, and emitEvent methods
  - Add proper event callback management and error handling
  - _Requirements: 2.6, 3.1, 4.3_

- [x] 7.2 Integrate event management with other modules
  - Ensure session manager can emit login/logout events
  - Add proper event cleanup and destroy functionality
  - Test event callback execution and error handling
  - _Requirements: 2.6, 3.1, 3.2_

- [x] 8. Create main DirectAuthClient facade class
- [x] 8.1 Implement DirectAuthClient as composition of managers
  - Create `packages/profile/lib/auth-client/direct-auth-client.ts`
  - Initialize all manager instances in constructor
  - Implement delegation methods to appropriate managers
  - _Requirements: 2.1, 3.1, 4.5_

- [x] 8.2 Add initialization and lifecycle management
  - Implement initialize method that coordinates all managers
  - Add proper error handling and offline mode support
  - Implement destroy method for cleanup
  - _Requirements: 3.1, 3.2, 4.3, 4.4_

- [x] 9. Create main export file and update profile package
- [x] 9.1 Create index file with proper exports
  - Create `packages/profile/lib/auth-client/index.ts`
  - Export DirectAuthClient class and all type definitions
  - Ensure clean public API surface
  - _Requirements: 3.4, 4.1, 4.2_

- [x] 9.2 Update profile package auth.ts to use new location
  - Update `packages/profile/lib/auth.ts` to import from new location
  - Test that getAuthClient function works with refactored code
  - Verify no breaking changes to profile package usage
  - _Requirements: 1.3, 3.2, 3.3_

- [x] 10. Clean up auth-client package and update references
- [x] 10.1 Remove old DirectAuthClient file and update exports
  - Delete `packages/auth-client/src/direct-auth-client.ts`
  - Update `packages/auth-client/src/mod.ts` to remove DirectAuthClient exports
  - Update oauth-auth-client.ts to import types from new location if needed
  - _Requirements: 1.2, 5.1, 5.2, 5.4_

- [x] 10.2 Update documentation and example files
  - Update any documentation files that reference the old location
  - Update example files in API package to use new import path
  - Verify no remaining references to old DirectAuthClient location
  - _Requirements: 5.3, 1.4_

- [x] 11. Comprehensive testing and verification
- [x] 11.1 Test all DirectAuthClient functionality
  - Test all authentication methods work correctly
  - Test session management and user profile operations
  - Test storage operations and event handling
  - _Requirements: 3.1, 3.2, 3.3_

- [x] 11.2 Verify profile package integration
  - Test profile package startup and authentication flows
  - Verify login, logout, and profile management work correctly
  - Test error handling and edge cases
  - _Requirements: 3.2, 3.3, 4.3, 4.4_