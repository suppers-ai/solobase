# Design Document

## Overview

The Legal Pages Extension provides a comprehensive solution for managing terms and conditions and privacy policy documents within Solobase. The extension follows the established extension architecture pattern, providing both administrative interfaces for content management and public endpoints for document viewing.

The extension integrates seamlessly with the existing admin interface using the same UI components and styling patterns, while providing public routes that are accessible without authentication. Content is stored in the extension's dedicated database schema with versioning support.

## Architecture

### Extension Structure
Following the standard Solobase extension pattern:
```
extensions/official/legalpages/
├── extension.go          # Main extension implementation
├── handlers.go           # HTTP request handlers
├── models.go            # Database models
├── services.go          # Business logic services
└── README.md            # Documentation
```

### Database Schema
The extension uses the `ext_legalpages` schema with the following tables:

**legal_documents table:**
- `id` (UUID, primary key)
- `document_type` (enum: 'terms', 'privacy')
- `title` (string)
- `content` (text, HTML content)
- `version` (integer, auto-incrementing)
- `is_published` (boolean)
- `created_at` (timestamp)
- `updated_at` (timestamp)
- `created_by` (UUID, references users.id)

### API Endpoints

**Admin Endpoints (Protected):**
- `GET /ext/legalpages/api/documents` - List all documents
- `GET /ext/legalpages/api/documents/{type}` - Get specific document type
- `POST /ext/legalpages/api/documents/{type}` - Create/update document
- `GET /ext/legalpages/api/documents/{type}/preview` - Preview document

**Public Endpoints (Unprotected):**
- `GET /terms` - Public terms and conditions page
- `GET /privacy` - Public privacy policy page

## Components and Interfaces

### Extension Implementation
The `LegalPagesExtension` struct implements the core `Extension` interface:

```go
type LegalPagesExtension struct {
    services *core.ExtensionServices
    db       *gorm.DB
    config   *LegalPagesConfig
}
```

**Key Methods:**
- `Metadata()` - Returns extension information
- `Initialize()` - Sets up database tables and services
- `RegisterRoutes()` - Registers both admin and public routes
- `RegisterTemplates()` - Registers admin UI templates
- `Health()` - Returns extension health status

### Service Layer
The `LegalPagesService` handles business logic:

```go
type LegalPagesService struct {
    db *gorm.DB
}
```

**Methods:**
- `GetDocument(docType string) (*LegalDocument, error)`
- `SaveDocument(docType, title, content string, userID string) error`
- `GetDocumentHistory(docType string) ([]*LegalDocument, error)`
- `PublishDocument(docType string, version int) error`

### Handler Layer
HTTP handlers manage request/response processing:

- `handleGetDocuments()` - Admin: List documents
- `handleGetDocument()` - Admin: Get specific document
- `handleSaveDocument()` - Admin: Save document
- `handlePreviewDocument()` - Admin: Preview document
- `handlePublicTerms()` - Public: Terms page
- `handlePublicPrivacy()` - Public: Privacy page

## Data Models

### LegalDocument Model
```go
type LegalDocument struct {
    ID           string    `gorm:"primaryKey" json:"id"`
    DocumentType string    `gorm:"not null" json:"document_type"`
    Title        string    `gorm:"not null" json:"title"`
    Content      string    `gorm:"type:text" json:"content"`
    Version      int       `gorm:"not null" json:"version"`
    IsPublished  bool      `gorm:"default:false" json:"is_published"`
    CreatedAt    time.Time `json:"created_at"`
    UpdatedAt    time.Time `json:"updated_at"`
    CreatedBy    string    `json:"created_by"`
}
```

**Table Name:** `ext_legalpages_legal_documents`

**Indexes:**
- Primary key on `id`
- Unique index on `document_type, version`
- Index on `document_type, is_published`

### Document Types
Enum values for `document_type`:
- `"terms"` - Terms and conditions
- `"privacy"` - Privacy policy

## Error Handling

### Error Types
- `ErrDocumentNotFound` - Document doesn't exist
- `ErrInvalidDocumentType` - Invalid document type provided
- `ErrValidationFailed` - Content validation failed
- `ErrUnauthorized` - User lacks required permissions

