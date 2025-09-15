#!/bin/bash

# Script to create a pre-populated demo database for read-only deployment

echo "Setting up demo database..."

# Check if we're in the right directory
if [ ! -f "go.mod" ]; then
    echo "Error: Must run this script from the Solobase root directory"
    exit 1
fi

# Remove old demo database if it exists
rm -f demo/deployment/demo.db

# Create a temporary environment without readonly mode to populate the database
export READONLY_MODE=false
export DATABASE_URL=file:./demo/deployment/demo.db
export DEFAULT_ADMIN_EMAIL=admin@example.com
export DEFAULT_ADMIN_PASSWORD=admin123

# Build and run solobase to create the database
echo "Building solobase..."
go build -o solobase-temp ./cmd/solobase

echo "Creating database and accounts..."
# Start solobase in background
./solobase-temp &
SOLOBASE_PID=$!

# Wait for server to start
echo "Waiting for server to start..."
sleep 5

# Create some demo data using the API
echo "Creating demo data..."

# Login as admin
ADMIN_TOKEN=$(curl -s -X POST http://localhost:8090/api/auth/login \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"${DEFAULT_ADMIN_EMAIL}\",\"password\":\"${DEFAULT_ADMIN_PASSWORD}\"}" \
  | grep -o '"token":"[^"]*' | cut -d'"' -f4)

if [ -z "$ADMIN_TOKEN" ]; then
  echo "Failed to login as admin"
  kill $SOLOBASE_PID
  exit 1
fi

echo "Admin token obtained"

# Create some sample settings
echo "Configuring demo settings..."
curl -X POST http://localhost:8090/api/admin/settings \
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
kill $SOLOBASE_PID
wait $SOLOBASE_PID 2>/dev/null

# Clean up
rm -f solobase-temp

echo "Demo database created at demo/deployment/demo.db"
echo ""
echo "The database contains:"
echo "  - Admin account: admin@example.com / admin123"
echo "  - Demo settings and notification"
echo "  - Sample storage buckets"
echo ""
echo "To deploy:"
echo "  cd demo/deployment"
echo "  fly deploy --app solobase-demo"