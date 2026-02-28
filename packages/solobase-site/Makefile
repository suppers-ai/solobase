.PHONY: deps build solobase run dev clean help

SOLOBASE := ./dist/solobase

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

deps: ## Install npm dependencies
	@if [ ! -d node_modules ]; then \
		echo "Installing npm dependencies..."; \
		npm install; \
	else \
		echo "Dependencies already installed."; \
	fi

build: deps ## Build site with Vite (outputs to dist/site/)
	@echo "Building site with Vite..."
	@npx vite build

solobase: ## Build solobase binary to dist/
	@mkdir -p dist
	@echo "Building solobase binary..."
	@cd ../solobase && cargo build --release
	@cp ../solobase/target/release/solobase ./dist/solobase
	@echo "Built: $(SOLOBASE)"

run: build solobase ## Build everything and run solobase serving the site
	@echo "Starting solobase with WEB_ROOT=./dist/site on port 8090..."
	WEB_ROOT=./dist/site \
	ENVIRONMENT=development \
	JWT_SECRET="dev-secret-key-minimum-32-characters-long" \
	DEFAULT_ADMIN_EMAIL="admin@example.com" \
	DEFAULT_ADMIN_PASSWORD="admin123" \
	PORT=8090 \
	$(SOLOBASE)

dev: deps ## Run Vite dev server with HMR
	@npx vite

clean: ## Remove build artifacts
	@rm -rf dist
	@echo "Cleaned dist/"
