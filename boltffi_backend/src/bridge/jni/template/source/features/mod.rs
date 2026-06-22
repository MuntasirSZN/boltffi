//! Source-fragment selection for generated JNI glue.
//!
//! The generated C file should include only the support code required by the
//! contract. A crate without streams does not need stream helpers. A crate
//! without closure returns does not need closure-handle return helpers.
//!
//! This module scans template views, not the binding IR, and records which
//! fragments the root source template must include: exceptions, byte arrays,
//! callback handles, closure handles, continuations, lifecycle hooks, stream
//! helpers, direct records, or status checks.

mod callback;
mod closure;
mod completion;
mod method;
mod source;
mod stream;

pub use source::SourceFeatures;
