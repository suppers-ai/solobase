# Admin Package Design Document

## Overview

The Admin Package is a comprehensive dashboard application built with Fresh framework that provides administrative functionality for the Suppers platform. It follows the established architecture patterns used in other packages (profile, store, docs) while providing specialized admin-only features for platform management.

The package will be structured as a standalone Fresh application with server-side rendering, client-side interactivity through islands, and integration with the existing Supabase backend and shared component library.

## Architecture

### Package Structure
```
packages/admin/
├── main.ts                    # Application entry point
├── dev.ts                     # Development server
├── deno.json                  # Package configuration
├── routes/                    # File-based routing
│   ├── index.tsx             # Dashboard overview
│   ├── applications/         # Application management routes
│   ├── users/                # User management routes
│   └── subscriptions/        # Subscription management routes
├── islands/                   # Client-side interactive components
│   ├── AdminDashboardIsland.tsx
│   ├── AdminSidebarIsland.tsx
│   ├── ApplicationManagementIsland.tsx
│   ├── UserManagementIsland.tsx
│   └── SubscriptionManagementIsland.tsx
├── components/               # Server-side components
│   ├── AdminLayout.tsx       # Main admin layout wrapper
│   └── AdminGuard.tsx        # Authentication/authorization guard
├── lib/                      # Utilities and API clients
│   ├── auth.ts              # Admin authentication utilities
│   ├── api-client.ts        # API client for admin operations
│   └── permissions.ts       # Permission checking utilities
├── static/                   # Static assets
│   └── styles.css           # Admin-specific styles
└── types/                    # Admin-specific types
    └── admin.ts             # Admin dashboard types
```

### Technology Stack
- **Framework**: Fresh 2.0 with file-based routing
- **Frontend**: Preact with JSX for islands
- **Styling**: Tailwind CSS + DaisyUI components
- **Database**: Supabase PostgreSQL with existing schema
- **Authentication**: Supabase Auth with admin role checking
- **State Management**: Preact signals for client-side state
- **API Integration**: Supabase client with admin service role

## Components and Interfaces

### Core Components

#### AdminLayout
A wrapper component that provides the consistent admin interface structure:
- Responsive sidebar navigation
- Header with user info and logout
- Main content area
- Toast notifications for feedback

#### AdminSidebarIsland
Client-side interactive sidebar with:
- Navigation menu (Dashboard, Applications, Users, Subscriptions)
- Active route highlighting
- Collapsible mobile view
- User profile section

#### AdminDashboardIsland
Main dashboard overview displaying:
- Key metrics cards (total applications, users, revenue)
- Recent activity feed
- Quick action buttons
- Charts for usage trends (using Chart.js or similar)

#### ApplicationManagementIsland
Application management interface:
- Searchable/filterable application list
- Application creation modal
- Application editing capabilities
- Status management (draft, pending, published, archived)
- Bulk operations

#### UserManagementIsland
User management interface:
- User list with search and filtering
- User detail views
- Role management
- Account status controls
- Activity monitoring

#### SubscriptionManagementIsland
Subscription management interface:
- Subscription plan list
- Plan creation and editing
- Pricing configuration
- Feature management
- Active subscriber tracking

### API Integration

#### Admin API Client
Centralized API client with methods for:
- Dashboard metrics retrieval
- Application CRUD operations
- User management operations
- Subscription management
- Analytics data fetching

#### Authentication & Authorization
- Admin role verification using existing `user_role` enum
- Route-level protection with AdminGuard component
- Session management with Supabase Auth
- Automatic redirect for non-admin users

## Data Models

### Dashboard Metrics
```typescript
interface DashboardMetrics {
  totalApplications: number;
  totalUsers: number;
  totalRevenue: number;
  monthlyActiveUsers: number;
  applicationsByStatus: {
    draft: number;
    pending: number;
    published: number;
    archived: number;
  };
}
```

