//! Top-level JNI C source file rendering.
//!
//! This module decides which runtime fragments are needed for a bridge contract
//! and feeds the root `source.c` template. The fragments stay separate so adding
//! callbacks, closures, streams, or lifecycle hooks does not turn one template
//! into a monolith.

mod features;
mod file;

pub use file::SourceFile;
