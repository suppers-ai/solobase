# Implementation Plan

- [x] 1. Create type mapping utilities and enhance shared types
  - Create `packages/shared/utils/type-mappers.ts` with conversion functions between database types and API/auth formats
  - Add derived type definitions to `packages/shared/types/api.ts` and `packages/shared/types/auth.ts` based on database schema
  - Write comprehensive unit tests for all type mapping functions
  - _Requirements: 1.1, 1.2, 4.1, 4.2_

- [x] 2. Update API functions to use canonical database types
  - Modify `packages/api/functions/sso/user.ts` to use `UserResponse` type from shared package instead of local interface
  - Update the user response mapping to use type mapper utilities
  - Add proper error handling for type conversions
  - _Requirements: 1.1, 2.1, 3.1_

- [x] 3. Refactor auth-client package to use shared types
  - Replace `AuthUser` interface in `packages/auth-client/types/auth.ts` with import from shared package
  - Update `AuthSession` interface to use canonical user type
  - Modify `packages/auth-client/src/auth-client.ts` to use shared type definitions
  - Update all auth-client components and providers to use unified types
  - _Requirements: 1.1, 2.1, 2.2_

- [x] 4. Consolidate auth state types across packages
  - Remove duplicate `AuthState` definitions from `packages/store/types/auth.ts` and `packages/ui-lib-website/shared/lib/auth-helpers.ts`
  - Update all auth providers and components to import `AuthState` from shared package
  - Ensure consistent auth state management across all packages
  - _Requirements: 2.1, 2.2, 3.1_

- [x] 5. Update UI library components to use canonical types
  - Modify `packages/ui-lib/shared/components/UserAvatar.tsx` and `UserClientAvatar.tsx` to use shared user types
  - Update `packages/ui-lib/components/navigation/user-profile-dropdown/UserProfileDropdown.tsx` to use canonical user type
  - Replace local user interfaces in page components with shared types
  - Update all user-related prop types and interfaces
  - _Requirements: 1.1, 3.1, 3.2_

- [x] 6. Refactor API helpers to eliminate duplicate types
  - Update `packages/ui-lib/shared/lib/api-helpers.ts` to remove duplicate `UpdateUserData` interface
  - Modify `packages/ui-lib-website/shared/lib/api-helpers.ts` to use shared type definitions
  - Replace local type definitions with imports from shared package
  - Update all API helper functions to use canonical types
  - _Requirements: 1.1, 3.1, 4.1_

- [x] 7. Create backward compatibility layer with deprecation warnings
  - Add deprecated type aliases in each package pointing to canonical shared types
  - Include TypeScript deprecation comments with migration instructions
  - Create migration guide documentation for developers
  - Add console warnings for deprecated type usage in development mode
  - _Requirements: 5.1, 5.2_

- [ ] 8. Update all import statements across packages
  - Replace local type imports with shared package imports in all TypeScript files
  - Update component prop types to use canonical definitions
  - Ensure consistent import paths across the entire codebase
  - Run TypeScript compiler to verify no type errors
  - _Requirements: 1.3, 3.2, 5.2_

- [ ] 9. Add comprehensive integration tests for type consistency
  - Write tests to verify API responses match expected type definitions
  - Create tests for auth flow type consistency across packages
  - Add tests for database operation type safety
  - Implement tests for type mapper edge cases and error handling
  - _Requirements: 3.3, 4.3_

- [ ] 10. Clean up deprecated types and finalize migration
  - Remove all deprecated type aliases and compatibility layers
  - Delete duplicate type definition files
  - Update package exports to only expose canonical types
  - Run full test suite to ensure no regressions
  - Update documentation to reflect new type structure
  - _Requirements: 3.1, 5.3_