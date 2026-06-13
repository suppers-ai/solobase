use std::collections::HashMap;

use wafer_block::db::{Filter, FilterOp, ListOptions, SortField};
use wafer_core::clients::{database as db, storage as store};
use wafer_run::{context::Context, ErrorCode, InputStream, Message, OutputStream};

use crate::blocks::helpers::{
    self, err_bad_request, err_forbidden, err_internal, err_not_found, ok_json, ResponseBuilder,
};

/// Buckets table — user-created storage containers (one row per bucket).
pub(crate) const BUCKETS_TABLE: &str = "suppers_ai__files__buckets";

/// Object metadata table — one row per uploaded file (sibling of the raw
/// storage blob in `wafer-run/storage`). Tracks size, content type, status,
/// uploader and timestamps.
pub(crate) const OBJECTS_TABLE: &str = "suppers_ai__files__objects";

pub async fn handle(ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/storage/buckets") => handle_list_buckets(ctx, &msg).await,
        ("create", "/storage/buckets") => handle_create_bucket(ctx, &msg, input).await,
        ("retrieve", _) if path.starts_with("/storage/buckets/") && path.contains("/objects") => {
            if path.contains("/objects/") {
                handle_get_object(ctx, &msg).await
            } else {
                handle_list_objects(ctx, &msg).await
            }
        }
        ("create", _) if path.starts_with("/storage/buckets/") && path.contains("/objects") => {
            handle_upload_object(ctx, &msg, input).await
        }
        ("delete", _) if path.starts_with("/storage/buckets/") && path.contains("/objects/") => {
            handle_delete_object(ctx, &msg).await
        }
        ("delete", _) if path.starts_with("/storage/buckets/") => {
            handle_delete_bucket(ctx, &msg).await
        }
        ("retrieve", "/storage/search") => handle_search(ctx, &msg).await,
        ("retrieve", "/storage/recent") => handle_recent(ctx, &msg).await,
        _ => err_not_found("not found"),
    }
}

pub async fn handle_admin(ctx: &dyn Context, msg: Message, _input: InputStream) -> OutputStream {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/storage/buckets") => handle_list_buckets(ctx, &msg).await,
        ("retrieve", "/admin/storage/stats") => handle_stats(ctx, &msg).await,
        _ => err_not_found("not found"),
    }
}

fn extract_bucket_name(path: &str) -> &str {
    let rest = path
        .strip_prefix("/storage/buckets/")
        .or_else(|| path.strip_prefix("/admin/storage/buckets/"))
        .unwrap_or("");
    if let Some(idx) = rest.find('/') {
        &rest[..idx]
    } else {
        rest
    }
}

fn extract_object_key(path: &str) -> &str {
    if let Some(idx) = path.find("/objects/") {
        &path[idx + 9..]
    } else {
        ""
    }
}

/// True when `user_id` owns a bucket named `bucket` (i.e. a matching row
/// exists in [`BUCKETS_TABLE`]). DB errors are logged and treated as "not
/// owned" (fail closed).
///
/// This is the single ownership predicate for the files block. Callers
/// decide the admin policy on top of it:
/// - JSON API handlers go through [`is_bucket_access_denied`], which grants
///   admins access to every bucket.
/// - The SSR user portal (`pages_user::object_list_page`) deliberately does
///   NOT bypass for admins — the portal is strictly owner-scoped so an
///   admin browsing `/b/storage/` sees only their own buckets; cross-user
///   inspection happens via the admin pages instead.
pub(super) async fn bucket_owned_by(ctx: &dyn Context, user_id: &str, bucket: &str) -> bool {
    let filters = vec![
        Filter {
            field: "name".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(bucket.to_string()),
        },
        Filter {
            field: "created_by".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.to_string()),
        },
    ];
    match db::list_all(ctx, BUCKETS_TABLE, filters).await {
        Ok(records) => !records.is_empty(),
        Err(e) => {
            tracing::warn!(error = %e, bucket = %bucket, "bucket-ownership check failed");
            false
        }
    }
}

/// Check if the current user owns the given bucket (or is admin).
/// Returns true if access is denied. See [`bucket_owned_by`] for the
/// admin-bypass policy split between the JSON API and the SSR portal.
pub(super) async fn is_bucket_access_denied(
    ctx: &dyn Context,
    msg: &Message,
    bucket: &str,
) -> bool {
    if helpers::is_admin(msg) {
        return false;
    }
    !bucket_owned_by(ctx, msg.user_id(), bucket).await
}

