# Product Overview

Solobase is a modern, self-hosted backend solution that provides a complete suite of features for building web applications. It's designed as an all-in-one platform that includes:

- **Authentication & Authorization**: Built-in user management with JWT tokens and role-based access control (RBAC) using Casbin
- **Database Management**: Multi-database support (SQLite, PostgreSQL, MySQL, SQL Server) with GORM ORM
- **File Storage**: Local and cloud storage (S3-compatible) with quota management
- **Admin Interface**: Svelte-based UI for managing users, database, and system settings
- **Extension System**: Compile-time plugin architecture for extending functionality
- **Real-time Capabilities**: WebSocket support and event hooks
- **API Management**: RESTful APIs with middleware support
- **Monitoring**: Built-in logging, metrics, and health checks

The project was originally built for a hackathon to test Kiro IDE capabilities and is designed to be deployed as a single binary with minimal configuration required.

## Key Features

- Single binary deployment
- Zero-configuration startup (works out of the box)
- Multi-tenant support with application ID isolation
- Comprehensive security with IAM integration
- Hot-reloadable configuration
- TypeScript SDK generation
- Docker support