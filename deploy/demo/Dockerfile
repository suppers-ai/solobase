# Dockerfile for Solobase Demo
# Build context should be the solobase repo root
FROM rust:1-bookworm AS builder

# Build argument for version (optional)
ARG BUILD_VERSION=demo

# Install Node.js for frontend build and sqlite3 for debugging
RUN apt-get update && apt-get install -y sqlite3 curl && rm -rf /var/lib/apt/lists/* && \
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Cargo files first for better dependency caching
COPY Cargo.toml Cargo.lock* build.rs ./
COPY ../wafer/Cargo.toml ../wafer/
COPY ../wafer-core/Cargo.toml ../wafer-core/

# Copy source
COPY . .

# Build the frontend
WORKDIR /app/frontend
RUN npm install && npm run build

# Build the Rust binary in release mode
WORKDIR /app
RUN cargo build --release

# --- Runtime stage ---
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y sqlite3 ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary
COPY --from=builder /app/target/release/solobase ./solobase

# Copy frontend build into storage directory (served by wafer-run/web via storage)
# Path includes wafer-run/web/ prefix for per-block storage isolation
COPY --from=builder /app/data/storage/wafer-run/web/site ./data/storage/wafer-run/web/site

# Create data directory
RUN mkdir -p /app/data

# Copy pre-populated demo database (REQUIRED for read-only mode)
# demo.db must be placed in the solobase repo root before building
COPY demo.db /app/data/solobase.db

# Environment variables
ENV BIND_ADDR=0.0.0.0:8080
ENV DB_PATH=/app/data/solobase.db

# Read-only mode for demo
ENV READONLY_MODE=true

# Admin account credentials (must exist in the demo.db)
ENV DEFAULT_ADMIN_EMAIL=admin@example.com
ENV DEFAULT_ADMIN_PASSWORD=admin123

EXPOSE 8080

# Run the app
CMD ["./solobase"]
