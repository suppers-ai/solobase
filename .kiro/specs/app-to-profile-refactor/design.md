# Design Document

## Overview

This design outlines the systematic refactoring of the "app" package to "profile" package to better reflect its purpose as a user profile and SSO authentication service. The refactor involves renaming the package, updating all references, and ensuring the core functionality remains intact while improving code clarity and maintainability.

The current "app" package serves as a comprehensive SSO authentication service with OAuth 2.0 endpoints, user profile management, and authentication flows. By renaming it to "profile", we make its purpose more explicit and align with its actual functionality.

## Architecture

### Current Package Structure
```
packages/app/
├── islands/              # Client-side interactive components
│   ├── AuthCallbackHandler.tsx
│   ├── LoginPageIsland.tsx
│   ├── LogoutHandler.tsx
│   ├── OAuthHandler.tsx
│   └── ProfilePageIsland.tsx
├── lib/                  # Server-side utilities and services
│   ├── api-client.ts
│   ├── auth-helpers.ts
│   ├── cleanup-service.ts
│   ├── middleware.ts
│   ├── oauth-service.ts
│   ├── security-config.ts
│   ├── supabase-client.ts
│   └── token-manager.ts
├── routes/               # API and page routes
│   ├── .well-known/      # OAuth discovery endpoints
│   ├── auth/             # Authentication routes
│   ├── oauth/            # OAuth 2.0 endpoints
│   ├── _app.tsx
│   ├── index.tsx
│   ├── login.tsx
│   └── profile.tsx
├── static/               # Static assets
├── deno.json
├── main.ts
└── README.md
```

### Target Package Structure
```
packages/profile/
├── islands/              # Client-side interactive components (unchanged)
├── lib/                  # Server-side utilities and services (unchanged)
├── routes/               # API and page routes (unchanged)
├── static/               # Static assets (unchanged)
├── deno.json             # Updated package name and metadata
├── main.ts               # Updated comments and configuration
└── README.md             # Updated documentation
```

### Refactoring Strategy

The refactoring will follow a systematic approach to ensure zero downtime and maintain all existing functionality:

1. **File System Changes**: Rename the package directory and update internal file references
2. **Configuration Updates**: Update package metadata, workspace configuration, and build scripts
3. **Cross-Package Dependencies**: Update any references from other packages
4. **Documentation Updates**: Update README and inline documentation
5. **Environment and Deployment**: Update environment variables and deployment configurations

## Components and Interfaces

### Package Metadata Changes

**Current deno.json:**
```json
{
  "name": "@suppers/app",
  "version": "1.0.0",
  "exports": "./mod.ts"
}
```

**Target deno.json:**
```json
{
  "name": "@suppers/profile",
  "version": "1.0.0",
  "exports": "./mod.ts"
}
```

### Workspace Configuration Changes

**Root deno.json workspace array:**
- Remove: `"packages/app"`
- Add: `"packages/profile"`

**Root deno.json tasks:**
- Update all `dev:app`, `start:app`, `build:app`, `test:app`, `check:app` tasks
- Change paths from `packages/app` to `packages/profile`
- Rename tasks to use `profile` instead of `app` (e.g., `dev:profile`)

### Script Updates

**scripts/dev-concurrent.ts:**
- Update `cwd` from `"./packages/app"` to `"./packages/profile"`
- Update `name` from `"APP"` to `"PROFILE"`
- Consider updating port from 8001 to 8002 for clarity

### Environment Variable Updates

**Current environment variables:**
- `APP_PORT` → `PROFILE_PORT`
- `APP_HOST` → `PROFILE_HOST`

**main.ts configuration:**
```typescript
// Current
const port = parseInt(Deno.env.get("APP_PORT") || "8001");
const hostname = Deno.env.get("APP_HOST") || "localhost";

// Target
const port = parseInt(Deno.env.get("PROFILE_PORT") || "8002");
const hostname = Deno.env.get("PROFILE_HOST") || "localhost";
```

## Data Models

### No Data Model Changes Required

The refactoring is purely structural and does not affect:
- Database schemas
- API contracts
- Data structures
- Authentication flows
- OAuth 2.0 endpoints

All existing data models, interfaces, and types remain unchanged to ensure backward compatibility.

## Error Handling

### Refactoring Risk Mitigation

1. **Broken References**: Systematic search and replace of all package references
2. **Import Failures**: Update all import statements across the codebase
3. **Build Failures**: Update all build scripts and configuration files
4. **Deployment Issues**: Update deployment scripts and environment configurations

### Validation Strategy

1. **Pre-refactor Validation**: Ensure all tests pass and application builds successfully
2. **Post-refactor Validation**: Run full test suite and verify all functionality
3. **Cross-package Integration**: Test that other packages can still interact correctly
4. **End-to-end Testing**: Verify OAuth flows and authentication still work

## Testing Strategy

### Test Categories

1. **Unit Tests**: Verify all existing unit tests continue to pass
2. **Integration Tests**: Ensure OAuth endpoints and authentication flows work
3. **Cross-package Tests**: Verify other packages can still reference the renamed package
4. **Build Tests**: Ensure all build and deployment scripts work correctly

### Test Execution Plan

1. **Baseline Testing**: Run all tests before refactoring to establish baseline
2. **Incremental Testing**: Test after each major change (directory rename, config updates, etc.)
3. **Final Validation**: Comprehensive test run after all changes are complete
4. **Regression Testing**: Verify no functionality has been lost or changed

### Specific Test Areas

1. **OAuth 2.0 Endpoints**: 
   - `/oauth/authorize`
   - `/oauth/token`
   - `/oauth/userinfo`
   - `/oauth/validate`
   - `/oauth/revoke`

2. **Authentication Flows**:
   - User login/logout
   - Password reset
   - OAuth provider integration
   - Session management

3. **Profile Management**:
   - Profile viewing
   - Profile editing
   - Avatar upload
   - Password changes

4. **Cross-package Integration**:
   - Verify no other packages import from the old package name
   - Ensure workspace configuration works correctly
   - Test build and development scripts

## Implementation Phases

### Phase 1: Preparation and Validation
- Run full test suite to establish baseline
- Document current functionality
- Identify all references to the current package

### Phase 2: File System Changes
- Rename `packages/app` to `packages/profile`
- Update internal file references and comments
- Update package metadata in deno.json

### Phase 3: Configuration Updates
- Update root workspace configuration
- Update build scripts and tasks
- Update development and deployment scripts

### Phase 4: Environment and Documentation
- Update environment variable names
- Update README and documentation
- Update any deployment configurations

### Phase 5: Validation and Testing
- Run full test suite
- Test all OAuth endpoints
- Verify cross-package functionality
- Test build and deployment processes

## Rollback Strategy

In case issues arise during the refactoring:

1. **Git-based Rollback**: Use version control to revert changes
2. **Incremental Rollback**: Revert specific phases if needed
3. **Configuration Rollback**: Restore original workspace and build configurations
4. **Environment Rollback**: Restore original environment variable names

The refactoring will be done in a feature branch to allow for easy rollback if needed.