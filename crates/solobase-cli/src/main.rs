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

use std::collections::HashMap;
use std::process;

use serde_json;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let base_url = std::env::var("SOLOBASE_URL").unwrap_or_else(|_| {
        eprintln!("Error: SOLOBASE_URL environment variable not set");
        eprintln!("Set it to your Solobase deployment URL, e.g.:");
        eprintln!("  export SOLOBASE_URL=https://solobase.example.com");
        process::exit(1);
    });

    let admin_secret = std::env::var("SOLOBASE_ADMIN_SECRET").unwrap_or_else(|_| {
        eprintln!("Error: SOLOBASE_ADMIN_SECRET environment variable not set");
        process::exit(1);
    });

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

    async fn get(&self, path: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}/_control/{}", self.base_url, path);
        let resp = self
            .http
            .get(&url)
            .header("X-Admin-Secret", &self.admin_secret)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("invalid response: {e}"))?;

        if !status.is_success() {
            let msg = body
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(format!("HTTP {}: {}", status, msg));
        }

        Ok(body)
    }

    async fn post(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let url = format!("{}/_control/{}", self.base_url, path);
        let resp = self
            .http
            .post(&url)
            .header("X-Admin-Secret", &self.admin_secret)
            .json(body)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("invalid response: {e}"))?;

        if !status.is_success() {
            let msg = body
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(format!("HTTP {}: {}", status, msg));
        }

        Ok(body)
    }

    async fn put(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let url = format!("{}/_control/{}", self.base_url, path);
        let resp = self
            .http
            .put(&url)
            .header("X-Admin-Secret", &self.admin_secret)
            .json(body)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("invalid response: {e}"))?;

        if !status.is_success() {
            let msg = body
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(format!("HTTP {}: {}", status, msg));
        }

        Ok(body)
    }

    async fn delete(&self, path: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}/_control/{}", self.base_url, path);
        let resp = self
            .http
            .delete(&url)
            .header("X-Admin-Secret", &self.admin_secret)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("invalid response: {e}"))?;

        if !status.is_success() {
            let msg = body
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(format!("HTTP {}: {}", status, msg));
        }

        Ok(body)
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
            match client.get("tenants").await {
                Ok(body) => {
                    let tenants = body
                        .get("tenants")
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
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
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

            match client
                .post(
                    "tenants",
                    &serde_json::json!({"subdomain": subdomain, "plan": plan}),
                )
                .await
            {
                Ok(body) => {
                    println!("Tenant created:");
                    println!("  Subdomain: {}", subdomain);
                    println!(
                        "  ID: {}",
                        body.get("id").and_then(|v| v.as_str()).unwrap_or("?")
                    );
                    println!("  Plan: {}", plan);
                    println!(
                        "  URL: https://{}.solobase.app",
                        subdomain
                    );
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }

        "get" | "show" => {
            if args.len() < 2 {
                eprintln!("Usage: solobase tenant get <subdomain>");
                process::exit(1);
            }

            match client.get(&format!("tenants/{}", args[1])).await {
                Ok(body) => {
                    println!("{}", serde_json::to_string_pretty(&body).unwrap());
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }

        "update" => {
            if args.len() < 2 {
                eprintln!("Usage: solobase tenant update <subdomain> [--plan <plan>]");
                process::exit(1);
            }

            let subdomain = &args[1];
            let mut updates = HashMap::new();
            if let Some(plan) = find_flag_value(&args[2..], "--plan") {
                updates.insert("plan".to_string(), serde_json::Value::String(plan));
            }

            match client
                .put(
                    &format!("tenants/{}", subdomain),
                    &serde_json::to_value(&updates).unwrap(),
                )
                .await
            {
                Ok(_) => println!("Tenant '{}' updated.", subdomain),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }

        "delete" | "rm" => {
            if args.len() < 2 {
                eprintln!("Usage: solobase tenant delete <subdomain>");
                process::exit(1);
            }

            let subdomain = &args[1];
            match client.delete(&format!("tenants/{}", subdomain)).await {
                Ok(_) => println!("Tenant '{}' deleted.", subdomain),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }

        other => {
            eprintln!("Unknown tenant command: {}", other);
            eprintln!("Usage: solobase tenant <list|create|get|update|delete>");
            process::exit(1);
        }
    }
}

async fn handle_migrate(client: &Client) {
    match client.post("migrate", &serde_json::json!({})).await {
        Ok(body) => {
            let msg = body
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("migrations applied");
            println!("{}", msg);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

async fn handle_health(client: &Client) {
    match client.get("health").await {
        Ok(body) => {
            println!("{}", serde_json::to_string_pretty(&body).unwrap());
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
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
