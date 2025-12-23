.PHONY: build build-tiny run clean tidy

# Standard Go build
build:
	@echo "Building Solobase (standard Go)..."
	@go build -o solobase .
	@echo "Built: solobase"
	@ls -lh solobase

# TinyGo build (standard binary, not WASM)
build-tiny:
	@echo "Building Solobase (TinyGo)..."
	@tinygo build -o solobase-tiny .
	@echo "Built: solobase-tiny"
	@ls -lh solobase-tiny

# Run the standard binary
run: build
	ENVIRONMENT=development ./solobase

# Clean build artifacts
clean:
	@rm -f solobase solobase-tiny
	@echo "Cleaned build artifacts"

# Tidy dependencies
tidy:
	@go mod tidy
