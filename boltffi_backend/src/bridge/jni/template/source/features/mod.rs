//! Feature scan for JNI source fragments.
//!
//! JNI glue only includes runtime fragments that the generated contract uses.
//! This module reads the already-renderable template views and records whether
//! the source needs exceptions, byte arrays, callback handles, closure handles,
//! continuations, lifecycle hooks, stream helpers, or status checks.

mod callback;
mod closure;
mod completion;
mod method;
mod source;
mod stream;

pub use source::SourceFeatures;
