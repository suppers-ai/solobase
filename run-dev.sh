#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
DATABASE_TYPE="${1:-sqlite}"
API_PORT=${API_PORT:-8090}
FRONTEND_PORT=${FRONTEND_PORT:-5173}

echo -e "${GREEN}=== Solobase Development Server ===${NC}"
echo -e "${YELLOW}Database Type: $DATABASE_TYPE${NC}"

# Show usage if needed
if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    echo "Usage: $0 [database_type] [clean]"
    echo "  database_type: sqlite (default) or postgres"
    echo "  clean: add 'clean' as second argument to reset SQLite database"
    echo ""
    echo "Examples:"
    echo "  $0                # Use SQLite with existing database"
    echo "  $0 sqlite clean   # Use SQLite with fresh database"
    echo "  $0 postgres       # Use PostgreSQL"
    exit 0
fi

# Kill existing processes
echo -e "${YELLOW}Stopping any existing servers...${NC}"
pkill -f "solobase" 2>/dev/null
pkill -f "./solobase" 2>/dev/null
pkill -f "go run ." 2>/dev/null
pkill -f "go run main.go" 2>/dev/null
pkill -f "npm run dev" 2>/dev/null
pkill -f "vite" 2>/dev/null

# Kill anything on our ports - more thorough cleanup
for port in $API_PORT $FRONTEND_PORT 5174 8090 8091 8092 8093 8094; do
    if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
        echo -e "${YELLOW}  Killing process on port $port${NC}"
        lsof -ti:$port | xargs kill -9 2>/dev/null
    fi
done
sleep 2

# Set database URL based on type
if [ "$DATABASE_TYPE" = "postgres" ]; then
    # Start PostgreSQL container if needed
    if ! docker ps | grep -q solobase-postgres; then
        echo -e "${YELLOW}Starting PostgreSQL container...${NC}"
        docker run -d \
            --name solobase-postgres \
            -e POSTGRES_USER=solobase \
            -e POSTGRES_PASSWORD=solobase123 \
            -e POSTGRES_DB=solobase \
            -p 5432:5432 \
            postgres:15-alpine 2>/dev/null || docker start solobase-postgres
        
        echo "Waiting for PostgreSQL to be ready..."
        sleep 5
    fi
    DATABASE_URL="postgresql://solobase:solobase123@localhost:5432/solobase?sslmode=disable"
else
    # For SQLite, optionally clean the database
    if [ "$2" = "clean" ]; then
        echo -e "${YELLOW}Cleaning SQLite database and internal storage...${NC}"
        # Remove database
        rm -f ./.data/solobase.db
        # Remove internal storage but preserve extension storage
        rm -rf ./.data/storage/int
        # Remove old locations just in case
        rm -f ./solobase.db
        rm -rf ./data
        echo -e "${GREEN}Preserved extension storage in ./.data/storage/ext/${NC}"
    fi
    # Use the proper database location in .data directory
    DATABASE_URL="file:./.data/solobase.db"
fi

# Build the solobase binary first for better performance
echo -e "${YELLOW}Building solobase...${NC}"
./compile.sh

# Start API server
echo -e "${YELLOW}Starting API server on port $API_PORT...${NC}"
# ENVIRONMENT=development ensures consistent JWT secret across restarts
ENVIRONMENT=development \
DATABASE_TYPE=$DATABASE_TYPE \
DATABASE_URL=$DATABASE_URL \
DEFAULT_ADMIN_EMAIL=admin@example.com \
DEFAULT_ADMIN_PASSWORD=admin123 \
PORT=$API_PORT \
./solobase &
API_PID=$!

# Wait for API to start
echo "Waiting for API server to start..."
sleep 3

# Install npm dependencies if needed
if [ ! -d "ui/node_modules" ]; then
    echo -e "${YELLOW}Installing npm dependencies...${NC}"
    cd ui && npm install
    cd ..
fi

# Start frontend dev server with API_PORT environment variable
echo -e "${YELLOW}Starting frontend dev server (API_PORT=$API_PORT)...${NC}"
cd ui && API_PORT=$API_PORT npm run dev &
FRONTEND_PID=$!
cd ..

# Wait for frontend to start and find actual port
sleep 5

# Try to detect actual frontend port
ACTUAL_FRONTEND_PORT=$FRONTEND_PORT
if lsof -Pi :5174 -sTCP:LISTEN -t >/dev/null 2>&1; then
    ACTUAL_FRONTEND_PORT=5174
    echo -e "${YELLOW}Note: Frontend is running on port 5174 (5173 was in use)${NC}"
elif ! lsof -Pi :$FRONTEND_PORT -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo -e "${RED}Warning: Frontend may not have started properly${NC}"
fi

echo ""
echo -e "${GREEN}=== Servers are running! ===${NC}"
echo -e "${YELLOW}Frontend:${NC} http://localhost:$ACTUAL_FRONTEND_PORT"
echo -e "${YELLOW}API:${NC}      http://localhost:$API_PORT/api"
echo ""
echo -e "${YELLOW}Default Admin:${NC}"
echo "  Email:    admin@example.com"
echo "  Password: admin123"
echo ""
echo -e "${YELLOW}Database Type:${NC} $DATABASE_TYPE"
if [ "$DATABASE_TYPE" = "sqlite" ]; then
    echo -e "${YELLOW}Database File:${NC} ./.data/solobase.db"
else
    echo -e "${YELLOW}Database URL:${NC} $DATABASE_URL"
fi
echo ""
echo -e "${GREEN}Press Ctrl+C to stop all servers${NC}"

# Function to cleanup on exit
cleanup() {
    echo -e "\n${YELLOW}Shutting down servers...${NC}"
    kill $API_PID 2>/dev/null
    kill $FRONTEND_PID 2>/dev/null
    
    if [ "$DATABASE_TYPE" = "postgres" ]; then
        echo -e "${YELLOW}Stopping PostgreSQL container...${NC}"
        docker stop solobase-postgres
    fi
    
    echo -e "${GREEN}Goodbye!${NC}"
    exit 0
}

# Set up trap to cleanup on Ctrl+C
trap cleanup INT

# Keep script running
wait
