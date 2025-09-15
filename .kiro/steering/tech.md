# Technology Stack

## Backend
- **Language**: Go 1.23.0
- **Web Framework**: Gorilla Mux for HTTP routing
- **Database ORM**: GORM with support for SQLite, PostgreSQL, MySQL, SQL Server
- **Authentication**: Authboss v3 with JWT tokens (golang-jwt/jwt/v5)
- **Authorization**: Casbin v2 for RBAC
- **Storage**: Local filesystem and S3-compatible cloud storage (AWS SDK)
- **Monitoring**: Prometheus metrics
- **Session Management**: Gorilla Sessions
- **Configuration**: Environment variables with godotenv

## Frontend
- **Framework**: SvelteKit with TypeScript
- **UI Library**: Skeleton UI components
- **Styling**: Tailwind CSS with forms and typography plugins
- **Charts**: Chart.js with date-fns adapter
- **Icons**: Lucide Svelte
- **Build Tool**: Vite

## Build System & Commands

### Go Commands
```bash
# Build main binary
go build -o solobase .

# Build from cmd directory
go build -o solobase cmd/solobase/main.go

# Run tests
go test ./...

# Install from source
go install github.com/suppers-ai/solobase/cmd/solobase@latest
```

### Make Commands
```bash
make build          # Build the Solobase binary
make run            # Run in development mode
make test           # Run all tests
make generate-types # Generate TypeScript types from GORM models
make clean          # Clean build artifacts
```

### UI Commands
```bash
cd ui
npm run dev         # Start development server on port 5173
npm run build       # Build for production
npm run build:go    # Build UI and Go binary together
```

### Development Scripts
- `./run-dev.sh` - Development server startup
- `./compile.sh` - Simple build script
- `./deploy-demo.sh` - Demo deployment

## Architecture Patterns

### Extension System
- Compile-time plugin architecture
- Schema isolation per extension
- Hook-based event system
- Middleware registration
- Resource quotas and security sandboxing

### Database Design
- Multi-database support with adapter pattern
- Schema prefixing for extensions (`ext_extensionname`)
- Migration system with versioning
- Connection pooling and metrics

### API Structure
- RESTful endpoints under `/api`
- Extension routes under `/api/ext/`
- IAM routes for authorization
- Middleware chain for auth, logging, metrics, CORS

### Storage Architecture
- Provider pattern for local/cloud storage
- Application ID isolation
- Quota management
- Token-based upload/download

## Configuration
- Environment-based configuration
- Sensible defaults for development
- Production security requirements
- Hot-reloadable extension configs