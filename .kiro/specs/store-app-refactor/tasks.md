# Implementation Plan

- [x] 1. Create profile package structure and basic setup
  - Create new `packages/profile` directory with Fresh application structure
  - Set up deno.json configuration with proper dependencies
  - Create basic routing structure with index, login, and profile routes
  - Configure Tailwind CSS and daisyUI for styling
  - _Requirements: 2.1, 2.2, 5.1, 5.4_

- [x] 2. Move authentication functionality from store to profile package
- [x] 2.1 Copy authentication islands and components
  - Move LoginPageIsland.tsx from store to profile package
  - Move ProfilePageIsland.tsx from store to profile package
  - Move AuthCallbackHandler.tsx and OAuthHandler.tsx islands
  - Update imports to use @suppers/ui-lib components
  - _Requirements: 3.2, 3.4_

- [x] 2.2 Copy authentication routes and handlers
  - Move login.tsx, profile.tsx routes from store to profile
  - Move auth callback, logout, and oauth routes from store to profile
  - Update route handlers to work with new package structure
  - _Requirements: 3.2, 3.4_

- [x] 2.3 Copy authentication helpers and services
  - Move auth-helpers.ts from store lib to profile lib
  - Move supabase-client.ts configuration to profile package
  - Update database types imports to use shared package
  - Create OAuth service using existing database types
  - _Requirements: 3.2, 3.4, 6.3_

- [x] 3. Remove authentication code from store package
- [x] 3.1 Remove authentication islands from store
  - Delete LoginPageIsland.tsx from store islands
  - Delete ProfilePageIsland.tsx from store islands
  - Delete AuthCallbackHandler.tsx and OAuthHandler.tsx from store
  - _Requirements: 3.1_

- [x] 3.2 Remove authentication routes from store
  - Delete login.tsx and profile.tsx routes from store
  - Delete auth directory and all auth routes from store
  - Update store routing to remove auth-related paths
  - _Requirements: 3.1_

- [x] 3.3 Remove authentication helpers from store
  - Delete auth-helpers.ts from store lib directory
  - Remove authentication-related imports from store components
  - Clean up unused authentication dependencies from store deno.json
  - _Requirements: 3.1_

- [x] 4. Create store marketplace interface
- [x] 4.1 Implement marketplace homepage component
  - Create MarketplaceHomepage island using Card, Hero, Button components from ui-lib
  - Display application templates gallery with preview cards
  - Add "Create New App" button and template selection functionality
  - Implement recent applications display using database types
  - _Requirements: 1.1, 1.4_

- [x] 4.2 Create application template management
  - Implement ApplicationTemplate interface and data structure
  - Create template gallery component using Card and Badge components
  - Add template categorization and filtering functionality
  - Create template preview modal using Modal component from ui-lib
  - _Requirements: 1.4_

- [x] 4.3 Build app generator form interface
  - Create AppGeneratorForm island using Input, Select, Checkbox components
  - Implement multi-step form using Steps component from ui-lib
  - Add application configuration form with validation
  - Create route configuration interface with dynamic form fields
  - _Requirements: 1.2, 1.3_

- [x] 5. Implement compiler integration service
- [x] 5.1 Create compiler service wrapper
  - Implement CompilerService class to interface with compiler package
  - Add generateApplication method that calls compiler programmatically
  - Create validateSpec method for application specification validation
  - Add error handling and user-friendly error messages
  - _Requirements: 4.1, 4.5_

- [x] 5.2 Add generation progress tracking
  - Create GenerationProgress component using Progress component from ui-lib
  - Implement real-time generation status updates
  - Add generation result display with download links
  - Create error display with suggestions for common issues
  - _Requirements: 4.2, 4.5_

- [x] 5.3 Implement application management features
  - Create generated applications list using Table component from ui-lib
  - Add application metadata display and management options
  - Implement download and deployment functionality
  - Create application deletion and cleanup features
  - _Requirements: 4.3, 4.4_

- [-] 6. Update profile package for OAuth and SSO functionality
- [x] 6.1 Enhance OAuth service implementation
  - Extend OAuth service to use Tables<"oauth_clients"> database types
  - Implement OAuth authorization code flow with proper state validation
  - Add token generation and validation using Tables<"oauth_tokens"> types
  - Create client registration and management functionality
  - _Requirements: 2.4, 2.5_

- [x] 6.2 Implement SSO provider endpoints
  - Create OAuth authorization endpoint (/oauth/authorize)
  - Implement token endpoint (/oauth/token) for code exchange
  - Add user info endpoint (/oauth/userinfo) for profile data
  - Create token validation and refresh endpoints
  - _Requirements: 2.4, 2.5_

- [x] 6.3 Add authentication middleware and security
  - Implement OAuth state parameter validation for CSRF protection
  - Add rate limiting for authentication endpoints
  - Create session management using Tables<"oauth_tokens"> types
  - Implement proper token expiration and refresh logic
  - _Requirements: 2.5, 5.5_

- [x] 7. Configure package environments and dependencies
- [x] 7.1 Set up profile package configuration
  - Configure profile package deno.json with proper dependencies
  - Set up environment variables for Supabase and OAuth configuration
  - Create separate port configuration for profile package (8002)
  - Configure CORS settings for cross-origin authentication requests
  - _Requirements: 6.1, 6.4, 6.5_

- [x] 7.2 Update store package configuration
  - Remove authentication-related dependencies from store deno.json
  - Add compiler package dependency for programmatic access
  - Configure store to run on port 8000 (default)
  - Update environment configuration for compiler integration
  - _Requirements: 6.1, 6.4_

- [x] 7.3 Update shared package integration
  - Ensure both packages properly import from shared package
  - Update database types usage across both packages
  - Configure shared constants and utilities access
  - Test cross-package type compatibility
  - _Requirements: 6.2, 6.3_

- [x] 8. Create development and deployment scripts
- [x] 8.1 Update development scripts
  - Create separate dev scripts for app and store packages
  - Update root deno.json to include both package dev commands
  - Create concurrent development script to run both packages
  - Add package-specific task commands (start, build, test)
  - _Requirements: 6.5_

- [x] 8.2 Create package documentation
  - Update README.md for profile package with authentication setup
  - Update README.md for store package with marketplace functionality
  - Create migration guide for existing authentication users
  - Document OAuth integration steps for external applications
  - _Requirements: 6.5_

- [x] 8.3 Implement testing for both packages
  - Create unit tests for profile package authentication flows
  - Add integration tests for store package compiler integration
  - Test cross-package communication and OAuth flows
  - Create end-to-end tests for complete application generation workflow
  - _Requirements: 3.4, 4.1_