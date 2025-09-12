# Build stage
FROM golang:1.23 AS builder

WORKDIR /build

# Copy all packages
COPY packages ./packages
COPY integrations ./integrations
COPY solobase ./solobase

WORKDIR /build/solobase

# Build the binary from the cmd/solobase directory where main.go is
# SQLite REQUIRES CGO_ENABLED=1
RUN CGO_ENABLED=1 GOOS=linux GOARCH=amd64 go build -o solobase ./cmd/solobase

# Make it executable in the build stage
RUN chmod +x solobase

# Final stage - use debian for glibc compatibility with CGO
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the executable binary
COPY --from=builder --chmod=755 /build/solobase/solobase /app/solobase

# Copy static files and config
COPY --from=builder /build/solobase/static ./static
COPY --from=builder /build/solobase/admin ./admin
COPY --from=builder /build/solobase/config ./config

# Verify the binary exists and is executable
RUN ls -la /app/solobase

ENV PORT=8080
ENV DATABASE_TYPE=sqlite
ENV DATABASE_URL=:memory:
ENV DEFAULT_ADMIN_EMAIL=demo@solobase.com
ENV DEFAULT_ADMIN_PASSWORD=DemoAccess2024!
ENV RATE_LIMIT_ENABLED=true
ENV RATE_LIMIT_REQUESTS_PER_MINUTE=30

EXPOSE 8080

CMD ["/app/solobase"]