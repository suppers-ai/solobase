# Solobase Admin Dashboard

A modern, full-featured admin dashboard built with **Svelte** (frontend) and **Go** (backend), compiled into a single binary for easy deployment.

## Features

- 🚀 **Single Binary Deployment** - Frontend and backend in one executable
- 🎨 **Modern UI** - Built with SvelteKit, Skeleton UI, and Tailwind CSS
- 🔐 **JWT Authentication** - Secure token-based authentication
- 📊 **Full Admin Dashboard** - Users, database browser, storage, collections, settings
- 🗄️ **Multi-Database Support** - PostgreSQL and SQLite
- 📦 **Storage Providers** - Local filesystem and S3 support
- 🔄 **Real-time Updates** - Reactive UI with Svelte stores
- 📱 **Responsive Design** - Works on desktop, tablet, and mobile

## Tech Stack

### Frontend
- **SvelteKit** - Modern web framework
- **Skeleton UI** - Beautiful UI component library
- **Tailwind CSS** - Utility-first CSS framework
- **TypeScript** - Type-safe JavaScript
- **Lucide Icons** - Clean, consistent icons

### Backend
- **Go** - Fast, compiled backend
- **Gorilla Mux** - HTTP router
- **JWT** - JSON Web Tokens for auth
- **GORM** - ORM for database operations
- **Embedded Files** - Frontend embedded in binary

## Quick Start

### Prerequisites

- Go 1.20+
- Node.js 20+
- Docker (optional, for PostgreSQL)

### Development Setup

1. **Clone the repository**
```bash
git clone <repository-url>
cd solobase
```

2. **Install frontend dependencies**
```bash
cd admin
npm install
cd ..
```

3. **Run development servers**

With PostgreSQL (requires Docker):
```bash
./run-dev.sh postgres
```

With SQLite (no Docker required):
```bash
./run-dev.sh sqlite
```

The script will:
- Start the database (PostgreSQL container or SQLite file)
- Run the Go API server on http://localhost:8080
- Run the Svelte dev server on http://localhost:5173
- Create a default admin user

4. **Access the application**
- Frontend: http://localhost:5173
- API: http://localhost:8080/api

**Default Admin Credentials:**
- Email: `admin@example.com`
- Password: `admin123`

## Production Build

### Build Single Binary

```bash
# Using Makefile
make build

# Or manually:
cd admin && npm run build && cd ..
go build -o solobase .
```

### Run Production Binary

```bash
# With PostgreSQL
DATABASE_URL="postgresql://user:pass@localhost/dbname" \
./solobase

# With SQLite
DATABASE_TYPE=sqlite \
DATABASE_URL="file:./data.db" \
./solobase
```

The production server runs on port 8080 by default (configurable via `PORT` env var).

## Project Structure

```
solobase/
├── admin/                 # Svelte frontend application
│   ├── src/
│   │   ├── routes/       # Page components
│   │   │   ├── +page.svelte        # Dashboard
│   │   │   ├── login/              # Login page
│   │   │   ├── users/              # User management
│   │   │   ├── database/           # Database browser
│   │   │   ├── storage/            # File storage
│   │   │   ├── collections/        # Collections manager
│   │   │   └── settings/           # App settings
│   │   ├── lib/
│   │   │   ├── api.ts             # API client
│   │   │   ├── types.ts           # TypeScript types
│   │   │   ├── stores/            # Svelte stores
│   │   │   └── components/        # Reusable components
│   │   └── app.css               # Global styles
│   └── build/                    # Production build (embedded)
│
├── api/                          # Go API handlers
│   ├── auth.go                  # Authentication endpoints
│   ├── users.go                 # User management
│   ├── database.go              # Database operations
│   ├── storage.go               # File storage
│   ├── collections.go           # Collections CRUD
│   ├── settings.go              # Settings management
│   ├── dashboard.go             # Dashboard stats
│   ├── middleware.go            # Auth & CORS middleware
│   └── router.go                # API route definitions
│
├── services/                     # Business logic layer
├── models/                       # Data models
├── config/                       # Configuration
├── embed.go                      # Embed frontend build
├── main.go                       # Application entry point
└── Makefile                      # Build commands
```

## Environment Variables

### Database Configuration

```bash
# Database type: postgres or sqlite
DATABASE_TYPE=postgres

# PostgreSQL
DATABASE_URL=postgresql://user:password@localhost:5432/dbname?sslmode=disable

# SQLite
DATABASE_URL=file:./database.db
```

### Application Settings

```bash
# Server port (default: 8080)
PORT=8080

# Default admin user (created on first run)
DEFAULT_ADMIN_EMAIL=admin@example.com
DEFAULT_ADMIN_PASSWORD=SecurePassword123!

# Storage configuration
STORAGE_TYPE=local  # or 's3'
STORAGE_PATH=/var/lib/solobase/storage

# S3 Configuration (if using S3)
AWS_ACCESS_KEY_ID=your-key
AWS_SECRET_ACCESS_KEY=your-secret
S3_BUCKET=your-bucket
S3_REGION=us-east-1
```

## API Endpoints

All API endpoints are prefixed with `/api`:

### Authentication
- `POST /api/auth/login` - User login
- `POST /api/auth/signup` - User registration
- `POST /api/auth/logout` - Logout
- `GET /api/auth/me` - Get current user

### Users
- `GET /api/users` - List users (paginated)
- `GET /api/users/:id` - Get user details
- `PATCH /api/users/:id` - Update user
- `DELETE /api/users/:id` - Delete user

### Database
- `GET /api/database/tables` - List tables
- `GET /api/database/tables/:table/columns` - Get table columns
- `POST /api/database/query` - Execute query (admin only)

### Storage
- `GET /api/storage/buckets` - List buckets
- `GET /api/storage/buckets/:bucket/objects` - List objects
- `POST /api/storage/buckets/:bucket/upload` - Upload file
- `DELETE /api/storage/buckets/:bucket/objects/:id` - Delete object

### Collections
- `GET /api/collections` - List collections
- `POST /api/collections` - Create collection
- `GET /api/collections/:id` - Get collection
- `PATCH /api/collections/:id` - Update collection
- `DELETE /api/collections/:id` - Delete collection

### Settings
- `GET /api/settings` - Get app settings
- `PATCH /api/settings` - Update settings (admin only)

### Dashboard
- `GET /api/dashboard/stats` - Get dashboard statistics

## Development Commands

```bash
# Install dependencies
cd admin && npm install

# Run development servers
./run-dev.sh postgres  # or sqlite

# Build for production
make build

# Run tests
go test ./...

# Format code
cd admin && npm run format
go fmt ./...

# Type checking
cd admin && npm run check
```

## Docker Deployment

```dockerfile
FROM golang:1.20-alpine AS builder
WORKDIR /app
COPY . .
RUN apk add --no-cache nodejs npm
RUN cd admin && npm install && npm run build
RUN go build -o solobase .

FROM alpine:latest
RUN apk --no-cache add ca-certificates
WORKDIR /root/
COPY --from=builder /app/solobase .
EXPOSE 8080
CMD ["./solobase"]
```

## Security Considerations

- JWT tokens expire after 24 hours
- Passwords are hashed using bcrypt
- CORS is configured for API endpoints
- SQL injection protection via parameterized queries
- XSS protection in frontend
- CSRF protection for state-changing operations

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests
5. Submit a pull request

## License

MIT License - see LICENSE file for details

## Support

For issues and questions, please open a GitHub issue.

---

Built with ❤️ using Svelte and Go