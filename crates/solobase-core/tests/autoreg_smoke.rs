//! Smoke test for linkme-based block auto-registration.
//!
//! Asserts that zero-arg `suppers-ai/*` blocks self-register at link time
//! via `register_static_block!` and that blocks whose constructors take
//! arguments (`LlmBlock` / framework `AuthBlock`) do NOT auto-register —
//! they're installed by helpers (`register_llm` / `register_auth`) called
//! from `solobase::SolobaseBuilder::build()`.

// Force solobase_core's object files to be linked so that the
// register_static_block! distributed-slice entries inside each block module
// are included in the test binary.  Without this import the linker would
// strip the crate's code entirely (no other symbols referenced) and
// STATIC_BLOCK_REGISTRATIONS would be empty.
use std::sync::Arc;

use solobase_core as _;
use wafer_run::{StaticConfigSource, Wafer};

#[test]
fn all_zero_arg_blocks_auto_register() {
    let w = Wafer::new(Arc::new(StaticConfigSource::default()))
        .expect("Wafer::new should succeed with no lockfile present");

    // Zero-arg blocks. `vector` is unconditionally registered (its
    // `pub mod vector;` in blocks/mod.rs has no cfg gate). `fastembed` is
    // feature-gated under `native-embedding` and checked in the test below.
    // `llm` requires `Arc<dyn ProviderAdmin>` and `auth` (framework) wraps
    // `Arc<dyn AuthService>`; both are checked separately.
    let always_on = [
        "suppers-ai/admin",
        "suppers-ai/auth-ui",
        "suppers-ai/email",
        "suppers-ai/files",
        "suppers-ai/legalpages",
        "suppers-ai/messages",
        "suppers-ai/products",
        "suppers-ai/system",
        "suppers-ai/userportal",
        "suppers-ai/vector",
    ];

    for name in always_on {
        assert!(
            w.has_block(name),
            "expected block {name} to be auto-registered via register_static_block!"
        );
    }

    // LlmBlock requires Arc<dyn ProviderAdmin>; can't auto-register.
    // It's only present once SolobaseBuilder::build() calls register_llm().
    assert!(
        !w.has_block("suppers-ai/llm"),
        "LlmBlock should not be auto-registered (constructor needs ProviderAdmin)"
    );

    // The framework AuthBlock wraps `Arc<dyn AuthService>`; can't
    // auto-register. It's only present once SolobaseBuilder::build() calls
    // `crate::blocks::register_auth()`.
    assert!(
        !w.has_block("suppers-ai/auth"),
        "framework AuthBlock should not be auto-registered (constructor needs Arc<dyn AuthService>)"
    );
}

#[cfg(feature = "native-embedding")]
#[test]
fn fastembed_block_auto_registers_when_feature_enabled() {
    let w = Wafer::new(Arc::new(StaticConfigSource::default()))
        .expect("Wafer::new with native-embedding feature");
    assert!(w.has_block("suppers-ai/fastembed"));
}
