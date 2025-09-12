# Webhooks Extension

A robust webhook management and delivery system for Solobase applications that enables real-time event notifications and seamless third-party integrations.

## Features

### 🪝 Complete Webhook Management
- Create, update, and delete webhooks through intuitive APIs
- Configure multiple webhooks for different events
- Enable/disable webhooks without deletion
- Organize webhooks with custom names and descriptions

### 🔐 Secure Delivery
- HMAC-SHA256 signature verification for payload authenticity
- Custom headers support for authentication
- Configurable secrets per webhook
- SSL/TLS enforcement for secure transmission

### 📊 Delivery Monitoring
- Real-time delivery status tracking
- Detailed delivery history with response codes
- Response time monitoring
- Automatic retry mechanism with exponential backoff

### 🎯 Event Filtering
- Subscribe to specific event types
- Wildcard event patterns (e.g., `user.*`, `order.*`)
- Multiple events per webhook
- Event payload customization

### 💾 Reliability Features
- Automatic retry on failure (configurable retries)
- Delivery queue management
- Idempotency keys to prevent duplicate deliveries
- Configurable timeout settings

## Installation

The webhooks extension is part of the Solobase official extensions and comes pre-installed with the framework.

## Configuration

Configure the extension in your application's config file:

```json
{
  "enabled": true,
  "maxRetries": 3,
  "retryDelay": 60,
  "timeout": 10
}
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | boolean | `true` | Enable or disable webhook delivery |
| `maxRetries` | integer | `3` | Maximum number of delivery retry attempts |
| `retryDelay` | integer | `60` | Delay between retries in seconds |
| `timeout` | integer | `10` | Request timeout in seconds |

## API Endpoints

All endpoints are prefixed with `/ext/webhooks/`

### GET `/ext/webhooks/dashboard`
Access the webhooks management dashboard

### GET `/ext/webhooks/api/webhooks`
List all configured webhooks

**Response:**
```json
{
  "webhooks": [
    {
      "id": "wh_1234567890",
      "name": "Order Notifications",
      "url": "https://api.example.com/webhooks/orders",
      "events": ["order.created", "order.updated"],
      "active": true,
      "created_at": "2024-01-15T10:00:00Z"
    }
  ],
  "total": 1
}
```

### POST `/ext/webhooks/api/webhooks/create`
Create a new webhook

**Request Body:**
```json
{
  "name": "User Notifications",
  "url": "https://api.example.com/webhooks/users",
  "events": ["user.created", "user.updated", "user.deleted"],
  "headers": {
    "Authorization": "Bearer your-api-token"
  },
  "secret": "your-webhook-secret",
  "active": true
}
```

### GET `/ext/webhooks/api/webhooks/{id}`
Get webhook details by ID

### PUT `/ext/webhooks/api/webhooks/{id}/update`
Update an existing webhook

### DELETE `/ext/webhooks/api/webhooks/{id}/delete`
Delete a webhook

### POST `/ext/webhooks/api/webhooks/{id}/test`
Send a test payload to verify webhook configuration

**Response:**
```json
{
  "success": true,
  "payload": {
    "event": "test",
    "timestamp": "2024-01-15T10:30:00Z",
    "message": "This is a test webhook"
  }
}
```

### GET `/ext/webhooks/api/webhooks/{id}/deliveries`
List delivery history for a specific webhook

**Response:**
```json
{
  "deliveries": [
    {
      "id": "del_9876543210",
      "webhook_id": "wh_1234567890",
      "event": "user.created",
      "status": 200,
      "duration": 245,
      "delivered_at": "2024-01-15T10:15:00Z"
    }
  ],
  "total": 1
}
```

## Database Schema

The extension creates tables in the `ext_webhooks` schema:

### webhooks
- `id` - UUID primary key
- `name` - Webhook name
- `url` - Target endpoint URL
- `events` - Array of subscribed events
- `headers` - Custom headers (JSONB)
- `secret` - HMAC secret for signatures
- `active` - Enable/disable flag
- `created_at` - Creation timestamp
- `updated_at` - Last update timestamp

### deliveries
- `id` - UUID primary key
- `webhook_id` - Reference to webhook
- `event` - Event name
- `payload` - Event data (JSONB)
- `status` - HTTP response code
- `response` - Response body
- `duration_ms` - Request duration in milliseconds
- `delivered_at` - Delivery timestamp

## Webhook Payload Format

All webhooks receive a standardized payload format:

```json
{
  "id": "evt_unique_id",
  "event": "user.created",
  "timestamp": "2024-01-15T10:00:00Z",
  "data": {
    // Event-specific data
  },
  "metadata": {
    "version": "1.0",
    "source": "solobase"
  }
}
```

## Request Headers

Every webhook request includes these headers:

| Header | Description |
|--------|-------------|
| `Content-Type` | Always `application/json` |
| `X-Webhook-Event` | The event type (e.g., `user.created`) |
| `X-Webhook-ID` | Unique webhook configuration ID |
| `X-Webhook-Signature` | HMAC-SHA256 signature (if secret configured) |
| `X-Webhook-Timestamp` | Request timestamp |
| `X-Webhook-Delivery-ID` | Unique delivery attempt ID |

## Signature Verification

When a secret is configured, verify the webhook signature:

```javascript
const crypto = require('crypto');

