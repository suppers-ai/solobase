# WAFER Tools — Design Doc

Turn every WAFER block into a multi-target tool automatically. One block definition produces an HTTP API, CLI command, AI-agent tool, and client SDK — with auth, validation, and persistence handled by the runtime.

## Current State

WAFER already has the building blocks:

1. **Transport-agnostic blocks** — blocks receive `*Message` and return `Result`. They don't know if the caller is HTTP, CLI, or an AI agent.
2. **Block manifests (`block.json`)** — declare message input/output schemas and database collections. These are most of a tool definition already.
3. **Interface definitions** — JSON Schema-based method contracts in the spec.
4. **HTTP bridge** — proves the pattern: convert external format → Message → flow → Result → external format.
5. **Standalone runtime** — `wafer-go` is its own Go module, decoupled from Solobase.

No blocks need to change. We add new bridges and a schema generator.

## Architecture

```
                    ┌─────────────────────────────────┐
                    │         WAFER RUNTIME           │
                    │                                  │
                    │   Block.Handle(ctx, msg) → Result │
                    │   Flows compose blocks            │
                    │   Context provides services       │
                    └────────────┬──────────────────────┘
                                 │
                    ┌────────────┴──────────────────────┐
                    │          BRIDGE LAYER              │
                    │                                    │
          ┌─────────┼──────────┬──────────┬─────────────┤
          │         │          │          │              │
     HTTP Bridge  Agent    CLI Bridge  SDK Generator  OpenAPI
     (exists)     Bridge   (new)       (new)          Generator
          │         │          │          │            (new)
          ▼         ▼          ▼          ▼              ▼
       REST API   MCP Tool   CLI Cmd   Go/TS/Py      Spec file
                  (stdio/    $ wafer   Client
                   SSE)      call ...
```

Each bridge does the same thing the HTTP bridge already does:

1. Convert inbound format → `*wafer.Message` (set Kind, Data, Meta)
2. Execute through a flow: `w.Execute(flowID, msg)`
3. Convert `wafer.Result` → outbound format

## What Exists Today

### Message (wafer-go/types.go)

```go
type Message struct {
    Kind string            `json:"kind"`
    Data []byte            `json:"data"`
    Meta map[string]string `json:"meta"`
}

// Convenience methods on *Message:
// msg.Action()     → meta["req.action"] ("retrieve", "create", "update", "delete")
// msg.Path()       → meta["req.resource"]
// msg.Var("id")    → meta["req.param.id"]
// msg.Query("page")→ meta["req.query.page"]
// msg.UserID()     → meta["auth.user_id"]
// msg.Decode(&v)   → unmarshal Data into struct
```

### Block Interface (wafer-go/block.go)

```go
type Block interface {
    Info() BlockInfo
    Handle(ctx Context, msg *Message) Result
    Lifecycle(ctx Context, event LifecycleEvent) error
}

type BlockInfo struct {
    Name         string
    Version      string
    Interface    string        // e.g., "database@v1"
    Summary      string
    InstanceMode InstanceMode
    AllowedModes []InstanceMode
    AdminUI      *AdminUIInfo
}
```

### Block Manifest (wafer-go/manifest/manifest.go)

```go
type BlockManifest struct {
    Name     string            `json:"name"`
    Version  string            `json:"version"`
    Message  *MessageManifest  `json:"message,omitempty"`
    Services *ManifestServices `json:"services,omitempty"`
}

type MessageManifest struct {
    Input  map[string]any `json:"input,omitempty"`   // what the block reads
    Output map[string]any `json:"output,omitempty"`  // what the block writes
}

type ManifestServices struct {
    Database *DatabaseManifest `json:"database,omitempty"`
    Storage  *StorageManifest  `json:"storage,omitempty"`
    Crypto   *CryptoManifest   `json:"crypto,omitempty"`
}
```

### Router (wafer-go/router.go)

Blocks use a `Router` to declare their operations:

```go
r := wafer.NewRouter()
r.Retrieve("/products",     b.handleList)
r.Retrieve("/products/{id}", b.handleGet)
r.Create("/products",       b.handleCreate)
r.Update("/products/{id}",  b.handleUpdate)
r.Delete("/products/{id}",  b.handleDelete)
```