### Extended Application Management
```typescript
interface AdminApplication extends Application {
  owner: {
    id: string;
    email: string;
    displayName: string;
  };
  metrics: {
    views: number;
    lastAccessed: string;
    storageUsed: number;
  };
  reviews: ApplicationReview[];
}
```

### User Management
```typescript
interface AdminUser {
  id: string;
  email: string;
  firstName?: string;
  lastName?: string;
  displayName?: string;
  role: 'user' | 'admin';
  storageUsed: number;
  storageLimit: number;
  bandwidthUsed: number;
  bandwidthLimit: number;
  createdAt: string;
  lastLoginAt?: string;
  applications: Application[];
  subscriptionStatus?: string;
}
```

### Subscription Management
```typescript
interface SubscriptionPlan {
  id: string;
  name: string;
  description: string;
  price: number;
  currency: string;
  interval: 'month' | 'year';
  features: SubscriptionFeature[];
  limits: {
    applications: number;
    storage: number; // in bytes
    bandwidth: number; // in bytes per month
  };
  isActive: boolean;
  subscriberCount: number;
  createdAt: string;
  updatedAt: string;
}

interface SubscriptionFeature {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
}
```

## Error Handling

### Client-Side Error Handling
- Toast notifications for user feedback
- Error boundaries for component-level error catching
- Graceful degradation for failed API calls
- Loading states for all async operations

### Server-Side Error Handling
- Proper HTTP status codes
- Structured error responses
- Logging for debugging and monitoring
- Fallback pages for critical errors

### Authentication Errors
- Automatic redirect to login for unauthenticated users
- Clear messaging for insufficient permissions
- Session timeout handling
- Secure error messages (no sensitive data exposure)

## Testing Strategy

### Unit Testing
- Component testing with Deno test
- API client testing with mocked responses
- Utility function testing
- Permission checking logic testing

### Integration Testing
- Route testing with Fresh testing utilities
- Database integration testing
- Authentication flow testing
- API endpoint testing

### E2E Testing
- Admin workflow testing with Playwright
- Cross-browser compatibility testing
- Mobile responsiveness testing
- Performance testing for large datasets

## Security Considerations

### Authentication & Authorization
- Server-side admin role verification on all routes
- Client-side role checking for UI elements
- Secure session management
- CSRF protection through Supabase

### Data Access
- Row-level security policies in database
- Admin-specific API endpoints with proper authorization
- Audit logging for sensitive operations
- Rate limiting for API endpoints

### Input Validation
- Zod schemas for all form inputs
- Server-side validation for all operations
- XSS prevention through proper escaping
- SQL injection prevention through parameterized queries

## Performance Optimization

### Client-Side Performance
- Code splitting for admin-specific functionality
- Lazy loading of heavy components
- Efficient state management with signals
- Optimized bundle size

### Server-Side Performance
- Database query optimization
- Caching for frequently accessed data
- Pagination for large datasets
- Connection pooling for database access

### User Experience
- Progressive loading for dashboard metrics
- Skeleton screens during data loading
- Optimistic updates for quick feedback
- Responsive design for all screen sizes

## Integration Points

### Existing Packages
- **Shared Package**: Common types, utilities, and constants
- **UI Library**: Reusable components and styling
- **Auth Client**: Authentication utilities
- **API Package**: Backend services and database access

### External Services
- **Supabase**: Database, authentication, and real-time features
- **Stripe**: Payment processing for subscriptions (future)
- **Analytics**: Usage tracking and reporting (future)

### Database Schema Extensions
The admin package will utilize existing database tables and may require additional tables for:
- Subscription plans and features
- Admin activity logs
- System configuration settings
- Usage analytics and metrics

## Deployment Considerations

### Environment Configuration
- Admin-specific environment variables
- Database connection with elevated permissions
- Secure API keys and secrets management
- Production vs development configuration

### Monitoring & Logging
- Admin action logging for audit trails
- Performance monitoring for dashboard queries
- Error tracking and alerting
- Usage analytics for admin features

### Scalability
- Efficient database queries for large datasets
- Caching strategies for frequently accessed data
- Horizontal scaling considerations
- Resource usage monitoring