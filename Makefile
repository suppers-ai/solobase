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
	go build -o solobase .

run:
	@echo "Running Solobase..."
	./run-dev.sh

test:
	@echo "Running tests..."
	go test ./...

generate-types:
	@echo "Generating TypeScript types from GORM models..."
	go run scripts/generate-types.go
	@echo "Types generated successfully in sdk/typescript/src/types/database/"

clean:
	@echo "Cleaning build artifacts..."
	rm -f solobase solobase-temp
	rm -rf sdk/typescript/dist