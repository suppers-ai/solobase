# Custom WASM Blocks on Cloudflare Workers

## Goal

Allow users to run custom blocks (written in Rust, Go, or TypeScript) on the Cloudflare Worker deployment of Solobase.

## Approach: Wasm Component Model + `jco`

The CF Worker runs inside V8. Instead of embedding a WASM interpreter (`wasmi`) inside the Worker WASM binary—which causes severe double-interpretation overhead—we will utilize the **Wasm Component Model**. 

By compiling guest blocks to `.wit` defined Components, we can use the Bytecode Alliance's [`jco`](https://github.com/bytecodealliance/jco) toolchain to transpile the Component into native JavaScript + standard Core Wasm. 

This allows Cloudflare's V8 engine to natively instantiate the component as a standard ES Module, completely eliminating the need for complex manual memory bridging and serialization boilerplate.

### Flow

1. User writes a block in Rust, Go, or TypeScript targeting the `wafer` `.wit` Component interface.
2. User compiles the block to a `.wasm` Component.
3. The Solobase CLI runs `jco transpile block.wasm -o out/` to generate the `.js` glue code and `.core.wasm` file.
4. The transpiled assets are uploaded to R2 (or bundled with the Worker depending on deployment strategy).
5. On request, the Worker dynamically imports the generated JS module, which natively instantiates the Wasm via V8 with auto-managed memory lifting/lowering.

### ABI (The Component Model)

The custom thin-ABI (`__wafer_handle`, `__wafer_alloc`, JSON over memory) is **deprecated**. 

We now strictly follow the Wasm Component Model defined in `wafer-run/wit/wit/`.
Guest blocks use `wit-bindgen` to safely and natively ingest structured data (like `Message`) and return structured responses (`BlockResult`), without any JSON serialization overhead.

### Implementation Tasks

- [ ] Define the official `.wit` interface for Wafer Blocks (`wafer-run/wit/`)
- [ ] Implement a `wit-bindgen` based guest SDK for Rust (`wafer-block`)
- [ ] Migrate the standalone engine (`wafer-run`) to `wasmtime` for native Component execution
- [ ] Implement `jco` transpilation in `solobase-cli` for uploading custom blocks to Cloudflare
- [ ] Update `solobase-cloudflare` Worker router to dynamically `import()` the `jco`-generated ES Modules from R2/KV.

### Why not `wasmi` or manual `wasm-bindgen`?

- **`wasmi`**: Running an interpreter inside a Cloudflare Wasm environment creates double-interpretation overhead, consuming the limited CPU time threshold (10-50ms) almost instantly.
- **Manual `wasm-bindgen`**: Writing custom host imports to bridge memory between V8 and the Worker's Rust code is error-prone, fundamentally defeats the purpose of the Component Model, and requires expensive JSON serialization. `jco` generates all of this bridging code automatically, fully optimized for V8.
