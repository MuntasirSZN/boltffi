//! JNI contract for inline closures.
//!
//! Inline closures are not callback traits. Rust calls them through a C function
//! pointer with user data and a release callback. The JVM owns the real callable,
//! so the bridge needs a small registered trampoline for each closure signature.
//!
//! This module owns those registered signatures. It deduplicates them across
//! functions, callbacks, and nested closure arguments, then records the JVM
//! bridge class, cached method ids, call trampoline, release trampoline, and
//! return handling for each signature.

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
