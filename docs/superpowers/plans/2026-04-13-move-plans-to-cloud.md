# Move solobase-plans to solobase-cloud Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move all plan/billing/addon logic out of the solobase monorepo into solobase-cloud, making the products block plan-agnostic. Addons become regular products with a `requires` dependency field.

**Architecture:** The `solobase-plans` crate moves to `solobase-cloud/crates/`. Addon endpoints are removed — addons are just products with a `requires` field pointing to a prerequisite product (e.g., an addon requires the user to own the "Pro" subscription product). The products block validates `requires` at checkout generically. The subscription status endpoint returns raw data without plan-limit enrichment. Project creation/activation plan-limit enforcement is removed from solobase-core (already enforced at the edge by `solobase-cloudflare/src/usage.rs`). The Stripe webhook addon sync reads addon metadata from Stripe item metadata directly, without referencing `solobase-plans` definitions.

**Tech Stack:** Rust, Cloudflare Workers (workers-rs), D1 (SQLite)

---

### Task 1: Move solobase-plans crate to solobase-cloud

**Files:**
- Move: `solobase/crates/solobase-plans/` -> `solobase-cloud/crates/solobase-plans/`
- Modify: `solobase-cloud/Cargo.toml`
- Modify: `solobase-cloud/crates/solobase-cloudflare/Cargo.toml:28`

- [ ] **Step 1: Copy the crate**

```bash
cp -r solobase/crates/solobase-plans solobase-cloud/crates/solobase-plans
```

- [ ] **Step 2: Add to solobase-cloud workspace**

In `solobase-cloud/Cargo.toml`, add `"crates/solobase-plans"` to workspace members:

```toml
[workspace]
resolver = "2"
members = [
    "crates/solobase-cloudflare",
    "crates/solobase-plans",
    "crates/solobase-worker",
]
```

- [ ] **Step 3: Update solobase-cloudflare dependency path**

In `solobase-cloud/crates/solobase-cloudflare/Cargo.toml`, change the solobase-plans dependency from cross-repo relative path to local:

```toml
# Plan definitions
solobase-plans = { path = "../solobase-plans" }
```

- [ ] **Step 4: Verify solobase-cloud builds**

```bash
cd solobase-cloud && cargo check
```

Expected: compiles successfully.

- [ ] **Step 5: Commit**

```bash
cd solobase-cloud
git add crates/solobase-plans/ Cargo.toml crates/solobase-cloudflare/Cargo.toml
git commit -m "feat: move solobase-plans crate into solobase-cloud"
```

---

### Task 2: Add `requires` field to products collection schema

Add a `requires` field to the products collection so any product can declare a dependency on another product (e.g., an addon product requires the user to own a specific plan product).

**Files:**
- Modify: `solobase/crates/solobase-core/src/blocks/products/mod.rs:60-81`

- [ ] **Step 1: Add requires field to products schema**

In `solobase/crates/solobase-core/src/blocks/products/mod.rs`, add a `requires` field to the products collection schema. Find:

```rust
                    .field_default("pricing_template_id", "string", "")
                    .field_default("created_by", "string", "")
```

Add after `pricing_template_id`:

```rust
                    .field_default("requires", "string", "")
```

The `requires` field holds the ID of a product the user must already own (as an active subscription or completed purchase) before they can buy this product. Empty string means no dependency.

- [ ] **Step 2: Verify solobase builds**

```bash
cd solobase && cargo check
```

Expected: compiles.

- [ ] **Step 3: Commit**

```bash
cd solobase
git add crates/solobase-core/src/blocks/products/mod.rs
git commit -m "feat: add requires dependency field to products schema"
```

---

### Task 3: Validate `requires` at checkout

When a user checks out a product that has a `requires` field, verify they own the required product before creating the Stripe checkout session.

**Files:**
- Modify: `solobase/crates/solobase-core/src/blocks/products/stripe.rs:14-50`

- [ ] **Step 1: Add requires validation to handle_checkout**

In `stripe.rs`, in the `handle_checkout` function, after the purchase ownership check (after line 43: `return err_forbidden(msg, "Cannot checkout another user's purchase");`), add the requires check. First, get the product from the purchase's line items to check its `requires` field:

