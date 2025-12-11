# Simple Dockerfile for Solobase Demo
FROM golang:1.23-bookworm

# Build argument for version (optional)
ARG BUILD_VERSION=demo

# Install sqlite3 for debugging and Node.js for UI build
RUN apt-get update && apt-get install -y sqlite3 curl && rm -rf /var/lib/apt/lists/* && \
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy go.mod first for better caching
COPY go.mod go.sum ./
RUN go mod download

# Copy everything else
COPY . .

# Build the frontend
WORKDIR /app/frontend
RUN npm install && npm run build

# Build the binary with clean cache
WORKDIR /app
RUN go clean -cache && go build -o solobase ./cmd/solobase

# Create data directory
RUN mkdir -p /app/.data

# Copy pre-populated demo database (REQUIRED for read-only mode)
# Must run demo/setup-demo-db.sh first to create this file
COPY demo/deployment/demo.db /app/.data/solobase.db

# Environment variables
ENV PORT=8080
ENV DATABASE_TYPE=sqlite
ENV DATABASE_URL=file:/app/.data/solobase.db

# Read-only mode for demo
ENV READONLY_MODE=true

# Admin account credentials (must exist in the demo.db)
ENV DEFAULT_ADMIN_EMAIL=admin@example.com
ENV DEFAULT_ADMIN_PASSWORD=admin123

EXPOSE 8080

# Run the app
CMD ["./solobase"]