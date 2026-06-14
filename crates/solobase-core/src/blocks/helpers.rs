//! TRANSITIONAL re-export shim — being removed within this PR.
//!
//! The generic HTTP/response helpers moved to [`crate::http`] and the
//! time/encoding/record/form helpers moved to [`crate::util`], so the
//! dependency direction reads infra → util and blocks → util instead of
//! infra reaching down into `blocks/` for them. This shim re-exports both
//! so the per-module call-site migration stays bisectable; the final commit
//! deletes it (no-compat-shims rule).

pub use crate::{
    http::{
        err_bad_request, err_conflict, err_forbidden, err_internal, err_internal_no_cause,
        err_not_found, err_unauthorized, ok_empty, ok_json, redirect, ResponseBuilder,
    },
    util::{
        block_request, field_as_string, forward_auth_meta, hex_encode, is_admin, json_as_i64,
        json_as_u64, json_map, now_millis, now_rfc3339, parse_body_value, parse_form_body,
        path_param, sha256, sha256_hex, stamp_created, stamp_updated, url_path_encode, urlencode,
        RecordExt,
    },
};
