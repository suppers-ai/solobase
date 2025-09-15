# Design Document

## Overview

The Dashboard package is a minimalist Fresh application that provides a centralized interface for users to create and manage their applications within the Suppers platform. It follows the established platform architecture patterns, using the shared UI library, authentication client, and API utilities to maintain consistency across the ecosystem.

The dashboard emphasizes simplicity and clarity, presenting only essential functionality in a clean, accessible interface. It serves as the primary entry point for application management while seamlessly integrating with the existing platform infrastructure.

## Architecture

### Application Structure
The dashboard follows the standard Fresh application pattern used across the platform:

```
packages/dashboard/
├── main.ts                 # Application entry point
├── dev.ts                  # Development server
├── deno.json              # Package configuration
├── routes/                # File-based routing
│   ├── _app.tsx          # Root layout with auth
│   ├── index.tsx         # Dashboard home page
│   └── applications/     # Application management routes
│       ├── [id].tsx      # Application detail page
│       └── new.tsx       # Application creation page
├── islands/              # Client-side interactive components
│   ├── ApplicationList.tsx
│   ├── CreateApplicationForm.tsx
│   └── DeleteConfirmationModal.tsx
├── lib/                  # Application-specific utilities
│   ├── auth.ts          # Authentication client setup
│   └── api.ts           # API client utilities
└── static/              # Static assets
    └── styles.css       # Application-specific styles
```

### Authentication Integration
The dashboard uses OAuth authentication to integrate with the profile service, following the same pattern as the store application:

- Uses `OAuthAuthClient` from `@suppers/auth-client`
- Connects to the profile service for SSO
- Implements route-level authentication guards
- Maintains session state across page navigation

### Data Flow
1. **Authentication**: User authenticates via OAuth flow with profile service
2. **Application Listing**: Dashboard fetches user's applications from existing API package
3. **CRUD Operations**: Create, read, update, delete operations via existing API package endpoints
4. **Real-time Updates**: UI updates immediately after successful operations

## Components and Interfaces

### Core Islands (Client-side Components)

#### ApplicationList Island
```typescript
interface ApplicationListProps {
  applications: Application[];
  onDelete: (id: string) => Promise<void>;
  onEdit: (application: Application) => void;
}
```

Responsibilities:
- Renders list of applications using `ApplicationCard` from UI library
- Handles delete confirmations
- Manages loading states during operations
- Provides empty state when no applications exist

#### CreateApplicationForm Island
```typescript
interface CreateApplicationFormProps {
  onSubmit: (data: CreateApplicationData) => Promise<void>;
  onCancel: () => void;
}
```

Responsibilities:
- Renders form using UI library components (`Input`, `Textarea`, `Button`)
- Validates form data using Zod schemas
- Handles form submission and error states
- Provides real-time validation feedback

#### DeleteConfirmationModal Island
```typescript
interface DeleteConfirmationModalProps {
  application: Application | null;
  isOpen: boolean;
  onConfirm: () => Promise<void>;
  onCancel: () => void;
}
```

Responsibilities:
- Displays confirmation dialog using `Modal` from UI library
- Shows application details and deletion consequences
- Handles confirmation and cancellation actions
- Manages loading state during deletion

### Route Components

#### Dashboard Home (`/`)
- Displays authenticated user's applications
- Provides "Create Application" call-to-action
- Shows empty state for new users
- Implements responsive grid layout

#### Application Detail (`/applications/[id]`)
- Shows detailed application information
- Provides edit and delete actions
- Displays application metadata and configuration
- Includes navigation back to dashboard

#### Create Application (`/applications/new`)
- Renders application creation form
- Handles form validation and submission
- Redirects to application detail on success
- Provides cancel navigation back to dashboard

### API Integration

The dashboard integrates with the existing API package endpoints for all application operations:

#### Application Management API
- **GET** `/api/applications` - List user's applications with pagination and filtering
- **POST** `/api/applications` - Create new application with validation
- **GET** `/api/applications/[id]` - Get specific application details
- **PUT** `/api/applications/[id]` - Update application data
- **DELETE** `/api/applications/[id]` - Delete application and associated data