These action+pattern pairs are the natural source for tool definitions.

### HTTP Bridge (wafer-go/bridge/http.go)

The existing bridge converts HTTP → Message → flow → HTTP:

```go
// httpToMessage: sets msg.Kind, msg.Data, msg.Meta (req.*, http.*)
// writeHTTPResponse: reads Result, writes status/headers/body
func WaferHandler(w *wafer.Wafer, flowID string) http.HandlerFunc
```

---

## Components To Build

### 1. Tool Schema Generator

Read block manifests + router registrations → emit tool definitions in multiple formats.

**Input sources:**

| Source | What it provides |
|--------|-----------------|
| `BlockInfo` | Name, summary, interface |
| `block.json` message manifest | Input/output field schemas |
| `block.json` database manifest | Collection fields → CRUD parameter schemas |
| `Router` action registrations | Available operations + path patterns |

**Output targets:**

| Target | Format | Use case |
|--------|--------|----------|
| MCP | `tools/list` JSON-RPC response | AI agents (Claude, etc.) |
| OpenAI | Function-calling JSON schema | ChatGPT, compatible APIs |
| OpenAPI | OpenAPI 3.x spec | Documentation, client generation |
| CLI | Cobra/subcommand definitions | `wafer call <tool> <args>` |

**Example — from manifest to tool schema:**

Given `block.json`:
```json
{
  "name": "products",
  "version": "0.1.0",
  "message": {
    "input": {
      "action": "string",
      "resource": "string",
      "body?": {
        "name": "string",
        "price": "number",
        "description?": "string"
      }
    }
  },
  "services": {
    "database": {
      "collections": {
        "products": {
          "fields": {
            "id":    { "type": "string", "primary": true },
            "name":  { "type": "string" },
            "price": { "type": "float" },
            "description": { "type": "string", "optional": true }
          }
        }
      }
    }
  }
}
```

Generated MCP tool definitions:
```json
[
  {
    "name": "products_list",
    "description": "List products with optional filters",
    "inputSchema": {
      "type": "object",
      "properties": {
        "limit":  { "type": "integer", "description": "Max results to return" },
        "offset": { "type": "integer", "description": "Results to skip" }
      }
    }
  },
  {
    "name": "products_create",
    "description": "Create a new product",
    "inputSchema": {
      "type": "object",
      "properties": {
        "name":        { "type": "string" },
        "price":       { "type": "number" },
        "description": { "type": "string" }
      },
      "required": ["name", "price"]
    }
  },
  {
    "name": "products_get",
    "description": "Get a product by ID",
    "inputSchema": {
      "type": "object",
      "properties": { "id": { "type": "string" } },
      "required": ["id"]
    }
  },
  {
    "name": "products_update",
    "description": "Update a product by ID",
    "inputSchema": {
      "type": "object",
      "properties": {
        "id":          { "type": "string" },
        "name":        { "type": "string" },
        "price":       { "type": "number" },
        "description": { "type": "string" }
      },
      "required": ["id"]
    }
  },
  {
    "name": "products_delete",
    "description": "Delete a product by ID",
    "inputSchema": {
      "type": "object",
      "properties": { "id": { "type": "string" } },
      "required": ["id"]
    }
  }
]
```

**Location:** `wafer-go/manifest/tools.go`

### 2. Agent Bridge

Like the HTTP bridge, but for tool calls. Converts tool name + arguments → Message → flow → tool result.

