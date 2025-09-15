# Requirements Document

## Introduction

This feature involves refactoring the existing store package and creating a new profile package to better separate concerns. The store will become a marketplace/generator interface for creating and managing applications using the compiler, while the new profile package will handle the SSO provider functionality that currently exists in the store.

## Requirements

### Requirement 1

**User Story:** As a developer, I want the store to be a web interface for creating and managing applications, so that I can easily generate new apps using the compiler without command-line tools.

#### Acceptance Criteria

1. WHEN a user visits the store homepage THEN the system SHALL display an application marketplace interface with options to create new applications
2. WHEN a user selects "Create New App" THEN the system SHALL provide a form interface for configuring application specifications
3. WHEN a user submits a valid application configuration THEN the system SHALL use the compiler to generate the application
4. WHEN a user views the store THEN the system SHALL display existing application templates and examples
5. WHEN a user interacts with the store THEN the system SHALL provide a modern, responsive UI for application management

### Requirement 2

**User Story:** As a developer, I want a dedicated profile package for SSO functionality, so that client applications have a clean authentication endpoint separate from the store interface.

#### Acceptance Criteria

1. WHEN the profile package is created THEN it SHALL contain only login and profile pages for SSO functionality
2. WHEN a user accesses the profile login page THEN the system SHALL provide authentication via email/password and OAuth providers
3. WHEN a user successfully authenticates THEN the system SHALL redirect them to their profile page or specified redirect URL
4. WHEN a user accesses the profile profile page THEN the system SHALL display user information and account management options
5. WHEN external applications integrate with the profile THEN the system SHALL provide OAuth authorization and token endpoints

### Requirement 3

**User Story:** As a developer, I want the SSO functionality moved from store to profile, so that the store can focus on application generation while profile handles authentication.

#### Acceptance Criteria

1. WHEN the refactoring is complete THEN the store package SHALL NOT contain any authentication pages or SSO provider logic
2. WHEN the refactoring is complete THEN the profile package SHALL contain all authentication islands, routes, and auth helpers from the store
3. WHEN external applications need authentication THEN they SHALL integrate with the profile package instead of the store
4. WHEN the migration is complete THEN existing authentication flows SHALL continue to work without breaking changes
5. WHEN both packages are running THEN they SHALL operate independently with separate concerns

### Requirement 4

**User Story:** As a user, I want the store to integrate with the compiler, so that I can generate applications through a web interface instead of command-line tools.

#### Acceptance Criteria

1. WHEN a user creates an application through the store THEN the system SHALL call the compiler's generate command programmatically
2. WHEN the compiler generates an application THEN the store SHALL display the generation progress and results
3. WHEN an application is generated THEN the store SHALL provide download links or deployment options
4. WHEN a user views generated applications THEN the store SHALL display application metadata and management options
5. WHEN the compiler encounters errors THEN the store SHALL display user-friendly error messages and suggestions

### Requirement 5

**User Story:** As a developer, I want the profile package to be lightweight and focused, so that it serves as a dedicated authentication service without unnecessary features.

#### Acceptance Criteria

1. WHEN the profile package is created THEN it SHALL only include authentication-related functionality
2. WHEN the profile package runs THEN it SHALL have minimal dependencies and fast startup time
3. WHEN the profile package serves requests THEN it SHALL focus only on login, profile, and OAuth endpoints
4. WHEN the profile package is deployed THEN it SHALL be independently deployable from the store
5. WHEN the profile package handles authentication THEN it SHALL maintain the same security standards as the current store implementation

### Requirement 6

**User Story:** As a developer, I want proper configuration management between packages, so that both store and profile can be configured independently while sharing common settings.

#### Acceptance Criteria

1. WHEN both packages are configured THEN they SHALL have separate environment configurations
2. WHEN shared configuration is needed THEN the system SHALL use the shared package for common constants and types
3. WHEN the profile package needs database access THEN it SHALL use the same Supabase configuration pattern as other packages
4. WHEN the store package needs to call the compiler THEN it SHALL have proper configuration for compiler integration
5. WHEN both packages are deployed THEN they SHALL be able to run on different ports and domains if needed