# CDN Package Design Document

## Overview

The CDN package will be a simple static asset server that centralizes common assets (backgrounds, logos, favicons) used across multiple applications in the monorepo. It will be implemented as a Fresh application that serves static files with proper caching headers and organized folder structure.

## Architecture

The CDN package follows the same architectural pattern as other packages in the monorepo:

```
packages/cdn/
├── deno.json           # Package configuration
├── main.ts            # Fresh application entry point
├── mod.ts             # Package exports
├── dev.ts             # Development server
├── routes/            # Route handlers for asset serving
│   └── [...path].ts   # Catch-all route for asset serving
├── static/            # Static assets organized by type
│   ├── backgrounds/   # Background images
│   ├── logos/         # Logo files (SVG, PNG)
│   └── favicons/      # Favicon files (ICO, PNG)
└── lib/               # Utility functions
    └── asset-handler.ts # Asset serving logic
```

## Components and Interfaces

### Asset Server Route Handler
- **Purpose**: Handle all asset requests through a catch-all route
- **Location**: `routes/[...path].ts`
- **Responsibilities**:
  - Parse requested asset path
  - Validate file existence
  - Set appropriate MIME types
  - Apply caching headers
  - Serve static files

### Asset Handler Library
- **Purpose**: Core logic for asset serving and caching
- **Location**: `lib/asset-handler.ts`
- **Functions**:
  - `serveAsset(path: string): Response` - Main asset serving function
  - `getMimeType(extension: string): string` - MIME type detection
  - `getCacheHeaders(fileType: string): Headers` - Cache header generation
  - `validateAssetPath(path: string): boolean` - Path validation

### Static Asset Organization
- **Structure**: Organized folders matching common asset types
- **Folders**:
  - `/backgrounds/` - Hero gradients, patterns, textures
  - `/logos/` - Brand logos in various formats and themes
  - `/favicons/` - Favicon files for different themes and sizes

## Data Models

### Asset Request
```typescript
interface AssetRequest {
  path: string;           // Requested asset path (e.g., "logos/long_dark.png")
  extension: string;      // File extension for MIME type detection
  folder: string;         // Asset category folder
  filename: string;       // Base filename
}
```

### Asset Response
```typescript
interface AssetResponse {
  body: ReadableStream;   // File content stream
  headers: Headers;       // Response headers including cache control
  status: number;         // HTTP status code
}
```

## Error Handling

### File Not Found
- **Scenario**: Requested asset doesn't exist
- **Response**: 404 status with appropriate error message
- **Headers**: No-cache headers to prevent caching of 404 responses

### Invalid Path
- **Scenario**: Path contains invalid characters or attempts directory traversal
- **Response**: 400 Bad Request status
- **Security**: Prevent access to files outside static directory

### Server Errors
- **Scenario**: File system errors or unexpected failures
- **Response**: 500 Internal Server Error
- **Logging**: Log errors for debugging while returning generic error to client

## Testing Strategy

### Unit Tests
- **Asset Handler Functions**: Test MIME type detection, cache header generation
- **Path Validation**: Test security and validation logic
- **Error Handling**: Test various error scenarios

### Integration Tests
- **Asset Serving**: Test end-to-end asset requests
- **Cache Headers**: Verify proper caching behavior
- **File Organization**: Test access to different asset folders

### Performance Tests
- **Load Testing**: Verify performance under concurrent requests
- **Cache Validation**: Test cache hit/miss scenarios
- **File Size Handling**: Test serving of various file sizes

## Implementation Details

### Fresh Application Setup
The CDN package will be a minimal Fresh application with:
- Single catch-all route for asset serving
- No islands or client-side JavaScript
- Optimized for static file serving performance

### Caching Strategy
- **Long-term caching**: 1 year cache for static assets
- **ETag support**: File-based ETags for cache validation
- **Conditional requests**: Support for If-None-Match headers
- **Cache-Control headers**: Appropriate directives for different asset types

### MIME Type Detection
Support for common asset formats:
- Images: `.png`, `.jpg`, `.jpeg`, `.webp`, `.svg`, `.ico`
- Vectors: `.svg` with proper XML content type
- Icons: `.ico` with appropriate favicon MIME type

### Security Considerations
- **Path traversal prevention**: Validate and sanitize all asset paths
- **File type restrictions**: Only serve allowed file extensions
- **Directory listing prevention**: No directory browsing capabilities
- **CORS headers**: Appropriate CORS configuration for cross-origin requests

### Development and Deployment
- **Development server**: Hot reload for asset changes during development
- **Build process**: Asset optimization and validation
- **Deployment**: Integration with existing monorepo deployment pipeline
- **Monitoring**: Basic access logging and error tracking