#!/bin/bash

echo "Custom Tables API Test Script"
echo "=============================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

API_BASE="http://localhost:8090/api"

# First, let's try to login (we'll need to create an admin user first)
echo -e "${BLUE}1. Attempting to login...${NC}"

# Try with default admin credentials (if they exist)
LOGIN_RESPONSE=$(curl -s -X POST "$API_BASE/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "admin123"
  }')

TOKEN=$(echo $LOGIN_RESPONSE | python3 -c "import sys, json; data = json.load(sys.stdin); print(data.get('token', ''))" 2>/dev/null)

if [ -z "$TOKEN" ]; then
  echo -e "${RED}Failed to login. Please ensure an admin user exists.${NC}"
  echo "Response: $LOGIN_RESPONSE"
  echo ""
  echo "To create an admin user, you can:"
  echo "1. Set DEFAULT_ADMIN_EMAIL and DEFAULT_ADMIN_PASSWORD environment variables before starting the server"
  echo "2. Or use the signup endpoint (if enabled)"
  exit 1
fi

echo -e "${GREEN}Successfully logged in!${NC}"
echo ""

# Function to make authenticated requests
auth_request() {
  local method=$1
  local endpoint=$2
  local data=$3

  if [ -z "$data" ]; then
    curl -s -X "$method" "$API_BASE$endpoint" \
      -H "Authorization: Bearer $TOKEN" \
      -H "Content-Type: application/json"
  else
    curl -s -X "$method" "$API_BASE$endpoint" \
      -H "Authorization: Bearer $TOKEN" \
      -H "Content-Type: application/json" \
      -d "$data"
  fi
}

# 2. List existing custom tables
echo -e "${BLUE}2. Listing existing custom tables...${NC}"
auth_request GET "/admin/custom-tables" | python3 -m json.tool
echo ""

# 3. Create a new custom table
echo -e "${BLUE}3. Creating a new custom table 'customer_feedback'...${NC}"
CREATE_RESPONSE=$(auth_request POST "/admin/custom-tables" '{
  "name": "customer_feedback",
  "description": "Customer feedback and ratings",
  "fields": [
    {
      "name": "customer_name",
      "type": "string",
      "size": 100,
      "nullable": false,
      "description": "Name of the customer"
    },
    {
      "name": "email",
      "type": "string",
      "size": 255,
      "nullable": false,
      "is_unique": true,
      "description": "Customer email address"
    },
    {
      "name": "rating",
      "type": "int",
      "nullable": false,
      "description": "Rating from 1 to 5"
    },
    {
      "name": "feedback",
      "type": "text",
      "nullable": true,
      "description": "Detailed feedback text"
    }
  ],
  "options": {
    "timestamps": true,
    "soft_delete": true
  }
}')

echo $CREATE_RESPONSE | python3 -m json.tool
echo ""

# 4. Get table schema
echo -e "${BLUE}4. Getting schema for 'customer_feedback' table...${NC}"
auth_request GET "/admin/custom-tables/customer_feedback" | python3 -m json.tool
echo ""

# 5. Insert data into the custom table
echo -e "${BLUE}5. Inserting sample data into the table...${NC}"
INSERT_RESPONSE=$(auth_request POST "/admin/custom-tables/customer_feedback/data" '{
  "data": {
    "customer_name": "John Doe",
    "email": "john@example.com",
    "rating": 5,
    "feedback": "Excellent product! Very satisfied with my purchase."
  }
}')

echo $INSERT_RESPONSE | python3 -m json.tool
echo ""

# 6. Query data from the custom table
echo -e "${BLUE}6. Querying data from the table...${NC}"
auth_request GET "/admin/custom-tables/customer_feedback/data?limit=10" | python3 -m json.tool
echo ""

# 7. List all custom tables again
echo -e "${BLUE}7. Final list of custom tables...${NC}"
auth_request GET "/admin/custom-tables" | python3 -m json.tool
echo ""

echo -e "${GREEN}Test completed!${NC}"
echo ""
echo "The custom tables feature allows admins to:"
echo "• Create new tables dynamically without code changes"
echo "• Define fields with various data types (string, int, float, bool, time, json)"
echo "• Set up indexes and constraints"
echo "• Enable features like timestamps and soft delete"
echo "• Perform CRUD operations on the data"
echo "• All tables are automatically prefixed with 'custom_' to avoid conflicts"