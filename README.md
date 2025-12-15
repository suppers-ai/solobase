> **Note:** This project was built for a hackathon by [Kiro](https://kiro.dev/) to test out the IDE. It is by no means complete or production ready.
>
> Please ⭐ star the project and join our [Discord](https://discord.com/invite/jKqMcbrVzm) if you think this will be useful and want me to continue developing it!

<p align="center">
  <img src="ui/static/logo_long.png" alt="Solobase Logo" />
</p>

# Solobase

What if deploying a backend was as simple as running a single file? No Docker containers, no Node.js runtime, no required dependencies - just download and run. Solobase delivers auth, database, storage, UI components, and extension capabilities, all in one single self-contained binary, that you can run in literally seconds. The backend you've been waiting for - powerful enough for production, simple enough for anyone, free from lock-in.

## Installation

```bash
go install github.com/suppers-ai/solobase/cmd/solobase@latest
```

## Usage

```bash
solobase
```

That's it. Really.

## Security & Recent Improvements

### 🔐 Security Enhancements
- **httpOnly Cookies**: Authentication tokens are now stored in secure httpOnly cookies (XSS protection)
- **SQL Injection Prevention**: All database queries use parameterized statements
- **JWT Security**: Enforced secure JWT secret configuration (no hardcoded fallbacks)
- **Open Redirect Protection**: OAuth callbacks validate redirect URLs against whitelist

### 🎨 Code Quality Improvements
- **Component Architecture**: Refactored large components (2500+ lines) into modular, reusable pieces
- **TypeScript Support**: Added comprehensive type definitions for better IDE support
- **Error Handling**: Centralized error handling with user-friendly toast notifications
- **Accessibility**: ARIA labels, keyboard navigation, and focus management

### 📁 Project Structure
```
solobase/
├── frontend/          # SvelteKit admin UI
│   ├── src/lib/
│   │   ├── components/   # Reusable UI components
│   │   ├── types/        # TypeScript definitions
│   │   ├── stores/       # Svelte stores
│   │   └── utils/        # Utility functions
│   └── src/routes/       # Page components
├── internal/          # Go backend
│   ├── api/          # HTTP handlers
│   ├── core/         # Business logic
│   ├── middleware/   # HTTP middleware
│   └── pkg/          # Shared packages
├── sdk/              # Client SDKs
│   └── typescript/   # TypeScript/JavaScript SDK
└── docs/             # Documentation
```

## Features

- **🔒 Authentication**: Email/password and OAuth (Google, GitHub, Facebook, Microsoft)
- **👥 User Management**: Full CRUD operations with role-based access control
- **🗄️ Database**: SQLite with admin UI for queries and data management
- **📦 Storage**: File storage with bucket management
- **🔌 Extensions**: Modular architecture for adding custom features
- **📊 Analytics**: Built-in analytics tracking
- **💳 Payments**: Stripe/PayPal integration ready
- **🎨 Admin UI**: Modern, responsive admin dashboard

## Quick Start

```bash
# Install
go install github.com/suppers-ai/solobase/cmd/solobase@latest

# Run with environment variables
JWT_SECRET="your-secret-key-minimum-32-characters" solobase

# Or create .env file
echo "JWT_SECRET=your-secret-key-minimum-32-characters" > .env
solobase
```

## Configuration

Create a `.env` file in your project root:

```env
# Required
JWT_SECRET=your-secret-key-minimum-32-characters

# Optional
PORT=8090
DATABASE_URL=sqlite://solobase.db
STORAGE_PATH=./storage

# OAuth (optional)
GOOGLE_CLIENT_ID=your-google-client-id
GOOGLE_CLIENT_SECRET=your-google-client-secret
GITHUB_CLIENT_ID=your-github-client-id
GITHUB_CLIENT_SECRET=your-github-client-secret

# Stripe Payments (optional)
STRIPE_SECRET_KEY=sk_test_...
STRIPE_WEBHOOK_SECRET=whsec_...
STRIPE_PUBLISHABLE_KEY=pk_test_...
```

## API Documentation

See [docs/API.md](docs/API.md) for complete API documentation.

## Security

See [docs/SECURITY.md](docs/SECURITY.md) for security documentation and best practices.

## Development

```bash
# Clone repository
git clone https://github.com/suppers-ai/solobase.git
cd solobase

# Backend development
go run .

# Frontend development
cd frontend
npm install
npm run dev
```

## Learn More

For detailed documentation and to try a live demo, visit [solobase.dev](https://solobase.dev/)