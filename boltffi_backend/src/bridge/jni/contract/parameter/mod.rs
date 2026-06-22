//! Native method parameters for JNI exports.
//!
//! JNI entry points receive Java values, but the C bridge expects C ABI
//! arguments. This module models the parameter shapes that can be forwarded to a
//! C bridge function: scalars, byte slices, direct records, direct vectors,
//! continuations, and closure triples.

mod build;
mod native;

pub use native::{NativeParameter, NativeParameterKind};
