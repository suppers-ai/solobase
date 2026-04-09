# Deploying Solobase Demo

## Quick Deploy (Using Script)

```bash
cd solobase-demo
./deploy-demo.sh

# Or specify a custom solobase repo location
SOLOBASE_DIR=/path/to/solobase ./deploy-demo.sh
```

## Manual Deploy (Copy & Paste)

```bash
cd solobase-demo

# Set app name and create it
APP_NAME="solobase-demo-$(date +%s)"
fly apps create $APP_NAME

# Generate and set secure passwords
ADMIN_PASSWORD=$(openssl rand -base64 24 | tr -d "=+/" | cut -c1-24)
fly secrets set ADMIN_PASSWORD="$ADMIN_PASSWORD" JWT_SECRET="$(openssl rand -base64 32)" -a $APP_NAME

# Create the demo database
./setup-demo-db.sh

# Copy demo.db into solobase repo for Docker build context
cp demo.db ../solobase/demo.db

# Deploy (build context is the solobase repo)
fly deploy --app $APP_NAME --dockerfile Dockerfile --config fly.toml ../solobase

# Clean up
rm ../solobase/demo.db

# Show credentials
echo "======================================"
echo "Deployment Complete!"
echo "URL: https://$APP_NAME.fly.dev"
echo "Admin: admin@solobase.demo"
echo "Password: $ADMIN_PASSWORD"
echo "======================================"
```

## Prerequisites

- [Fly.io CLI](https://fly.io/docs/hands-on/install-flyctl/) installed and authenticated
- The `solobase` repo checked out alongside this repo (or set `SOLOBASE_DIR`)
- Go toolchain installed (for building the demo database)

Expected workspace layout:
```
workspace/
├── solobase/          # Solobase BaaS source
├── solobase-demo/     # This repo
└── ...
```

## Docker Local Testing

Before deploying to Fly.io, you can test locally:

```bash
# Create the demo database first
./setup-demo-db.sh

# Copy demo.db into solobase for build context
cp demo.db ../solobase/demo.db

# Build the image
docker build -f Dockerfile -t solobase-demo ../solobase

# Clean up
rm ../solobase/demo.db

# Run locally
docker run -p 8080:8080 \
  -e ADMIN_PASSWORD="TestPassword123!" \
  solobase-demo

# Access at http://localhost:8080
```

## Credentials

After deployment, you'll have two accounts:

### Admin Account
- **Email**: admin@solobase.demo
- **Password**: Check deployment logs or set via ADMIN_PASSWORD

### Viewer Account (Auto-created)
- **Email**: viewer@solobase.demo
- **Password**: demo123
- **Role**: admin_viewer (read-only)

## Troubleshooting

### "Missing app name" Error

Either:
1. Specify the app name: `fly deploy -a YOUR_APP_NAME ...`
2. Or add it to fly.toml: `app = 'your-app-name'`

### Build Fails

Make sure solobase repo is available and demo.db exists:
```bash
ls -la demo.db
ls -la ../solobase/go.mod
```

### App Won't Start

Check the logs:
```bash
fly logs -a YOUR_APP_NAME
```

Common issues:
- Port mismatch (should be 8080)
- Missing environment variables
- Database initialization errors

### Can't Access the App

```bash
fly status -a YOUR_APP_NAME

# If machines are stopped:
fly machine start -a YOUR_APP_NAME
```

## Cleanup

```bash
fly apps destroy YOUR_APP_NAME
```

## Environment Variables

You can customize the deployment with these environment variables:

```bash
fly secrets set \
  DEFAULT_ADMIN_EMAIL="admin@example.com" \
  VIEWER_EMAIL="demo@example.com" \
  VIEWER_PASSWORD="custom123" \
  JWT_SECRET="your-secret-key-minimum-32-chars" \
  -a YOUR_APP_NAME
```

## Cost Optimization

The demo is configured to:
- Auto-stop when idle (`auto_stop_machines = 'stop'`)
- Use minimal resources (256MB RAM, 1 shared CPU)
- Run 0-1 machines (scales to zero)

This keeps costs minimal for demo deployments.
