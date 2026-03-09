//! Solobase CLI — manage tenants, run migrations, and administer deployments.
//!
//! # Usage
//!
//! ```bash
//! # Set your deployment URL and admin secret
//! export SOLOBASE_URL=https://solobase.example.com
//! export SOLOBASE_ADMIN_SECRET=your-secret
//!
//! # Tenant management
//! solobase tenant list
//! solobase tenant create myapp --plan hobby
//! solobase tenant get myapp
//! solobase tenant update myapp --plan pro
//! solobase tenant delete myapp
//!
//! # Database migrations
//! solobase migrate
//!
//! # Platform health
//! solobase health
//! ```

use std::process;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let base_url = require_env("SOLOBASE_URL", Some("Set it to your Solobase deployment URL, e.g.:\n  export SOLOBASE_URL=https://solobase.example.com"));
    let admin_secret = require_env("SOLOBASE_ADMIN_SECRET", None);
    let client = Client::new(&base_url, &admin_secret);

    match args[1].as_str() {
        "tenant" => handle_tenant(&client, &args[2..]).await,
        "migrate" => handle_migrate(&client).await,
        "health" => handle_health(&client).await,
        "help" | "--help" | "-h" => print_usage(),
        other => {
            eprintln!("Unknown command: {}", other);
            print_usage();
            process::exit(1);
        }
    }
}

fn require_env(key: &str, hint: Option<&str>) -> String {
    std::env::var(key).unwrap_or_else(|_| {
        eprintln!("Error: {} environment variable not set", key);
        if let Some(h) = hint {
            eprintln!("{}", h);
        }
        process::exit(1);
    })
}

fn exit_on_error<T>(result: Result<T, String>) -> T {
    match result {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("Solobase CLI — manage your Solobase deployment");
    eprintln!();
    eprintln!("Usage: solobase <command> [args...]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  tenant list              List all tenants");
    eprintln!("  tenant create <sub>      Create a new tenant");
    eprintln!("  tenant get <sub>         Get tenant details");
    eprintln!("  tenant update <sub>      Update tenant config");
    eprintln!("  tenant delete <sub>      Delete a tenant");
    eprintln!("  migrate                  Run database migrations");
    eprintln!("  health                   Check platform health");
    eprintln!();
    eprintln!("Environment:");
    eprintln!("  SOLOBASE_URL             Deployment URL (required)");
    eprintln!("  SOLOBASE_ADMIN_SECRET    Admin API secret (required)");
}

// ---------------------------------------------------------------------------
// HTTP client
// ---------------------------------------------------------------------------

struct Client {
    base_url: String,
    admin_secret: String,
    http: reqwest::Client,
}

impl Client {
    fn new(base_url: &str, admin_secret: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            admin_secret: admin_secret.to_string(),
            http: reqwest::Client::new(),
        }
    }

    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let url = format!("{}/_control/{}", self.base_url, path);
        let mut builder = self.http.request(method, &url)
            .header("X-Admin-Secret", &self.admin_secret);

        if let Some(body) = body {
            builder = builder.json(body);
        }

        let resp = builder.send().await
            .map_err(|e| format!("request failed: {e}"))?;

        let status = resp.status();
        let body: serde_json::Value = resp.json().await
            .map_err(|e| format!("invalid response: {e}"))?;

        if !status.is_success() {
            let msg = body.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(format!("HTTP {}: {}", status, msg));
        }

        Ok(body)
    }

    async fn get(&self, path: &str) -> Result<serde_json::Value, String> {
        self.request(reqwest::Method::GET, path, None).await
    }

    async fn post(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value, String> {
        self.request(reqwest::Method::POST, path, Some(body)).await
    }

    async fn put(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value, String> {
        self.request(reqwest::Method::PUT, path, Some(body)).await
    }

    async fn delete(&self, path: &str) -> Result<serde_json::Value, String> {
        self.request(reqwest::Method::DELETE, path, None).await
    }
}

// ---------------------------------------------------------------------------
// Command handlers
// ---------------------------------------------------------------------------

async fn handle_tenant(client: &Client, args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: solobase tenant <list|create|get|update|delete> [args...]");
        process::exit(1);
    }

    match args[0].as_str() {
        "list" | "ls" => {
            let body = exit_on_error(client.get("tenants").await);
            let tenants = body.get("tenants")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            if tenants.is_empty() {
                println!("No tenants found.");
            } else {
                println!("Tenants ({}):", tenants.len());
                for t in tenants {
                    if let Some(s) = t.as_str() {
                        println!("  - {}", s);
                    }
                }
            }
        }

        "create" | "new" => {
            if args.len() < 2 {
                eprintln!("Usage: solobase tenant create <subdomain> [--plan <plan>]");
                process::exit(1);
            }
            let subdomain = &args[1];
            let plan = find_flag_value(&args[2..], "--plan").unwrap_or_else(|| "hobby".to_string());

            let body = exit_on_error(
                client.post("tenants", &serde_json::json!({"subdomain": subdomain, "plan": plan})).await
            );
            println!("Tenant created:");
            println!("  Subdomain: {}", subdomain);
            println!("  ID: {}", body.get("id").and_then(|v| v.as_str()).unwrap_or("?"));
            println!("  Plan: {}", plan);
            println!("  URL: https://{}.solobase.app", subdomain);
        }

        "get" | "show" => {
            if args.len() < 2 {
                eprintln!("Usage: solobase tenant get <subdomain>");
                process::exit(1);
            }
            let body = exit_on_error(client.get(&format!("tenants/{}", args[1])).await);
            println!("{}", serde_json::to_string_pretty(&body).unwrap_or_default());
        }

        "update" => {
            if args.len() < 2 {
                eprintln!("Usage: solobase tenant update <subdomain> [--plan <plan>]");
                process::exit(1);
            }
            let subdomain = &args[1];
            let mut updates = serde_json::json!({});
            if let Some(plan) = find_flag_value(&args[2..], "--plan") {
                updates["plan"] = serde_json::Value::String(plan);
            }
            exit_on_error(client.put(&format!("tenants/{}", subdomain), &updates).await);
            println!("Tenant '{}' updated.", subdomain);
        }

        "delete" | "rm" => {
            if args.len() < 2 {
                eprintln!("Usage: solobase tenant delete <subdomain>");
                process::exit(1);
            }
            let subdomain = &args[1];
            exit_on_error(client.delete(&format!("tenants/{}", subdomain)).await);
            println!("Tenant '{}' deleted.", subdomain);
        }

        other => {
            eprintln!("Unknown tenant command: {}", other);
            eprintln!("Usage: solobase tenant <list|create|get|update|delete>");
            process::exit(1);
        }
    }
}

async fn handle_migrate(client: &Client) {
    let body = exit_on_error(client.post("migrate", &serde_json::json!({})).await);
    let msg = body.get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("migrations applied");
    println!("{}", msg);
}

async fn handle_health(client: &Client) {
    let body = exit_on_error(client.get("health").await);
    println!("{}", serde_json::to_string_pretty(&body).unwrap_or_default());
}

fn find_flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find_map(|pair| {
        if pair[0] == flag {
            Some(pair[1].clone())
        } else {
            None
        }
    })
}