/// Validate a storage key for path traversal attacks.
/// Rejects keys containing `..`, absolute paths, null bytes, or backslashes.
///
/// SEC-064: backslash is rejected because backends running on Windows-style
/// paths (or any backend that ever normalises `\` to `/`) would otherwise
/// allow `..\..\etc\passwd`-style traversal that the `..` check alone would
/// not catch when the segment separator is `\` instead of `/`.
///
/// `pub(super)` so the share-creation path (`cloud.rs::handle_create_share`)
/// enforces exactly the same rule rather than re-inlining a copy that drifts
/// (the SEC-064 backslash check was the missing piece there).
pub(super) fn is_valid_storage_key(key: &str) -> bool {
    !key.is_empty()
        && !key.contains("..")
        && !key.starts_with('/')
        && !key.contains('\0')
        && !key.contains('\\')
}

/// Minimum / maximum bucket-name length (S3-compatible).
pub(super) const BUCKET_NAME_MIN_LEN: usize = 3;
pub(super) const BUCKET_NAME_MAX_LEN: usize = 63;

/// HTML5 `pattern=` attribute source for the bucket-name input — the single
/// source of truth shared with the server-side [`is_valid_bucket_name`] check
/// so the client modal and the API enforce identically. S3-compatible:
/// lowercase letters, digits, and hyphens; must start and end with a letter
/// or digit. (Length is enforced separately via `minlength`/`maxlength` on the
/// input and the length check in [`is_valid_bucket_name`].)
pub(super) const BUCKET_NAME_PATTERN: &str = "[a-z0-9]([a-z0-9-]*[a-z0-9])?";

/// Validate a bucket name against the S3-compatible rule the client modal
/// advertises ([`BUCKET_NAME_PATTERN`] + length bounds): 3–63 chars,
/// lowercase letters / digits / hyphens, must start and end with a letter or
/// digit. This rejects path traversal (`..`, `/`, `\`), NUL, uppercase, and
/// leading/trailing hyphens by construction.
///
/// `pub(super)` so the share path uses the identical rule.
pub(super) fn is_valid_bucket_name(name: &str) -> bool {
    let len = name.len();
    if len < BUCKET_NAME_MIN_LEN || len > BUCKET_NAME_MAX_LEN {
        return false;
    }
    let bytes = name.as_bytes();
    let is_alnum = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit();
    // First and last char must be a lowercase letter or digit.
    if !is_alnum(bytes[0]) || !is_alnum(bytes[len - 1]) {
        return false;
    }
    // Interior chars: lowercase letter, digit, or hyphen.
    bytes.iter().all(|&b| is_alnum(b) || b == b'-')
}

/// Collect an `InputStream` into `Vec<u8>` with a hard size cap. Errors out
/// as soon as the running total exceeds `cap_bytes`, so a multi-GB body
/// can't OOM the process before we check quota. Returns `Err(())` when
/// the cap is exceeded.
async fn collect_with_cap(
    mut input: wafer_run::InputStream,
    cap_bytes: i64,
) -> Result<Vec<u8>, ()> {
    use futures::StreamExt;
    let cap = if cap_bytes <= 0 {
        usize::MAX
    } else {
        cap_bytes as usize
    };
    let mut out = Vec::new();
    while let Some(chunk) = input.next().await {
        if out.len().saturating_add(chunk.len()) > cap {
            return Err(());
        }
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}

/// Escape SQL LIKE wildcards (`%`, `_`) and the escape char itself (`\`) in
/// user-supplied search terms so a user searching for `100% off` doesn't
/// also match arbitrary characters. The literal escape character `\` is
/// already understood by SQLite / Postgres' default LIKE.
fn escape_like(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\\' | '%' | '_' => {
                out.push('\\');
                out.push(c);
            }
            other => out.push(other),
        }
    }
    out
}

