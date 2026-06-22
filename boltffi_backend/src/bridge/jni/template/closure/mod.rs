//! Template data for inline closure trampolines.
//!
//! A JVM-owned closure is stored as a global Java object and called from Rust
//! through a C function pointer. The generated source therefore needs a call
//! trampoline, release trampoline, argument setup, return conversion, and
//! optional callback-handle helpers for each registered closure signature.
//!
//! This module prepares those template views from the closure contract. It keeps
//! closure rendering reusable across functions, callback methods, nested
//! closures, and returned closures.

mod argument;
mod callback_handle;
mod registration;

pub use argument::{
    ClosureBytesArgumentView, ClosureCParameterView, ClosureDirectVectorArgumentView,
    ClosureHandleArgumentView,
};
pub use callback_handle::CallbackClosureHandleView;
pub use registration::ClosureRegistrationView;