function verifyWebhookSignature(payload, signature, secret) {
  const hash = crypto
    .createHmac('sha256', secret)
    .update(payload)
    .digest('hex');
  
  return hash === signature;
}

// In your webhook handler
app.post('/webhook', (req, res) => {
  const signature = req.headers['x-webhook-signature'];
  const payload = JSON.stringify(req.body);
  
  if (!verifyWebhookSignature(payload, signature, YOUR_SECRET)) {
    return res.status(401).send('Invalid signature');
  }
  
  // Process webhook
  res.status(200).send('OK');
});
```

## Event Types

Common events that trigger webhooks:

### User Events
- `user.created` - New user registration
- `user.updated` - User profile updated
- `user.deleted` - User account deleted
- `user.verified` - Email/phone verified
- `user.login` - User logged in

### Resource Events
- `resource.created` - New resource created
- `resource.updated` - Resource modified
- `resource.deleted` - Resource removed
- `resource.published` - Resource made public
- `resource.archived` - Resource archived

### System Events
- `system.backup` - Backup completed
- `system.maintenance` - Maintenance mode changed
- `system.error` - System error occurred

## Retry Logic

Failed webhook deliveries are automatically retried with exponential backoff:

1. First retry: 1 minute after failure
2. Second retry: 5 minutes after first retry
3. Third retry: 15 minutes after second retry

You can configure the retry behavior through the extension settings.

## Best Practices

### For Webhook Providers

1. **Use HTTPS endpoints** - Always use SSL/TLS encrypted endpoints
2. **Configure secrets** - Use webhook secrets for signature verification
3. **Handle duplicates** - Use delivery IDs to prevent duplicate processing
4. **Return quickly** - Respond with 2xx status within timeout period
5. **Monitor deliveries** - Regular check delivery history for failures

### For Webhook Consumers

1. **Verify signatures** - Always validate webhook signatures
2. **Respond quickly** - Process asynchronously if needed
3. **Return proper status codes** - Use 2xx for success, 4xx for client errors
4. **Log everything** - Keep detailed logs for debugging
5. **Handle retries** - Be prepared for duplicate deliveries

## Use Cases

### E-commerce Integration
- Send order notifications to fulfillment systems
- Update inventory in external systems
- Trigger email campaigns on purchase events
- Sync customer data with CRM

### DevOps Automation
- Trigger CI/CD pipelines on code events
- Send alerts to monitoring systems
- Update issue trackers
- Notify team chat channels

### Data Synchronization
- Keep external databases in sync
- Update search indices in real-time
- Propagate changes to cache systems
- Mirror data to analytics platforms

### Communication
- Send SMS/email notifications
- Update live dashboards
- Trigger push notifications
- Post to social media

## Troubleshooting

### Webhook Not Firing
1. Check if webhook is active
2. Verify event name matches exactly
3. Ensure extension is enabled
4. Check system logs for errors

### Delivery Failures
1. Verify endpoint URL is correct
2. Check if endpoint is accessible
3. Validate SSL certificates
4. Review timeout settings
5. Check response status codes

### Signature Verification Failures
1. Ensure secret matches on both sides
2. Verify signature algorithm (SHA256)
3. Check payload hasn't been modified
4. Validate header name spelling

## Performance Considerations

- Webhooks are delivered asynchronously
- Delivery queue is processed in parallel
- Failed deliveries don't block new events
- Database indices optimize query performance
- Old delivery records are automatically purged

## Security

- All webhook URLs must use HTTPS in production
- Secrets are stored encrypted in database
- Delivery payloads are sanitized
- Rate limiting prevents webhook flooding
- IP whitelisting available for additional security

## Requirements

- Solobase v1.0.0 or higher
- PostgreSQL or SQLite database
- Network access to webhook endpoints

## License

MIT - Part of the Solobase Official Extensions

## Support

For issues, feature requests, or contributions, visit: https://github.com/suppers-ai/solobase