async fn handle_list_buckets(ctx: &dyn Context, msg: &Message) -> OutputStream {
    // Only show buckets owned by the current user (or all for admin)
    let user_id = msg.user_id();
    let is_admin = helpers::is_admin(msg);
    if is_admin {
        match store::list_folders(ctx).await {
            Ok(folders) => ok_json(&serde_json::json!({"buckets": folders})),
            Err(e) => err_internal("Storage error", e),
        }
    } else {
        let filters = vec![Filter {
            field: "created_by".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.to_string()),
        }];
        match db::list_all(ctx, BUCKETS_TABLE, filters).await {
            Ok(records) => {
                let names: Vec<&str> = records
                    .iter()
                    .filter_map(|r| r.data.get("name").and_then(|v| v.as_str()))
                    .collect();
                ok_json(&serde_json::json!({"buckets": names}))
            }
            Err(e) => err_internal("Database error", e),
        }
    }
}

async fn handle_create_bucket(
    ctx: &dyn Context,
    msg: &Message,
    input: InputStream,
) -> OutputStream {
    #[derive(serde::Deserialize)]
    struct Req {
        name: String,
        #[serde(default)]
        public: bool,
    }
    let raw = input.collect_to_bytes().await;
    let body: Req = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    if body.name.is_empty() {
        return err_bad_request("Bucket name is required");
    }
    if !is_valid_bucket_name(&body.name) {
        return err_bad_request("Invalid bucket name");
    }

    match store::create_folder(ctx, &body.name, body.public).await {
        Ok(()) => {
            // Track in DB
            let mut data = HashMap::new();
            data.insert(
                "name".to_string(),
                serde_json::Value::String(body.name.clone()),
            );
            data.insert("public".to_string(), serde_json::Value::Bool(body.public));
            data.insert(
                "created_by".to_string(),
                serde_json::Value::String(msg.user_id().to_string()),
            );
            data.insert(
                "created_at".to_string(),
                serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
            );
            if let Err(e) = db::create(ctx, BUCKETS_TABLE, data).await {
                tracing::warn!("Failed to track bucket creation in database: {e}");
            }
            ok_json(&serde_json::json!({"name": body.name, "created": true}))
        }
        Err(e) => err_internal("Failed to create bucket", e),
    }
}

async fn handle_delete_bucket(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let path = msg.path();
    let bucket = extract_bucket_name(path);
    if bucket.is_empty() {
        return err_bad_request("Missing bucket name");
    }
    if !is_valid_bucket_name(bucket) {
        return err_bad_request("Invalid bucket name");
    }
    if is_bucket_access_denied(ctx, msg, bucket).await {
        return err_forbidden("Access denied to this bucket");
    }

    match store::delete_folder(ctx, bucket).await {
        Ok(()) => {
            // Clean up DB metadata for the bucket and its objects
            db::delete_by_field(
                ctx,
                BUCKETS_TABLE,
                "name",
                serde_json::Value::String(bucket.to_string()),
            )
            .await
            .ok();
            db::delete_by_field(
                ctx,
                OBJECTS_TABLE,
                "bucket",
                serde_json::Value::String(bucket.to_string()),
            )
            .await
            .ok();
            ok_json(&serde_json::json!({"deleted": true}))
        }
        Err(e) => err_internal("Failed to delete bucket", e),
    }
}

async fn handle_list_objects(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let path = msg.path();
    let bucket = extract_bucket_name(path);
    if bucket.is_empty() {
        return err_bad_request("Missing bucket name");
    }
    if !is_valid_bucket_name(bucket) {
        return err_bad_request("Invalid bucket name");
    }
    if is_bucket_access_denied(ctx, msg, bucket).await {
        return err_forbidden("Access denied to this bucket");
    }

    let prefix = msg.query("prefix").to_string();
    let (_, page_size, offset) = msg.pagination_params(50);

    let opts = store::ListOptions {
        prefix,
        limit: page_size as i64,
        offset: offset as i64,
    };

    match store::list(ctx, bucket, &opts).await {
        Ok(list) => ok_json(&list),
        Err(e) => err_internal("Storage error", e),
    }
}

