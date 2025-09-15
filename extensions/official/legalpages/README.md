# Legal Pages Extension

A Solobase extension for managing legal documents including Terms and Conditions and Privacy Policy pages.

## Features

- **Document Management**: Create, edit, and manage legal documents through an intuitive admin interface
- **Version Control**: Automatic versioning of all document changes
- **Rich Text Editor**: WYSIWYG editor with formatting options for professional documents
- **Preview Mode**: Preview documents before publishing
- **Public Pages**: Automatically serves public `/terms` and `/privacy` endpoints
- **HTML Sanitization**: Secure HTML content sanitization to prevent XSS attacks

## Installation

The extension is included in the official Solobase extensions and will be automatically loaded when enabled.

## Configuration

Add the following to your Solobase configuration:

```json
{
  "extensions": {
    "legalpages": {
      "enabled": true,
      "config": {
        "enable_terms": true,
        "enable_privacy": true,
        "company_name": "Your Company Name"
      }
    }
  }
}
```

## Usage

### Admin Interface

Access the admin interface at `/ext/legalpages/admin` (requires admin authentication).

The interface provides:
- Separate tabs for Terms and Privacy Policy
- Rich text editor with formatting toolbar
- Save draft and publish options
- Version information display
- Live preview functionality

### API Endpoints

#### Admin Endpoints (Protected)
- `GET /ext/legalpages/api/documents` - List all documents
- `GET /ext/legalpages/api/documents/{type}` - Get specific document
- `POST /ext/legalpages/api/documents/{type}` - Save new document version
- `POST /ext/legalpages/api/documents/{type}/publish` - Publish document version
- `GET /ext/legalpages/api/documents/{type}/preview` - Preview document
- `GET /ext/legalpages/api/documents/{type}/history` - Get version history

#### Public Endpoints
- `GET /terms` - Public terms and conditions page
- `GET /privacy` - Public privacy policy page

### Document Types

- `terms` - Terms and Conditions
- `privacy` - Privacy Policy

## Database Schema

The extension creates a table `ext_legalpages_legal_documents` with the following structure:

| Column | Type | Description |
|--------|------|-------------|
| id | UUID | Primary key |
| document_type | VARCHAR | Document type (terms/privacy) |
| title | VARCHAR | Document title |
| content | TEXT | HTML content |
| version | INTEGER | Auto-incrementing version number |
| is_published | BOOLEAN | Publication status |
| created_at | TIMESTAMP | Creation timestamp |
| updated_at | TIMESTAMP | Last update timestamp |
| created_by | UUID | User ID of creator |

## Security

- All HTML content is sanitized using a whitelist approach
- Admin endpoints require authentication
- CSRF protection on admin forms
- XSS prevention through proper escaping

## Permissions

The extension requires the following permission:
- `legalpages.admin` - Manage legal pages content

## License

MIT