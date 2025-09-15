# Implementation Plan

- [x] 1. Create extension directory structure and core files
  - Create the extension directory at `extensions/official/legalpages/`
  - Create placeholder files: `extension.go`, `models.go`, `handlers.go`, `services.go`, `README.md`
  - _Requirements: 7.1, 7.2, 7.3_

- [x] 2. Implement database models and schema
  - [x] 2.1 Create LegalDocument model with GORM annotations
    - Define LegalDocument struct with all required fields (ID, DocumentType, Title, Content, Version, IsPublished, timestamps, CreatedBy)
    - Add GORM table name method to use `ext_legalpages_legal_documents` table name
    - Add JSON tags for API serialization
    - _Requirements: 1.2, 2.2, 7.4_

  - [x] 2.2 Add model validation and constraints
    - Add GORM validation tags for required fields and constraints
    - Create document type constants for "terms" and "privacy"
    - Add model methods for common operations (GetLatestVersion, IsCurrentVersion)
    - _Requirements: 1.1, 2.1, 5.4_

- [x] 3. Create extension service layer
  - [x] 3.1 Implement LegalPagesService struct and constructor
    - Create service struct with database dependency
    - Implement NewLegalPagesService constructor
    - Add service methods interface definition
    - _Requirements: 1.2, 2.2_

  - [x] 3.2 Implement document CRUD operations
    - Create GetDocument method to retrieve latest published document by type
    - Create SaveDocument method to create new document versions
    - Create GetDocumentHistory method to retrieve all versions
    - Add HTML content sanitization in SaveDocument method
    - _Requirements: 1.1, 1.2, 2.1, 2.2, 5.4_

  - [x] 3.3 Add document publishing and versioning logic
    - Implement PublishDocument method to mark document as published
    - Add automatic version incrementing in SaveDocument
    - Create GetPublishedDocument method for public access
    - _Requirements: 6.1, 6.2_

- [x] 4. Implement HTTP handlers
  - [x] 4.1 Create admin API handlers for document management
    - Implement handleGetDocuments for listing all documents
    - Implement handleGetDocument for retrieving specific document type
    - Implement handleSaveDocument for creating/updating documents
    - Add proper error handling and JSON responses
    - _Requirements: 1.1, 1.2, 2.1, 2.2, 7.2_

  - [x] 4.2 Create preview functionality handler
    - Implement handlePreviewDocument for admin preview mode
    - Add HTML rendering with sanitization
    - Return formatted HTML response for preview
    - _Requirements: 6.1, 6.2, 6.3_

  - [x] 4.3 Implement public page handlers
    - Create handlePublicTerms for /terms endpoint
    - Create handlePublicPrivacy for /privacy endpoint
    - Add HTML template rendering for public pages
    - Handle cases where documents don't exist with default messages
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4_

- [x] 5. Create main extension implementation
  - [x] 5.1 Implement Extension interface methods
    - Create LegalPagesExtension struct with required fields
    - Implement Metadata method with extension information
    - Implement Initialize method with database setup and migration
    - Implement Start, Stop, and Health methods
    - _Requirements: 7.1, 7.2, 7.3, 7.4_

  - [x] 5.2 Implement configuration and permissions
    - Create ConfigSchema method returning JSON schema
    - Implement ValidateConfig and ApplyConfig methods
    - Add RequiredPermissions method with admin permissions
    - Set DatabaseSchema method to return "ext_legalpages"
    - _Requirements: 7.2, 7.3_

  - [x] 5.3 Register routes and middleware
    - Implement RegisterRoutes method to register all endpoints
    - Register admin API routes under /ext/legalpages/api/
    - Register public routes /terms and /privacy at root level
    - Add authentication middleware for admin routes
    - _Requirements: 3.1, 4.1, 7.2, 7.3_

- [x] 6. Create admin UI templates and integration
  - [x] 6.1 Create admin page HTML template
    - Design admin interface template with rich text editor
    - Add separate sections for Terms and Privacy Policy editing
    - Include preview functionality and save/publish buttons
    - Use existing Solobase admin UI components and styling
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 6.1, 6.2, 6.3, 7.3_

  - [x] 6.2 Add JavaScript for rich text editing
    - Integrate TinyMCE or similar WYSIWYG editor
    - Add preview mode toggle functionality
    - Implement auto-save and form validation
    - Add AJAX calls to admin API endpoints
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 6.1, 6.2, 6.3_

  - [x] 6.3 Create public page templates
    - Design clean, readable templates for terms and privacy pages
    - Add proper HTML structure with meta tags for SEO
    - Include navigation back to main site
    - Handle empty/missing document states gracefully
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4_

- [x] 7. Add extension to registry and configuration
  - [x] 7.1 Register extension in extensions configuration
    - Add legalpages extension to extensions/config.json
    - Update extension registry to include the new extension
    - Ensure extension is loaded during application startup
    - _Requirements: 7.1, 7.4_

  - [x] 7.2 Update admin navigation to include Legal Pages
    - Modify admin UI navigation to add "Legal Pages" menu item
    - Position menu item appropriately in admin interface
    - Add proper icons and styling consistent with other admin sections
    - _Requirements: 7.3_

- [ ] 8. Create comprehensive tests
  - [ ] 8.1 Write unit tests for models and services
    - Test LegalDocument model validation and constraints
    - Test LegalPagesService CRUD operations
    - Test HTML sanitization functionality
    - Test version management and publishing logic
    - _Requirements: 1.1, 1.2, 2.1, 2.2, 5.4_

  - [ ] 8.2 Write integration tests for handlers
    - Test admin API endpoints with authentication
    - Test public page rendering and content display
    - Test error handling for missing documents
    - Test preview functionality
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4, 6.1, 6.2_

  - [ ] 8.3 Write extension lifecycle tests
    - Test extension initialization and database migration
    - Test route registration and middleware setup
    - Test extension health checks and error states
    - Test configuration validation and application
    - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [x] 9. Add documentation and README
  - Create comprehensive README.md with setup instructions
  - Document API endpoints and request/response formats
  - Add configuration options and examples
  - Include screenshots of admin interface
  - _Requirements: 7.1, 7.2, 7.3_