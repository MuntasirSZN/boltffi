//! JNI contract for Rust callback traits.
//!
//! A callback trait is represented in C as a vtable. The JNI bridge turns that
//! vtable into JVM method dispatch: cached callback classes, vtable slot
//! functions, argument conversion, async completion helpers, and returned handle
//! construction.
//!
//! This module owns the callback-side contract only. Inline closures have their
//! own module because they use function pointers and registration by signature.

mod argument;
mod bytes;
mod c_parameter;
mod closure;
mod completion;
mod direct_vector;
mod handle;
mod method;
mod parameter;
mod record;
mod registration;
mod return_value;

pub use argument::CallbackArgument;
pub use bytes::CallbackBytesArgument;
pub use c_parameter::CallbackCParameter;
pub use closure::CallbackClosureArgument;
pub use completion::{
    CallbackCompletionArgument, CallbackCompletionInvoker, CallbackCompletionPayload,
};
pub use direct_vector::CallbackDirectVectorArgument;
pub use handle::CallbackHandleArgument;
pub use method::{CallbackClosureReturn, CallbackMethod};
pub use parameter::CallbackParameter;
pub use record::CallbackRecordArgument;
pub use registration::CallbackRegistration;
pub use return_value::CallbackReturn;
