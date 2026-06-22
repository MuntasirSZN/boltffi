//! JNI contract for inline closures.
//!
//! Inline closures cross the bridge as C function pointers plus user data and a
//! release callback. The JNI bridge registers each closure signature once, keeps
//! the JVM method ids for that signature, and emits the call/release trampolines
//! Rust will invoke.
//!
//! This is separate from callback traits because closures are identified by
//! signature and can appear nested inside callback arguments or closure returns.

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
