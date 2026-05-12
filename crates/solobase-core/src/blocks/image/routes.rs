//! HTTP routes for the image block.
//!
//! Forwards POST /b/image/api/generate to the runtime image router via the
//! typed native client. PNG bytes flow back as binary; failures map to a
//! structured JSON error envelope with an HTTP status appropriate to the
//! `ErrorCode`.

use serde::Deserialize;
use wafer_block::context::Context;
use wafer_block::WaferError;
use wafer_core::clients::image::{self as image_client, ImageParams, ImageRequest};
use wafer_run::OutputStream;

use crate::blocks::helpers::{err_bad_request, err_internal, ok_json, ResponseBuilder};
use wafer_run::InputStream;

#[derive(Debug, Deserialize)]
struct GenerateBody {
    backend_id: String,
    model: String,
    prompt: String,
    #[serde(default)]
    params: Option<ImageParams>,
}

pub async fn handle_generate(ctx: &dyn Context, input: InputStream) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let body: GenerateBody = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("invalid json: {e}")),
    };
    if body.prompt.trim().is_empty() {
        return err_bad_request("missing or empty prompt");
    }
    if body.backend_id.trim().is_empty() {
        return err_bad_request("missing backend_id");
    }
    if body.model.trim().is_empty() {
        return err_bad_request("missing model");
    }

    let req = ImageRequest {
        backend_id: body.backend_id,
        model: body.model,
        prompt: body.prompt,
        params: body.params.unwrap_or_default(),
        extra: serde_json::Value::Null,
    };

    match image_client::generate(ctx, &req).await {
        Ok(resp) => {
            let Some(img) = resp.images.into_iter().next() else {
                return err_internal("image service returned no image");
            };
            ResponseBuilder::new()
                .status(200)
                .set_header("Cache-Control", "no-store")
                .body(img.bytes, &img.mime_type)
        }
        Err(e) => wafer_err_to_http(e),
    }
}

pub async fn handle_list_models(ctx: &dyn Context) -> OutputStream {
    match image_client::list_models(ctx).await {
        Ok(models) => ok_json(&serde_json::json!({ "models": models })),
        Err(e) => wafer_err_to_http(e),
    }
}

fn wafer_err_to_http(e: WaferError) -> OutputStream {
    use wafer_block::ErrorCode;
    let status = match e.code {
        ErrorCode::InvalidArgument => 400,
        ErrorCode::NotFound => 404,
        ErrorCode::Unimplemented => 501,
        ErrorCode::Cancelled => 499,
        ErrorCode::Unavailable => 503,
        ErrorCode::PermissionDenied => 403,
        ErrorCode::Unauthenticated => 401,
        _ => 502,
    };
    let body = serde_json::to_vec(&serde_json::json!({
        "error": {
            "code": format!("{:?}", e.code),
            "message": e.message,
        }
    }))
    .unwrap_or_else(|_| Vec::new());
    ResponseBuilder::new()
        .status(status)
        .body(body, "application/json")
}
