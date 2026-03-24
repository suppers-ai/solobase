//! Stripe billing — checkout, webhooks, subscription management, portal.
//!
//! Handles /api/billing/* routes. Uses raw Stripe API calls via fetch().
//! JWT verification uses solobase_core::crypto.

use std::collections::HashMap;

use serde::Deserialize;
use worker::*;

use crate::helpers::{error_json, json_response};
use crate::project::get_plan_limits;

// ---------------------------------------------------------------------------
// Route handler
// ---------------------------------------------------------------------------

pub async fn handle_billing_route(
    req: &Request,
    env: &Env,
    path: &str,
    body: &[u8],
) -> Result<Response> {
    let method = req.method().to_string();

    // POST /api/billing/webhook — unauthenticated (verified by Stripe signature)
    if path == "webhook" && method == "POST" {
        return handle_webhook(env, req, body).await;
    }

    // All other routes require authentication
    let jwt_secret = get_env_str(env, "JWT_SECRET");
    if jwt_secret.is_empty() {
        return error_json("internal", "JWT_SECRET not configured", 500);
    }

    let claims = match authenticate(req, &jwt_secret) {
        Ok(c) => c,
        Err(resp) => return Ok(resp),
    };

    let user_id = claims
        .get("sub")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if user_id.is_empty() {
        return error_json("unauthenticated", "missing user id in token", 401);
    }

    match (method.as_str(), path) {
        ("POST", "checkout") => handle_checkout(env, &user_id, body).await,
        ("GET", "subscription") => handle_subscription(env, &user_id).await,
        ("POST", "portal") => handle_portal(env, &user_id).await,
        _ => error_json("not_found", &format!("unknown billing route: {} {}", method, path), 404),
    }
}

// ---------------------------------------------------------------------------
// JWT authentication
// ---------------------------------------------------------------------------

fn authenticate(
    req: &Request,
    jwt_secret: &str,
) -> std::result::Result<HashMap<String, serde_json::Value>, Response> {
    let auth_header = req
        .headers()
        .get("authorization")
        .ok()
        .flatten()
        .unwrap_or_default();

    if !auth_header.starts_with("Bearer ") {
        return Err(Response::ok(
            serde_json::json!({"error":"unauthenticated","message":"missing or invalid Authorization header"}).to_string()
        ).unwrap().with_status(401));
    }

    let token = &auth_header[7..];
    match solobase_core::crypto::jwt_verify(token, jwt_secret) {
        Ok(claims) => Ok(claims),
        Err(_) => Err(Response::ok(
            serde_json::json!({"error":"unauthenticated","message":"invalid or expired token"}).to_string()
        ).unwrap().with_status(401)),
    }
}

// ---------------------------------------------------------------------------
// Stripe API helper
// ---------------------------------------------------------------------------

