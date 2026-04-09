//! Centralized plan definitions for Solobase.
//!
//! This is the single source of truth for all plan limits. Every part of the
//! system (solobase-core blocks, CF dispatch worker) depends on this crate.
//!
//! Zero dependencies — compiles for any target (native, wasm32).
//!
//! ## Plan model
//!
//! - **Free**: Account only. Can create up to 2 projects but cannot activate them.
//!   No API requests, no storage. Must subscribe to unlock resources.
//! - **Starter**: 2 projects, 500K requests/month, 2GB R2, 500MB D1.
//! - **Pro**: 10 projects, 3M requests/month, 20GB R2, 5GB D1.
//! - **Platform**: Unlimited (internal/self-hosted use).

/// All resource limits for a plan.
#[derive(Debug, Clone)]
pub struct PlanLimits {
    /// Maximum number of projects a user can create.
    pub max_projects_created: usize,
    /// Maximum number of projects that can be active simultaneously.
    pub max_projects_active: usize,
    /// Maximum API requests per month (across all projects).
    pub max_requests_per_month: u64,
    /// Maximum R2 (object storage) bytes per project.
    pub max_r2_storage_bytes: u64,
    /// Maximum D1 (database) bytes per project.
    pub max_d1_storage_bytes: u64,
}

/// Known plan names.
pub const PLAN_FREE: &str = "free";
pub const PLAN_STARTER: &str = "starter";
pub const PLAN_PRO: &str = "pro";
pub const PLAN_PLATFORM: &str = "platform";

const GB: u64 = 1_073_741_824;
const MB: u64 = 1_048_576;

/// Get limits for a plan by name. Unknown plans default to free.
pub fn get_limits(plan: &str) -> PlanLimits {
    match plan {
        PLAN_STARTER => PlanLimits {
            max_projects_created: 2,
            max_projects_active: 2,
            max_requests_per_month: 500_000,
            max_r2_storage_bytes: 2 * GB,
            max_d1_storage_bytes: 500 * MB,
        },
        PLAN_PRO => PlanLimits {
            max_projects_created: 10,
            max_projects_active: 10,
            max_requests_per_month: 3_000_000,
            max_r2_storage_bytes: 20 * GB,
            max_d1_storage_bytes: 5 * GB,
        },
        PLAN_PLATFORM => PlanLimits {
            max_projects_created: usize::MAX,
            max_projects_active: usize::MAX,
            max_requests_per_month: u64::MAX,
            max_r2_storage_bytes: u64::MAX,
            max_d1_storage_bytes: u64::MAX,
        },
        // Free (or unknown) — account only, no resources
        _ => PlanLimits {
            max_projects_created: 2,
            max_projects_active: 0,
            max_requests_per_month: 0,
            max_r2_storage_bytes: 0,
            max_d1_storage_bytes: 0,
        },
    }
}

/// Check if a plan allows activating projects.
pub fn can_activate(plan: &str) -> bool {
    get_limits(plan).max_projects_active > 0
}

/// Check if a plan is the free tier.
pub fn is_free(plan: &str) -> bool {
    !matches!(plan, PLAN_STARTER | PLAN_PRO | PLAN_PLATFORM)
}

// ---------------------------------------------------------------------------
// Add-on packs
// ---------------------------------------------------------------------------

/// A recurring add-on that extends plan limits. Added as a Stripe subscription
/// item — billed monthly until cancelled.
#[derive(Debug, Clone)]
pub struct AddonPack {
    /// Unique identifier (used in Stripe metadata and addon_id fields).
    pub id: &'static str,
    /// Display name.
    pub name: &'static str,
    /// Price in cents (USD) per month.
    pub price_cents: u64,
    /// Config key for the Stripe Price ID (e.g. "STRIPE_PRICE_ADDON_REQUESTS_500K").
    /// The actual price ID is configured as an environment variable.
    pub stripe_price_env: &'static str,
    /// Extra API requests granted per month.
    pub extra_requests: u64,
    /// Extra R2 storage bytes granted.
    pub extra_r2_bytes: u64,
    /// Extra D1 storage bytes granted.
    pub extra_d1_bytes: u64,
    /// Extra project slots (create + active).
    pub extra_projects: usize,
}

