//! Platform service initialization for Solobase.
//!
//! Creates concrete implementations of each WAFER platform service
//! (database, storage, crypto, config, logger, network) and bundles
//! them into a `wafer_run::services::Services` struct that gets registered
//! with the runtime so every block can access them.
//!
//! Configuration is read from a TOML file (default: `solobase.toml`)
//! with environment variable overrides.

use std::sync::Arc;

use wafer_run::services::config::ConfigService;
use wafer_run::services::config_toml::TomlConfigService;
use wafer_run::services::logger::TracingLogger;
use wafer_run::services::Services;

#[cfg(feature = "full")]
use wafer_run::services::database_sqlite::SQLiteDatabaseService;

#[cfg(feature = "postgres")]
use wafer_run::services::database_postgres::PostgresDatabaseService;

#[cfg(feature = "full")]
use wafer_run::services::storage_local::LocalStorageService;

#[cfg(feature = "storage-s3")]
use wafer_run::services::storage_s3::S3StorageService;

/// Load the config service from TOML file + env vars.
///
/// The file path is read from `SOLOBASE_CONFIG` env var (default: `solobase.toml`).
pub fn load_config() -> Arc<TomlConfigService> {
    let config_path = std::env::var("SOLOBASE_CONFIG")
        .unwrap_or_else(|_| "solobase.toml".to_string());
    Arc::new(TomlConfigService::load_or_default(&config_path))
}

/// Build all platform services and return the assembled struct.
///
/// Configuration is read from the TOML config file and environment variables.
///
/// | Key (TOML / Env)              | Description                       | Default              |
/// |-------------------------------|-----------------------------------|----------------------|
/// | `database.type` / `DB_TYPE`   | Database backend: sqlite, postgres| `sqlite`             |
/// | `database.path` / `DB_PATH`   | SQLite database file path         | `data/solobase.db`   |
/// | `database.url` / `DATABASE_URL`| PostgreSQL connection string     | (none)               |
/// | `storage.type` / `STORAGE_TYPE`| Storage backend: local, s3       | `local`              |
/// | `storage.root` / `STORAGE_ROOT`| Local file storage root dir      | `data/storage`       |
/// | `auth.jwt_secret` / `JWT_SECRET`| Secret key for JWT signing      | (random)             |
///
pub fn build_platform_services(config_svc: &Arc<TomlConfigService>) -> Services {
    // --- Database ---
    let database_svc = build_database_service(config_svc.as_ref());

    // --- Storage ---
    let storage_svc = build_storage_service(config_svc.as_ref());

    // --- Crypto (argon2 + JWT) ---
    let crypto_svc = build_crypto_service(config_svc.as_ref());

    // --- Logger (tracing) ---
    let logger_svc: Arc<dyn wafer_run::services::logger::LoggerService> = Arc::new(TracingLogger);

    // --- Network (HTTP client) ---
    let network_svc = build_network_service();

    Services {
        database: database_svc,
        storage: storage_svc,
        logger: Some(logger_svc),
        crypto: crypto_svc,
        config: Some(config_svc.clone() as Arc<dyn ConfigService>),
        network: network_svc,
    }
}

// ---------------------------------------------------------------------------
// Database
// ---------------------------------------------------------------------------

#[cfg(feature = "full")]
fn build_database_service(
    config: &dyn ConfigService,
) -> Option<Arc<dyn wafer_run::services::database::DatabaseService>> {
    let db_type = config.get_default("database.type", "sqlite");

    match db_type.as_str() {
        "sqlite" => build_sqlite_service(config),
        "postgres" | "postgresql" => build_postgres_service(config),
        other => {
            tracing::error!(db_type = %other, "unknown database.type — expected 'sqlite' or 'postgres'");
            None
        }
    }
}

#[cfg(feature = "full")]
fn build_sqlite_service(
    config: &dyn ConfigService,
) -> Option<Arc<dyn wafer_run::services::database::DatabaseService>> {
    let db_path = config.get_default("database.path", "data/solobase.db");

    // Ensure the parent directory exists
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    match SQLiteDatabaseService::open(&db_path) {
        Ok(svc) => {
            tracing::info!(path = %db_path, "SQLite database opened");
            Some(Arc::new(svc))
        }
        Err(e) => {
            tracing::error!(path = %db_path, error = %e, "failed to open SQLite database");
            None
        }
    }
}