```rust
    // Check product dependency (requires field)
    let line_items = db::query_raw(
        ctx,
        &format!(
            "SELECT product_id FROM {} WHERE purchase_id = ?1 LIMIT 1",
            super::LINE_ITEMS_COLLECTION
        ),
        &[serde_json::Value::String(body.purchase_id.clone())],
    )
    .await;

    if let Ok(items) = &line_items {
        for item in items {
            let product_id = item.data.get("product_id").and_then(|v| v.as_str()).unwrap_or("");
            if product_id.is_empty() {
                continue;
            }
            if let Ok(product) = db::get(ctx, super::PRODUCTS_COLLECTION, product_id).await {
                let requires = product.data.get("requires").and_then(|v| v.as_str()).unwrap_or("");
                if !requires.is_empty() {
                    let has_required = user_owns_product(ctx, purchase_user, requires).await;
                    if !has_required {
                        // Revert to pending
                        return err_bad_request(
                            msg,
                            "You must own the required product before purchasing this item.",
                        );
                    }
                }
            }
        }
    }
```

Note: the `requires` validation happens before the status transition to `checkout_started`, so we don't need to revert status on failure. Move it before the atomic status transition (before line 55).

- [ ] **Step 2: Add user_owns_product helper**

Add this helper function at the bottom of `stripe.rs` (before the `#[cfg(test)]` block if any):

```rust
/// Check if a user owns a product — either via an active subscription that
/// references it, or a completed purchase containing it as a line item.
async fn user_owns_product(ctx: &dyn Context, user_id: &str, product_id: &str) -> bool {
    // Check subscriptions: the plan field or metadata may reference the product
    let sub_rows = db::query_raw(
        ctx,
        &format!(
            "SELECT 1 FROM {} WHERE user_id = ?1 AND status = 'active' AND plan = ?2 LIMIT 1",
            SUBSCRIPTIONS
        ),
        &[
            serde_json::Value::String(user_id.to_string()),
            serde_json::Value::String(product_id.to_string()),
        ],
    )
    .await;

    if matches!(&sub_rows, Ok(rows) if !rows.is_empty()) {
        return true;
    }

    // Check completed purchases containing this product as a line item
    let purchase_rows = db::query_raw(
        ctx,
        &format!(
            "SELECT 1 FROM {} p JOIN {} li ON li.purchase_id = p.id \
             WHERE p.user_id = ?1 AND p.status = 'completed' AND li.product_id = ?2 LIMIT 1",
            super::PURCHASES_COLLECTION,
            super::LINE_ITEMS_COLLECTION,
        ),
        &[
            serde_json::Value::String(user_id.to_string()),
            serde_json::Value::String(product_id.to_string()),
        ],
    )
    .await;

    matches!(&purchase_rows, Ok(rows) if !rows.is_empty())
}
```

- [ ] **Step 3: Verify solobase builds**

```bash
cd solobase && cargo check
```

- [ ] **Step 4: Commit**

```bash
cd solobase
git add crates/solobase-core/src/blocks/products/stripe.rs
git commit -m "feat: validate product requires dependency at checkout"
```

---

### Task 4: Remove addon endpoints from products block

The addon endpoints (`/b/products/addons`, `/b/products/addons/subscribe`, `/b/products/addons/cancel`) are removed. Addons are now regular products in the catalog — users buy them through the normal checkout flow.

**Files:**
- Delete: `solobase/crates/solobase-core/src/blocks/products/addons.rs`
- Modify: `solobase/crates/solobase-core/src/blocks/products/mod.rs:1,136,145-146`
- Modify: `solobase/crates/solobase-core/src/blocks/products/handlers.rs:172-177`

- [ ] **Step 1: Remove addons module declaration**

In `solobase/crates/solobase-core/src/blocks/products/mod.rs`, remove line 1:

```rust
mod addons;
```

- [ ] **Step 2: Remove addon endpoint registrations**

In `solobase/crates/solobase-core/src/blocks/products/mod.rs`, remove the two addon endpoint entries from the `endpoints` vec (lines 145-146):