async fn stripe_request(
    secret_key: &str,
    method_str: &str,
    api_path: &str,
    form_body: Option<&HashMap<String, String>>,
) -> std::result::Result<serde_json::Value, String> {
    let url = format!("https://api.stripe.com/v1{}", api_path);
    let method = match method_str {
        "POST" => Method::Post,
        "GET" => Method::Get,
        _ => Method::Post,
    };

    let mut init = RequestInit::new();
    init.with_method(method);

    if let Some(body) = form_body {
        let encoded: String = body
            .iter()
            .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        init.with_body(Some(wasm_bindgen::JsValue::from_str(&encoded)));
    }

    let mut worker_req = Request::new_with_init(&url, &init)
        .map_err(|e| format!("request init: {e}"))?;

    let headers = worker_req.headers_mut().map_err(|e| format!("headers: {e}"))?;
    headers
        .set("Authorization", &format!("Bearer {}", secret_key))
        .map_err(|e| format!("set auth: {e}"))?;
    headers
        .set("Content-Type", "application/x-www-form-urlencoded")
        .map_err(|e| format!("set ct: {e}"))?;

    let mut resp = Fetch::Request(worker_req)
        .send()
        .await
        .map_err(|e| format!("fetch: {e}"))?;

    let text = resp.text().await.map_err(|e| format!("read body: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("parse json: {e}"))
}

fn url_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// POST /api/billing/checkout
// ---------------------------------------------------------------------------

async fn handle_checkout(
    env: &Env,
    user_id: &str,
    body: &[u8],
) -> Result<Response> {
    #[derive(Deserialize)]
    struct Req {
        plan: String,
        name: Option<String>,
    }

    let req: Req = serde_json::from_slice(body)
        .map_err(|e| Error::RustError(format!("invalid body: {e}")))?;

    if req.plan != "starter" && req.plan != "pro" {
        return error_json("invalid_argument", "plan must be 'starter' or 'pro'", 400);
    }

    let stripe_key = get_env_str(env, "STRIPE_SECRET_KEY");
    if stripe_key.is_empty() {
        return error_json("internal", "Stripe not configured", 500);
    }

    let db = env.d1("DB").map_err(|e| Error::RustError(format!("D1: {e}")))?;

    // Check if user already has an active subscription
    let existing = db
        .prepare("SELECT id, status FROM subscriptions WHERE user_id = ?1 AND status IN ('active', 'trialing')")
        .bind(&[user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    if existing.is_some() {
        return error_json("already_exists", "you already have an active subscription", 409);
    }

    // Look up or create Stripe customer
    let stripe_customer_id = match get_stripe_customer_id(&db, user_id).await {
        Some(id) => id,
        None => {
            // Get user email
            let email = db
                .prepare("SELECT email FROM auth_users WHERE id = ?1")
                .bind(&[user_id.into()])?
                .first::<serde_json::Value>(None)
                .await?
                .and_then(|r| r.get("email").and_then(|v| v.as_str().map(String::from)))
                .unwrap_or_default();

            let mut params = HashMap::new();
            params.insert("metadata[user_id]".to_string(), user_id.to_string());
            if !email.is_empty() {
                params.insert("email".to_string(), email);
            }

            match stripe_request(&stripe_key, "POST", "/customers", Some(&params)).await {
                Ok(customer) => customer
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .ok_or_else(|| Error::RustError("failed to create Stripe customer".into()))?,
                Err(e) => return error_json("internal", &format!("Stripe error: {e}"), 500),
            }
        }
    };

    // Select price ID based on plan
    let price_id = if req.plan == "starter" {
        get_env_str(env, "STRIPE_PRICE_STARTER")
    } else {
        get_env_str(env, "STRIPE_PRICE_PRO")
    };

    if price_id.is_empty() {
        return error_json("internal", "Stripe price not configured for this plan", 500);
    }

    // Create checkout session
    let mut params = HashMap::new();
    params.insert("mode".into(), "subscription".into());
    params.insert("customer".into(), stripe_customer_id);
    params.insert("line_items[0][price]".into(), price_id);
    params.insert("line_items[0][quantity]".into(), "1".into());
    params.insert("success_url".into(), "https://cloud.solobase.dev/blocks/dashboard/?checkout=success".into());
    params.insert("cancel_url".into(), "https://solobase.dev/pricing/?checkout=cancelled".into());
    params.insert("metadata[user_id]".into(), user_id.to_string());
    params.insert("metadata[plan]".into(), req.plan.clone());
    if let Some(ref name) = req.name {
        params.insert("metadata[project_name]".into(), name.clone());
    }

    match stripe_request(&stripe_key, "POST", "/checkout/sessions", Some(&params)).await {
        Ok(session) => {
            if let Some(url) = session.get("url").and_then(|v| v.as_str()) {
                json_response(&serde_json::json!({"url": url}), 200)
            } else {
                let err_msg = session.get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown Stripe error");
                error_json("internal", err_msg, 500)
            }
        }
        Err(e) => error_json("internal", &format!("Stripe error: {e}"), 500),
    }
}

// ---------------------------------------------------------------------------
// POST /api/billing/webhook
// ---------------------------------------------------------------------------

async fn handle_webhook(env: &Env, req: &Request, body: &[u8]) -> Result<Response> {
    let sig_header = req
        .headers()
        .get("stripe-signature")
        .ok()
        .flatten()
        .unwrap_or_default();

    if sig_header.is_empty() {
        return error_json("invalid_argument", "missing Stripe-Signature header", 400);
    }

    let webhook_secret = get_env_str(env, "STRIPE_WEBHOOK_SECRET");
    if webhook_secret.is_empty() {
        return error_json("internal", "webhook secret not configured", 500);
    }

    let raw_body = std::str::from_utf8(body).unwrap_or("");

    if !verify_stripe_signature(raw_body, &sig_header, &webhook_secret) {
        return error_json("unauthenticated", "invalid webhook signature", 401);
    }

    let event: serde_json::Value = serde_json::from_slice(body)
        .map_err(|e| Error::RustError(format!("invalid JSON: {e}")))?;

    let event_type = event
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let data_object = event
        .get("data")
        .and_then(|d| d.get("object"))
        .cloned()
        .unwrap_or_default();

    let db = env.d1("DB").map_err(|e| Error::RustError(format!("D1: {e}")))?;
    let kv = env.kv("PROJECTS").map_err(|e| Error::RustError(format!("KV: {e}")))?;

    match event_type {
        "checkout.session.completed" => on_checkout_completed(&db, &kv, &data_object, env).await?,
        "customer.subscription.updated" => on_subscription_updated(&db, &kv, &data_object).await?,
        "invoice.payment_failed" => on_payment_failed(&db, &data_object).await?,
        "customer.subscription.deleted" => on_subscription_deleted(&db, &kv, &data_object).await?,
        _ => {} // Acknowledge unhandled events
    }

    json_response(&serde_json::json!({"received": true}), 200)
}

// ---------------------------------------------------------------------------
// Webhook event handlers
// ---------------------------------------------------------------------------

async fn on_checkout_completed(
    db: &D1Database,
    kv: &kv::KvStore,
    session: &serde_json::Value,
    _env: &Env,
) -> Result<()> {
    let user_id = session.pointer("/metadata/user_id").and_then(|v| v.as_str()).unwrap_or("");
    let plan = session.pointer("/metadata/plan").and_then(|v| v.as_str()).unwrap_or("starter");
    let project_name = session.pointer("/metadata/project_name").and_then(|v| v.as_str());
    let stripe_customer_id = session.get("customer").and_then(|v| v.as_str()).unwrap_or("");
    let stripe_sub_id = session.get("subscription").and_then(|v| v.as_str()).unwrap_or("");

    if user_id.is_empty() {
        console_log!("checkout.session.completed missing user_id");
        return Ok(());
    }

    let now = chrono::Utc::now().to_rfc3339();
    let sub_id = format!("sub_{}_{}", user_id, chrono::Utc::now().timestamp_millis());

    // Upsert subscription (idempotent)
    db.prepare(
        "INSERT INTO subscriptions (id, user_id, stripe_customer_id, stripe_subscription_id, plan, status, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, ?6) \
         ON CONFLICT (user_id) DO UPDATE SET \
           stripe_customer_id = excluded.stripe_customer_id, \
           stripe_subscription_id = excluded.stripe_subscription_id, \
           plan = excluded.plan, \
           status = 'active', \
           updated_at = excluded.updated_at"
    )
    .bind(&[
        sub_id.into(), user_id.into(), stripe_customer_id.into(),
        stripe_sub_id.into(), plan.into(), now.clone().into(),
    ])?
    .run()
    .await?;

    // Create project if name was provided
    if let Some(name) = project_name {
        let subdomain = sanitize_subdomain(name);
        let project_id = format!("dep_{}_{}", user_id, chrono::Utc::now().timestamp_millis());

        db.prepare(
            "INSERT INTO block_deployments (id, user_id, name, subdomain, plan, status, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, 'inactive', ?6, ?6) \
             ON CONFLICT (subdomain) DO UPDATE SET plan = excluded.plan, updated_at = excluded.updated_at"
        )
        .bind(&[
            project_id.clone().into(), user_id.into(), name.into(),
            subdomain.clone().into(), plan.into(), now.clone().into(),
        ])?
        .run()
        .await?;

        // Create project config in KV
        let config = crate::project::ProjectConfig {
            id: project_id,
            subdomain: subdomain.clone(),
            name: name.to_string(),
            plan: plan.to_string(),
            status: "inactive".to_string(),
            owner_user_id: Some(user_id.to_string()),
            db_id: None,
            db_binding: None,
            config: crate::project::ProjectAppConfig::all_enabled(),
            blocks: Vec::new(),
        };
        let json = serde_json::to_string(&config).unwrap_or_default();
        kv.put(&format!("project:{}:config", subdomain), json)
            .map_err(|e| Error::RustError(format!("KV put: {e}")))?
            .execute()
            .await?;
    }

    // Activate projects up to plan limit
    sync_project_activation(db, kv, user_id, plan).await?;
    Ok(())
}

async fn on_subscription_updated(
    db: &D1Database,
    kv: &kv::KvStore,
    subscription: &serde_json::Value,
) -> Result<()> {
    let stripe_sub_id = subscription.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let status = subscription.get("status").and_then(|v| v.as_str()).unwrap_or("");
    let plan = subscription
        .pointer("/items/data/0/price/lookup_key")
        .or_else(|| subscription.pointer("/items/data/0/price/metadata/plan"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let now = chrono::Utc::now().to_rfc3339();

    if let Some(ref plan) = plan {
        db.prepare("UPDATE subscriptions SET status = ?1, plan = ?2, updated_at = ?3 WHERE stripe_subscription_id = ?4")
            .bind(&[status.into(), plan.as_str().into(), now.as_str().into(), stripe_sub_id.into()])?
            .run()
            .await?;
    } else {
        db.prepare("UPDATE subscriptions SET status = ?1, updated_at = ?2 WHERE stripe_subscription_id = ?3")
            .bind(&[status.into(), now.as_str().into(), stripe_sub_id.into()])?
            .run()
            .await?;
    }

    // Re-evaluate project activation
    if let Some(user_id) = get_user_for_stripe_sub(db, stripe_sub_id).await {
        let effective_plan = plan.as_deref().unwrap_or("free");
        sync_project_activation(db, kv, &user_id, effective_plan).await?;
    }

    Ok(())
}

async fn on_payment_failed(db: &D1Database, invoice: &serde_json::Value) -> Result<()> {
    let stripe_sub_id = invoice.get("subscription").and_then(|v| v.as_str()).unwrap_or("");
    if stripe_sub_id.is_empty() {
        return Ok(());
    }

    let now = chrono::Utc::now().to_rfc3339();
    let grace_end = (chrono::Utc::now() + chrono::Duration::days(7)).to_rfc3339();

    db.prepare("UPDATE subscriptions SET status = 'past_due', grace_period_end = ?1, updated_at = ?2 WHERE stripe_subscription_id = ?3")
        .bind(&[grace_end.into(), now.into(), stripe_sub_id.into()])?
        .run()
        .await?;

    Ok(())
}

async fn on_subscription_deleted(
    db: &D1Database,
    kv: &kv::KvStore,
    subscription: &serde_json::Value,
) -> Result<()> {
    let stripe_sub_id = subscription.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let now = chrono::Utc::now().to_rfc3339();

    let user_id = get_user_for_stripe_sub(db, stripe_sub_id).await;

    db.prepare("UPDATE subscriptions SET status = 'cancelled', updated_at = ?1 WHERE stripe_subscription_id = ?2")
        .bind(&[now.into(), stripe_sub_id.into()])?
        .run()
        .await?;

    if let Some(ref uid) = user_id {
        deactivate_all_projects(db, kv, uid).await?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// GET /api/billing/subscription
// ---------------------------------------------------------------------------

async fn handle_subscription(env: &Env, user_id: &str) -> Result<Response> {
    let db = env.d1("DB").map_err(|e| Error::RustError(format!("D1: {e}")))?;

    let sub = db
        .prepare("SELECT id, plan, status, stripe_subscription_id, grace_period_end, created_at, updated_at FROM subscriptions WHERE user_id = ?1")
        .bind(&[user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    match sub {
        Some(s) => {
            let plan = s.get("plan").and_then(|v| v.as_str()).unwrap_or("free");
            let limits = get_plan_limits(plan);
            let month = chrono::Utc::now().format("%Y-%m").to_string();

            let usage = db
                .prepare("SELECT requests, addon_requests, r2_bytes, addon_r2_bytes FROM project_usage WHERE project_id = ?1 AND month = ?2")
                .bind(&[user_id.into(), month.clone().into()])?
                .first::<serde_json::Value>(None)
                .await?
                .unwrap_or_default();

            let addon_req = usage.get("addon_requests").and_then(|v| v.as_u64()).unwrap_or(0);

            json_response(&serde_json::json!({
                "subscription": s,
                "usage": {
                    "month": month,
                    "requests": {
                        "used": usage.get("requests").and_then(|v| v.as_u64()).unwrap_or(0),
                        "limit": limits.max_requests_per_month + addon_req,
                    },
                    "r2Storage": {
                        "usedBytes": usage.get("r2_bytes").and_then(|v| v.as_u64()).unwrap_or(0),
                        "limitBytes": limits.max_r2_storage_bytes + usage.get("addon_r2_bytes").and_then(|v| v.as_u64()).unwrap_or(0),
                    },
                }
            }), 200)
        }
        None => json_response(&serde_json::json!({"subscription": null, "usage": null}), 200),
    }
}

// ---------------------------------------------------------------------------
// POST /api/billing/portal
// ---------------------------------------------------------------------------

async fn handle_portal(env: &Env, user_id: &str) -> Result<Response> {
    let db = env.d1("DB").map_err(|e| Error::RustError(format!("D1: {e}")))?;

    let sub = db
        .prepare("SELECT stripe_customer_id FROM subscriptions WHERE user_id = ?1")
        .bind(&[user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    let customer_id = sub
        .and_then(|s| s.get("stripe_customer_id").and_then(|v| v.as_str().map(String::from)))
        .unwrap_or_default();

    if customer_id.is_empty() {
        return error_json("not_found", "no subscription found", 404);
    }

    let stripe_key = get_env_str(env, "STRIPE_SECRET_KEY");
    let mut params = HashMap::new();
    params.insert("customer".into(), customer_id);
    params.insert("return_url".into(), "https://cloud.solobase.dev/blocks/dashboard/".into());

    match stripe_request(&stripe_key, "POST", "/billing_portal/sessions", Some(&params)).await {
        Ok(session) => {
            if let Some(url) = session.get("url").and_then(|v| v.as_str()) {
                json_response(&serde_json::json!({"url": url}), 200)
            } else {
                error_json("internal", "failed to create portal session", 500)
            }
        }
        Err(e) => error_json("internal", &format!("Stripe error: {e}"), 500),
    }
}

// ---------------------------------------------------------------------------
// Project activation sync
// ---------------------------------------------------------------------------

/// Activate/deactivate projects based on plan limits (oldest first).
async fn sync_project_activation(
    db: &D1Database,
    kv: &kv::KvStore,
    user_id: &str,
    plan: &str,
) -> Result<()> {
    let limits = get_plan_limits(plan);
    let max_active = limits.max_projects;

    let rows = db
        .prepare("SELECT id, subdomain, status FROM block_deployments WHERE user_id = ?1 AND deleted_at IS NULL ORDER BY created_at ASC")
        .bind(&[user_id.into()])?
        .all()
        .await?;

    let results = rows.results::<serde_json::Value>()?;
    let now = chrono::Utc::now().to_rfc3339();

    for (i, row) in results.iter().enumerate() {
        let should_be_active = i < max_active;
        let desired = if should_be_active { "active" } else { "inactive" };
        let current = row.get("status").and_then(|v| v.as_str()).unwrap_or("");

        if current != desired {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let subdomain = row.get("subdomain").and_then(|v| v.as_str()).unwrap_or("");

            db.prepare("UPDATE block_deployments SET status = ?1, updated_at = ?2 WHERE id = ?3")
                .bind(&[desired.into(), now.clone().into(), id.into()])?
                .run()
                .await?;

            if !subdomain.is_empty() {
                update_kv_project_status(kv, subdomain, desired).await;
            }
        }
    }

    Ok(())
}

/// Deactivate ALL projects for a user (subscription cancelled).
async fn deactivate_all_projects(
    db: &D1Database,
    kv: &kv::KvStore,
    user_id: &str,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();

    db.prepare("UPDATE block_deployments SET status = 'inactive', updated_at = ?1 WHERE user_id = ?2 AND deleted_at IS NULL")
        .bind(&[now.into(), user_id.into()])?
        .run()
        .await?;

    let rows = db
        .prepare("SELECT subdomain FROM block_deployments WHERE user_id = ?1 AND deleted_at IS NULL")
        .bind(&[user_id.into()])?
        .all()
        .await?;

    for row in rows.results::<serde_json::Value>()? {
        if let Some(subdomain) = row.get("subdomain").and_then(|v| v.as_str()) {
            if !subdomain.is_empty() {
                update_kv_project_status(kv, subdomain, "inactive").await;
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn update_kv_project_status(kv: &kv::KvStore, subdomain: &str, status: &str) {
    let key = format!("project:{}:config", subdomain);
    if let Ok(Some(mut config)) = kv.get(&key).json::<serde_json::Value>().await {
        config.as_object_mut().map(|o| o.insert("status".into(), status.into()));
        if let Ok(json) = serde_json::to_string(&config) {
            let _ = kv.put(&key, json).map(|p| p.execute());
        }
    }
}

async fn get_stripe_customer_id(db: &D1Database, user_id: &str) -> Option<String> {
    db.prepare("SELECT stripe_customer_id FROM subscriptions WHERE user_id = ?1 ORDER BY created_at DESC LIMIT 1")
        .bind(&[user_id.into()])
        .ok()?
        .first::<serde_json::Value>(None)
        .await
        .ok()?
        .and_then(|r| r.get("stripe_customer_id").and_then(|v| v.as_str().map(String::from)))
        .filter(|s| !s.is_empty())
}

async fn get_user_for_stripe_sub(db: &D1Database, stripe_sub_id: &str) -> Option<String> {
    db.prepare("SELECT user_id FROM subscriptions WHERE stripe_subscription_id = ?1")
        .bind(&[stripe_sub_id.into()])
        .ok()?
        .first::<serde_json::Value>(None)
        .await
        .ok()?
        .and_then(|r| r.get("user_id").and_then(|v| v.as_str().map(String::from)))
}

fn get_env_str(env: &Env, key: &str) -> String {
    env.secret(key)
        .map(|s| s.to_string())
        .or_else(|_| env.var(key).map(|v| v.to_string()))
        .unwrap_or_default()
}

fn sanitize_subdomain(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Verify Stripe webhook signature (HMAC-SHA256 with constant-time comparison).
fn verify_stripe_signature(raw_body: &str, sig_header: &str, webhook_secret: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // Parse t=timestamp,v1=signature
    let mut timestamp = "";
    let mut expected_sig = "";
    for item in sig_header.split(',') {
        if let Some((key, val)) = item.split_once('=') {
            match key.trim() {
                "t" => timestamp = val,
                "v1" => expected_sig = val,
                _ => {}
            }
        }
    }

    if timestamp.is_empty() || expected_sig.is_empty() {
        return false;
    }

    // Check timestamp within 5 minutes
    if let Ok(ts) = timestamp.parse::<i64>() {
        let now = chrono::Utc::now().timestamp();
        if (now - ts).abs() > 300 {
            return false;
        }
    } else {
        return false;
    }

    // Compute HMAC-SHA256
    let payload = format!("{}.{}", timestamp, raw_body);
    let mut mac = match Hmac::<Sha256>::new_from_slice(webhook_secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(payload.as_bytes());
    let result = mac.finalize().into_bytes();

    // Hex encode
    let computed: String = result.iter().map(|b| format!("{:02x}", b)).collect();

    // Constant-time comparison
    constant_time_eq(computed.as_bytes(), expected_sig.as_bytes())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
