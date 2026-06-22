//! JNI contract for callback traits implemented on the JVM.
//!
//! Rust sees callback traits as C vtables. The JVM sees static methods on a
//! generated callback class. This module connects those views: it records the C
//! vtable slot, the JVM method descriptor, cached method ids, argument
//! conversion, return conversion, and async completion hooks for each callback
//! method.
//!
//! Callback traits are different from inline closures. A callback trait is a
//! named protocol with vtable slots. An inline closure is registered by function
//! signature and travels as call, context, and release values. Keeping those two
//! contracts separate prevents one path from guessing the ownership rules of the
//! other.

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
