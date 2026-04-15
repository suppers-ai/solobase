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
    EmailNotVerified,
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
            Self::EmailNotVerified => "email_not_verified",
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
            Self::InvalidCredentials
            | Self::NotAuthenticated
            | Self::InvalidToken
            | Self::TokenExpired => 401,

            Self::Forbidden
            | Self::AdminRequired
            | Self::AccountDisabled
            | Self::EmailNotVerified => 403,

            Self::NotFound => 404,

            Self::EmailAlreadyExists | Self::Conflict => 409,

            Self::PasswordTooShort
            | Self::PasswordTooLong
            | Self::InvalidEmail
            | Self::InvalidInput
            | Self::InvalidPurchaseStatus => 400,

            Self::QuotaExceeded | Self::FileTooLarge => 413,
            Self::RateLimitExceeded => 429,

            Self::PaymentNotConfigured
            | Self::ConfigurationError
            | Self::DatabaseError
            | Self::InternalError
            | Self::RefundFailed => 500,
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_status_codes() {
        // Auth errors -> 401
        assert_eq!(ErrorCode::InvalidCredentials.status_code(), 401);
        assert_eq!(ErrorCode::NotAuthenticated.status_code(), 401);
        assert_eq!(ErrorCode::InvalidToken.status_code(), 401);
        assert_eq!(ErrorCode::TokenExpired.status_code(), 401);

        // Forbidden -> 403
        assert_eq!(ErrorCode::Forbidden.status_code(), 403);
        assert_eq!(ErrorCode::AdminRequired.status_code(), 403);
        assert_eq!(ErrorCode::AccountDisabled.status_code(), 403);

        // Not found -> 404
        assert_eq!(ErrorCode::NotFound.status_code(), 404);

        // Conflict -> 409
        assert_eq!(ErrorCode::EmailAlreadyExists.status_code(), 409);
        assert_eq!(ErrorCode::Conflict.status_code(), 409);

        // Bad request -> 400
        assert_eq!(ErrorCode::PasswordTooShort.status_code(), 400);
        assert_eq!(ErrorCode::PasswordTooLong.status_code(), 400);
        assert_eq!(ErrorCode::InvalidEmail.status_code(), 400);
        assert_eq!(ErrorCode::InvalidInput.status_code(), 400);

        // Quota -> 413
        assert_eq!(ErrorCode::QuotaExceeded.status_code(), 413);
        assert_eq!(ErrorCode::FileTooLarge.status_code(), 413);

        // Rate limit -> 429
        assert_eq!(ErrorCode::RateLimitExceeded.status_code(), 429);

        // Server errors -> 500
        assert_eq!(ErrorCode::InternalError.status_code(), 500);
        assert_eq!(ErrorCode::DatabaseError.status_code(), 500);
        assert_eq!(ErrorCode::ConfigurationError.status_code(), 500);
    }

    #[test]
    fn test_error_code_as_str() {
        assert_eq!(
            ErrorCode::InvalidCredentials.as_str(),
            "invalid_credentials"
        );
        assert_eq!(
            ErrorCode::EmailAlreadyExists.as_str(),
            "email_already_exists"
        );
        assert_eq!(ErrorCode::RateLimitExceeded.as_str(), "rate_limit_exceeded");
        assert_eq!(ErrorCode::QuotaExceeded.as_str(), "quota_exceeded");
    }

    #[test]
    fn test_error_code_display() {
        assert_eq!(format!("{}", ErrorCode::NotFound), "not_found");
        assert_eq!(format!("{}", ErrorCode::InvalidToken), "invalid_token");
    }
}

/// Helper to create a JSON error response with a structured error code.
///
/// Maps a solobase `ErrorCode` to the appropriate wafer `ErrorCode` and
/// returns an `OutputStream::error(...)` with the error code as part of the message.
///
/// The `_msg` parameter is retained for call-site compatibility during migration;
/// it is not used in the new streaming protocol.
pub fn error_response(
    _msg: &wafer_run::Message,
    code: ErrorCode,
    message: &str,
) -> wafer_run::OutputStream {
    let wafer_code = solobase_error_code_to_wafer(code);
    let full_message = format!("[{}] {}", code.as_str(), message);
    wafer_run::OutputStream::error(wafer_run::WaferError {
        code: wafer_code,
        message: full_message,
        meta: vec![],
    })
}

/// Map a solobase `ErrorCode` to a wafer `ErrorCode`.
fn solobase_error_code_to_wafer(code: ErrorCode) -> wafer_run::ErrorCode {
    match code {
        ErrorCode::InvalidCredentials
        | ErrorCode::NotAuthenticated
        | ErrorCode::InvalidToken
        | ErrorCode::TokenExpired => wafer_run::ErrorCode::Unauthenticated,

        ErrorCode::Forbidden
        | ErrorCode::AdminRequired
        | ErrorCode::AccountDisabled
        | ErrorCode::EmailNotVerified => wafer_run::ErrorCode::PermissionDenied,

        ErrorCode::NotFound => wafer_run::ErrorCode::NotFound,

        ErrorCode::EmailAlreadyExists | ErrorCode::Conflict => wafer_run::ErrorCode::AlreadyExists,

        ErrorCode::PasswordTooShort
        | ErrorCode::PasswordTooLong
        | ErrorCode::InvalidEmail
        | ErrorCode::InvalidInput
        | ErrorCode::InvalidPurchaseStatus => wafer_run::ErrorCode::InvalidArgument,

        ErrorCode::QuotaExceeded | ErrorCode::FileTooLarge => {
            wafer_run::ErrorCode::ResourceExhausted
        }

        ErrorCode::RateLimitExceeded => wafer_run::ErrorCode::ResourceExhausted,

        ErrorCode::PaymentNotConfigured
        | ErrorCode::ConfigurationError
        | ErrorCode::DatabaseError
        | ErrorCode::InternalError
        | ErrorCode::RefundFailed => wafer_run::ErrorCode::Internal,
    }
}
