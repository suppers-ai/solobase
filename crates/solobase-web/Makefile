.PHONY: build dev clean serve

SQL_JS_VERSION := 1.11.0

# Download sql.js assets from npm (guaranteed matching JS+WASM pair)
pkg/sql-wasm.wasm pkg/sql-wasm.js:
	@mkdir -p pkg
	@echo "Downloading sql.js $(SQL_JS_VERSION) from npm..."
	@cd /tmp && npm pack sql.js@$(SQL_JS_VERSION) --silent 2>/dev/null && \
		tar xzf sql.js-$(SQL_JS_VERSION).tgz && \
		cp package/dist/sql-wasm.js $(CURDIR)/pkg/sql-wasm.js && \
		cp package/dist/sql-wasm.wasm $(CURDIR)/pkg/sql-wasm.wasm && \
		rm -rf package sql.js-$(SQL_JS_VERSION).tgz

pkg/sql-wasm-esm.js: pkg/sql-wasm.js
	@echo "Creating sql.js ESM wrapper..."
	@printf '// ESM wrapper for sql.js $(SQL_JS_VERSION) UMD build (IIFE to isolate scope)\nconst _sqlJs = (function() {\n  var module = { exports: {} };\n  var exports = module.exports;\n' > pkg/sql-wasm-esm.js
	@cat pkg/sql-wasm.js >> pkg/sql-wasm-esm.js
	@printf '\n  return module.exports.default || module.exports;\n})();\nexport default _sqlJs;\n' >> pkg/sql-wasm-esm.js

# Build for production (content-hashes assets, renders templates)
build: pkg/sql-wasm-esm.js
	wasm-pack build --target web --release --out-dir pkg
	cp js/sw.js.tmpl pkg/
	cp js/index.html.tmpl pkg/
	cp js/loader.js js/ai-bridge.js js/manifest.json pkg/
	cargo run -p solobase-web-bundle --release -- pkg/ --repo-dir $(CURDIR)/../..

# Build for development (no hashing; canonical filenames)
dev: pkg/sql-wasm-esm.js
	wasm-pack build --target web --dev --out-dir pkg
	cp js/sw.js.tmpl pkg/
	cp js/index.html.tmpl pkg/
	cp js/loader.js js/ai-bridge.js js/manifest.json pkg/
	cargo run -p solobase-web-bundle --release -- pkg/ --repo-dir $(CURDIR)/../.. --dev

# Serve locally for testing (localhost is exempt from HTTPS requirement for SW)
serve: dev
	@echo "Serving at http://localhost:8080"
	python3 -m http.server 8080 -d pkg

clean:
	rm -rf pkg target
