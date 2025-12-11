.PHONY: help build run test generate-types clean

help:
	@echo "Available targets:"
	@echo "  build         - Build the Solobase binary"
	@echo "  run           - Run Solobase in development mode"
	@echo "  test          - Run tests"
	@echo "  generate-types - Generate TypeScript types from GORM models"
	@echo "  clean         - Clean build artifacts"

build:
	@echo "Building Solobase..."
	go build -o solobase ./cmd/solobase

run:
	@echo "Running Solobase..."
	./run-dev.sh

test:
	@echo "Running tests..."
	go test ./...

generate-types:
	@echo "Generating TypeScript types from GORM models..."
	go run scripts/generate-types.go
	@echo "Copying types to frontend..."
	@mkdir -p frontend/src/lib/types/generated
	@cp sdk/typescript/src/types/database/index.ts frontend/src/lib/types/generated/database.ts
	@echo "Types generated successfully in sdk/typescript/src/types/database/ and frontend/src/lib/types/generated/"

clean:
	@echo "Cleaning build artifacts..."
	rm -f solobase solobase-temp
	rm -rf sdk/typescript/dist