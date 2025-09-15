# Requirements Document

## Introduction

This feature involves refactoring the `DirectAuthClient` class from the `auth-client` package to the `profile` package, since the profile package is the only consumer of this client. The current implementation is over 1000 lines of code in a single file, making it difficult to maintain and understand. The refactoring will split the functionality into smaller, more focused modules while maintaining all existing functionality.

## Requirements

### Requirement 1

**User Story:** As a developer, I want the DirectAuthClient to be located in the profile package, so that it's co-located with its only consumer and reduces unnecessary package dependencies.

#### Acceptance Criteria

1. WHEN the refactoring is complete THEN the DirectAuthClient SHALL be moved from `packages/auth-client/src/direct-auth-client.ts` to `packages/profile/lib/auth-client/`
2. WHEN the refactoring is complete THEN the auth-client package SHALL no longer contain the DirectAuthClient
3. WHEN the refactoring is complete THEN all imports in the profile package SHALL be updated to use the new location
4. WHEN the refactoring is complete THEN no other packages SHALL reference the DirectAuthClient

### Requirement 2

**User Story:** As a developer, I want the DirectAuthClient functionality to be split into smaller, focused modules, so that the code is more maintainable and easier to understand.

#### Acceptance Criteria

1. WHEN the DirectAuthClient is refactored THEN it SHALL be split into logical modules with no single file exceeding 300 lines
2. WHEN the DirectAuthClient is refactored THEN authentication methods SHALL be in a separate module
3. WHEN the DirectAuthClient is refactored THEN user management methods SHALL be in a separate module
4. WHEN the DirectAuthClient is refactored THEN storage/file operations SHALL be in a separate module
5. WHEN the DirectAuthClient is refactored THEN session management SHALL be in a separate module
6. WHEN the DirectAuthClient is refactored THEN event handling SHALL be in a separate module

### Requirement 3

**User Story:** As a developer, I want all existing functionality to be preserved during the refactoring, so that no breaking changes are introduced to the profile package.

#### Acceptance Criteria

1. WHEN the refactoring is complete THEN all public methods of DirectAuthClient SHALL remain available with the same signatures
2. WHEN the refactoring is complete THEN all existing functionality SHALL work exactly as before
3. WHEN the refactoring is complete THEN the profile package SHALL continue to work without any changes to its usage of the auth client
4. WHEN the refactoring is complete THEN all type definitions SHALL be preserved

### Requirement 4

**User Story:** As a developer, I want the refactored code to follow the project's architectural patterns, so that it's consistent with the rest of the codebase.

#### Acceptance Criteria

1. WHEN the refactoring is complete THEN the new modules SHALL follow the project's file naming conventions (kebab-case)
2. WHEN the refactoring is complete THEN the new modules SHALL use proper TypeScript types and interfaces
3. WHEN the refactoring is complete THEN the new modules SHALL include proper error handling
4. WHEN the refactoring is complete THEN the new modules SHALL include appropriate logging
5. WHEN the refactoring is complete THEN the main DirectAuthClient class SHALL act as a facade that composes the smaller modules

### Requirement 5

**User Story:** As a developer, I want proper cleanup of the old auth-client package structure, so that there are no orphaned files or broken references.

#### Acceptance Criteria

1. WHEN the refactoring is complete THEN the old `direct-auth-client.ts` file SHALL be removed
2. WHEN the refactoring is complete THEN the auth-client package exports SHALL be updated to remove DirectAuthClient references
3. WHEN the refactoring is complete THEN any documentation or examples SHALL be updated to reflect the new location
4. WHEN the refactoring is complete THEN the oauth-auth-client SHALL be updated to import types from the new location if needed