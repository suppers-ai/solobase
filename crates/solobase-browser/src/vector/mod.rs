//! Browser-side `VectorService` and `EmbeddingService` implementations.
//!
//! Backed by sql.js (vectors stored as BLOBs in the shared OPFS database)
//! and Transformers.js running in the page via the SW↔page bridge.

pub mod score;
pub mod sql;
pub mod service;

pub use service::BrowserVectorService;