async fn handle_get_object(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let path = msg.path();
    let bucket = extract_bucket_name(path);
    let key = extract_object_key(path);
    if bucket.is_empty() || key.is_empty() {
        return err_bad_request("Missing bucket name or object key");
    }
    if !is_valid_storage_key(key) {
        return err_bad_request("Invalid object key");
    }
    if is_bucket_access_denied(ctx, msg, bucket).await {
        return err_forbidden("Access denied to this bucket");
    }

    // Track view in DB
    let mut data = HashMap::new();
    data.insert(
        "bucket".to_string(),
        serde_json::Value::String(bucket.to_string()),
    );
    data.insert(
        "key".to_string(),
        serde_json::Value::String(key.to_string()),
    );
    data.insert(
        "user_id".to_string(),
        serde_json::Value::String(msg.user_id().to_string()),
    );
    data.insert(
        "viewed_at".to_string(),
        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
    );
    if let Err(e) = db::create(ctx, super::VIEWS_TABLE, data).await {
        tracing::warn!("Failed to track storage object view: {e}");
    }

    match store::get(ctx, bucket, key).await {
        Ok((data, info)) => ResponseBuilder::new().body(data, &info.content_type),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found("Object not found"),
        Err(e) => err_internal("Storage error", e),
    }
}

async fn handle_upload_object(
    ctx: &dyn Context,
    msg: &Message,
    input: InputStream,
) -> OutputStream {
    let path = msg.path();
    let bucket = extract_bucket_name(path);
    if bucket.is_empty() {
        return err_bad_request("Missing bucket name");
    }

    let key = msg.query("key").to_string();
    if key.is_empty() {
        return err_bad_request("Missing object key (pass as ?key=filename)");
    }
    if !is_valid_storage_key(&key) {
        return err_bad_request("Invalid object key");
    }
    if is_bucket_access_denied(ctx, msg, bucket).await {
        return err_forbidden("Access denied to this bucket");
    }

    let content_type = {
        let ct = msg.get_meta("req.content_type");
        if ct.is_empty() {
            "application/octet-stream".to_string()
        } else {
            ct.to_string()
        }
    };

    // Best-effort sweep before quota check: orphan `pending` rows (from
    // previous uploads where the storage put failed AND the compensating
    // delete also failed) would otherwise inflate this user's quota usage
    // and lock them out. 1h cutoff.
    super::quota::sweep_stale_pending(ctx, msg.user_id(), 3600).await;

    // Stream the upload body chunk-by-chunk so an attacker who streams a
    // multi-GB body can't OOM us before quota check fires. Two bounds:
    //   - per-file `max_file_size_bytes` (cheap to check on the running
    //     total; abort as soon as the chunked total exceeds it)
    //   - total `max_storage_bytes` (depends on current usage; checked once
    //     after we know the body's full size)
    // The chunked check uses the user's *file-size* cap as a hard ceiling
    // since that's the smaller of the two.
    let quota = super::quota::get_user_quota(ctx, msg.user_id()).await;
    let body_bytes = match collect_with_cap(input, quota.max_file_size_bytes).await {
        Ok(buf) => buf,
        Err(_) => {
            return err_bad_request(&format!(
                "File exceeds maximum size of {} bytes",
                quota.max_file_size_bytes
            ));
        }
    };

    if let Err(r) = super::quota::check_quota(ctx, msg.user_id(), body_bytes.len() as i64).await {
        return r;
    }

    // Insert a pending record BEFORE uploading so concurrent quota checks see it.
    // This closes the TOCTOU race between check_quota and the actual upload.
    let mut pending_data = HashMap::new();
    pending_data.insert(
        "bucket".to_string(),
        serde_json::Value::String(bucket.to_string()),
    );
    pending_data.insert("key".to_string(), serde_json::Value::String(key.clone()));
    pending_data.insert("size".to_string(), serde_json::json!(body_bytes.len()));
    pending_data.insert(
        "content_type".to_string(),
        serde_json::Value::String(content_type.clone()),
    );
    pending_data.insert(
        "status".to_string(),
        serde_json::Value::String("pending".to_string()),
    );
    pending_data.insert(
        "uploaded_by".to_string(),
        serde_json::Value::String(msg.user_id().to_string()),
    );
    pending_data.insert(
        "uploaded_at".to_string(),
        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
    );

    let pending_record = match db::create(ctx, OBJECTS_TABLE, pending_data).await {
        Ok(record) => record,
        Err(e) => return err_internal("Failed to reserve upload slot", e),
    };

    match store::put(ctx, bucket, &key, &body_bytes, &content_type).await {
        Ok(()) => {
            // Upload succeeded — mark the pending record as complete.
            let mut update_data = HashMap::new();
            update_data.insert(
                "status".to_string(),
                serde_json::Value::String("complete".to_string()),
            );
            if let Err(e) = db::update(ctx, OBJECTS_TABLE, &pending_record.id, update_data).await {
                tracing::warn!("Failed to mark upload as complete: {e}");
            }
            ok_json(&serde_json::json!({"bucket": bucket, "key": key, "uploaded": true}))
        }
        Err(e) => {
            // Upload failed — delete the pending record so it doesn't block quota.
            if let Err(del_err) = db::delete(ctx, OBJECTS_TABLE, &pending_record.id).await {
                tracing::warn!("Failed to clean up pending record: {del_err}");
            }
            err_internal("Upload failed", e)
        }
    }
}