#[cfg(feature = "postgres")]
fn build_postgres_service(
    config: &dyn ConfigService,
) -> Option<Arc<dyn wafer_run::services::database::DatabaseService>> {
    let db_url = config
        .get("database.url")
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty()));

    let db_url = match db_url {
        Some(url) => url,
        None => {
            tracing::error!(
                "PostgreSQL database.url (or DATABASE_URL env) not set — \
                 configure database.url in solobase.toml or set DATABASE_URL"
            );
            return None;
        }
    };

    let handle = tokio::runtime::Handle::current();
    let result = tokio::task::block_in_place(|| {
        handle.block_on(PostgresDatabaseService::connect(&db_url))
    });

    match result {
        Ok(svc) => {
            tracing::info!("PostgreSQL database connected");
            Some(Arc::new(svc))
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to connect to PostgreSQL database");
            None
        }
    }
}

#[cfg(all(feature = "full", not(feature = "postgres")))]
fn build_postgres_service(
    _config: &dyn ConfigService,
) -> Option<Arc<dyn wafer_run::services::database::DatabaseService>> {
    tracing::error!("PostgreSQL support not available — compiled without 'postgres' feature");
    None
}

#[cfg(not(feature = "full"))]
fn build_database_service(
    _config: &dyn ConfigService,
) -> Option<Arc<dyn wafer_run::services::database::DatabaseService>> {
    tracing::warn!("database service not available — compiled without 'full' feature");
    None
}

// ---------------------------------------------------------------------------
// Storage
// ---------------------------------------------------------------------------

#[cfg(feature = "full")]
fn build_storage_service(
    config: &dyn ConfigService,
) -> Option<Arc<dyn wafer_run::services::storage::StorageService>> {
    let storage_type = config.get_default("storage.type", "local");

    match storage_type.as_str() {
        "local" => build_local_storage(config),
        "s3" => build_s3_storage(config),
        other => {
            tracing::error!(storage_type = %other, "unknown storage.type — expected 'local' or 's3'");
            None
        }
    }
}

#[cfg(feature = "full")]
fn build_local_storage(
    config: &dyn ConfigService,
) -> Option<Arc<dyn wafer_run::services::storage::StorageService>> {
    let storage_root = config.get_default("storage.root", "data/storage");

    match LocalStorageService::new(&storage_root) {
        Ok(svc) => {
            tracing::info!(root = %storage_root, "local storage service initialized");
            Some(Arc::new(svc))
        }
        Err(e) => {
            tracing::error!(root = %storage_root, error = %e, "failed to initialize local storage");
            None
        }
    }
}

#[cfg(feature = "storage-s3")]
fn build_s3_storage(
    config: &dyn ConfigService,
) -> Option<Arc<dyn wafer_run::services::storage::StorageService>> {
    let bucket = config.get_default("storage.bucket", "solobase");
    let prefix = config.get_default("storage.prefix", "");
    let endpoint = config.get("storage.endpoint").unwrap_or_default();
    let region = config.get_default("storage.region", "us-east-1");

    let handle = tokio::runtime::Handle::current();

    let result = tokio::task::block_in_place(|| {
        if endpoint.is_empty() {
            handle.block_on(S3StorageService::new(&bucket, &prefix))
        } else {
            handle.block_on(S3StorageService::with_endpoint(
                &bucket, &prefix, &endpoint, &region,
            ))
        }
    });

    match result {
        Ok(svc) => {
            tracing::info!(
                bucket = %bucket,
                prefix = %prefix,
                endpoint = if endpoint.is_empty() { "default" } else { &endpoint },
                "S3 storage service initialized"
            );
            Some(Arc::new(svc))
        }
        Err(e) => {
            tracing::error!(bucket = %bucket, error = %e, "failed to initialize S3 storage");
            None
        }
    }
}

#[cfg(all(feature = "full", not(feature = "storage-s3")))]
fn build_s3_storage(
    _config: &dyn ConfigService,
) -> Option<Arc<dyn wafer_run::services::storage::StorageService>> {
    tracing::error!("S3 storage support not available — compiled without 'storage-s3' feature");
    None
}

#[cfg(not(feature = "full"))]
fn build_storage_service(
    _config: &dyn ConfigService,
) -> Option<Arc<dyn wafer_run::services::storage::StorageService>> {
    tracing::warn!("storage service not available — compiled without 'full' feature");
    None
}

// ---------------------------------------------------------------------------
// Crypto
// ---------------------------------------------------------------------------

fn build_crypto_service(
    config: &dyn ConfigService,
) -> Option<Arc<dyn wafer_run::services::crypto::CryptoService>> {
    let jwt_secret = config
        .get("auth.jwt_secret")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            tracing::warn!("JWT_SECRET not set — generating a random secret (tokens will not survive restarts)");
            use std::time::{SystemTime, UNIX_EPOCH};
            let seed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            format!("solobase-dev-secret-{}", seed)
        });

    Some(Arc::new(SolobaseCryptoService::new(jwt_secret)))
}

