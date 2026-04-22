//! View-models passed from page handlers into maud templates.
//!
//! Kept separate from `templates/` so non-UI tests do not pull maud into scope.
//! Shared across Plan D templates (login, signup, dashboard, cli_login,
//! orgs_detail). These are pure data holders — rendering lives in
//! [`crate::blocks::auth::templates`].

use wafer_core::interfaces::auth::service::{OrgSummary, Role, UserProfile};

/// Minimal user shape rendered in the nav bar. Derived from
/// [`UserProfile`] so non-profile fields (org list, email) stay out of the
/// template layer.
#[derive(Debug, Clone)]
pub struct NavUser {
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub is_admin: bool,
}

impl NavUser {
    pub fn from_profile(p: &UserProfile) -> Self {
        Self {
            display_name: p.display_name.clone(),
            avatar_url: p.avatar_url.clone(),
            is_admin: matches!(p.role, Role::Admin),
        }
    }
}

/// View-model for `GET /auth/login`.
#[derive(Debug, Clone)]
pub struct LoginViewModel {
    pub error: Option<String>,
    pub signup_enabled: bool,
    /// OAuth provider names that currently have complete client credentials
    /// (e.g. `["github", "google"]`). Empty when no OAuth is configured.
    pub oauth_providers: Vec<String>,
    /// Path to redirect to after login (preserved through the form).
    pub next_path: Option<String>,
}

/// View-model for `GET /auth/signup`.
#[derive(Debug, Clone)]
pub struct SignupViewModel {
    pub error: Option<String>,
    pub password_min_length: u32,
    pub next_path: Option<String>,
}

/// View-model for `GET /auth/dashboard`.
#[derive(Debug, Clone)]
pub struct DashboardViewModel {
    pub user: NavUser,
    pub email: String,
    pub orgs: Vec<OrgSummary>,
    pub pats: Vec<PatRow>,
}

/// Row rendered in the dashboard PAT table.
#[derive(Debug, Clone)]
pub struct PatRow {
    pub id: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub created_at_iso: String,
    pub last_used_at_iso: Option<String>,
}

/// View-model for `GET /auth/cli` main page.
#[derive(Debug, Clone)]
pub struct CliLoginViewModel {
    pub user: NavUser,
}

/// View-model for the htmx fragment returned by `POST /auth/cli/issue`.
#[derive(Debug, Clone)]
pub struct CliCodeFragmentViewModel {
    /// Raw one-time code; displayed exactly once.
    pub code: String,
    pub expires_in_minutes: u32,
}

/// View-model for `GET /auth/orgs/{name}`.
#[derive(Debug, Clone)]
pub struct OrgsDetailViewModel {
    pub user: Option<NavUser>,
    pub org: OrgSummary,
    /// True iff `verify_org_admin` passed for the viewer — gates the
    /// Manage section.
    pub viewer_is_admin: bool,
    pub is_reserved: bool,
}

#[cfg(test)]
mod tests {
    use wafer_core::interfaces::auth::service::UserId;

    use super::*;

    #[test]
    fn nav_user_from_profile_strips_private_fields() {
        let profile = UserProfile {
            id: UserId("u1".into()),
            email: "a@b.com".into(),
            display_name: "Alice".into(),
            avatar_url: Some("https://x/y.png".into()),
            role: Role::User,
            orgs: vec![],
        };
        let nav = NavUser::from_profile(&profile);
        assert_eq!(nav.display_name, "Alice");
        assert_eq!(nav.avatar_url.as_deref(), Some("https://x/y.png"));
        assert!(!nav.is_admin);
    }

    #[test]
    fn nav_user_marks_admin_role() {
        let profile = UserProfile {
            id: UserId("u1".into()),
            email: "a@b.com".into(),
            display_name: "Admin".into(),
            avatar_url: None,
            role: Role::Admin,
            orgs: vec![],
        };
        let nav = NavUser::from_profile(&profile);
        assert!(nav.is_admin);
    }

    #[test]
    fn login_view_model_carries_error_and_oauth_buttons() {
        let vm = LoginViewModel {
            error: Some("invalid".into()),
            signup_enabled: true,
            oauth_providers: vec!["github".into(), "google".into()],
            next_path: Some("/auth/dashboard".into()),
        };
        assert_eq!(vm.oauth_providers.len(), 2);
        assert!(vm.signup_enabled);
        assert_eq!(vm.error.as_deref(), Some("invalid"));
    }
}