```rust
                BlockEndpoint::get("/b/products/addons").summary("List add-on packs").auth(AuthLevel::Authenticated),
                BlockEndpoint::post("/b/products/addons/subscribe").summary("Subscribe to add-on").auth(AuthLevel::Authenticated),
```

- [ ] **Step 3: Update block description**

In `solobase/crates/solobase-core/src/blocks/products/mod.rs`, update the `.description()` to remove addon references (line 136):

```rust
            .description("Product catalog, pricing engine, and payment processing. Manages products, groups, pricing templates with formula evaluation, purchases, and Stripe integration for checkout and recurring subscriptions.")
```

- [ ] **Step 4: Remove addon route handlers**

In `solobase/crates/solobase-core/src/blocks/products/handlers.rs`, remove the addon routing block (lines 172-177):

```rust
        // Add-on packs (recurring subscription items)
        ("retrieve", "/b/products/addons") => super::addons::handle_list(ctx, msg).await,
        ("create", "/b/products/addons/subscribe") => {
            super::addons::handle_subscribe(ctx, msg).await
        }
        ("create", "/b/products/addons/cancel") => super::addons::handle_cancel(ctx, msg).await,
```

- [ ] **Step 5: Delete addons.rs**

```bash
rm solobase/crates/solobase-core/src/blocks/products/addons.rs
```

- [ ] **Step 6: Verify build** (expect failure — stripe.rs still calls `addons::sync_addons_from_stripe`, fixed in Task 5)

```bash
cd solobase && cargo check 2>&1 | head -20
```

---

### Task 5: Decouple Stripe webhook addon sync from solobase-plans

The Stripe webhook handler for `customer.subscription.updated` calls `addons::sync_addons_from_stripe()` which looks up `AddonPack` definitions. Replace it with inline logic that reads addon metadata directly from Stripe subscription item metadata fields.

**Files:**
- Modify: `solobase/crates/solobase-core/src/blocks/products/stripe.rs:357-362`

- [ ] **Step 1: Replace sync_addons_from_stripe call**

In `stripe.rs`, find this code (around line 357):

```rust
            // Sync addon totals from the subscription's items
            let user_id = get_user_for_stripe_sub(ctx, stripe_sub_id).await;
            if let Some(ref uid) = user_id {
                if let Some(items) = data_object.get("items") {
                    super::addons::sync_addons_from_stripe(ctx, uid, items).await;
                }
            }
```

Replace with:

```rust
            // Sync addon totals from Stripe subscription items metadata.
            // Each addon subscription item has metadata fields: extra_projects,
            // extra_requests, extra_r2_bytes, extra_d1_bytes (set when creating
            // the subscription item via Stripe API).
            let user_id = get_user_for_stripe_sub(ctx, stripe_sub_id).await;
            if let Some(ref uid) = user_id {
                if let Some(items) = data_object.get("items") {
                    sync_addon_totals_from_items(ctx, uid, items).await;
                }
            }
```

- [ ] **Step 2: Add sync_addon_totals_from_items function**

Add this function at the bottom of `stripe.rs`:

```rust
/// Sync addon column totals from Stripe subscription items.
///
/// Reads addon values from item metadata (set by the platform when creating
/// subscription items). This keeps the products block plan-agnostic — it
/// doesn't need to know what addon packs exist, just what Stripe reports.
async fn sync_addon_totals_from_items(ctx: &dyn Context, user_id: &str, items: &serde_json::Value) {
    let mut total_projects: i64 = 0;
    let mut total_requests: i64 = 0;
    let mut total_r2: i64 = 0;
    let mut total_d1: i64 = 0;

    if let Some(data) = items.get("data").and_then(|v| v.as_array()) {
        for item in data {
            let meta = item.get("metadata")
                .or_else(|| item.pointer("/price/metadata"));
            let meta = match meta {
                Some(m) => m,
                None => continue,
            };

            // Skip non-addon items (the base plan item won't have addon_id)
            if meta.get("addon_id").is_none() {
                continue;
            }

            let qty = item.get("quantity").and_then(|v| v.as_i64()).unwrap_or(1);
            let parse = |key: &str| -> i64 {
                meta.get(key)
                    .and_then(|v| v.as_str().and_then(|s| s.parse().ok()).or_else(|| v.as_i64()))
                    .unwrap_or(0)
            };
            total_projects += parse("extra_projects") * qty;
            total_requests += parse("extra_requests") * qty;
            total_r2 += parse("extra_r2_bytes") * qty;
            total_d1 += parse("extra_d1_bytes") * qty;
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    let sql = format!(
        "UPDATE {SUBSCRIPTIONS} SET \
           addon_projects = ?1, addon_requests = ?2, \
           addon_r2_bytes = ?3, addon_d1_bytes = ?4, \
           updated_at = ?5 \
         WHERE user_id = ?6 AND status = 'active'"
    );
    let _ = db::exec_raw(
        ctx,
        &sql,
        &[
            serde_json::json!(total_projects),
            serde_json::json!(total_requests),
            serde_json::json!(total_r2),
            serde_json::json!(total_d1),
            serde_json::Value::String(now),
            serde_json::Value::String(user_id.to_string()),
        ],
    )
    .await;
}
```

