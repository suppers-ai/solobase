# Deploying Solobase Demo

## Quick Deploy (Using Script)

The easiest way is to use the deployment script:

```bash
cd ~/Programs/suppers-ai/solobase
./deploy-demo.sh

# Or specify a custom app name
./deploy-demo.sh my-custom-app-name

# To save credentials to a file
SAVE_CREDS=true ./deploy-demo.sh
```

## Manual Deploy (Copy & Paste)

Or run these commands directly:

```bash
cd ~/Programs/suppers-ai/solobase

# Set app name and create it
APP_NAME="solobase-demo-$(date +%s)"
fly apps create $APP_NAME

# Generate and set secure passwords
ADMIN_PASSWORD=$(openssl rand -base64 24 | tr -d "=+/" | cut -c1-24)
fly secrets set ADMIN_PASSWORD="$ADMIN_PASSWORD" JWT_SECRET="$(openssl rand -base64 32)" -a $APP_NAME

# Deploy
fly deploy --app $APP_NAME --dockerfile demo/deployment/Dockerfile --config demo/deployment/fly.toml

# Show credentials
echo "======================================"
echo "Deployment Complete!"
echo "URL: https://$APP_NAME.fly.dev"
echo "Admin: admin@solobase.demo"
echo "Password: $ADMIN_PASSWORD"
echo "======================================"
```

## Step-by-Step Deployment

### 1. Prerequisites

```bash
# Navigate to root directory
cd ~/Programs/suppers-ai/solobase

# Login to Fly.io (if not already logged in)
fly auth login
```

### 2. Create App and Deploy

```bash
# Set your app name
export APP_NAME="solobase-demo-$(date +%s)"

# Create the app
fly apps create $APP_NAME

# Generate secure passwords
export ADMIN_PASSWORD=$(openssl rand -base64 24 | tr -d "=+/" | cut -c1-24)
export JWT_SECRET=$(openssl rand -base64 32)

# Set secrets
fly secrets set \
  ADMIN_PASSWORD="$ADMIN_PASSWORD" \
  JWT_SECRET="$JWT_SECRET" \
  -a $APP_NAME

# Deploy
fly deploy \
  --app $APP_NAME \
  --dockerfile demo/deployment/Dockerfile \
  --config demo/deployment/fly.toml

# Display credentials
echo "Admin Password: $ADMIN_PASSWORD"
```

### 3. Access Your Demo

```bash
# Open in browser
fly open -a $APP_NAME

# View logs to see viewer account creation
fly logs -a $APP_NAME

# Check status
fly status -a $APP_NAME
```

## Docker Local Testing

Before deploying to Fly.io, you can test locally:

```bash
# Build the image
docker build -f demo/deployment/Dockerfile -t solobase-demo .

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

If you get this error, either:
1. Specify the app name: `fly deploy -a YOUR_APP_NAME ...`
2. Or add it to fly.toml: `app = 'your-app-name'`

### Build Fails

Make sure you're in the root directory and all files exist:
```bash
ls -la demo/deployment/
# Should show: Dockerfile, fly.toml, init-viewer.sh
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

Ensure it's running:
```bash
fly status -a YOUR_APP_NAME
```

If machines are stopped:
```bash
fly machine start -a YOUR_APP_NAME
```

## Cleanup

To destroy the demo:
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