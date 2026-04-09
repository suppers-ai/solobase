# Demo Security Configuration

## Security Layers

### 1. Read-Only Database
- SQLite opened with `?mode=ro` flag
- Database driver rejects all write operations
- Returns "attempt to write a readonly database" errors

### 2. HTTP Method Blocking
```go
// ReadOnlyMiddleware blocks:
POST   (except /api/auth/login, /api/auth/logout)
PUT    (all endpoints)
PATCH  (all endpoints)
DELETE (all endpoints)
```

### 3. Security Headers
```
CSP: default-src 'self'; script-src 'self' 'unsafe-inline'
X-Frame-Options: DENY
X-Content-Type-Options: nosniff
X-XSS-Protection: 1; mode=block
Referrer-Policy: same-origin
HSTS: max-age=31536000 (production only)
```

### 4. Rate Limiting
| Endpoint | Limit | Per |
|----------|-------|-----|
| General | 60 | minute |
| /api/admin/logs | 10 | minute |
| /api/admin/database/query | 10 | minute |

## Attack Vectors Mitigated

### ✅ Protected Against:
- **SQL Injection**: Read-only database, parameterized queries
- **XSS**: CSP headers, input sanitization
- **CSRF**: Same-origin policy, token validation
- **Clickjacking**: X-Frame-Options DENY
- **DoS**: Rate limiting, auto-restart
- **Data Exfiltration**: Limited by rate limits
- **Privilege Escalation**: Read-only mode blocks all writes
- **Path Traversal**: File operations blocked
- **Resource Exhaustion**: Memory limits, auto-stop

### ⚠️ Remaining Risks:
- **Information Disclosure**: Demo data visible to all
- **Brute Force**: Rate limited but not fully prevented
- **Session Hijacking**: Use HTTPS, short session timeout
- **Timing Attacks**: Minimal impact in read-only mode

## Monitoring Indicators

### Normal Activity:
```
INFO: Read-only mode enabled
INFO: Rate limit: 60/min, remaining: 45
GET /api/users 200 OK
```

### Suspicious Activity:
```
WARN: Rate limit exceeded for IP: x.x.x.x
ERROR: Write operation blocked in read-only mode
ERROR: Database is locked (attempt to write)
```

### Attack Patterns:
```
Multiple 403 responses -> Write attempts
Multiple 429 responses -> Rate limit abuse
Repeated /database/query -> SQL injection attempts
Large request bodies -> Upload attempts
```

## Emergency Response

### If Under Attack:

1. **Immediate**: Check logs
```bash
fly logs -a solobase-demo --since 1h
```

2. **Block IP** (if severe):
```bash
# Use Fly.io firewall or Cloudflare
```

3. **Restart** (clears state):
```bash
fly apps restart solobase-demo
```

4. **Scale Down** (if needed):
```bash
fly scale count 0 -a solobase-demo
```

## Configuration Files

### middleware/readonly.go
- Controls which endpoints bypass read-only
- Add exceptions carefully

### middleware/security.go
- Security headers configuration
- Rate limit thresholds

### demo/deployment/Dockerfile
- READONLY_MODE environment variable
- Database path configuration

## Best Practices

1. **Never Include**:
   - Real user data
   - Production secrets
   - Internal documentation
   - Source code references

2. **Always Include**:
   - Demo disclaimer
   - Rate limit headers
   - Error messages that don't leak info
   - Audit logging

3. **Regular Maintenance**:
   - Review logs weekly
   - Update dependencies monthly
   - Rotate demo passwords quarterly
   - Test security headers

## Testing Security

### Verify Read-Only:
```bash
curl -X POST https://demo.fly.dev/api/users \
  -H "Content-Type: application/json" \
  -d '{"email":"test@test.com"}'
# Should return 403 Forbidden
```

### Check Rate Limiting:
```bash
for i in {1..100}; do
  curl https://demo.fly.dev/api/health
done
# Should get 429 after limit
```

### Test Security Headers:
```bash
curl -I https://demo.fly.dev
# Should show all security headers
```

## Incident Response Plan

1. **Detection**: Monitor logs, alerts
2. **Assessment**: Identify attack type
3. **Containment**: Rate limit, block IPs
4. **Eradication**: Restart, patch if needed
5. **Recovery**: Restore service
6. **Lessons**: Update security measures