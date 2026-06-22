//! Template data for inline closure trampolines.
//!
//! A foreign JVM closure is stored as a global Java object plus cached method
//! ids. Rust calls it through a C function pointer, so the generated JNI source
//! needs call, release, and optional callback-handle helpers for each closure
//! signature.

mod argument;
mod callback_handle;
mod registration;

pub use argument::{
    ClosureBytesArgumentView, ClosureCParameterView, ClosureDirectVectorArgumentView,
    ClosureHandleArgumentView,
};
pub use callback_handle::CallbackClosureHandleView;
pub use registration::ClosureRegistrationView;
