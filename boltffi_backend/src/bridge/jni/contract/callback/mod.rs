//! JNI contract for Rust callback traits.
//!
//! Rust calls callback traits through C vtables. On the JVM side, those calls
//! need to become static Java method invocations with cached classes and method
//! ids, converted arguments, return handling, and async completion helpers when a
//! callback method is asynchronous.
//!
//! This module owns callback-trait dispatch. Inline closures live in the closure
//! module because they are registered by function signature and travel as
//! function pointer, context, and release triples rather than vtable slots.

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
