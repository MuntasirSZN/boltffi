//! JNI contract for inline closures owned by the JVM.
//!
//! Inline closures enter Rust as a C function pointer, user-data handle, and
//! release function. The callable itself lives on the JVM. The bridge therefore
//! needs a registered trampoline for each closure signature so Rust can call back
//! into the JVM without knowing anything about Java objects.
//!
//! This module owns those registered signatures. It deduplicates them across
//! functions, callback methods, nested closure arguments, and returned closures,
//! then records the JVM bridge class, cached method ids, call trampoline,
//! release trampoline, arguments, and return handling for each signature.

mod argument;
mod callback_handle;
mod names;
mod parameter;
mod registration;

pub use argument::{
    ClosureArgument, ClosureBytesArgument, ClosureCParameter, ClosureDirectVectorArgument,
    ClosureHandleArgument,
};
pub use callback_handle::CallbackClosureHandle;
pub use parameter::ClosureParameter;
pub use registration::ClosureRegistration;