```go
// wafer-go/bridge/agent.go

type ToolCall struct {
    Name      string          // e.g., "products_create"
    Arguments json.RawMessage // e.g., {"name": "Widget", "price": 10}
}

type ToolResult struct {
    Content string // JSON response body
    IsError bool
}

func HandleToolCall(w *wafer.Wafer, call ToolCall, authMeta map[string]string) ToolResult {
    // 1. Parse tool name → block + action
    //    "products_create" → block="products", action="create"
    block, action := parseToolName(call.Name)

    // 2. Build Message
    msg := &wafer.Message{
        Kind: string(action) + ":/" + block,
        Data: call.Arguments,
        Meta: map[string]string{
            "req.action":   string(action),
            "req.resource": "/" + block,
        },
    }
    // Copy auth context (agent's identity)
    for k, v := range authMeta {
        msg.Meta[k] = v
    }

    // 3. Execute through flow (same flows as HTTP)
    result := w.Execute(block, msg)

    // 4. Convert result
    switch result.Action {
    case wafer.ActionRespond:
        return ToolResult{Content: string(result.Response.Data)}
    case wafer.ActionError:
        return ToolResult{
            Content: fmt.Sprintf(`{"error":"%s","message":"%s"}`, result.Error.Code, result.Error.Message),
            IsError: true,
        }
    case wafer.ActionContinue:
        return ToolResult{Content: string(result.Message.Data)}
    default: // Drop
        return ToolResult{Content: `{"status":"ok"}`}
    }
}
```

**Location:** `wafer-go/bridge/agent.go`

### 3. MCP Server

