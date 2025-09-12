# Analytics Extension

A comprehensive analytics and tracking system for Solobase applications that provides real-time insights into user behavior, page views, and custom events.

## Features

### 📊 Real-Time Analytics Dashboard
- Beautiful, responsive dashboard accessible at `/ext/analytics/dashboard`
- Live statistics including total page views, unique visitors, session duration, and bounce rate
- Visual charts showing page view trends over time
- Top pages report showing your most visited content

### 🔍 Automatic Page Tracking
- Zero-configuration page view tracking via middleware
- Captures essential metrics: URL, referrer, user agent, IP address
- Session tracking with anonymous session identifiers
- Respects user privacy with configurable data retention

### 📈 Custom Event Tracking
- Track any custom event via simple API endpoints
- Store arbitrary JSON data with events
- Perfect for tracking user actions, conversions, or feature usage
- Automatic user association when authenticated

### 🔐 Security & Permissions
- Role-based access control with two permission levels:
  - `analytics.view` - Read-only access to analytics data
  - `analytics.admin` - Full administrative access
- All data stored in isolated `ext_analytics` schema
- Row-level security policies for data protection

### ⚙️ Flexible Configuration
- Enable/disable tracking on demand
- Exclude specific paths from tracking (e.g., `/api/`, `/admin/`)
- Configurable data retention periods (1-365 days)
- JSON-based configuration with validation

## Installation

The analytics extension is part of the Solobase community extensions and can be enabled through your application's extension configuration.

## Configuration

Configure the extension in your application's config file:

```json
{
  "enabled": true,
  "excludePaths": ["/api/", "/ext/", "/admin/"],
  "retentionDays": 90
}
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | boolean | `true` | Enable or disable analytics tracking |
| `excludePaths` | string[] | `["/api/", "/ext/"]` | URL paths to exclude from tracking |
| `retentionDays` | integer | `90` | Number of days to retain analytics data (1-365) |

## API Endpoints

All endpoints are prefixed with `/ext/analytics/`

### GET `/ext/analytics/dashboard`
Access the analytics dashboard UI

### POST `/ext/analytics/api/track`
Track custom events

**Request Body:**
```json
{
  "event": "button_click",
  "properties": {
    "button_id": "signup",
    "page": "/home"
  }
}
```

### GET `/ext/analytics/api/pageviews`
Retrieve page view statistics (last 7 days, top 10 pages)

**Response:**
```json
{
  "pageViews": [
    {
      "url": "/products",
      "views": 1234
    }
  ]
}
```

### GET `/ext/analytics/api/stats`
Get aggregated analytics statistics

**Response:**
```json
{
  "totalViews": 5000,
  "uniqueUsers": 1200,
  "lastUpdated": "2024-01-15T10:30:00Z"
}
```

## Database Schema

The extension creates two main tables in the `ext_analytics` schema:

### page_views
- `id` - Unique identifier
- `user_id` - Associated user (if authenticated)
- `session_id` - Anonymous session identifier
- `page_url` - Visited page URL
- `referrer` - Referring page
- `user_agent` - Browser user agent
- `ip_address` - Client IP address
- `created_at` - Timestamp

### events
- `id` - Unique identifier
- `user_id` - Associated user (if authenticated)
- `event_name` - Name of the custom event
- `event_data` - JSON data associated with the event
- `created_at` - Timestamp

## Middleware & Hooks

### Page Tracking Middleware
- **Priority:** 100
- **Function:** Automatically tracks all page views
- **Behavior:** Runs asynchronously to avoid impacting page load times

### Post-Authentication Hook
- **Priority:** 50
- **Function:** Tracks user login events
- **Behavior:** Creates a "login" event when users authenticate

## Use Cases

### E-commerce
- Track product views and purchases
- Monitor cart abandonment rates
- Analyze user shopping patterns
- Measure conversion funnels

### SaaS Applications
- Monitor feature adoption
- Track user engagement metrics
- Analyze usage patterns
- Identify power users

### Content Websites
- Track article/page popularity
- Analyze reading patterns
- Monitor content performance
- Understand traffic sources

### Marketing
- Campaign performance tracking
- A/B testing metrics
- User journey analysis
- ROI measurement

## Privacy Considerations

- No personally identifiable information (PII) is collected by default
- IP addresses can be anonymized if required
- Configurable data retention periods
- Respects Do Not Track headers (when configured)
- GDPR-compliant data handling

## Performance

- Asynchronous tracking to prevent blocking
- Indexed database tables for fast queries
- Efficient data aggregation
- Minimal overhead on page loads
- Automatic old data cleanup based on retention settings

## Why Use This Extension?

1. **Zero Configuration** - Works out of the box with sensible defaults
2. **Privacy-First** - Designed with user privacy in mind
3. **Lightweight** - Minimal performance impact on your application
4. **Comprehensive** - Tracks both automatic and custom events
5. **Actionable Insights** - Provides data that helps make informed decisions
6. **Self-Hosted** - Keep your analytics data in your own database
7. **Extensible** - Easy to add custom tracking and reports
8. **No External Dependencies** - No third-party services required

## Requirements

- Solobase v1.0.0 or higher
- PostgreSQL or SQLite database
- Appropriate database permissions for schema creation

## License

MIT - Part of the Solobase Community Extensions

## Support

For issues, feature requests, or contributions, visit: https://github.com/suppers-ai/solobase