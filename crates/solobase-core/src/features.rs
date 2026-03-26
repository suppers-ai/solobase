//! Feature configuration trait — shared between Cloudflare and native.
//!
//! Both `TenantAppConfig` (CF) and `AppConfig` (native) implement this trait.

use serde_json::Value;

/// Trait for querying which solobase features are enabled.
pub trait FeatureConfig: wafer_run::MaybeSend + wafer_run::MaybeSync {
    fn auth_enabled(&self) -> bool;
    fn admin_enabled(&self) -> bool;
    fn files_enabled(&self) -> bool;
    fn products_enabled(&self) -> bool;
    fn projects_enabled(&self) -> bool;
    fn legalpages_enabled(&self) -> bool;
    fn userportal_enabled(&self) -> bool;
}

/// Shared helper: determine if an `Option<Value>` means "enabled".
///
/// - `None` (absent) → disabled
/// - `Some(false)` or `Some(null)` → disabled
/// - `Some({})`, `Some(true)`, `Some({...})` → enabled
pub fn is_feature_enabled(val: &Option<Value>) -> bool {
    !matches!(val, None | Some(Value::Bool(false)) | Some(Value::Null))
}

/// A config with all features enabled (useful for dev/testing).
pub struct AllEnabled;

impl FeatureConfig for AllEnabled {
    fn auth_enabled(&self) -> bool { true }
    fn admin_enabled(&self) -> bool { true }
    fn files_enabled(&self) -> bool { true }
    fn products_enabled(&self) -> bool { true }
    fn projects_enabled(&self) -> bool { true }
    fn legalpages_enabled(&self) -> bool { true }
    fn userportal_enabled(&self) -> bool { true }
}