/// Available add-on packs.
pub const ADDON_PACKS: &[AddonPack] = &[
    AddonPack {
        id: "addon_requests_500k",
        name: "500K Extra Requests",
        price_cents: 500,
        stripe_price_env: "SUPPERS_AI__PRODUCTS__STRIPE_PRICE_ADDON_REQUESTS_500K",
        extra_requests: 500_000,
        extra_r2_bytes: 0,
        extra_d1_bytes: 0,
        extra_projects: 0,
    },
    AddonPack {
        id: "addon_r2_5gb",
        name: "5GB Extra Object Storage",
        price_cents: 300,
        stripe_price_env: "SUPPERS_AI__PRODUCTS__STRIPE_PRICE_ADDON_R2_5GB",
        extra_requests: 0,
        extra_r2_bytes: 5 * GB,
        extra_d1_bytes: 0,
        extra_projects: 0,
    },
    AddonPack {
        id: "addon_d1_1gb",
        name: "1GB Extra Database Storage",
        price_cents: 200,
        stripe_price_env: "SUPPERS_AI__PRODUCTS__STRIPE_PRICE_ADDON_D1_1GB",
        extra_requests: 0,
        extra_r2_bytes: 0,
        extra_d1_bytes: GB,
        extra_projects: 0,
    },
    AddonPack {
        id: "addon_projects_2",
        name: "2 Extra Project Slots",
        price_cents: 500,
        stripe_price_env: "SUPPERS_AI__PRODUCTS__STRIPE_PRICE_ADDON_PROJECTS_2",
        extra_requests: 0,
        extra_r2_bytes: 0,
        extra_d1_bytes: 0,
        extra_projects: 2,
    },
];

/// Look up an add-on pack by ID.
pub fn get_addon(id: &str) -> Option<&'static AddonPack> {
    ADDON_PACKS.iter().find(|a| a.id == id)
}

/// Look up an add-on pack by its Stripe Price ID config key.
pub fn get_addon_by_price_env(env_key: &str) -> Option<&'static AddonPack> {
    ADDON_PACKS.iter().find(|a| a.stripe_price_env == env_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_tier_has_no_resources() {
        let limits = get_limits("free");
        assert_eq!(limits.max_projects_created, 2);
        assert_eq!(limits.max_projects_active, 0);
        assert_eq!(limits.max_requests_per_month, 0);
        assert_eq!(limits.max_r2_storage_bytes, 0);
        assert_eq!(limits.max_d1_storage_bytes, 0);
    }

    #[test]
    fn unknown_plan_defaults_to_free() {
        let limits = get_limits("unknown");
        assert_eq!(limits.max_projects_active, 0);
        assert_eq!(limits.max_requests_per_month, 0);
    }

    #[test]
    fn starter_limits() {
        let limits = get_limits("starter");
        assert_eq!(limits.max_projects_created, 2);
        assert_eq!(limits.max_projects_active, 2);
        assert_eq!(limits.max_requests_per_month, 500_000);
        assert_eq!(limits.max_r2_storage_bytes, 2 * GB);
        assert_eq!(limits.max_d1_storage_bytes, 500 * MB);
    }

    #[test]
    fn pro_limits() {
        let limits = get_limits("pro");
        assert_eq!(limits.max_projects_created, 10);
        assert_eq!(limits.max_projects_active, 10);
        assert_eq!(limits.max_requests_per_month, 3_000_000);
    }

    #[test]
    fn platform_unlimited() {
        let limits = get_limits("platform");
        assert_eq!(limits.max_projects_created, usize::MAX);
        assert_eq!(limits.max_requests_per_month, u64::MAX);
    }

    #[test]
    fn free_cannot_activate() {
        assert!(!can_activate("free"));
        assert!(can_activate("starter"));
        assert!(can_activate("pro"));
    }
}
