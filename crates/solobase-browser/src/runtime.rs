//! Service-Worker-side Wafer runtime storage and dispatch.
//!
//! `store_wafer` stashes a fully-started `Wafer` in a `thread_local` cell;
//! `dispatch_request` converts an incoming `web_sys::Request` into a WAFER
//! `Message`, dispatches it through the stored `Wafer`'s `site-main` flow,
//! and converts the output back into a `web_sys::Response`. WASM is
//! single-threaded, so the thread_local is safe without Send/Sync bounds.

use std::cell::RefCell;

use wasm_bindgen::prelude::*;

use crate::convert;

thread_local! {
    pub(crate) static RUNTIME: RefCell<Option<wafer_run::Wafer>> = const { RefCell::new(None) };
}

/// True if `store_wafer` has been called in this SW context.
pub fn is_initialized() -> bool {
    RUNTIME.with(|r| r.borrow().is_some())
}

/// Store a fully-started `Wafer` in the SW's thread_local. Subsequent
/// `dispatch_request` calls route through this Wafer.
///
/// Panics in debug if called twice. In release, silently overwrites the
/// previous value — consumers should guard with `is_initialized()` at the
/// top of their `initialize()` to make the double-call case explicit.
pub fn store_wafer(wafer: wafer_run::Wafer) {
    RUNTIME.with(|r| {
        let mut borrow = r.borrow_mut();
        debug_assert!(borrow.is_none(), "store_wafer called twice");
        *borrow = Some(wafer);
    });
}

/// Convert a browser `Request` into a WAFER `Message`, dispatch through
/// the stored `Wafer`'s `site-main` flow, and return a browser `Response`.
/// Returns a 503-shaped `Response` if called before `store_wafer`.
/// Internal errors return a 500-shaped `Response`.
pub async fn dispatch_request(request: web_sys::Request) -> Result<web_sys::Response, JsValue> {
    // SAFETY: wasm32 is single-threaded, and the RefCell value is never
    // replaced after `store_wafer()` stores it. Using a raw pointer avoids
    // holding a RefCell borrow across `.await`, which would break when
    // concurrent fetch events interleave at await points.
    let wafer_ptr = RUNTIME.with(|r| {
        let borrow = r.borrow();
        match borrow.as_ref() {
            Some(w) => Ok(w as *const wafer_run::Wafer),
            None => Err(()),
        }
    });

    let wafer_ptr = match wafer_ptr {
        Ok(p) => p,
        Err(()) => {
            return Ok(build_error_response(
                503,
                "solobase-browser: runtime not initialized — call store_wafer() first",
            )?);
        }
    };

    let (msg, input) = convert::request_to_message(&request).await?;
    let wafer = unsafe { &*wafer_ptr };
    let output = wafer.run("site-main", msg, input).await;
    convert::output_to_response(output).await
}

fn build_error_response(status: u16, body: &str) -> Result<web_sys::Response, JsValue> {
    let init = web_sys::ResponseInit::new();
    init.set_status(status);
    web_sys::Response::new_with_opt_str_and_init(Some(body), &init)
}
