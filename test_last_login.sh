#!/bin/bash

echo "Testing last_login functionality..."

# Start the server in background
cd /home/joris/Programs/suppers-ai/solobase
./solobase &
SERVER_PID=$!

# Wait for server to start
sleep 3

# Test login endpoint
echo "Testing login..."
LOGIN_RESPONSE=$(curl -s -X POST http://localhost:8090/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@example.com","password":"admin123"}')

echo "Login response: $LOGIN_RESPONSE"

# Extract token
TOKEN=$(echo $LOGIN_RESPONSE | grep -o '"token":"[^"]*' | cut -d'"' -f4)

if [ -n "$TOKEN" ]; then
  echo "Login successful, token received"

  # Get users to check last_login
  echo "Fetching users..."
  USERS_RESPONSE=$(curl -s -X GET http://localhost:8090/api/admin/users \
    -H "Authorization: Bearer $TOKEN")

  echo "Users response: $USERS_RESPONSE"
else
  echo "Login failed"
fi

# Kill the server
kill $SERVER_PID

echo "Test complete"