//! Smoke test for linkme-based block auto-registration.
//!
//! Asserts that the 11 zero-arg suppers-ai/* blocks self-register at link
//! time via register_static_block! and that LlmBlock — which has a
//! non-zero-arg constructor — does NOT auto-register (it's manually
//! registered by `solobase_core::blocks::register_llm()` from
//! `solobase::SolobaseBuilder::build()`).

// Force solobase_core's object files to be linked so that the
// register_static_block! distributed-slice entries inside each block module
// are included in the test binary.  Without this import the linker would
// strip the crate's code entirely (no other symbols referenced) and
// STATIC_BLOCK_REGISTRATIONS would be empty.
use solobase_core as _;
use wafer_run::Wafer;

#[test]
fn all_zero_arg_blocks_auto_register() {
    let w = Wafer::new().expect("Wafer::new should succeed with no lockfile present");

    // The 11 always-on zero-arg blocks. `vector` is unconditionally registered
    // (its `pub mod vector;` in blocks/mod.rs has no cfg gate). Only `fastembed`
    // is feature-gated under `native-embedding` and checked in the test below.
    // `llm` requires a non-zero-arg constructor and is checked separately.
    let always_on = [
        "suppers-ai/admin",
        "suppers-ai/auth",
        "suppers-ai/email",
        "suppers-ai/files",
        "suppers-ai/legalpages",
        "suppers-ai/messages",
        "suppers-ai/products",
        "suppers-ai/projects",
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

    // LlmBlock requires Arc<ProviderLlmService>; can't auto-register.
    // It's only present once SolobaseBuilder::build() calls register_llm().
    assert!(
        !w.has_block("suppers-ai/llm"),
        "LlmBlock should not be auto-registered (constructor needs ProviderLlmService)"
    );
}

#[cfg(feature = "native-embedding")]
#[test]
fn fastembed_block_auto_registers_when_feature_enabled() {
    let w = Wafer::new().expect("Wafer::new with native-embedding feature");
    assert!(w.has_block("suppers-ai/fastembed"));
}
