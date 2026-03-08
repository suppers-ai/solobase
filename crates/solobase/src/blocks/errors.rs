/// Standardized error codes for solobase API responses.
/// Used in place of string-based error matching for reliable error handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Auth errors
    InvalidCredentials,
    EmailAlreadyExists,
    AccountDisabled,
    NotAuthenticated,
    InvalidToken,
    TokenExpired,
    PasswordTooShort,
    PasswordTooLong,
    InvalidEmail,
    InvalidInput,

    // Authorization
    Forbidden,
    AdminRequired,

    // Resource errors
    NotFound,
    Conflict,

    // Database
    DatabaseError,

    // Payment
    PaymentNotConfigured,
    InvalidPurchaseStatus,
    RefundFailed,

    // Storage
    QuotaExceeded,
    FileTooLarge,

    // System
    InternalError,
    ConfigurationError,
    RateLimitExceeded,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidCredentials => "invalid_credentials",
            Self::EmailAlreadyExists => "email_already_exists",
            Self::AccountDisabled => "account_disabled",
            Self::NotAuthenticated => "not_authenticated",
            Self::InvalidToken => "invalid_token",
            Self::TokenExpired => "token_expired",
            Self::PasswordTooShort => "password_too_short",
            Self::PasswordTooLong => "password_too_long",
            Self::InvalidEmail => "invalid_email",
            Self::InvalidInput => "invalid_input",
            Self::Forbidden => "forbidden",
            Self::AdminRequired => "admin_required",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::DatabaseError => "database_error",
            Self::PaymentNotConfigured => "payment_not_configured",
            Self::InvalidPurchaseStatus => "invalid_purchase_status",
            Self::RefundFailed => "refund_failed",
            Self::QuotaExceeded => "quota_exceeded",
            Self::FileTooLarge => "file_too_large",
            Self::InternalError => "internal_error",
            Self::ConfigurationError => "configuration_error",
            Self::RateLimitExceeded => "rate_limit_exceeded",
        }
    }

    pub fn status_code(&self) -> u16 {
        match self {
            Self::InvalidCredentials | Self::NotAuthenticated |
            Self::InvalidToken | Self::TokenExpired => 401,

            Self::Forbidden | Self::AdminRequired | Self::AccountDisabled => 403,

            Self::NotFound => 404,

            Self::EmailAlreadyExists | Self::Conflict => 409,

            Self::PasswordTooShort | Self::PasswordTooLong |
            Self::InvalidEmail | Self::InvalidInput |
            Self::InvalidPurchaseStatus => 400,

            Self::QuotaExceeded | Self::FileTooLarge => 413,
            Self::RateLimitExceeded => 429,

            Self::PaymentNotConfigured | Self::ConfigurationError |
            Self::DatabaseError | Self::InternalError |
            Self::RefundFailed => 500,
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Helper to create a JSON error response with a structured error code.
pub fn error_response(
    msg: &mut wafer_run::types::Message,
    code: ErrorCode,
    message: &str,
) -> wafer_run::types::Result_ {
    let status = code.status_code();
    let body = serde_json::json!({
        "error": {
            "code": code.as_str(),
            "message": message
        }
    });
    wafer_run::helpers::ResponseBuilder::new(msg)
        .status(status)
        .json(&body)
}
