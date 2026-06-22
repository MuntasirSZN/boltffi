//! Template data for JVM-owned inline closure trampolines.
//!
//! A JVM-owned closure is stored as a global Java object and called from Rust
//! through a C function pointer. The generated C source therefore needs a call
//! trampoline, release trampoline, argument setup, return conversion, and
//! optional callback-handle helpers for each registered closure signature.
//!
//! This module prepares those source views from the closure contract. The same
//! rendering path is used whether the closure appears as a function parameter, a
//! callback method parameter, a nested closure argument, or a returned closure.

mod argument;
mod callback_handle;
mod registration;

pub use argument::{
    ClosureBytesArgumentView, ClosureCParameterView, ClosureDirectVectorArgumentView,
    ClosureHandleArgumentView,
};
pub use callback_handle::CallbackClosureHandleView;
pub use registration::ClosureRegistrationView;