/// Argon2 + JWT crypto service.
///
/// Password hashing uses argon2id with default parameters.
/// Token signing uses HMAC-SHA256 via the `jsonwebtoken` crate.
struct SolobaseCryptoService {
    jwt_secret: String,
}

impl SolobaseCryptoService {
    fn new(jwt_secret: String) -> Self {
        Self { jwt_secret }
    }
}

impl wafer_run::services::crypto::CryptoService for SolobaseCryptoService {
    fn hash(&self, password: &str) -> Result<String, wafer_run::services::crypto::CryptoError> {
        use argon2::{
            password_hash::{rand_core::OsRng, SaltString},
            Argon2, PasswordHasher,
        };
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| wafer_run::services::crypto::CryptoError::HashError(e.to_string()))
    }

    fn compare_hash(
        &self,
        password: &str,
        hash: &str,
    ) -> Result<(), wafer_run::services::crypto::CryptoError> {
        use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
        let parsed = PasswordHash::new(hash)
            .map_err(|e| wafer_run::services::crypto::CryptoError::HashError(e.to_string()))?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .map_err(|_| wafer_run::services::crypto::CryptoError::PasswordMismatch)
    }

    fn sign(
        &self,
        claims: std::collections::HashMap<String, serde_json::Value>,
        expiry: std::time::Duration,
    ) -> Result<String, wafer_run::services::crypto::CryptoError> {
        use jsonwebtoken::{encode, EncodingKey, Header};

        let now = chrono::Utc::now();
        let exp = now + chrono::Duration::from_std(expiry).unwrap_or(chrono::Duration::hours(1));

        let mut payload = claims;
        payload.insert("iat".to_string(), serde_json::json!(now.timestamp()));
        payload.insert("exp".to_string(), serde_json::json!(exp.timestamp()));

        let key = EncodingKey::from_secret(self.jwt_secret.as_bytes());
        encode(&Header::default(), &payload, &key)
            .map_err(|e| wafer_run::services::crypto::CryptoError::SignError(e.to_string()))
    }

    fn verify(
        &self,
        token: &str,
    ) -> Result<
        std::collections::HashMap<String, serde_json::Value>,
        wafer_run::services::crypto::CryptoError,
    > {
        use jsonwebtoken::{decode, DecodingKey, Validation};

        let key = DecodingKey::from_secret(self.jwt_secret.as_bytes());
        let validation = Validation::default();

        let data = decode::<std::collections::HashMap<String, serde_json::Value>>(
            token,
            &key,
            &validation,
        )
        .map_err(|e| wafer_run::services::crypto::CryptoError::VerifyError(e.to_string()))?;

        Ok(data.claims)
    }

    fn random_bytes(&self, n: usize) -> Result<Vec<u8>, wafer_run::services::crypto::CryptoError> {
        use argon2::password_hash::rand_core::{OsRng, RngCore};
        let mut buf = vec![0u8; n];
        OsRng.fill_bytes(&mut buf);
        Ok(buf)
    }
}

// ---------------------------------------------------------------------------
// Network
// ---------------------------------------------------------------------------

fn build_network_service() -> Option<Arc<dyn wafer_run::services::network::NetworkService>> {
    Some(Arc::new(HttpNetworkService))
}

/// Simple reqwest-based network service for outbound HTTP calls.
struct HttpNetworkService;

impl wafer_run::services::network::NetworkService for HttpNetworkService {
    fn do_request(
        &self,
        req: &wafer_run::services::network::Request,
    ) -> Result<wafer_run::services::network::Response, wafer_run::services::network::NetworkError>
    {
        let client = reqwest::blocking::Client::new();

        let method = req.method.parse::<reqwest::Method>().map_err(|e| {
            wafer_run::services::network::NetworkError::RequestError(format!(
                "invalid method: {}",
                e
            ))
        })?;

        let mut builder = client.request(method, &req.url);

        for (key, value) in &req.headers {
            builder = builder.header(key, value);
        }

        if let Some(ref body) = req.body {
            builder = builder.body(body.clone());
        }

        let response = builder.send().map_err(|e| {
            wafer_run::services::network::NetworkError::RequestError(e.to_string())
        })?;

        let status_code = response.status().as_u16();

        let mut headers = std::collections::HashMap::new();
        for (name, value) in response.headers() {
            let entry = headers.entry(name.to_string()).or_insert_with(Vec::new);
            if let Ok(v) = value.to_str() {
                entry.push(v.to_string());
            }
        }

        let body = response.bytes().map_err(|e| {
            wafer_run::services::network::NetworkError::RequestError(format!(
                "reading body: {}",
                e
            ))
        })?;

        Ok(wafer_run::services::network::Response {
            status_code,
            headers,
            body: body.to_vec(),
        })
    }
}
