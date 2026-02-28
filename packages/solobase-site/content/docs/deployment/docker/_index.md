---
title: "Docker Deployment"
description: "Deploy Solobase using Docker and Docker Compose"
weight: 10
tags: ["deployment", "docker", "containers", "production"]
---

# Docker Deployment

Docker provides the easiest way to deploy Solobase in production. This guide covers single-container deployment, multi-container setups with Docker Compose, and production best practices.

## Quick Start with Docker

### Single Container

Run Solobase with SQLite (good for testing):

```bash
docker run -d \
  --name solobase \
  -p 8090:8090 \
  -v solobase_data:/data \
  solobase/solobase
```

### With PostgreSQL

```bash
docker run -d \
  --name solobase \
  -p 8090:8090 \
  -v solobase_storage:/data/storage \
  -e DB_TYPE=postgres \
  -e DATABASE_URL="postgres://user:pass@db:5432/solobase" \
  -e JWT_SECRET="your-jwt-secret-here" \
  solobase/solobase
```

## Docker Compose Deployment

### Basic Setup

Create a `docker-compose.yml` file:

```yaml
version: '3.8'

services:
  solobase:
    image: solobase/solobase:latest
    ports:
      - "8090:8090"
    environment:
      - BIND_ADDR=0.0.0.0:8090
      - DB_TYPE=sqlite
      - JWT_SECRET=your-super-secret-jwt-key
    volumes:
      - solobase_data:/data
    restart: unless-stopped

volumes:
  solobase_data:
```

Run with:

```bash
docker-compose up -d
```

### Production Setup with PostgreSQL

```yaml
version: '3.8'

services:
  solobase:
    image: solobase/solobase:latest
    ports:
      - "8090:8090"
    environment:
      - BIND_ADDR=0.0.0.0:8090
      - DB_TYPE=postgres
      - DATABASE_URL=postgres://solobase:${DB_PASSWORD}@db:5432/solobase
      - STORAGE_TYPE=s3
      - S3_BUCKET=${S3_BUCKET}
      - S3_REGION=${S3_REGION}
      - JWT_SECRET=${JWT_SECRET}
    depends_on:
      - db
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8090/api/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  db:
    image: postgres:15-alpine
    environment:
      - POSTGRES_DB=solobase
      - POSTGRES_USER=solobase
      - POSTGRES_PASSWORD=${DB_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U solobase"]
      interval: 10s
      timeout: 5s
      retries: 5

volumes:
  postgres_data:
```

Create a `.env` file:

```bash
# .env
DB_PASSWORD=secure_database_password
JWT_SECRET=your-256-bit-jwt-secret-key
S3_BUCKET=solobase-files
S3_REGION=us-east-1
```

### With Reverse Proxy (Nginx)

Add Nginx for SSL termination:

```yaml
version: '3.8'

services:
  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/nginx/ssl:ro
    depends_on:
      - solobase
    restart: unless-stopped

  solobase:
    image: solobase/solobase:latest
    expose:
      - "8090"
    environment:
      - BIND_ADDR=0.0.0.0:8090
      - DB_TYPE=postgres
      - DATABASE_URL=postgres://solobase:${DB_PASSWORD}@db:5432/solobase
      - JWT_SECRET=${JWT_SECRET}
    depends_on:
      - db
    restart: unless-stopped

  db:
    image: postgres:15-alpine
    environment:
      - POSTGRES_DB=solobase
      - POSTGRES_USER=solobase
      - POSTGRES_PASSWORD=${DB_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
    restart: unless-stopped

volumes:
  postgres_data:
```

Nginx configuration (`nginx.conf`):

```nginx
events {
    worker_connections 1024;
}

http {
    upstream solobase {
        server solobase:8090;
    }

    server {
        listen 80;
        server_name solobase.example.com;
        return 301 https://$server_name$request_uri;
    }

    server {
        listen 443 ssl http2;
        server_name solobase.example.com;

        ssl_certificate /etc/nginx/ssl/cert.pem;
        ssl_certificate_key /etc/nginx/ssl/key.pem;
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_ciphers HIGH:!aNULL:!MD5;

        client_max_body_size 100M;

        location / {
            proxy_pass http://solobase;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }
    }
}
```

## Environment Variables

All Solobase configuration can be set via environment variables. See the [Configuration guide](/docs/configuration/) for the full reference.

### Common Variables

```bash
# Server
BIND_ADDR=0.0.0.0:8090

# Database
DB_TYPE=postgres                    # "sqlite" or "postgres"
DATABASE_URL=postgres://user:pass@host:port/dbname
DB_PATH=data/solobase.db           # SQLite path

# Storage
STORAGE_TYPE=s3                     # "local" or "s3"
STORAGE_ROOT=data/storage           # Local storage path
S3_BUCKET=my-bucket
S3_REGION=us-east-1
S3_ENDPOINT=https://custom.endpoint # For MinIO/B2/R2

# Auth
JWT_SECRET=your-secret-key
```

## Custom Docker Image

Create a custom image with your `solobase.toml` baked in:

```dockerfile
FROM solobase/solobase:latest

COPY solobase.toml /etc/solobase/solobase.toml
ENV SOLOBASE_CONFIG=/etc/solobase/solobase.toml

EXPOSE 8090

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8090/api/health || exit 1
```

Build and run:

```bash
docker build -t my-solobase:latest .
docker run -d \
  --name my-solobase \
  -p 8090:8090 \
  -v solobase_data:/data \
  my-solobase:latest
```

## Production Best Practices

### 1. Resource Limits

```yaml
services:
  solobase:
    image: solobase/solobase:latest
    deploy:
      resources:
        limits:
          memory: 1G
          cpus: '1.0'
        reservations:
          memory: 512M
          cpus: '0.5'
```

### 2. Health Checks

```yaml
services:
  solobase:
    image: solobase/solobase:latest
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8090/api/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 10s
```

### 3. Logging

```yaml
services:
  solobase:
    image: solobase/solobase:latest
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
```

### 4. Security Hardening

```yaml
services:
  solobase:
    image: solobase/solobase:latest
    read_only: true
    tmpfs:
      - /tmp
    volumes:
      - solobase_data:/data
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    cap_add:
      - NET_BIND_SERVICE
```

## Backup and Recovery

### Database Backups

Automated PostgreSQL backups:

```yaml
services:
  backup:
    image: postgres:15-alpine
    environment:
      - PGPASSWORD=${DB_PASSWORD}
    volumes:
      - ./backups:/backups
    command: |
      sh -c '
      while true; do
        pg_dump -h db -U solobase solobase > /backups/backup_$$(date +%Y%m%d_%H%M%S).sql
        find /backups -name "backup_*.sql" -mtime +7 -delete
        sleep 86400
      done'
    depends_on:
      - db
```

## Troubleshooting

### Container Won't Start

```bash
docker logs solobase
docker logs -f solobase
docker ps -a
```

### Database Connection Issues

```bash
# Test database connectivity from the solobase container
docker exec -it solobase sh -c 'curl -s http://localhost:8090/api/health'
```

### Performance Issues

```bash
docker stats
docker system df
```

## Next Steps

- [Solobase Cloud](/docs/cloud/) - Managed hosting, no Docker needed
- [Configuration](/docs/configuration/) - Full TOML configuration reference
- [WASM Blocks](/docs/wasm/) - Extend Solobase with WebAssembly
