.PHONY: help build build-wasm run dev-wasm deploy-wasm test generate-types clean tidy

help:
	@echo "Solobase Build System"
	@echo ""
	@echo "Standard Go (recommended for local development):"
	@echo "  build              - Build standard Go binary"
	@echo "  run                - Run in development mode with local SQLite"
	@echo ""
	@echo "WASM + Cloudflare Workers:"
	@echo "  build-wasm         - Build WASM module with TinyGo"
	@echo "  dev-wasm           - Run WASM locally with wrangler (uses D1 SQLite)"
	@echo "  deploy-wasm        - Deploy WASM to Cloudflare Workers"
	@echo ""
	@echo "Other:"
	@echo "  build-all          - Build all targets"
	@echo "  test               - Run tests"
	@echo "  generate-types     - Generate TypeScript types"
	@echo "  tidy               - Run go mod tidy"
	@echo "  clean              - Clean build artifacts"

# Ensure dist directory exists
dist:
	@mkdir -p dist

# Standard Go build
build: dist
	@echo "Building Solobase (standard Go)..."
	cd builds/go && go build -o ../../dist/solobase .
	@echo "Built: dist/solobase"
	@ls -lh dist/solobase

# WASM build using TinyGo
# Note: TinyGo embed requires files in the main package directory
# So we copy frontend/build to builds/wasm/frontend/build
build-wasm: dist
	@echo "Building Solobase WASM..."
	@# Copy frontend to WASM build directory for embed
	@rm -rf builds/wasm/frontend/build
	@mkdir -p builds/wasm/frontend
	@if [ -d frontend/build ]; then \
		echo "Copying frontend build..."; \
		cp -r frontend/build builds/wasm/frontend/; \
		if [ -d builds/wasm/frontend/build/_app ]; then \
			echo "Renaming _app to app for TinyGo compatibility..."; \
			mv builds/wasm/frontend/build/_app builds/wasm/frontend/build/app; \
			find builds/wasm/frontend/build -name "*.html" -exec sed -i 's|/_app/|/app/|g' {} \; 2>/dev/null || \
			find builds/wasm/frontend/build -name "*.html" -exec sed -i '' 's|/_app/|/app/|g' {} \;; \
		fi; \
	else \
		echo "Warning: No frontend build found"; \
	fi
	tinygo build -target=wasip1 -gc=leaking -no-debug -tags wasm -o dist/solobase.wasm ./builds/wasm
	@echo "Built: dist/solobase.wasm"
	@ls -lh dist/solobase.wasm

# Build all targets
build-all: build build-wasm

# Run standard binary in development mode
run: build
	@echo "Running Solobase (standard Go)..."
	@echo "Server: http://localhost:8090"
	@echo ""
	ENVIRONMENT=development \
	JWT_SECRET="dev-secret-key-minimum-32-characters-long" \
	DEFAULT_ADMIN_EMAIL="admin@example.com" \
	DEFAULT_ADMIN_PASSWORD="admin123" \
	./dist/solobase

# Run WASM locally with Cloudflare wrangler (uses local D1 SQLite)
dev-wasm: build-wasm
	@echo "Running Solobase WASM with wrangler..."
	@# Kill any existing process on port 8787
	@-lsof -ti:8787 | xargs kill 2>/dev/null || true
	@echo "Server: http://localhost:8787"
	@echo ""
	@if [ ! -d builds/wasm/host/node_modules ]; then \
		echo "Installing dependencies..."; \
		cd builds/wasm/host && npm install; \
	fi
	cd builds/wasm/host && npm run dev

# Deploy WASM to Cloudflare Workers
deploy-wasm: build-wasm
	@echo "Deploying Solobase WASM to Cloudflare..."
	@if [ ! -d builds/wasm/host/node_modules ]; then \
		echo "Installing dependencies..."; \
		cd builds/wasm/host && npm install; \
	fi
	cd builds/wasm/host && npm run deploy

# Run tests
test:
	@echo "Running tests..."
	go test ./...

# Generate TypeScript types from Go models
generate-types:
	@echo "Generating TypeScript types..."
	go run scripts/generate-types.go
	@mkdir -p frontend/src/lib/types/generated
	@cp sdk/typescript/src/types/database/index.ts frontend/src/lib/types/generated/database.ts
	@echo "Types generated"

# Tidy all modules
tidy:
	@echo "Tidying modules..."
	go mod tidy
	cd builds/go && go mod tidy
	@echo "Done"

# Clean build artifacts
clean:
	@echo "Cleaning..."
	rm -rf dist/
	rm -rf builds/wasm/frontend/build
	rm -rf builds/wasm/host/dist
	rm -rf builds/wasm/*.wasm
	rm -rf sdk/typescript/dist
	@echo "Clean complete"
