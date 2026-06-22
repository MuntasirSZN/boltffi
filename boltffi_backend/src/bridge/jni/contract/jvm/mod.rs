//! JVM method contracts shared by callbacks and closures.
//!
//! The generated C code often calls static JVM methods and then translates the
//! result back into the C ABI expected by Rust. This module owns the return-side
//! contract for those calls.

mod method_return;

pub use method_return::JvmMethodReturn;
