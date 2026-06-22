//! Askama-backed JNI source rendering.
//!
//! The contract modules decide what must be rendered. This module turns those
//! typed contracts into template data and lets the templates own the generated C
//! syntax.
//!
//! Keeping generated C in templates matters here because JNI glue is large and
//! mostly language syntax, not Rust logic.

mod callback;
mod closure;
mod method;
mod source;
mod stream;

pub use self::source::SourceFile;
