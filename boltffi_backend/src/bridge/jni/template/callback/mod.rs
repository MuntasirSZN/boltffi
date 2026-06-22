//! Template data for callback vtable glue.
//!
//! Callback contracts describe how Rust calls into the JVM. The templates need
//! the concrete C declarations, local JNI setup, Java method call arguments,
//! return conversion, cleanup, and async completion invokers for those calls.
//!
//! This module is the rendering adapter for callback glue. It keeps callback C
//! syntax out of the contract layer while still making templates consume typed
//! data instead of rebuilding callback behavior.

mod argument;
mod closure_return;
mod completion;
mod direct_vector;
mod method;
mod registration;

pub use argument::{
    CallbackBytesArgumentView, CallbackCParameterView, CallbackClosureArgumentView,
    CallbackCompletionArgumentView, CallbackHandleArgumentView, CallbackRecordArgumentView,
};
pub use closure_return::CallbackClosureReturnView;
pub use completion::CallbackCompletionInvokerView;
pub use direct_vector::CallbackDirectVectorArgumentView;
pub use method::CallbackMethodView;
pub use registration::CallbackRegistrationView;
