//! `HttpHarness`: a real in-process HTTP server wired to
//! `SolobaseAuthBlock`, used by the Layer-3 integration tests.
//!
//! Binding strategy: `127.0.0.1:0` — the kernel assigns an ephemeral port
//! so tests can run in parallel without port collisions.
//!
//! Request dispatch: every axum request is converted to a wafer-run
//! `Message` via `wafer_block_http_listener::http_to_message`, the auth
//! block handles it, and the output stream is translated back into an
//! axum response via `wafer_block_http_listener::wafer_output_to_response`.
//! This is the same transport surface the production `wafer-run/http-listener`
//! block uses — no custom shim.

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::{
    body::Body,
    extract::{Request, State},
    http::Response,
    routing::any,
    Router,
};
use solobase_core::blocks::auth::{
    block,
    config::{AuthConfig, PASSWORD_MIN_LENGTH_KEY, SIGNUP_ENABLED_KEY},
    migrations,
    providers::{registry::ProviderRegistry, OAuthProvider},
    repo::{local_credentials, users},
    session,
};
use wafer_block_http_listener::{http_to_message, wafer_output_to_response};
use wafer_core::clients::crypto as crypto_client;
use wafer_run::{block::Block, context::Context, BlockRegistry, InputStream, RuntimeError};

// We can't reference the `tests/auth/common.rs` module from another test
// binary, so this file declares its own `MigrationTestCtx` — same contract
// as `auth::common::MigrationTestCtx` (routes `wafer-run/database` +
// `crypto` to in-memory services, everything else NotFound).

pub struct HttpHarness {
    pub base_url: String,
    pub ctx: Arc<dyn Context>,
    pub _shutdown: tokio::sync::oneshot::Sender<()>,
}

impl HttpHarness {
    /// Construct a URL against the harness' ephemeral base address.
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Start a harness wired to the auth block. Returns once the TCP
    /// listener is bound — subsequent `reqwest` calls against `base_url`
    /// will hit a live server.
    pub async fn start_with_auth() -> Self {
        Self::start_with(None, AuthConfig::from_env_for_test(&[])).await
    }

    /// Builder for tests that need a non-default [`AuthConfig`] (signup
    /// flag, password minimum length, …).
    pub fn builder() -> HttpHarnessBuilder {
        HttpHarnessBuilder::default()
    }

    /// Start a harness with an explicit OAuth [`ProviderRegistry`]. Lets
    /// E2E tests inject a fake provider into the real HTTP dispatch path
    /// without racing on `std::env`.
    pub async fn start_with_providers(providers: ProviderRegistry) -> Self {
        Self::start_with(Some(providers), AuthConfig::from_env_for_test(&[])).await
    }

    async fn start_with(providers: Option<ProviderRegistry>, config: AuthConfig) -> Self {
        let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
        migrations::apply(ctx.as_ref())
            .await
            .expect("auth migrations");

        let mut registry = TestRegistry::default();
        match providers {
            Some(p) => {
                block::register_with_providers_for_test(&mut registry, ctx.clone(), config, p)
                    .expect("register auth block")
            }
            None => block::register_with_config(&mut registry, ctx.clone(), config)
                .expect("register auth block"),
        }
        let auth_block = registry
            .blocks
            .remove("suppers-ai/auth")
            .expect("auth block registered");

        let state = AppState {
            ctx: ctx.clone(),
            auth_block,
        };
        let app = Router::new()
            .route("/{*rest}", any(dispatch))
            .route("/", any(dispatch))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral");
        let addr = listener.local_addr().expect("local addr");
        let base_url = format!("http://{}", addr);

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = rx.await;
                })
                .await;
        });

        HttpHarness {
            base_url,
            ctx,
            _shutdown: tx,
        }
    }

    /// Seed a user with a password hashed via the same Argon2 crypto
    /// service the auth block uses at runtime.
    pub async fn seed_user_with_password(&self, email: &str, password: &str) -> String {
        let ctx = self.ctx.as_ref();
        let u = users::insert(
            ctx,
            users::NewUser {
                email: email.into(),
                display_name: "T".into(),
                avatar_url: None,
                role: "user".into(),
            },
        )
        .await
        .expect("insert user");
        let hash = crypto_client::hash(ctx, password).await.expect("hash");
        local_credentials::insert(ctx, &u.id, &hash, false)
            .await
            .expect("insert credentials");
        u.id
    }

    /// Seed a user + issue a session and return the raw session-cookie
    /// value. Callers send it as `Cookie: wafer_session=<value>`.
    pub async fn seed_user_and_session(&self, email: &str, password: &str) -> String {
        let user_id = self.seed_user_with_password(email, password).await;
        let issued = session::issue_for(self.ctx.as_ref(), &user_id, 30)
            .await
            .expect("issue session");
        issued.raw_token
    }
}

/// Builder for [`HttpHarness`] — used by page-handler tests that need to
/// flip `SIGNUP_ENABLED` or raise `PASSWORD_MIN_LENGTH`.
#[derive(Default)]
pub struct HttpHarnessBuilder {
    signup_enabled: bool,
    password_min_length: Option<u32>,
    providers: Option<ProviderRegistry>,
}