- [ ] **Step 3: Verify solobase builds**

```bash
cd solobase && cargo check
```

Expected: may still fail due to `crate::plans` usage in `handlers.rs` and `projects/handlers.rs`. Fixed in Tasks 6 and 7.

- [ ] **Step 4: Commit Tasks 4+5 together**

```bash
cd solobase
git add -A crates/solobase-core/src/blocks/products/
git commit -m "refactor: remove addon endpoints and plan dependency from products block"
```

---

### Task 6: Simplify subscription endpoint (remove plan-limit enrichment)

The `handle_subscription` function calls `crate::plans::get_limits()` to enrich the response with plan limits and usage. Make it return raw subscription data only — the cloud platform provides the enriched view.

**Files:**
- Modify: `solobase/crates/solobase-core/src/blocks/products/handlers.rs:7,716-813`

- [ ] **Step 1: Remove DEPLOYMENTS/PROJECT_USAGE imports**

In `handlers.rs`, remove line 7:

```rust
use crate::blocks::projects::{PROJECTS_COLLECTION as DEPLOYMENTS, PROJECT_USAGE};
```

- [ ] **Step 2: Simplify handle_subscription**

Replace the `handle_subscription` function (lines 716-813) with:

```rust
async fn handle_subscription(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return err_unauthorized(msg, "Not authenticated");
    }

    let rows = db::query_raw(
        ctx,
        &format!("SELECT id, plan, status, stripe_subscription_id, grace_period_end, \
                COALESCE(addon_projects, 0) as addon_projects, \
                COALESCE(addon_requests, 0) as addon_requests, \
                COALESCE(addon_r2_bytes, 0) as addon_r2_bytes, \
                COALESCE(addon_d1_bytes, 0) as addon_d1_bytes, \
                created_at, updated_at \
         FROM {SUBSCRIPTIONS} WHERE user_id = ?1"),
        &[serde_json::Value::String(user_id)],
    )
    .await;

    let sub = match rows {
        Ok(records) if !records.is_empty() => Some(records[0].data.clone()),
        _ => None,
    };

    json_respond(msg, &serde_json::json!({"subscription": sub}))
}
```

- [ ] **Step 3: Verify solobase builds**

```bash
cd solobase && cargo check
```

Expected: may still fail due to `crate::plans` in `projects/handlers.rs`. Fixed in Task 7.

- [ ] **Step 4: Commit**

```bash
cd solobase
git add crates/solobase-core/src/blocks/products/handlers.rs
git commit -m "refactor: simplify subscription endpoint to return raw data"
```

---

### Task 7: Remove plan-limit enforcement from projects block

The projects block currently queries the subscriptions table and calls `plans::get_limits()` to enforce project creation/activation limits. Remove this — solobase-cloudflare enforces limits at the edge.

**Files:**
- Modify: `solobase/crates/solobase-core/src/blocks/projects/handlers.rs:3,12-53,373-385,499-516`

- [ ] **Step 1: Remove plan imports**

In `projects/handlers.rs`, remove line 3:

```rust
use crate::blocks::products::SUBSCRIPTIONS;
```

And remove line 12:

```rust
use crate::plans;
```

- [ ] **Step 2: Remove UserPlan struct and get_user_plan function**

