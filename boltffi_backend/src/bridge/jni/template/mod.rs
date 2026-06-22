//! Askama-backed JNI source rendering.
//!
//! The contract modules decide what exists and what each piece means. This
//! module prepares the Askama data used to print the generated C source file.
//! Rust code should build typed template views; the C syntax itself belongs in
//! templates.
//!
//! That split keeps the bridge readable. Adding a callback shape or a stream
//! helper should change the typed contract and a focused template view, not bury
//! generated C inside Rust string concatenation.

mod callback;
mod closure;
mod method;
mod source;
mod stream;

pub use self::source::SourceFile;