impl HttpHarnessBuilder {
    pub fn signup_enabled(mut self, v: bool) -> Self {
        self.signup_enabled = v;
        self
    }

    pub fn password_min_length(mut self, v: u32) -> Self {
        self.password_min_length = Some(v);
        self
    }

    #[allow(dead_code)]
    pub fn providers(mut self, p: ProviderRegistry) -> Self {
        self.providers = Some(p);
        self
    }

    pub async fn spawn(self) -> HttpHarness {
        let min_len = self
            .password_min_length
            .map(|n| n.to_string())
            .unwrap_or_default();
        let mut pairs: Vec<(&str, &str)> = Vec::new();
        if self.signup_enabled {
            pairs.push((SIGNUP_ENABLED_KEY, "true"));
        }
        if !min_len.is_empty() {
            pairs.push((PASSWORD_MIN_LENGTH_KEY, min_len.as_str()));
        }
        let cfg = AuthConfig::from_env_for_test(&pairs);
        HttpHarness::start_with(self.providers, cfg).await
    }
}

// -----------------------------------------------------------------------
// Internals
// -----------------------------------------------------------------------

#[derive(Clone)]
struct AppState {
    ctx: Arc<dyn Context>,
    auth_block: Arc<dyn Block>,
}

async fn dispatch(State(state): State<AppState>, req: Request) -> Response<Body> {
    let (parts, body) = req.into_parts();
    const MAX_BODY: usize = 10 * 1024 * 1024;
    let body_bytes = axum::body::to_bytes(body, MAX_BODY)
        .await
        .unwrap_or_default()
        .to_vec();
    let uri = &parts.uri;
    let path = uri.path();
    let query = uri.query().unwrap_or("");
    let remote_addr = parts
        .extensions
        .get::<SocketAddr>()
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|| "unknown".into());

    let msg = http_to_message(parts.method, path, query, &parts.headers, &remote_addr);
    let input = InputStream::from_bytes(body_bytes);
    let output = state
        .auth_block
        .handle(state.ctx.as_ref(), msg, input)
        .await;
    wafer_output_to_response(output).await
}

// -----------------------------------------------------------------------
// Test context — same shape as tests/auth/common.rs but private to this
// binary (integration test crates can't share modules across binaries).
// -----------------------------------------------------------------------

pub struct MigrationTestCtx {
    db_block: Arc<dyn Block>,
    crypto_block: Arc<dyn Block>,
}

impl MigrationTestCtx {
    pub fn new() -> Self {
        let svc = Arc::new(
            wafer_block_sqlite::service::SQLiteDatabaseService::open_in_memory()
                .expect("open in-memory sqlite"),
        );
        let db_block: Arc<dyn Block> = Arc::new(
            wafer_core::service_blocks::database::DatabaseBlock::new(svc),
        );
        let crypto_svc = Arc::new(wafer_block_crypto::service::Argon2JwtCryptoService::new(
            "test-jwt-secret".to_string(),
        ));
        let crypto_block: Arc<dyn Block> = Arc::new(
            wafer_core::service_blocks::crypto::CryptoBlock::new(crypto_svc),
        );
        Self {
            db_block,
            crypto_block,
        }
    }
}

#[async_trait::async_trait]
impl Context for MigrationTestCtx {
    async fn call_block(
        &self,
        block_name: &str,
        msg: wafer_run::types::Message,
        input: InputStream,
    ) -> wafer_run::OutputStream {
        match block_name {
            "wafer-run/database" => self.db_block.handle(self, msg, input).await,
            "wafer-run/crypto" => self.crypto_block.handle(self, msg, input).await,
            _ => wafer_run::OutputStream::error(wafer_run::types::WaferError::new(
                wafer_run::types::ErrorCode::NOT_FOUND,
                format!("block '{block_name}' not registered in test ctx"),
            )),
        }
    }

    fn is_cancelled(&self) -> bool {
        false
    }

    fn config_get(&self, _key: &str) -> Option<&str> {
        None
    }
}

/// Build a one-entry [`ProviderRegistry`] keyed by `provider_name` for an
/// `OAuthProvider` double.
pub fn registry_with(
    provider_name: &'static str,
    provider: Arc<dyn OAuthProvider>,
) -> ProviderRegistry {
    let mut m: HashMap<&'static str, Arc<dyn OAuthProvider>> = HashMap::new();
    m.insert(provider_name, provider);
    ProviderRegistry::from_map(m)
}

#[derive(Default)]
struct TestRegistry {
    blocks: HashMap<String, Arc<dyn Block>>,
}

impl BlockRegistry for TestRegistry {
    fn register_block(&mut self, name: &str, blk: Arc<dyn Block>) -> Result<(), RuntimeError> {
        self.blocks.insert(name.into(), blk);
        Ok(())
    }
    fn add_alias(&mut self, _: &str, _: &str) {}
    fn add_block_config(&mut self, _: &str, _: serde_json::Value) {}
}