async fn handle_delete_object(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let path = msg.path();
    let bucket = extract_bucket_name(path);
    let key = extract_object_key(path);
    if bucket.is_empty() || key.is_empty() {
        return err_bad_request("Missing bucket name or object key");
    }
    if !is_valid_storage_key(key) {
        return err_bad_request("Invalid object key");
    }
    if is_bucket_access_denied(ctx, msg, bucket).await {
        return err_forbidden("Access denied to this bucket");
    }

    match store::delete(ctx, bucket, key).await {
        Ok(()) => {
            // Clean up metadata
            db::delete_by_filters(
                ctx,
                OBJECTS_TABLE,
                vec![
                    Filter {
                        field: "bucket".to_string(),
                        operator: FilterOp::Equal,
                        value: serde_json::Value::String(bucket.to_string()),
                    },
                    Filter {
                        field: "key".to_string(),
                        operator: FilterOp::Equal,
                        value: serde_json::Value::String(key.to_string()),
                    },
                ],
            )
            .await
            .ok();
            ok_json(&serde_json::json!({"deleted": true}))
        }
        Err(e) if e.code == ErrorCode::NotFound => err_not_found("Object not found"),
        Err(e) => err_internal("Delete failed", e),
    }
}

async fn handle_search(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let query = msg.query("q").to_string();
    if query.is_empty() {
        return err_bad_request("Missing search query");
    }

    let (_, page_size, offset) = msg.pagination_params(20);
    let opts = ListOptions {
        filters: vec![
            Filter {
                field: "key".to_string(),
                operator: FilterOp::Like,
                value: serde_json::Value::String(format!("%{}%", escape_like(&query))),
            },
            // Only show the current user's files
            Filter {
                field: "uploaded_by".to_string(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String(msg.user_id().to_string()),
            },
            // Exclude pending uploads
            Filter {
                field: "status".to_string(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String("complete".to_string()),
            },
        ],
        sort: vec![SortField {
            field: "uploaded_at".to_string(),
            desc: true,
        }],
        limit: page_size as i64,
        offset: offset as i64,
        skip_count: false,
    };

    match db::list(ctx, OBJECTS_TABLE, &opts).await {
        Ok(result) => ok_json(&result),
        Err(e) => err_internal("Search failed", e),
    }
}

async fn handle_recent(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let user_id = msg.user_id().to_string();

    let opts = ListOptions {
        filters: vec![Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id),
        }],
        sort: vec![SortField {
            field: "viewed_at".to_string(),
            desc: true,
        }],
        limit: 20,
        ..Default::default()
    };

    match db::list(ctx, super::VIEWS_TABLE, &opts).await {
        Ok(result) => ok_json(&result),
        Err(e) => err_internal("Database error", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bucket_name() {
        assert_eq!(
            extract_bucket_name("/storage/buckets/my-bucket"),
            "my-bucket"
        );
        assert_eq!(
            extract_bucket_name("/storage/buckets/my-bucket/objects"),
            "my-bucket"
        );
        assert_eq!(
            extract_bucket_name("/storage/buckets/my-bucket/objects/file.txt"),
            "my-bucket"
        );
        assert_eq!(
            extract_bucket_name("/admin/storage/buckets/admin-bucket"),
            "admin-bucket"
        );
        assert_eq!(extract_bucket_name("/other/path"), "");
    }

    #[test]
    fn test_extract_object_key() {
        assert_eq!(
            extract_object_key("/storage/buckets/b/objects/file.txt"),
            "file.txt"
        );
        assert_eq!(
            extract_object_key("/storage/buckets/b/objects/dir/file.txt"),
            "dir/file.txt"
        );
        assert_eq!(extract_object_key("/storage/buckets/b"), "");
        assert_eq!(extract_object_key("/other/path"), "");
    }

    #[test]
    fn test_is_valid_storage_key() {
        // Valid keys
        assert!(is_valid_storage_key("file.txt"));
        assert!(is_valid_storage_key("dir/file.txt"));
        assert!(is_valid_storage_key("a/b/c/file.txt"));
        assert!(is_valid_storage_key("file-name_123.txt"));

        // Invalid keys
        assert!(!is_valid_storage_key(""));
        assert!(!is_valid_storage_key("../etc/passwd"));
        assert!(!is_valid_storage_key("dir/../secret"));
        assert!(!is_valid_storage_key("/absolute/path"));
        assert!(!is_valid_storage_key("file\0name"));
        assert!(!is_valid_storage_key(".."));
    }

    #[test]
    fn test_is_valid_bucket_name() {
        // Valid bucket names (S3-compatible: 3-63 chars, lowercase/digits/
        // hyphens, start+end alnum).
        assert!(is_valid_bucket_name("my-bucket"));
        assert!(is_valid_bucket_name("bucket123"));
        assert!(is_valid_bucket_name("uploads"));
        assert!(is_valid_bucket_name("a1b"));

        // Invalid bucket names
        assert!(!is_valid_bucket_name(""));
        assert!(!is_valid_bucket_name("../other"));
        assert!(!is_valid_bucket_name("bucket/subdir"));
        assert!(!is_valid_bucket_name("bucket\0name"));
        assert!(!is_valid_bucket_name(".."));
        // Too short / too long.
        assert!(!is_valid_bucket_name("ab"));
        assert!(!is_valid_bucket_name(&"a".repeat(64)));
        // Uppercase rejected (S3 rule + matches the modal pattern).
        assert!(!is_valid_bucket_name("MyBucket"));
        // Leading / trailing hyphen rejected (start+end must be alnum).
        assert!(!is_valid_bucket_name("-bucket"));
        assert!(!is_valid_bucket_name("bucket-"));
        // Backslash rejected (SEC-064; not in the allowed alphabet).
        assert!(!is_valid_bucket_name("bucket\\name"));
    }

    /// The server-side validator enforces the same alphabet the HTML
    /// `pattern=` attribute ([`BUCKET_NAME_PATTERN`]) advertises, so the client
    /// modal and the API agree on what a valid bucket name is (modulo length,
    /// which the input enforces separately via minlength/maxlength). This pins
    /// the cases the pattern accepts/rejects against the validator.
    #[test]
    fn bucket_name_validator_matches_advertised_pattern() {
        // Sanity-check the constant is the S3 alphabet we documented.
        assert_eq!(BUCKET_NAME_PATTERN, "[a-z0-9]([a-z0-9-]*[a-z0-9])?");
        // Pattern-accepted names (length-valid) the validator must accept.
        for name in ["my-bucket", "bucket123", "a1b", "abc"] {
            assert!(is_valid_bucket_name(name), "validator should accept {name}");
        }
        // Pattern-rejected names the validator must reject.
        for name in ["MyBucket", "-bucket", "bucket-", "bucket/sub", "bucket\\x"] {
            assert!(!is_valid_bucket_name(name), "validator should reject {name}");
        }
    }
}

async fn handle_stats(ctx: &dyn Context, _msg: &Message) -> OutputStream {
    let complete_filter = &[Filter {
        field: "status".to_string(),
        operator: FilterOp::Equal,
        value: serde_json::Value::String("complete".to_string()),
    }];
    let total_objects = db::count(ctx, OBJECTS_TABLE, complete_filter)
        .await
        .unwrap_or(0);
    let total_size = db::sum(ctx, OBJECTS_TABLE, "size", complete_filter)
        .await
        .unwrap_or(0.0);
    let bucket_count = store::list_folders(ctx).await.map(|f| f.len()).unwrap_or(0);

    ok_json(&serde_json::json!({
        "total_objects": total_objects,
        "total_size_bytes": total_size as i64,
        "bucket_count": bucket_count
    }))
}