### Error Responses
All API endpoints return consistent error responses:
```json
{
  "error": "error_code",
  "message": "Human readable message",
  "details": {}
}
```

### HTML Sanitization
Content is sanitized using a whitelist approach:
- Allowed tags: `p, br, strong, em, ul, ol, li, h1, h2, h3, h4, h5, h6, a`
- Allowed attributes: `href` (for links, validated URLs only)
- All other HTML is stripped or escaped

## Testing Strategy

### Unit Tests
- **Model Tests:** Validate GORM model behavior and constraints
- **Service Tests:** Test business logic with mocked database
- **Handler Tests:** Test HTTP endpoints with test database
- **Validation Tests:** Test HTML sanitization and content validation

### Integration Tests
- **Database Tests:** Test schema creation and migrations
- **Extension Tests:** Test full extension lifecycle
- **Route Tests:** Test route registration and middleware
- **Permission Tests:** Test admin access controls

### Test Data
- Sample terms and conditions content
- Sample privacy policy content
- Test user accounts with different permission levels
- Invalid HTML content for sanitization testing

### Test Coverage
Target minimum 80% code coverage across:
- Business logic functions
- HTTP handlers
- Database operations
- Error handling paths

## Admin UI Integration

### Navigation Integration
The extension adds a "Legal Pages" section to the admin navigation menu, positioned after "Users" and before "Settings".

### UI Components
Reuses existing Solobase admin UI components:
- **Card layouts** for content sections
- **Form components** for document editing
- **Button styles** for actions
- **Modal dialogs** for confirmations
- **Notification system** for feedback

### Rich Text Editor
Integrates a rich text editor for content management:
- **Editor:** TinyMCE or similar WYSIWYG editor
- **Toolbar:** Basic formatting (bold, italic, lists, headings, links)
- **Preview mode:** Real-time preview of formatted content
- **Validation:** Client-side HTML validation before submission

### Admin Page Layout
```
Legal Pages
├── Terms and Conditions
│   ├── Editor (with toolbar)
│   ├── Preview button
│   └── Save/Publish buttons
└── Privacy Policy
    ├── Editor (with toolbar)
    ├── Preview button
    └── Save/Publish buttons
```

## Public Page Rendering

### Template System
Uses Go's `html/template` package for server-side rendering:
- **Base template:** Common layout with navigation and footer
- **Content template:** Document-specific content rendering
- **CSS styling:** Consistent with main application theme

### SEO Optimization
- **Meta tags:** Appropriate title and description tags
- **Structured data:** Legal document schema markup
- **Canonical URLs:** Proper canonical URL specification
- **Responsive design:** Mobile-friendly layout

### Caching Strategy
- **In-memory cache:** Published documents cached for 1 hour
- **Cache invalidation:** Automatic invalidation on document updates
- **CDN compatibility:** Proper cache headers for CDN integration

## Security Considerations

### Access Control
- **Admin endpoints:** Require authentication and admin role
- **Public endpoints:** No authentication required
- **Content validation:** All HTML content sanitized
- **CSRF protection:** Admin forms include CSRF tokens

### Content Security
- **HTML sanitization:** Whitelist-based HTML cleaning
- **XSS prevention:** All user content properly escaped
- **Input validation:** Server-side validation of all inputs
- **SQL injection prevention:** Parameterized queries only

### Audit Trail
- **Change tracking:** All document changes logged with user ID
- **Version history:** Complete version history maintained
- **Access logging:** Admin access logged for security monitoring

## Performance Considerations

### Database Optimization
- **Indexes:** Appropriate indexes on frequently queried columns
- **Query optimization:** Efficient queries for document retrieval
- **Connection pooling:** Reuse of database connections

### Caching
- **Document caching:** Published documents cached in memory
- **Template caching:** Compiled templates cached
- **Static assets:** CSS/JS served with appropriate cache headers

### Scalability
- **Stateless design:** No server-side session state
- **Database scaling:** Compatible with read replicas
- **CDN integration:** Static assets can be served from CDN