Remove lines 14-53 entirely (the `UserPlan` struct and `get_user_plan` function).

- [ ] **Step 3: Remove plan limit check from project creation**

Remove the plan limit check block (lines 373-385):

```rust
    // Check plan limit for project creation (count non-deleted projects)
    let user_plan = get_user_plan(ctx, &user_id).await;
    let limits = plans::get_limits(&user_plan.plan);
    let max_created = limits
        .max_projects_created
        .saturating_add(user_plan.addon_projects);
    let current_count = count_live_projects(ctx, &user_id).await;
    if current_count as usize >= max_created {
        return err_bad_request(
            msg,
            "Project limit reached. Upgrade your plan or purchase extra project slots.",
        );
    }
```

- [ ] **Step 4: Remove plan limit check from project activation**

Remove the plan limit check block (lines 499-516):

```rust
    // Check plan capacity for active projects
    let user_plan = get_user_plan(ctx, user_id).await;
    let limits = plans::get_limits(&user_plan.plan);
    let max_active = limits
        .max_projects_active
        .saturating_add(user_plan.addon_projects);

    if max_active == 0 {
        return err_forbidden(msg, "Upgrade to a paid plan to activate projects");
    }

    let active_count = count_active_projects(ctx, user_id).await;
    if active_count as usize >= max_active {
        return err_bad_request(
            msg,
            "Active project limit reached. Deactivate a project or upgrade.",
        );
    }
```

- [ ] **Step 5: Remove count_live_projects and count_active_projects if now unused**

Check if `count_live_projects` and `count_active_projects` are still used elsewhere in the file. If only used by the removed plan-limit checks, remove them too.

- [ ] **Step 6: Verify solobase builds**

```bash
cd solobase && cargo check
```

Expected: may still fail due to `pub mod plans` in `lib.rs`. Fixed in Task 8.

- [ ] **Step 7: Commit**

```bash
cd solobase
git add crates/solobase-core/src/blocks/projects/handlers.rs
git commit -m "refactor: remove plan-limit enforcement from projects block"
```

---

### Task 8: Remove solobase-plans from solobase monorepo

With all references removed, clean up the plans module and crate dependency.

**Files:**
- Delete: `solobase/crates/solobase-plans/` (entire directory)
- Delete: `solobase/crates/solobase-core/src/plans.rs`
- Modify: `solobase/crates/solobase-core/src/lib.rs:12`
- Modify: `solobase/crates/solobase-core/Cargo.toml:30`

- [ ] **Step 1: Remove plans module from lib.rs**

In `solobase/crates/solobase-core/src/lib.rs`, remove line 12:

```rust
pub mod plans;
```

- [ ] **Step 2: Delete plans.rs**

```bash
rm solobase/crates/solobase-core/src/plans.rs
```

- [ ] **Step 3: Remove solobase-plans dependency from Cargo.toml**

In `solobase/crates/solobase-core/Cargo.toml`, remove line 30:

```toml
solobase-plans = { path = "../solobase-plans" }
```

- [ ] **Step 4: Delete the solobase-plans crate directory**

```bash
rm -rf solobase/crates/solobase-plans
```

- [ ] **Step 5: Verify solobase builds and tests pass**

```bash
cd solobase && cargo check && cargo test
```

Expected: compiles and all tests pass.

- [ ] **Step 6: Commit**

```bash
cd solobase
git add -A
git commit -m "refactor: remove solobase-plans crate and plans module from solobase"
```

---

### Task 9: Final verification

- [ ] **Step 1: Verify solobase builds and tests pass**

```bash
cd solobase && cargo check && cargo test
```

- [ ] **Step 2: Verify solobase-cloud builds**

```bash
cd solobase-cloud && cargo check
```

- [ ] **Step 3: Verify no remaining references to solobase-plans in solobase repo**

```bash
cd solobase && grep -r "solobase.plans\|solobase_plans" --include="*.rs" --include="*.toml" crates/
```

Expected: no matches.

- [ ] **Step 4: Verify solobase-plans exists only in solobase-cloud**

```bash
ls solobase-cloud/crates/solobase-plans/src/lib.rs
ls solobase/crates/solobase-plans 2>&1  # should not exist
```
