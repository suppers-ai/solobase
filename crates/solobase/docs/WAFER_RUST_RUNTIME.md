# WAFER Rust Runtime — Research & Architecture

Decision: Rewrite the WAFER runtime in Rust. WAFER is a standalone framework ("tool to create tools"), not just Solobase's internal runtime. The WASM ecosystem is Rust-native, and building in Rust aligns with the tools and community we'd be adjacent to.

---

## Why Rust Over Go

### The WASM Ecosystem Is Rust

Wasmtime, wasm-tools, wasm-merge, Wizer, wit-bindgen, Fermyon Spin, Extism's kernel, Javy — all Rust. Building a WASM-centric framework in Go means constantly fighting upstream. In Rust, we swim downstream.

### Concrete Advantages

| Factor | Go (current) | Rust |
|--------|-------------|------|
| WASM runtime | wazero (pure Go, no Component Model, no plans to add it) | Wasmtime (reference implementation, full Component Model) |
| Runtime compiles to WASM | TinyGo (broken stdlib, `encoding/json` fails) | Clean (~1-2MB, no GC, first-class target) |
| Component Model | Blocked indefinitely | Available today |
| Pre-compilation | Not available in wazero | Wasmtime AOT (~1ms block startup vs ~100ms) |
| Binary size as WASM | 10-20MB (standard Go) or 1-3MB (TinyGo with caveats) | 1-2MB |
| Ecosystem alignment | Outsider | Native |

### What We Lose

| Factor | Impact |
|--------|--------|
| Development speed | Rust is slower to write than Go |
| Learning curve | Steeper for contributors |
| Solobase compatibility | Solobase blocks become WASM modules (Go → .wasm) |
| CF Workers deployment | Cannot run Wasmtime inside CF Workers (see Edge Deployment section) |
| Goroutines | Replaced by async/tokio (harder but performant) |

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    SINGLE DEPLOYABLE BINARY                   │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│   Rust WAFER Runtime                                        │
│   ├── Chain executor, routing, first-match                   │
│   ├── Context + capabilities (log, config, services)         │
│   ├── Lifecycle management (Init → Start → Handle → Stop)   │
│   └── Observability hooks                                    │
│                                                               │
│   Bridges                                                    │
│   ├── HTTP bridge (axum)                                     │
│   ├── MCP bridge (stdio JSON-RPC)                            │
│   └── CLI bridge (wafer call)                               │
│                                                               │
│   Wasmtime Engine (runtime-only, no Cranelift — ~1.2 MB)     │
│   ├── Module::deserialize() — loads pre-compiled native code │
│   ├── Per-block sandboxed Instance (isolated memory/stack)   │
│   └── Host function registration (wafer.send, etc.)         │
│                                                               │
│   Embedded Pre-compiled Blocks (include_bytes! .cwasm)       │
│   ├── auth.cwasm       (pre-compiled at build time)          │
│   ├── database.cwasm   (each block: own memory sandbox)      │
│   ├── storage.cwasm    (startup: ~1ms per block)             │
│   └── ...                                                    │
│                                                               │
│   Embedded Frontend (rust-embed)                             │
│   ├── index.html                                             │
│   ├── app.js                                                 │
│   └── style.css                                              │
│                                                               │
│   Host-Provided Services                                     │
│   ├── Database (SQLite via rusqlite, Postgres via sqlx)      │
│   ├── Storage (local fs, S3)                                 │
│   ├── Crypto (argon2, JWT)                                   │
│   ├── Logger (tracing crate)                                 │
│   └── Config (env vars, files)                               │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

---

## Embedding Frontend Files

Rust has `rust-embed`, the direct equivalent of Go's `//go:embed`:

```rust
use rust_embed::Embed;

#[derive(Embed, Clone)]
#[folder = "frontend/dist/"]
struct FrontendAssets;

// Access at runtime
let file = FrontendAssets::get("index.html").unwrap();
let content = file.data;

// In debug mode: reads from filesystem (hot-reload)
// In release mode: compiled into binary
```

Integrates directly with axum via `axum-embed`:

