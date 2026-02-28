---
title: "WASM Blocks"
description: "Extend Solobase with WebAssembly extensions"
weight: 10
tags: ["wasm", "extensions", "webassembly", "blocks"]
---

# WASM Blocks

WASM Blocks are WebAssembly extensions that let you add custom server-side logic to Solobase. They run in a sandboxed environment with fine-grained capability controls.

## What are WASM Blocks?

A WASM block is a compiled WebAssembly module that Solobase loads and executes in a secure sandbox. Blocks can:

- Process API requests with custom logic
- Validate and transform data before storage
- Implement custom authentication flows
- Generate computed fields or aggregations
- Call external APIs and webhooks

Blocks are isolated from each other and from the host system. They communicate through a well-defined API with explicit capability grants.

## Capability Manifest

Each WASM block declares the capabilities it needs in a manifest. Solobase enforces these at runtime -- a block cannot access resources it hasn't been granted.

### Available Capabilities

| Capability | Description | Example |
|-----------|-------------|---------|
| `collections` | Read/write access to database collections | `["products", "orders"]` |
| `storage` | Read/write access to file storage | `["uploads/*"]` |
| `network` | Make outbound HTTP requests | `["api.example.com", "*.stripe.com"]` |
| `crypto` | Cryptographic operations (hashing, signing) | `["sha256", "hmac"]` |

### Example Manifest

```toml
# block.toml
[block]
name = "order-processor"
version = "1.0.0"
entrypoint = "process"

[capabilities]
collections = ["orders", "products", "inventory"]
storage = ["receipts/*"]
network = ["api.stripe.com", "hooks.slack.com"]
crypto = ["sha256", "hmac-sha256"]
```

## Loading WASM Blocks

### From the Filesystem

Place your compiled `.wasm` file and its `block.toml` manifest in the blocks directory:

```
data/blocks/
  order-processor/
    block.toml
    block.wasm
```

Configure the blocks directory in `solobase.toml`:

```toml
[blocks]
dir = "data/blocks"
```

### Via the Admin Dashboard

1. Go to **Extensions** in the admin sidebar
2. Click **Upload Block**
3. Upload your `.wasm` file and `block.toml`
4. Review the requested capabilities
5. Click **Enable**

### Via the API

```bash
curl -X POST http://localhost:8090/api/admin/blocks \
  -H "Authorization: Bearer $TOKEN" \
  -F "wasm=@block.wasm" \
  -F "manifest=@block.toml"
```

## Security Model

WASM blocks run in a strictly sandboxed environment with multiple layers of protection.

### Epoch-Based Timeouts

Every block execution has a maximum runtime enforced by epoch interrupts. If a block exceeds its time budget, it is terminated immediately. Default timeout is 5 seconds per invocation.

```toml
# solobase.toml
[blocks]
timeout_ms = 5000  # Maximum execution time per invocation
```

### Memory Limits

Each block is limited in how much memory it can allocate. Default is 16MB per block instance.

```toml
[blocks]
max_memory_mb = 16
```

### Network Filtering

Outbound network requests are filtered against the block's capability manifest. A block can only reach hosts explicitly listed in its `network` capability. DNS resolution is performed by the host, not the block.

### Capability Enforcement

- **Collections**: A block can only read/write collections listed in its manifest. Attempting to access other collections results in a permission error.
- **Storage**: File access is restricted to paths matching the patterns in the manifest.
- **Crypto**: Only the listed algorithms are available to the block.
- **Network**: Only the listed hostnames can be contacted.

### No Filesystem Access

WASM blocks have no access to the host filesystem. All data access goes through the Solobase API layer, which enforces capabilities.

## Writing a WASM Block

WASM blocks can be written in any language that compiles to WebAssembly. Rust is the recommended choice.

### Rust Example

```rust
use solobase_sdk::*;

#[solobase_block]
fn process(ctx: &Context, req: &Request) -> Result<Response> {
    // Read from a collection
    let product = ctx.collection("products").get(req.param("product_id"))?;

    // Validate
    let price = product.get_f64("price")?;
    if price <= 0.0 {
        return Err(Error::validation("price must be positive"));
    }

    // Build a charge payload and call an external API
    let charge = json!({
        "amount": (price * 100.0) as i64,
        "currency": "usd",
        "source": req.param("token"),
    });
    let _stripe_resp = ctx.http_post("https://api.stripe.com/v1/charges", &charge)?;

    // Create an order record
    let order = json!({
        "product_id": req.param("product_id"),
        "amount": price,
        "status": "paid",
    });
    ctx.collection("orders").insert(&order)?;

    Ok(Response::json(&order))
}
```

### Build

```bash
cargo build --target wasm32-wasi --release
cp target/wasm32-wasi/release/my_block.wasm data/blocks/my-block/block.wasm
```

## Debugging

View block execution logs in the admin dashboard under **Extensions** > **Logs**, or via the API:

```bash
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8090/api/admin/blocks/order-processor/logs
```

## Next Steps

- [Configuration](/docs/configuration/) -- Configure block settings in `solobase.toml`
- [API Reference](/docs/api/database/) -- Understand the collections API that blocks interact with
- [Solobase Cloud](/docs/cloud/) -- Deploy blocks on managed infrastructure