[MCP (Model Context Protocol)](https://modelcontextprotocol.io/) — the standard for exposing tools to AI agents. JSON-RPC over stdio or SSE.

Methods to implement:
- `initialize` — server capabilities
- `tools/list` — return generated tool schemas from manifests
- `tools/call` — route through agent bridge → flow → tool result

**Option A: Separate binary (stdio)**
```
wafer-go/cmd/wafer-mcp/main.go
```

Agents launch it as a subprocess. Simplest, most composable.

**Option B: Embedded endpoint (SSE)**
```
wafer-go/bridge/mcp.go
```

Runs alongside HTTP server. Better for remote agents.

Recommend starting with Option A — it's the standard MCP pattern and keeps things simple.

### 4. CLI Bridge

A `wafer` CLI that can invoke any block directly:

```bash
# List available tools
wafer tools

# Call a tool
wafer call products.list --limit 10
wafer call products.create --name "Widget" --price 10
wafer call products.get --id abc123

# Generate schemas
wafer generate mcp         # MCP tool definitions
wafer generate openapi     # OpenAPI spec
wafer generate openai      # OpenAI function schemas
```

The CLI bridge converts command-line args → Message → flow → printed output. Same pattern as HTTP and agent bridges.

**Location:** `wafer-go/cmd/wafer/main.go`

### 5. Flow-as-Tool (Compound Tools)

Flows that compose multiple blocks become compound tools with auto-generated schemas:

```
flow "onboard-user" = validate → create-user → assign-role → send-welcome
```

The tool schema is derived from:
- **Input:** the first block's input schema
- **Output:** the last block's output schema
- **Description:** the flow's `Summary` field

This lets AI agents call high-level workflows without knowing the individual blocks. Developers get the same benefit via CLI.

**Manifest extension:**
```json
{
  "id": "onboard-user",
  "summary": "Create a new user with role assignment and welcome email",
  "tool": {
    "expose": true,
    "name": "onboard_user",
    "description": "Onboard a new user with default role and welcome email"
  }
}
```

---

## Auth for Tools

Agents and CLI users authenticate the same way HTTP clients do. The flow system already handles auth via `auth-pipe`.

| Caller | Auth method |
|--------|------------|
| HTTP | JWT cookie or Authorization header |
| MCP agent | API key passed in MCP request metadata |
| CLI | API key from config file or env var |
| SDK | API key or JWT passed in client constructor |

The agent bridge sets `auth.*` meta keys before flow execution. Auth blocks validate as usual.

For local MCP servers (agent launches subprocess), the simplest option is running with a fixed user context — the API key is in the MCP server config.

---

## Which Blocks to Expose

Not every block should be a tool. Admin blocks (users, settings, IAM) probably shouldn't be exposed to AI agents by default.

Add a `tool` field to block manifests:

```json
{
  "name": "products",
  "tool": {
    "expose": true,
    "description": "Manage products catalog"
  }
}
```

Defaults to `false`. Block authors opt in. The schema generator skips blocks where `tool.expose` is false.

For flows, the same field on the flow definition controls exposure.

---

## Tool Granularity

**One tool per action** (products_list, products_create, etc.):
- Maps directly to router registrations
- Each tool has a focused, specific schema
- Works well with AI agents that prefer precise tools

This is the default. The generator creates one tool per Router action registration.

For blocks or flows that want a single tool with an `action` parameter, the manifest can override:

```json
{
  "tool": {
    "expose": true,
    "style": "single"
  }
}
```

---

## Implementation Order

| Phase | Component | Location | Effort |
|-------|-----------|----------|--------|
| 1 | Tool schema generator | `wafer-go/manifest/tools.go` | Small |
| 2 | Agent bridge | `wafer-go/bridge/agent.go` | Small |
| 3 | MCP stdio server | `wafer-go/cmd/wafer-mcp/main.go` | Medium |
| 4 | CLI bridge + `wafer call` | `wafer-go/cmd/wafer/main.go` | Medium |
| 5 | Manifest `tool` field | `wafer-go/manifest/manifest.go` | Small |
| 6 | Flow-as-tool schemas | `wafer-go/manifest/tools.go` | Small |
| 7 | OpenAPI generator | `wafer-go/manifest/openapi.go` | Medium |
| 8 | SDK generator (Go/TS) | `wafer-go/cmd/wafer/generate.go` | Large |

Phases 1-3 get the core value: write a block, get an AI tool. Phases 4-8 extend it to other consumers.

---

## The Value Prop

Write a WAFER block → automatically get:
- An HTTP API (existing bridge)
- An AI-agent tool via MCP (agent bridge)
- A CLI command (CLI bridge)
- An OpenAPI spec (generator)
- Client SDKs (generator)

Same block code, same flows, same auth. Five interfaces, no extra work for block authors.

---

## Multi-Language Blocks

Blocks can be written in any language that compiles to WASM, not just Go. The runtime already has a fully working WASM loader (`wafer-go/wasm/`) built on wazero (pure Go, zero CGO). The approach is to keep this existing infrastructure and provide thin **guest SDKs** per language that wrap the WASM ABI.

No external plugin frameworks needed. No new dependencies.

### What Already Works

The WASM loader (`wafer-go/wasm/`) is fully implemented:

**WASM ABI contract — guest must export:**

| Export | Signature | Purpose |
|--------|-----------|---------|
| `info` | `() → (ptr, len)` | Return BlockInfo as JSON |
| `handle` | `(msg_ptr, msg_len) → (result_ptr, result_len)` | Process a message, return result |
| `lifecycle` | `(event_ptr, event_len) → (result_ptr, result_len)` | Handle Init/Start/Stop events |
| `malloc` | `(size) → ptr` | Allocate memory for host→guest data |

**Host module `"wafer"` — runtime provides to guest:**

| Function | Signature | Purpose |
|----------|-----------|---------|
| `send` | `(msg_ptr, msg_len) → (result_ptr, result_len)` | Call runtime capabilities (log, config, etc.) |
| `capabilities` | `() → (list_ptr, list_len)` | Discover available capabilities |
| `is_cancelled` | `() → i32` | Check if context is cancelled |

**Wire format:** All data crosses the WASM boundary as JSON. Messages use `[key, value]` pairs for metadata (instead of maps) for WASM compatibility:

```json
// Message (host → guest)
{
  "kind": "create:/products",
  "data": [98, 111, 100, 121],
  "meta": [["req.action", "create"], ["auth.user_id", "u123"]]
}

// Result (guest → host)
{
  "action": "respond",
  "response": { "data": [123, 125], "meta": [["resp.status", "201"]] }
}

// BlockInfo (guest → host)
{
  "name": "my-block",
  "version": "0.1.0",
  "interface": "processor@v1",
  "summary": "Processes things",
  "instance_mode": "per_node"
}
```

### Go Guest SDK

With Go 1.24+ (`go:wasmexport`), block authors write standard Go — no TinyGo needed. The guest SDK is a thin wrapper that handles JSON serialization and memory management.

**What block authors write:**

```go
package main

import "github.com/suppers-ai/wafer-go/guest"

type MyBlock struct{}

func (b *MyBlock) Info() guest.BlockInfo {
    return guest.BlockInfo{
        Name:      "my-block",
        Version:   "0.1.0",
        Interface: "processor@v1",
        Summary:   "Processes things",
    }
}

func (b *MyBlock) Handle(ctx guest.Context, msg *guest.Message) guest.Result {
    // Read input
    var input struct{ Name string }
    msg.Decode(&input)

    // Use runtime capabilities
    ctx.Log("info", "processing: " + input.Name)

    // Return response
    return msg.JSONRespond(200, map[string]string{"status": "ok"})
}

func (b *MyBlock) Lifecycle(ctx guest.Context, event guest.LifecycleEvent) error {
    return nil
}

func main() {
    guest.Register(&MyBlock{})
}
```

**Build:**
```bash
GOOS=wasip1 GOARCH=wasm go build -o my-block.wasm .
```

**What the guest SDK does internally:**

```go
// guest/guest.go — the thin SDK

// Register sets up the WASM exports. Called from main().
func Register(block Block) {
    registered = block
}

//go:wasmexport info
func info() (uint32, uint32) {
    bi := registered.Info()
    data, _ := json.Marshal(bi)
    ptr, len := allocAndWrite(data)
    return ptr, len
}

//go:wasmexport handle
func handle(msgPtr, msgLen uint32) (uint32, uint32) {
    // Read message from memory
    data := readMemory(msgPtr, msgLen)
    var wm wasmMessage
    json.Unmarshal(data, &wm)
    msg := fromWASM(wm)

    // Create context with host function access
    ctx := &guestContext{}

    // Call the block
    result := registered.Handle(ctx, msg)

    // Write result back
    resultData, _ := json.Marshal(toWASMResult(result))
    ptr, len := allocAndWrite(resultData)
    return ptr, len
}

//go:wasmexport malloc
func malloc(size uint32) uint32 {
    buf := make([]byte, size)
    return uint32(uintptr(unsafe.Pointer(&buf[0])))
}
```

**Location:** `wafer-go/guest/` (new package)

### Rust Guest SDK

Rust compiles to WASM natively and is the strongest non-Go option. The Rust SDK implements the same ABI.

**What block authors write:**

```rust
use wafer_guest::*;

struct MyBlock;

impl Block for MyBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "my-rust-block",
            version: "0.1.0",
            interface_name: "processor@v1",
            summary: "Rust block",
            instance_mode: InstanceMode::PerNode,
        }
    }

    fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result {
        ctx.log("info", "hello from rust");
        msg.json_respond(200, &serde_json::json!({"status": "ok"}))
    }

    fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), String> {
        Ok(())
    }
}

wafer_guest::register!(MyBlock);
```

**Build:**
```bash
cargo build --target wasm32-wasip1 --release
```

**What the Rust SDK does internally:**
- `register!` macro generates the `#[no_mangle] pub extern "C" fn info/handle/lifecycle/malloc` exports
- JSON serialization via serde
- Host function calls via `extern "C"` imports from the `"wafer"` module
- Memory management via a simple bump allocator

**Location:** `wafer-rust-guest/` (separate crate, published to crates.io)

### Loading WASM Blocks

The existing loader (`wafer-go/wasm/LoadWASMBlock`) already handles everything. No changes needed.

Block registration in the runtime:

```go
// Load a .wasm file and register it as a block
block, err := wasm.LoadWASMBlock("blocks/my-block.wasm")
if err != nil { ... }
w.RegisterBlock("my-block", block)
```

For auto-discovery, add a convention: the runtime scans a `blocks/` directory for `.wasm` files and loads them alongside native Go blocks.

```go
// wafer-go/wasm/discover.go (new)

func DiscoverBlocks(dir string) (map[string]*WASMBlock, error) {
    // Glob for *.wasm files in dir
    // Load each one
    // Use block.Info().Name as the registry key
}
```

### What Needs Building

| Component | Location | Effort | Description |
|-----------|----------|--------|-------------|
| Go guest SDK | `wafer-go/guest/` | Small | Types + `go:wasmexport` glue (~200 lines) |
| Rust guest SDK | `wafer-rust-guest/` | Medium | Types + `register!` macro + serde (~500 lines) |
| Block discovery | `wafer-go/wasm/discover.go` | Small | Scan directory, load `.wasm` files (~50 lines) |
| Example Go WASM block | `wafer-go/examples/wasm-go/` | Small | Hello-world block compiled to WASM |
| Example Rust block | `wafer-go/examples/wasm-rust/` | Small | Hello-world block in Rust |

### What Does NOT Change

- The WASM loader (`wasm/loader.go`, `memory.go`, `host.go`) — already works
- The Block interface — WASM blocks implement it via the loader
- Flow execution — runtime doesn't know or care if a block is native or WASM
- Bridges (HTTP, agent, CLI) — they call blocks through the same interface
- wazero dependency — stays as-is, pure Go, no CGO

### Language Support Summary

| Language | How | Status |
|----------|-----|--------|
| **Go (native)** | Direct `Block` interface implementation | Existing |
| **Go (WASM)** | `go:wasmexport` + guest SDK → `.wasm` | Go 1.24+, needs guest SDK |
| **Rust** | `wasm32-wasip1` target + guest SDK → `.wasm` | Needs guest SDK |
| **C/C++** | WASI SDK → `.wasm` | Works with existing loader, no SDK yet |
| **AssemblyScript** | Native WASM target → `.wasm` | Works with existing loader, no SDK yet |
| **TinyGo** | `tinygo build -target=wasip1` → `.wasm` | Works today (used by Solobase WASM build) |

The guest SDK is optional — any language that can produce a WASM module with the right exports (`info`, `handle`, `lifecycle`, `malloc`) works. The SDK just makes it ergonomic.

### Why Not Extism / External Frameworks?

Evaluated and decided against:

- **Extism** — maintained but slow development pace, small team (9 employees), single seed round from 2023. Adds a dependency for what amounts to memory management helpers we already have.
- **wasmtime-go** — requires CGO (wraps C API), breaks pure-Go cross-compilation. Component Model support is nice but not worth the trade-off.
- **wasmer-go** — effectively stale, last release ~2022.
- **knqyf263/go-plugin** — interesting (protobuf over wazero) but adds protobuf dependency and codegen for a problem JSON already solves.

The existing wazero-based loader is ~450 lines of working code. The guest SDKs are ~200 lines each. Adding an external framework would add more code than it saves and introduce dependency risk.

### Future: WASM Component Model

When wazero adds WASM Component Model support (tracking the WASI standardization), we can migrate from JSON-over-shared-memory to WIT-defined typed interfaces. This would give us:

- Auto-generated, type-safe guest bindings per language
- No JSON serialization overhead
- Cross-component composition

This is the long-term endgame but is not available in wazero today. The current JSON ABI works well and the migration path is clear: swap the serialization layer inside the guest SDKs, keep the Block interface unchanged.

---

## Full Implementation Order

| Phase | Component | Location | Effort |
|-------|-----------|----------|--------|
| 1 | Tool schema generator | `wafer-go/manifest/tools.go` | Small |
| 2 | Agent bridge | `wafer-go/bridge/agent.go` | Small |
| 3 | MCP stdio server | `wafer-go/cmd/wafer-mcp/main.go` | Medium |
| 4 | Go guest SDK (WASM blocks) | `wafer-go/guest/` | Small |
| 5 | Example Go WASM block | `wafer-go/examples/wasm-go/` | Small |
| 6 | CLI bridge + `wafer call` | `wafer-go/cmd/wafer/main.go` | Medium |
| 7 | Manifest `tool` field | `wafer-go/manifest/manifest.go` | Small |
| 8 | WASM block auto-discovery | `wafer-go/wasm/discover.go` | Small |
| 9 | Rust guest SDK | `wafer-rust-guest/` | Medium |
| 10 | Flow-as-tool schemas | `wafer-go/manifest/tools.go` | Small |
| 11 | OpenAPI generator | `wafer-go/manifest/openapi.go` | Medium |
| 12 | SDK generator (Go/TS) | `wafer-go/cmd/wafer/generate.go` | Large |

Phases 1-3: Core tool generation (write a block, get an AI tool).
Phases 4-5: Multi-language blocks (write a block in any language).
Phases 6-8: Developer experience (CLI, discovery).
Phases 9-12: Ecosystem (more languages, more output formats).
