.PHONY: build dev clean serve

# Build for production (framework provides assets + hashing)
build:
	wasm-pack build --target web --release --out-dir pkg
	cp js/manifest.json pkg/
	cargo run -p solobase-browser --release --bin export-assets -- pkg/ --repo-dir $(CURDIR)/../..

# Build for development (no hashing; canonical filenames)
dev:
	wasm-pack build --target web --dev --out-dir pkg
	cp js/manifest.json pkg/
	cargo run -p solobase-browser --release --bin export-assets -- pkg/ --repo-dir $(CURDIR)/../.. --dev

# Serve locally for testing (localhost is exempt from HTTPS requirement for SW)
serve: dev
	@echo "Serving at http://localhost:8080"
	python3 -m http.server 8080 -d pkg

clean:
	rm -rf pkg target