```rust
use axum::Router;
use axum_embed::ServeEmbed;

let app = Router::new()
    .nest_service("/", ServeEmbed::<FrontendAssets>::new());
```

---

## Embedding WASM Blocks in a Single Binary

### Why Not Merge All WASMs Into One?

Tools like `wasm-merge` can combine multiple `.wasm` modules into a single module. **Don't do this.** You lose the core security benefit of WASM: isolation.

- **Separate modules**: Each block gets its own linear memory, its own stack, its own sandbox. A buggy block can't corrupt another block's data.
- **Merged module**: All blocks share one memory space. A malicious or buggy block can read/write another block's data. You've lost the sandbox.
- **Practical issues**: Name collisions (every block exports `info`, `handle`, `lifecycle`, `malloc`), no independent lifecycle (can't load/unload individual blocks), harder debugging.

The correct approach is **pre-compile + embed**: each block stays in its own sandboxed module, but everything gets baked into a single binary.

### The Build Pipeline

```
Source Code (Go/Rust/AS)
        │
        ▼  (language toolchain: go build, cargo, asc)
   Raw .wasm files (~100KB-3MB each)
        │
        ▼  (build.rs — Wasmtime pre-compilation via Cranelift)
   Pre-compiled .cwasm files (native machine code, ~2-3x larger)
        │
        ▼  (include_bytes! — Rust compiler embeds as byte arrays)
   Single binary with everything baked in
```

### Step 1: Build WASM Blocks

Each block is compiled independently by its language toolchain:

```bash
# Go block (standard Go 1.24+)
GOOS=wasip1 GOARCH=wasm go build -o blocks/auth.wasm ./blocks/auth/

# Rust block
cargo build --target wasm32-wasip1 --release -p my-block
cp target/wasm32-wasip1/release/my_block.wasm blocks/

# AssemblyScript block
npx asc blocks/validator/index.ts -o blocks/validator.wasm --optimize
```

Result:
```
blocks/
  auth.wasm          (2.5 MB — Go)
  storage.wasm       (2.8 MB — Go)
  validator.wasm     (30 KB — AssemblyScript)
  transform.wasm     (150 KB — Rust)
```

### Step 2: build.rs Pre-compiles to Native Code

Wasmtime's `Engine::precompile_module()` runs Cranelift **at build time**, producing native machine code for the target CPU (x86_64/aarch64):

```rust
// build.rs
use wasmtime::Engine;
use std::fs;

fn main() -> anyhow::Result<()> {
    let engine = Engine::default();
    let out_dir = std::env::var("OUT_DIR")?;

    for entry in fs::read_dir("blocks")? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "wasm") {
            let stem = path.file_stem().unwrap().to_str().unwrap();
            let wasm_bytes = fs::read(&path)?;

            // Cranelift: WASM → native x86_64/aarch64 machine code
            let precompiled = engine.precompile_module(&wasm_bytes)?;

            fs::write(format!("{}/{}.cwasm", out_dir, stem), &precompiled)?;
            println!("cargo:rerun-if-changed=blocks/{}.wasm", stem);
        }
    }
    Ok(())
}
```

This runs **once** during `cargo build`, not at runtime. The output `.cwasm` files contain serialized native code ready to load directly into memory.

### Step 3: Embed in the Binary

```rust
// blocks.rs
static AUTH: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/auth.cwasm"));
static STORAGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/storage.cwasm"));
static VALIDATOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/validator.cwasm"));
static TRANSFORM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/transform.cwasm"));
```

Each `.cwasm` becomes a `&[u8]` literal baked into the executable's `.rodata` section. No files to ship.

### Step 4: Runtime Loading (~1ms per block)

```rust
fn load_blocks(engine: &Engine) -> anyhow::Result<HashMap<String, Module>> {
    let mut blocks = HashMap::new();

    // SAFETY: These bytes were produced by our build.rs using the same
    // Engine configuration. deserialize() skips all compilation — it just
    // maps the native code into memory.
    let pairs = [
        ("auth", AUTH),
        ("storage", STORAGE),
        ("validator", VALIDATOR),
        ("transform", TRANSFORM),
    ];

    for (name, bytes) in pairs {
        let module = unsafe { Module::deserialize(engine, bytes)? };
        blocks.insert(name.to_string(), module);
    }

    Ok(blocks)
}
```

`Module::deserialize` is `unsafe` because Wasmtime trusts the bytes are a valid pre-compiled module from the same engine version. Since `build.rs` produces them during the same `cargo build`, this is guaranteed.

**What "~1ms" means**: No parsing, no validation, no compilation. It essentially maps native code into memory and sets up function pointers.

### Step 5: Each Block Gets Its Own Sandbox

```rust
fn instantiate_block(
    engine: &Engine,
    module: &Module,
    wafer_ctx: WaferContext,
) -> anyhow::Result<Instance> {
    let mut store = Store::new(engine, wafer_ctx);
    let mut linker = Linker::new(engine);

    // Register host functions — each instance gets its own
    linker.func_wrap("wafer", "send",
        |mut caller: Caller<'_, WaferContext>, msg_ptr: i32, msg_len: i32| -> (i32, i32) {
            // Read msg from THIS instance's memory
            // Dispatch via THIS instance's WaferContext
            // Write result back to THIS instance's memory
        })?;

    linker.func_wrap("wafer", "capabilities",
        |caller: Caller<'_, WaferContext>| -> (i32, i32) {
            // Query THIS instance's available capabilities
        })?;

    linker.func_wrap("wafer", "is_cancelled",
        |caller: Caller<'_, WaferContext>| -> i32 {
            // Check THIS instance's cancellation token
        })?;

    // Each Instance has isolated memory and stack
    let instance = linker.instantiate(&mut store, module)?;
    Ok(instance)
}
```

Each `Instance` has:
- **Its own linear memory** — block A cannot address block B's memory
- **Its own call stack** — no cross-block stack access
- **Its own `Store`/`WaferContext`** — controls what capabilities each block can access
- **WASM's formal guarantee** — a module can only touch memory it was given

### Stripping Cranelift from the Release Binary

Since all blocks are pre-compiled at build time, the release binary **doesn't need the compiler at all**:

```toml
[dependencies]
wasmtime = { version = "29", default-features = false, features = ["runtime"] }
#                              ^^^^^^^^^^^^^^^^^^^^     ^^^^^^^^^^^^^^^^^^^^^
#                              Remove Cranelift,        Keep only the loader
#                              Component Model,         that runs pre-compiled
#                              WASI, threads, GC        native code
```

This drops Wasmtime's contribution from **~19 MB → ~1.2 MB**. The trade-off: you can't load new `.wasm` files at runtime (only pre-compiled `.cwasm`). For a self-contained deployment, that's exactly what you want.

To make this work, `build.rs` needs Cranelift as a build dependency while the final binary does not:

```toml
[build-dependencies]
wasmtime = "29"  # Full features — Cranelift available for pre-compilation

[dependencies]
wasmtime = { version = "29", default-features = false, features = ["runtime"] }
```

### Wasmtime Feature Flags (Size Impact)

| Feature | Default? | What Disabling Removes |
|---------|----------|----------------------|
| `cranelift` | Yes | The entire optimizing compiler (~largest saving) |
| `component-model` | Yes | Component Model support (canonical ABI) |
| `winch` | Yes | Baseline single-pass compiler |
| `wasi-http` | Yes (CLI) | HTTP support via tokio + hyper |
| `parallel-compilation` | Yes | Removes rayon dependency |
| `cache` | Yes | Module caching infrastructure |
| `wat` | Yes | Text-format parser (only binary `.wasm` accepted) |
| `async` | Yes | Async runtime support |
| `gc` | Yes | Garbage collection proposals |
| `threads` | Yes | WASM threads proposal |

### Hot-Reload for Development

During development, skip pre-compilation and JIT-compile from `.wasm` files directly:

```rust
fn load_block(engine: &Engine, name: &str) -> anyhow::Result<Module> {
    if cfg!(debug_assertions) {
        // Dev: JIT compile from .wasm (~100ms, supports hot reload)
        Module::from_file(engine, format!("blocks/{}.wasm", name))
    } else {
        // Prod: load pre-compiled .cwasm (~1ms, embedded in binary)
        let bytes = get_embedded_cwasm(name);
        unsafe { Module::deserialize(engine, bytes) }
    }
}
```

In dev mode the engine needs Cranelift enabled. Use a Cargo feature to toggle:

```toml
[features]
default = ["dev"]
dev = ["wasmtime/cranelift"]  # JIT for development
# Release builds: cargo build --release --no-default-features
```

---

## WASM ABI (Unchanged)

The same ABI from the current Go implementation. Guest modules export:

| Export | Signature | Purpose |
|--------|-----------|---------|
| `info` | `() → (ptr, len)` | Return BlockInfo as JSON |
| `handle` | `(msg_ptr, msg_len) → (result_ptr, result_len)` | Process a message |
| `lifecycle` | `(event_ptr, event_len) → (result_ptr, result_len)` | Handle Init/Start/Stop |
| `malloc` | `(size) → ptr` | Allocate memory for host→guest data |

Host provides (via Wasmtime `Linker`):

| Function | Signature | Purpose |
|----------|-----------|---------|
| `wafer.send` | `(msg_ptr, msg_len) → (result_ptr, result_len)` | Call runtime capabilities |
| `wafer.capabilities` | `() → (list_ptr, list_len)` | Discover capabilities |
| `wafer.is_cancelled` | `() → i32` | Check context cancellation |

Wire format: JSON with `[key, value]` pairs for metadata (same as current).

The ABI is language-agnostic. Any language that can produce a WASM module with these exports works, regardless of whether the runtime is Go or Rust.

---

## Host Function Registration (Wasmtime)

```rust
use wasmtime::*;

fn register_host_functions(linker: &mut Linker<WaferState>) -> anyhow::Result<()> {
    // wafer.send — guest calls runtime capabilities
    linker.func_wrap(
        "wafer", "send",
        |mut caller: Caller<'_, WaferState>, msg_ptr: i32, msg_len: i32| -> (i32, i32) {
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let data = memory.data(&caller)[msg_ptr as usize..(msg_ptr + msg_len) as usize].to_vec();

            let msg: WasmMessage = serde_json::from_slice(&data).unwrap();
            let result = caller.data().context.send(&msg);

            let result_bytes = serde_json::to_vec(&result).unwrap();
            let alloc = caller.get_export("malloc").unwrap().into_func().unwrap();
            let mut results = [Val::I32(0)];
            alloc.call(&mut caller, &[Val::I32(result_bytes.len() as i32)], &mut results).unwrap();
            let result_ptr = results[0].unwrap_i32();

            memory.data_mut(&mut caller)[result_ptr as usize..result_ptr as usize + result_bytes.len()]
                .copy_from_slice(&result_bytes);

            (result_ptr, result_bytes.len() as i32)
        },
    )?;

    // wafer.capabilities
    linker.func_wrap(
        "wafer", "capabilities",
        |mut caller: Caller<'_, WaferState>| -> (i32, i32) {
            let caps = caller.data().context.capabilities();
            let data = serde_json::to_vec(&caps).unwrap();
            // ... write to memory, return (ptr, len)
            (0, 0) // simplified
        },
    )?;

    // wafer.is_cancelled
    linker.func_wrap(
        "wafer", "is_cancelled",
        |caller: Caller<'_, WaferState>| -> i32 {
            if caller.data().context.is_cancelled() { 1 } else { 0 }
        },
    )?;

    Ok(())
}
```

---

## Multi-Language Guest SDKs

All SDKs target the same WASM ABI. The runtime doesn't know or care what language produced the `.wasm` file.

### Go Guest SDK

Location: `wafer-go/guest/` (stays in Go, published as a Go module)

```go
package main

import "github.com/aspect/wafer-go/guest"

type MyBlock struct{}

func (b *MyBlock) Info() guest.BlockInfo {
    return guest.BlockInfo{
        Name:      "my-block",
        Version:   "0.1.0",
        Interface: "processor@v1",
        Summary:   "Go block",
    }
}

func (b *MyBlock) Handle(ctx guest.Context, msg *guest.Message) guest.Result {
    ctx.Log("info", "hello from Go")
    return msg.JSONRespond(200, map[string]string{"status": "ok"})
}

func (b *MyBlock) Lifecycle(_ guest.Context, _ guest.LifecycleEvent) error { return nil }

func main() { guest.Register(&MyBlock{}) }
```

Build: `GOOS=wasip1 GOARCH=wasm go build -o my-block.wasm .`

JSON handling: `encoding/json` works with standard Go. For smaller binaries via TinyGo, use `CosmWasm/tinyjson` (code-gen, no reflection) or `tidwall/gjson` (read-only parser).

### Rust Guest SDK

Location: `wafer-guest-rs/` (separate crate, published to crates.io)

```rust
use wafer_sdk::*;

struct MyBlock;

impl Block for MyBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "my-block",
            version: "0.1.0",
            interface_name: "processor@v1",
            summary: "Rust block",
            instance_mode: InstanceMode::PerNode,
        }
    }

    fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result {
        ctx.log("info", "hello from Rust");
        msg.json_respond(200, &serde_json::json!({"status": "ok"}))
    }

    fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), String> {
        Ok(())
    }
}

wafer_guest::register!(MyBlock);
```

Build: `cargo build --target wasm32-wasip1 --release`

JSON handling: serde — excellent, built-in derive macros, the gold standard.

### AssemblyScript Guest SDK

Location: `wafer-guest-as/` (separate package, published to npm)

```typescript
import { wafer } from "@aspect/wafer-as-guest";

class MyBlock extends wafer.Block {
    info(): wafer.BlockInfo {
        return {
            name: "my-block",
            version: "0.1.0",
            interfaceName: "processor@v1",
            summary: "AssemblyScript block",
            instanceMode: "per_node",
        };
    }

    handle(ctx: wafer.Context, msg: wafer.Message): wafer.Result {
        ctx.log("info", "hello from AssemblyScript");
        return msg.jsonRespond(200, '{"status":"ok"}');
    }
}

wafer.register(new MyBlock());
```

Build: `npx asc assembly/index.ts --target release -o my-block.wasm`

JSON handling: No built-in `JSON.parse()`. The guest SDK provides typed helpers using `assemblyscript-json` internally. Block authors work with typed objects, not raw JSON strings.

**AssemblyScript is NOT TypeScript.** It has TypeScript-like syntax but no union types, no `any`, no closures over mutable variables, no npm package compatibility. It's best for focused blocks (validation, transformation, routing) that don't need the Node.js ecosystem.

### Language Comparison

| | Go | Rust | AssemblyScript |
|---|---|---|---|
| JSON | `encoding/json` (standard Go) or tinyjson (TinyGo) | serde (excellent) | assemblyscript-json (basic) |
| Binary size | 2-3MB (standard) or 100-300KB (TinyGo) | 100KB-1MB | 10-50KB |
| Ecosystem | Full Go stdlib | Full Rust/crates.io | Very limited |
| Build | `go build` | `cargo build` | `npx asc` |
| Best for | General blocks, complex logic | Performance-critical blocks | Lightweight filters/transforms |

---

## SDK Repository Structure

```
wafer/                   ← Rust runtime (main repo)
  src/
    runtime/              ← Core executor, chains, routing, context
    bridge/
      http.rs             ← axum HTTP bridge
      agent.rs            ← MCP agent bridge
      cli.rs              ← CLI bridge
    wasm/                 ← Wasmtime block loader
    manifest/             ← Block manifests, tool schema generator
  Cargo.toml

wafer-sdk-go/            ← Go SDK (separate repo → Go module)
  guest.go
  types.go
  context.go
  go.mod

wafer-sdk-rs/            ← Rust SDK (separate repo → crates.io)
  src/lib.rs
  Cargo.toml

wafer-sdk-as/            ← AssemblyScript SDK (separate repo → npm)
  assembly/index.ts
  package.json
```

Why separate repos:
- Each SDK publishes to its language's package registry (Go modules, crates.io, npm)
- Independent versioning per language
- The WASM ABI contract is the stable interface between them
- The Go SDK is a Go module, cannot live inside a Rust Cargo workspace

---

## HTTP Bridge (axum)

```rust
use axum::{extract::State, http::StatusCode, response::IntoResponse, Router};
use rust_embed::Embed;
use std::sync::Arc;

#[derive(Embed, Clone)]
#[folder = "frontend/dist/"]
struct FrontendAssets;

struct AppState {
    runtime: WaferRuntime,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let runtime = WaferRuntime::new()?;
    let state = Arc::new(AppState { runtime });

    let app = Router::new()
        .route("/api/*path", axum::routing::any(handle_api))
        .route("/health", axum::routing::get(|| async { "ok" }))
        .fallback(static_handler)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn handle_api(
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    // Convert HTTP request → WAFER Message
    // Execute through chain
    // Convert Result → HTTP response
    // Same pattern as the current Go bridge
}

async fn static_handler(uri: axum::http::Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match FrontendAssets::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (StatusCode::OK, [("content-type", mime.as_ref().to_owned())], file.data.into_owned()).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
```

---

## Edge Deployment

### The CF Workers Problem

**Wasmtime cannot run inside Cloudflare Workers.** Wasmtime generates native machine code — it can't compile to WASM itself. This means the "Rust runtime + Go WASM blocks on CF Workers" model doesn't work.

Nested WASM (using `wasmi` interpreter inside a CF Worker) is technically possible but 10-100x slower — impractical for a multi-block processing pipeline.

### Recommended: Native Binary Deployment

Deploy the Rust binary to container-friendly platforms:

| Platform | How | Cold start |
|----------|-----|-----------|
| **Fly.io** | `fly deploy` with Dockerfile | ~200ms (with pre-compiled blocks) |
| **Railway** | Git push | ~200ms |
| **Any Docker host** | Single binary in scratch container | ~200ms |
| **Bare metal** | Just copy the binary | Instant |

The single binary includes everything — runtime, blocks, frontend, services. No external dependencies at runtime.

### Future: Component Model on Edge

When edge platforms adopt the WASM Component Model (Fermyon Cloud already does, others following), WAFER blocks could deploy natively as components without a custom runtime. This is the long-term direction the ecosystem is heading.

---

## Rust Crate Dependencies

Core runtime (release — no compiler):

| Crate | Purpose | Notes |
|-------|---------|-------|
| `wasmtime` (runtime-only) | WASM execution | `default-features = false, features = ["runtime"]` → ~1.2 MB |
| `axum` + `tokio` | HTTP server | Standard Rust web stack |
| `serde` + `serde_json` | JSON serialization | Standard |
| `rust-embed` | Static file embedding | Equivalent of `go:embed` |
| `tracing` | Structured logging | Replaces Go's `log` package |
| `anyhow` / `thiserror` | Error handling | Standard Rust patterns |

Build dependencies (used only by build.rs, not in final binary):

| Crate | Purpose | Notes |
|-------|---------|-------|
| `wasmtime` (full) | Pre-compilation | Cranelift compiles .wasm → .cwasm at build time |

Services (optional, depends on deployment):

| Crate | Purpose |
|-------|---------|
| `rusqlite` | SQLite database |
| `sqlx` | PostgreSQL/MySQL |
| `aws-sdk-s3` | S3 storage |
| `argon2` | Password hashing |
| `jsonwebtoken` | JWT signing/verification |

---

## Binary Size Estimates

### Wasmtime Runtime Size

Wasmtime's binary footprint depends heavily on which features are enabled:

| Configuration | Wasmtime Size |
|---------------|---------------|
| Default features (Cranelift + Component Model + all) | ~19 MB |
| `--no-default-features`, features = `["runtime"]` | ~2.1 MB |
| + LTO + `panic=abort` + strip | **~1.2 MB** |
| + nightly `-Zbuild-std` | **~698 KB** |

For production (pre-compiled blocks, no JIT needed), use the **~1.2 MB** configuration.

### Full Binary Breakdown

With `wasmtime = { default-features = false, features = ["runtime"] }` + LTO:

| Component | Raw .wasm | Pre-compiled .cwasm | Notes |
|-----------|-----------|-------------------|-------|
| Wasmtime runtime (loader only) | — | — | ~1.2 MB |
| axum + tokio + serde | — | — | ~3-5 MB |
| Each Go block (standard Go) | ~2-3 MB | ~5-7 MB | 2-3x expansion for native code |
| Each Go block (TinyGo) | ~200-500 KB | ~500 KB-1.5 MB | Smaller source, same expansion |
| Each Rust block | ~100 KB-1 MB | ~200 KB-2 MB | Smallest practical blocks |
| Each AssemblyScript block | ~10-50 KB | ~30-150 KB | Tiny source, tiny output |
| Frontend assets | — | — | ~1-5 MB (gzipped in binary) |

### Example Totals

| Scenario | Estimated Binary |
|----------|-----------------|
| 2 Go blocks + 2 AS blocks + frontend | ~18-25 MB |
| 5 TinyGo blocks + frontend | ~10-15 MB |
| 5 Rust blocks + frontend | ~8-14 MB |
| Mixed (2 Go + 2 Rust + 1 AS) + frontend | ~20-28 MB |

### Build Profile for Minimum Size

```toml
[profile.release]
opt-level = "s"        # Optimize for size
lto = true             # Link-time optimization (critical — saves ~40%)
codegen-units = 1      # Single codegen unit (better LTO)
panic = "abort"        # No unwinding machinery
strip = "debuginfo"    # Strip debug info (or strip = true for symbols too)
```

Pre-compiled `.cwasm` blocks are ~2-3x larger than raw `.wasm` but load in **~1ms instead of ~100ms**. This is the right trade-off for server deployment. A Docker scratch container with just the binary works well.

---

## Migration Path from Go

### Phase 1: Rust Runtime Core

Rewrite the core runtime in Rust:
- Types: Message, Result, Action, BlockInfo, Chain, Node
- Executor: executeNode, executeChainRef, executeFirstMatch
- Registry: block registration, factory pattern
- Context: Send, Capabilities, Done, Services
- Config: chain/node parsing from JSON

### Phase 2: WASM Block Loader

Replace wazero loader with Wasmtime:
- Load .wasm files, validate required exports
- Register host functions (wafer.send, etc.)
- Handle message serialization across WASM boundary
- Pre-compilation support

### Phase 3: HTTP Bridge

Rewrite the HTTP bridge using axum:
- httpToMessage → Message conversion
- Chain execution
- writeHTTPResponse → axum response
- Static file serving via rust-embed

### Phase 4: Guest SDKs

- Go guest SDK (`wafer-go-guest/`) — thin wrapper for `go:wasmexport`
- Rust guest SDK (`wafer-guest-rs/`) — `register!` macro + serde
- AssemblyScript guest SDK (`wafer-guest-as/`) — npm package

### Phase 5: Tool Generation

- Tool schema generator (manifest → MCP/OpenAI/OpenAPI)
- Agent bridge (MCP server)
- CLI bridge (`wafer call`)

### Phase 6: Solobase Migration

Solobase blocks compile to WASM (via standard Go or TinyGo) and load into the Rust runtime. Solobase becomes a set of WASM blocks + a chain configuration + a frontend, all embedded in a single Rust binary.

---

## Comparison with Similar Projects

| Project | Runtime | Language | Approach |
|---------|---------|----------|----------|
| **Fermyon Spin** | Wasmtime | Rust | Component Model, edge-focused |
| **Extism** | Wasmtime/wazero | Rust kernel | Plugin framework, bytes in/out |
| **Dagger** | Containers | Go | GraphQL API, container-per-module |
| **WAFER** | Wasmtime | Rust | Block/chain composition, multi-bridge |

WAFER's differentiation: blocks compose into chains with routing, and the same blocks are automatically exposed as HTTP APIs, CLI tools, and AI-agent tools. No other framework does the multi-bridge generation from a single block definition.