#### API Client Integration
The dashboard uses shared API utilities from `@suppers/shared` to communicate with the API package:
- Consistent error handling across the platform
- Automatic authentication token management
- Request/response type safety with TypeScript
- Standardized API response format

## Data Models

### Application Data Structure
Based on the existing platform schema:

```typescript
interface Application {
  id: string;
  name: string;
  description?: string;
  template_id: string;
  configuration: Record<string, unknown>;
  status: "draft" | "pending" | "published" | "archived";
  created_at: string;
  updated_at: string;
  user_id: string;
}
```

### Form Data Models
```typescript
interface CreateApplicationData {
  name: string;
  description?: string;
  template_id: string;
}

interface UpdateApplicationData {
  name?: string;
  description?: string;
  configuration?: Record<string, unknown>;
  status?: "draft" | "pending" | "published" | "archived";
}
```

### Validation Schemas
Using Zod for consistent validation:

```typescript
const createApplicationSchema = z.object({
  name: z.string().min(1).max(100),
  description: z.string().max(500).optional(),
  template_id: z.string().min(1),
});

const updateApplicationSchema = createApplicationSchema.partial();
```

## Error Handling

### Client-side Error Handling
- Form validation errors displayed inline
- API errors shown using toast notifications from UI library
- Network errors handled with retry mechanisms
- Loading states prevent duplicate submissions

### Server-side Error Handling
- Input validation using Zod schemas
- Authentication errors return 401 status
- Authorization errors return 403 status
- Not found errors return 404 status
- Server errors return 500 with generic message

### Error Recovery
- Failed operations can be retried
- Form data preserved during validation errors
- Navigation state maintained during errors
- Graceful degradation for network issues

## Testing Strategy

### Unit Testing
- Component logic testing with Deno test
- Form validation testing
- API utility function testing
- Authentication flow testing

### Integration Testing
- API endpoint testing with mock database
- Authentication integration testing
- Form submission flow testing
- Error handling scenario testing

### End-to-End Testing
- Complete user workflows with Playwright
- Cross-browser compatibility testing
- Responsive design testing
- Accessibility compliance testing

### Visual Testing
- Component visual regression testing
- Layout consistency across screen sizes
- Theme compatibility testing
- Loading state visual testing

## Performance Considerations

### Client-side Performance
- Lazy loading of non-critical components
- Optimistic UI updates for better perceived performance
- Efficient re-rendering with Preact signals
- Minimal JavaScript bundle size

### Server-side Performance
- Database query optimization
- Response caching where appropriate
- Efficient pagination for large datasets
- Connection pooling for database access

### Network Performance
- Minimal API payloads
- Compression for static assets
- CDN integration for static resources
- Progressive enhancement approach

## Security Considerations

### Authentication Security
- OAuth flow validation
- Session management
- CSRF protection
- Secure cookie handling

### Authorization Security
- User ownership validation
- Resource access control
- API endpoint protection
- Input sanitization

### Data Security
- SQL injection prevention
- XSS protection
- Secure data transmission
- Sensitive data handling

## Accessibility Features

### Keyboard Navigation
- Full keyboard accessibility
- Logical tab order
- Focus management
- Keyboard shortcuts

### Screen Reader Support
- Semantic HTML structure
- ARIA labels and descriptions
- Live region updates
- Alternative text for images

### Visual Accessibility
- High contrast support
- Scalable text
- Color-blind friendly design
- Reduced motion support

## Responsive Design

### Mobile-first Approach
- Touch-friendly interface
- Optimized for small screens
- Swipe gestures where appropriate
- Mobile-specific interactions

### Tablet Optimization
- Efficient use of screen space
- Touch and mouse input support
- Adaptive layouts
- Orientation handling

### Desktop Enhancement
- Keyboard shortcuts
- Hover states
- Multi-column layouts
- Advanced interactions