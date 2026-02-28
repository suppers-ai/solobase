#!/bin/bash

# Script to create a pre-populated demo database for read-only deployment
# Run this from the solobase-demo directory

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Setting up demo database..."

# Check that the solobase repo exists alongside this repo
SOLOBASE_DIR="${SOLOBASE_DIR:-$(cd "$SCRIPT_DIR/.." && pwd)/solobase}"
if [ ! -f "$SOLOBASE_DIR/go.mod" ]; then
    echo "Error: Cannot find solobase repo at $SOLOBASE_DIR"
    echo "Set SOLOBASE_DIR to the path of your solobase checkout"
    exit 1
fi

# Remove old demo database if it exists
rm -f "$SCRIPT_DIR/demo.db"

# Create a temporary environment without readonly mode to populate the database
export READONLY_MODE=false
export DATABASE_URL="file:$SCRIPT_DIR/demo.db"
export DEFAULT_ADMIN_EMAIL=admin@example.com
export DEFAULT_ADMIN_PASSWORD=admin123
export JWT_SECRET=$(openssl rand -hex 32)

echo "JWT Secret set $JWT_SECRET"

# Build and run solobase to create the database
echo "Building solobase..."
(cd "$SOLOBASE_DIR" && go build -o "$SCRIPT_DIR/solobase-temp" ./cmd/solobase)

echo "Creating database and accounts..."
# Start solobase in background
"$SCRIPT_DIR/solobase-temp" &
SOLOBASE_PID=$!

# Wait for server to start
echo "Waiting for server to start..."
sleep 5

# Create some demo data using the API
echo "Creating demo data..."

# Login as admin (token is now returned in Set-Cookie header, not response body)
ADMIN_TOKEN=$(curl -s -i -X POST http://localhost:8090/api/auth/login \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"${DEFAULT_ADMIN_EMAIL}\",\"password\":\"${DEFAULT_ADMIN_PASSWORD}\"}" \
  | grep -i 'set-cookie: auth_token=' | sed 's/.*auth_token=\([^;]*\).*/\1/')

if [ -z "$ADMIN_TOKEN" ]; then
  echo "Failed to login as admin"
  kill $SOLOBASE_PID
  exit 1
fi

echo "Admin token obtained"

# Create some sample settings (PATCH for bulk update)
echo "Configuring demo settings..."
curl -X PATCH http://localhost:8090/api/admin/settings \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "app_name": "Solobase Demo",
    "notification": "Welcome to the Solobase demo! This is a read-only demo environment. You can explore all features but cannot modify any data.",
    "max_file_size": 10485760,
    "allowed_file_types": [".jpg",".png",".pdf",".txt",".md"],
    "enable_user_registration": false
  }'

# Create some sample buckets
echo "Creating sample storage buckets..."
curl -X POST http://localhost:8090/api/storage/buckets \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "documents", "public": false}'

curl -X POST http://localhost:8090/api/storage/buckets \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "images", "public": true}'

# Stop the server
echo "Stopping server..."
kill $SOLOBASE_PID 2>/dev/null || true
wait $SOLOBASE_PID 2>/dev/null || true

# Clean up
rm -f "$SCRIPT_DIR/solobase-temp"

echo "Demo database created at $SCRIPT_DIR/demo.db"
echo ""
echo "The database contains:"
echo "  - Admin account: admin@example.com / admin123"
echo "  - Demo settings and notification"
echo "  - Sample storage buckets"
echo ""
echo "To deploy:"
echo "  ./deploy-demo.sh"
